// SPDX-License-Identifier: GPL-2.0-only
//! Ciclos de cabeceras en el panel Museo (eje Elegancia): la ficha E-003 que ensena que es,
//! y cada ciclo con su vuelta, el motivo de cada paso y donde cortar.
//!
//! La animacion: la onda da vueltas sin salida (como el `#include` que vuelve a empezar),
//! y el paso donde cortar late en rojo con su marca.

use crate::Colores;
use carpetas_nucleo::ciclos::{Ciclo, Ciclos};
use carpetas_nucleo::{Accion, AccionGrupo};
use eframe::egui::{self, epaint::QuadraticBezierShape, Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, StrokeKind, Vec2};

/// Cuantos ciclos se ensenan (el resto se cuenta).
const VISTOS: usize = 12;

fn corto(ruta: &str, max: usize) -> String {
    let n = ruta.rsplit('/').next().unwrap_or(ruta);
    if n.chars().count() <= max { n.to_string() } else { format!("{}…", n.chars().take(max - 1).collect::<String>()) }
}

pub fn seccion(ui: &mut egui::Ui, c: &Colores, ciclos: &Ciclos, acciones: &mut Vec<Accion>) {
    ui.add_space(6.0);
    ui.label(RichText::new(format!("CICLOS DE CABECERAS · {} ({} archivos)", ciclos.total, ciclos.archivos)).color(c.acento).strong().size(12.0));
    if ciclos.condicionales > 0 {
        ui.label(RichText::new(format!("{} dependen de un #if: solo existen al compilar con esa condicion (por ejemplo, para Windows). Van al final.", ciclos.condicionales))
            .color(c.tenue).size(11.0));
    }
    if let Some(f) = carpetas_nucleo::museo::ficha("E-003") {
        egui::CollapsingHeader::new(RichText::new(format!("E-003 · {} (que es y por que importa)", f.titulo)).color(c.acento).size(11.5))
            .id_salt("ficha-e003").default_open(ciclos.total == 0).show(ui, |ui| {
                ui.label(RichText::new(f.regla).color(c.texto).size(11.5));
                ui.label(RichText::new("Control valido").color(c.verde).size(11.0));
                ui.label(RichText::new(f.control).monospace().color(Color32::WHITE).size(11.5));
                ui.label(RichText::new(format!("Limite: {}", f.limite)).color(c.tenue).size(11.0));
                ui.hyperlink_to(RichText::new(f.referencia).size(11.0), f.url);
            });
    }
    if ciclos.total == 0 {
        ui.label(RichText::new("Ningun ciclo entre archivos de C, C++ y ASM.").color(c.verde).size(12.0));
        return;
    }
    for (i, ciclo) in ciclos.lista.iter().take(VISTOS).enumerate() {
        let titulo = format!("{} archivos · vuelta de {} pasos · {} aristas dentro{}", ciclo.archivos.len(), ciclo.vuelta.len(), ciclo.aristas,
                             if ciclo.condicional { " · depende de un #if" } else { "" });
        egui::CollapsingHeader::new(RichText::new(titulo).color(c.texto).size(11.5)).id_salt(("ciclo", i)).default_open(i == 0).show(ui, |ui| {
            un_ciclo(ui, c, ciclo, acciones);
        });
    }
    if ciclos.total as usize > VISTOS { ui.label(RichText::new(format!("+{} ciclos mas (los mas grandes primero)", ciclos.total as usize - VISTOS)).color(c.tenue).size(11.0)); }
}

fn un_ciclo(ui: &mut egui::Ui, c: &Colores, ciclo: &Ciclo, acciones: &mut Vec<Accion>) {
    anillo(ui, c, ciclo);
    for (k, e) in ciclo.vuelta.iter().enumerate() {
        let cortar = ciclo.corte == Some(k);
        ui.horizontal_wrapped(|ui| {
            let donde = match &e.motivo { Some(m) => format!("{}:{}", corto(&e.desde, 30), m.linea), None => corto(&e.desde, 30) };
            let r = ui.add(egui::Label::new(RichText::new(donde).monospace().color(Color32::WHITE).size(11.5)).wrap().sense(Sense::click()))
                .on_hover_text(format!("{}\nclic: abrir el editor en esa linea", e.desde));
            if r.clicked() {
                acciones.push(match &e.motivo { Some(m) => Accion::IrALinea { id: e.desde.clone(), linea: m.linea }, None => Accion::Revelar { id: e.desde.clone(), editor: true } });
            }
            let escrito = e.motivo.as_ref().map(|m| {
                let entero = m.escrito(&e.desde);
                if entero.chars().count() > 40 && m.texto.contains('/') { entero.replace(&m.texto, &format!("…/{}", corto(&m.texto, 30))) } else { entero }
            }).unwrap_or_else(|| format!("usa {}", corto(&e.hasta, 30)));
            ui.add(egui::Label::new(RichText::new(escrito).monospace().color(c.aviso).size(11.5)).wrap()).on_hover_text(&e.hasta);
            if let Some(cond) = e.motivo.as_ref().and_then(|m| m.condicion.as_ref()) {
                ui.label(RichText::new(format!("solo si {cond}")).color(c.verde).size(11.0)).on_hover_text("Este #include esta dentro de un condicional: con otra plataforma u otras opciones no existe.");
            }
            if cortar { ui.label(RichText::new(format!("cortar aqui: lo usan {}", e.lo_usan)).color(c.error).size(11.0)); }
            else { ui.label(RichText::new(format!("· {} usan {}", e.lo_usan, corto(&e.hasta, 24))).color(c.tenue).size(11.0)); }
        });
    }
    if let Some(cond) = ciclo.vuelta.iter().find_map(|e| e.motivo.as_ref().and_then(|m| m.condicion.clone())) {
        ui.label(RichText::new(format!("Solo existe {cond}. Suele ser a proposito: la parte de una plataforma al final de la cabecera. \
                                        Si no compilas con esa condicion, este ciclo no te afecta; si si, cortarlo cuesta lo que dice la lista.", cond = cond))
            .color(c.verde).size(11.0));
    } else if let Some(e) = ciclo.corte.map(|k| &ciclo.vuelta[k]) {
        ui.label(RichText::new(format!("Para romper la vuelta: {} incluye {}, que solo usan {} archivos. Si solo hace falta un puntero, \
                                        basta declarar el tipo (struct x;) en vez de incluirlo. Comprobarlo compilando.",
                                       corto(&e.desde, 30), corto(&e.hasta, 30), e.lo_usan)).color(c.error).size(11.0));
    }
    ui.horizontal(|ui| {
        if ui.small_button("Ver en Combinar").on_hover_text("Mete la vuelta en el grupo: el panel Combinar y la Ciudad la dibujan").clicked() {
            acciones.push(Accion::Grupo(AccionGrupo::Limpiar));
            acciones.push(Accion::Grupo(AccionGrupo::Sumar(ciclo.vuelta.iter().map(|e| e.desde.clone()).collect())));
            if ciclo.vuelta.len() >= 2 { acciones.push(Accion::Grupo(AccionGrupo::Extremos(ciclo.vuelta[0].desde.clone(), ciclo.vuelta[1].desde.clone()))); }
            acciones.push(Accion::Lateral(carpetas_nucleo::estado::Lateral::Combinar));
        }
        if ciclo.archivos.len() > ciclo.vuelta.len() {
            ui.label(RichText::new(format!("la componente tiene {} archivos; esta es su vuelta mas corta", ciclo.archivos.len())).color(c.tenue).size(10.5));
        }
    });
}

/// La vuelta en un anillo: flechas de quien incluye a quien y la onda dando vueltas sin salida.
fn anillo(ui: &mut egui::Ui, c: &Colores, ciclo: &Ciclo) {
    let n = ciclo.vuelta.len();
    if n == 0 { return; }
    let ancho = ui.available_width().max(200.0);
    let (caja, _) = ui.allocate_exact_size(Vec2::new(ancho, 150.0), Sense::hover());
    let p = ui.painter_at(caja);
    p.rect(caja, 6, c.fondo, Stroke::new(1.0, c.linea), StrokeKind::Inside);
    let centro = caja.center();
    let (rx, ry) = ((ancho / 2.0 - 60.0).max(30.0), 48.0);
    // Mas de 16 en la vuelta: los puntos sin nombre (el nombre, en la lista de abajo).
    let nombres = n <= 16;
    let pos: Vec<Pos2> = (0..n).map(|i| {
        // Con dos, uno a cada lado (los arcos se curvan cada uno por su lado).
        let a = if n == 2 { std::f32::consts::PI * (1.0 - i as f32) } else { -std::f32::consts::FRAC_PI_2 + i as f32 / n as f32 * std::f32::consts::TAU };
        centro + Vec2::new(a.cos() * rx, a.sin() * ry)
    }).collect();
    let ahora = ui.input(|i| i.time);
    // La onda: recorre la vuelta entera y vuelve a empezar, sin salida.
    let fase = ((ahora * 0.9) % n as f64) as f32;
    for i in 0..n {
        let (a, b) = (pos[i], pos[(i + 1) % n]);
        let medio = a + (b - a) / 2.0;
        // Hacia fuera del anillo, y siempre algo a un lado: con dos, la ida y la vuelta no se pisan.
        let lado_ab = Vec2::new(-(b - a).y, (b - a).x).normalized() * ((b - a).length() * 0.22).min(44.0);
        let ctl = medio + (medio - centro) * 0.35 + lado_ab;
        let cortar = ciclo.corte == Some(i);
        let latido = if cortar { 0.6 + 0.4 * ((ahora * 4.0).sin() as f32).abs() } else { 1.0 };
        let col = if cortar { c.error } else { c.acento };
        p.add(QuadraticBezierShape::from_points_stroke([a, ctl, b], false, Color32::TRANSPARENT, Stroke::new(6.0, col.gamma_multiply(0.15 * latido))));
        p.add(QuadraticBezierShape::from_points_stroke([a, ctl, b], false, Color32::TRANSPARENT, Stroke::new(1.8, col.gamma_multiply(latido))));
        let en = |s: f32| { let u = 1.0 - s; (a.to_vec2() * u * u + ctl.to_vec2() * 2.0 * u * s + b.to_vec2() * s * s).to_pos2() };
        // Flecha cerca del incluido (fuera de su nombre).
        let punta = en(if nombres { 0.80 } else { 0.9 });
        let dir = (en(0.84) - en(0.76)).normalized();
        let lado = Vec2::new(-dir.y, dir.x);
        p.add(egui::Shape::convex_polygon(vec![punta, punta - dir * 8.0 + lado * 4.0, punta - dir * 8.0 - lado * 4.0], col.gamma_multiply(latido), Stroke::NONE));
        if cortar {
            let m = en(0.5);
            for k in [-3.0, 3.0] { p.line_segment([m + dir * k - lado * 7.0, m + dir * k + lado * 7.0], Stroke::new(1.6, c.error)); }
        }
        if (fase - i as f32) >= 0.0 && (fase - i as f32) < 1.0 {
            let q = en(fase - i as f32);
            for (r, al) in [(8.0, 0.12), (5.0, 0.3), (3.0, 1.0)] { p.circle_filled(q, r, col.gamma_multiply(al)); }
            p.circle_filled(q, 1.4, Color32::WHITE);
        }
    }
    for (i, e) in ciclo.vuelta.iter().enumerate() {
        if nombres {
            let g = p.layout_no_wrap(corto(&e.desde, 16), FontId::proportional(10.5), Color32::WHITE);
            let r = egui::Rect::from_center_size(pos[i], g.size() + Vec2::new(10.0, 6.0));
            p.rect(r, 4, c.superficie, Stroke::new(1.2, c.acento), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, corto(&e.desde, 16), FontId::proportional(10.5), Color32::WHITE);
        } else {
            p.circle(pos[i], 3.5, c.acento, Stroke::NONE);
        }
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
}
