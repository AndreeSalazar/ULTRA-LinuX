// SPDX-License-Identifier: GPL-2.0-only
//! Panel «Combinar» del SUPER GRAFO: la seleccion grupal (Ctrl+clic o Shift+arrastrar en
//! la Ciudad, o la banda del grafo) dibujada como nodos con sus cables, en vivo.
//!
//! - Cable blanco con flecha: uno del grupo usa a otro.
//! - Cable del acento hacia abajo: lo que comparten (un archivo de fuera que usan dos o mas).
//! - Cable verde hacia arriba: quien los junta (un archivo de fuera que usa a dos o mas).
//! - Cada miembro lleva su triangulo (Seguridad, Rendimiento, Elegancia) y el borde rojo
//!   si tiene algo de Seguridad.

use crate::{App, Colores};
use carpetas_nucleo::combinar::{combinar, Combinacion};
use carpetas_nucleo::organizar::es_grupo;
use carpetas_nucleo::{Accion, AccionGrupo, Estado};
use eframe::egui::{self, epaint::CubicBezierShape, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

/// La combinacion del grupo actual; se rehace solo si cambia el grupo o llega otro analisis.
#[derive(Default)]
pub struct Cache {
    clave: Option<(Vec<String>, u64)>,
    pub c: Combinacion,
    /// Los ejes del triangulo de cada miembro (sumados si es una carpeta).
    pub ejes: Vec<[u32; 3]>,
    /// El grupo viene de la seleccion del grafo (no de la Ciudad).
    pub del_grafo: bool,
    /// Lo que tardo la ultima combinacion (con los ejes).
    pub micros: u64,
}

/// Lo que se combina: el grupo de la Ciudad; si esta vacio, la seleccion del grafo.
fn elegidos(e: &Estado) -> (Vec<String>, bool) {
    if !e.grupo.is_empty() { return (e.grupo.clone(), false); }
    (e.seleccion.iter().filter(|id| !es_grupo(id)).cloned().collect(), true)
}

impl Cache {
    pub fn al_dia(&mut self, e: &Estado) {
        let (grupo, del_grafo) = elegidos(e);
        let clave = (grupo, e.analisis.version);
        if self.clave.as_ref() == Some(&clave) { return; }
        let t = std::time::Instant::now();
        self.c = combinar(&e.analisis.dep, &clave.0);
        self.ejes = self.c.miembros.iter().map(|m| ejes_de(e, &m.id)).collect();
        self.del_grafo = del_grafo;
        self.clave = Some(clave);
        self.micros = t.elapsed().as_micros() as u64;
    }
}

fn ejes_de(e: &Estado, id: &str) -> [u32; 3] {
    let an = &e.analisis;
    if an.sospechas.contains_key(id) { return an.conteo_ejes(id); }
    let prefijo = format!("{id}/");
    let mut t = [0; 3];
    for (k, _) in an.sospechas.range(prefijo.clone()..).take_while(|(k, _)| k.starts_with(&prefijo)) {
        let x = an.conteo_ejes(k);
        for i in 0..3 { t[i] += x[i]; }
    }
    t
}

/// Nombre corto para un nodo: el final de la ruta, recortado.
fn corto(ruta: &str, max: usize) -> String {
    let n = ruta.rsplit('/').next().unwrap_or(ruta);
    if n.chars().count() <= max { n.to_string() } else { format!("{}…", n.chars().take(max - 1).collect::<String>()) }
}

/// Desde cuantos miembros se dibujan como puntos.
const DENSO: usize = 12;
/// Cuantos puntos llevan nombre cuando son muchos.
const ROTULOS: usize = 10;

/// Que hay bajo el raton en el lienzo.
#[derive(Clone, Copy, PartialEq)]
enum Bajo { Miembro(usize), Comparte(usize), Junta(usize) }

impl App {
    pub(crate) fn panel_combinar(&mut self, ui: &mut egui::Ui, c: &Colores) {
        self.combinar.al_dia(&self.estado);
        let mut acciones = Vec::new();
        let k = &self.combinar;
        let cb = &k.c;
        let e = &self.estado;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("SUPER GRAFO // COMBINAR").color(c.acento).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !e.grupo.is_empty() && ui.small_button("Limpiar grupo").on_hover_text("Vaciar la seleccion grupal de la Ciudad").clicked() {
                    acciones.push(Accion::Grupo(AccionGrupo::Limpiar));
                }
            });
        });
        ui.label(RichText::new("Ctrl+clic en edificios de la Ciudad (o Shift+arrastrar) para juntar varios; en el grafo vale la seleccion con la banda. Se rehace al momento.")
            .color(c.tenue).size(11.0));
        if !e.analisis.listo {
            ui.add_space(8.0);
            ui.label(RichText::new("esperando el analisis de dependencias (carril Museo)…").color(c.tenue));
            for a in acciones { self.despachar(a); }
            return;
        }
        let n = cb.miembros.len();
        ui.add_space(4.0);
        let origen = if k.del_grafo { "seleccion del grafo" } else { "grupo de la Ciudad" };
        ui.label(RichText::new(format!("{n} {} ({origen}) · {} directas · comparten {} · los juntan {} · {} {} · {:.2} ms",
                                       if n == 1 { "miembro" } else { "miembros" }, cb.directas.len(), cb.total_comparten, cb.total_juntan, cb.piezas,
                                       if cb.piezas == 1 { "pieza" } else { "piezas" }, k.micros as f64 / 1000.0)).color(c.texto).size(12.0));
        if n < 2 {
            ui.add_space(10.0);
            ui.label(RichText::new(if n == 0 { "Elige al menos dos para ver como se combinan." } else { "Falta uno mas: Ctrl+clic en otro edificio." }).color(c.tenue));
        }
        if n > 0 { self::lienzo(ui, c, k, &mut acciones); }

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let fila = |ui: &mut egui::Ui, ruta: &str, detalle: String, col: Color32, acciones: &mut Vec<Accion>| {
                ui.horizontal(|ui| {
                    let r = ui.add(egui::Label::new(RichText::new(ruta).monospace().color(col).size(12.0)).sense(Sense::click()))
                        .on_hover_text("clic: mostrar (la Ciudad lleva la camara) · Ctrl+clic: meter o sacar del grupo");
                    if r.clicked() {
                        if ui.input(|i| i.modifiers.command) { acciones.push(Accion::Grupo(AccionGrupo::Alternar(ruta.to_string()))); }
                        else { acciones.push(Accion::Revelar { id: ruta.to_string(), editor: false }); }
                    }
                    ui.label(RichText::new(detalle).color(c.tenue).size(11.0));
                });
            };
            if !cb.directas.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("DIRECTAS (a usa b)").color(c.texto).strong().size(12.0));
                for &(a, b, veces) in &cb.directas {
                    fila(ui, &cb.miembros[a].id, format!("usa {}{}", cb.miembros[b].id, if veces > 1 { format!(" ({veces} aristas)") } else { String::new() }), c.texto, &mut acciones);
                }
            }
            if !cb.comparten.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("LO QUE COMPARTEN · {}", cb.total_comparten)).color(c.acento).strong().size(12.0));
                for p in &cb.comparten { fila(ui, &p.ruta, format!("lo usan {} del grupo", p.miembros.len()), c.acento, &mut acciones); }
            }
            if !cb.juntan.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("QUIEN LOS JUNTA · {}", cb.total_juntan)).color(c.verde).strong().size(12.0));
                for p in &cb.juntan { fila(ui, &p.ruta, format!("usa a {} del grupo", p.miembros.len()), c.verde, &mut acciones); }
            }
            if !cb.sueltos.is_empty() && n >= 2 {
                ui.add_space(6.0);
                ui.label(RichText::new("SUELTOS (no se tocan con ningun otro del grupo)").color(c.tenue).strong().size(12.0));
                for &i in &cb.sueltos { fila(ui, &cb.miembros[i].id, String::new(), c.tenue, &mut acciones); }
            }
            if !cb.sin_datos.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("SIN DEPENDENCIAS CONOCIDAS").color(c.tenue).strong().size(12.0));
                ui.label(RichText::new("No es C, C++, ASM, Rust ni Python, o nada lo usa ni usa nada.").color(c.tenue).size(11.0));
                for id in &cb.sin_datos { fila(ui, id, String::new(), c.tenue, &mut acciones); }
            }
            if cb.recortado { ui.label(RichText::new(format!("solo los primeros {}", carpetas_nucleo::combinar::MAX_GRUPO)).color(c.aviso).size(11.0)); }
            ui.add_space(6.0);
            ui.label(RichText::new("Analisis lexico de #include, use, import y %include: lo que decide el preprocesador no se ve.").color(c.tenue).size(10.5));
        });
        for a in acciones { self.despachar(a); }
    }
}

/// El dibujo de nodos: los miembros en un anillo, lo compartido abajo y quien los junta arriba.
fn lienzo(ui: &mut egui::Ui, c: &Colores, k: &Cache, acciones: &mut Vec<Accion>) {
    let cb = &k.c;
    let ancho = ui.available_width().max(200.0);
    let (caja, resp) = ui.allocate_exact_size(Vec2::new(ancho, 360.0), Sense::click());
    let p = ui.painter_at(caja);
    p.rect(caja, 6, c.fondo, Stroke::new(1.0, c.linea), StrokeKind::Inside);
    let n = cb.miembros.len();
    let fila_arriba = caja.top() + 26.0;
    let fila_abajo = caja.bottom() - 26.0;
    let centro = Pos2::new(caja.center().x, (fila_arriba + fila_abajo) / 2.0);
    let (rx, ry) = ((ancho / 2.0 - 70.0).max(30.0), (fila_abajo - fila_arriba) / 2.0 - 52.0);
    let pos_m: Vec<Pos2> = (0..n).map(|i| {
        if n == 1 { return centro; }
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 / n as f32 * std::f32::consts::TAU;
        centro + Vec2::new(a.cos() * rx, a.sin() * ry.max(20.0))
    }).collect();
    let en_fila = |cuantos: usize, i: usize, y: f32| Pos2::new(caja.left() + ancho * (i as f32 + 0.5) / cuantos as f32, y);
    let pos_c: Vec<Pos2> = (0..cb.comparten.len()).map(|i| en_fila(cb.comparten.len(), i, fila_abajo)).collect();
    let pos_j: Vec<Pos2> = (0..cb.juntan.len()).map(|i| en_fila(cb.juntan.len(), i, fila_arriba)).collect();
    if !pos_j.is_empty() { p.text(Pos2::new(caja.left() + 6.0, caja.top() + 4.0), Align2::LEFT_TOP, "los juntan", FontId::proportional(10.0), c.verde.gamma_multiply(0.8)); }
    if !pos_c.is_empty() { p.text(Pos2::new(caja.left() + 6.0, caja.bottom() - 4.0), Align2::LEFT_BOTTOM, "comparten", FontId::proportional(10.0), c.acento.gamma_multiply(0.8)); }

    // Cajas de los nodos (para el raton y para cortar los cables en su borde).
    let medida = |t: &str, tam: f32| p.layout_no_wrap(t.to_string(), FontId::proportional(tam), Color32::WHITE).size() + Vec2::new(14.0, 8.0);
    // Con muchos, cada miembro es un punto y solo se rotulan los mas unidos (y el de debajo
    // del raton); con pocos, una pastilla con su nombre.
    let denso = n > DENSO;
    let caja_m: Vec<Rect> = (0..n).map(|i| if denso { Rect::from_center_size(pos_m[i], Vec2::splat(13.0)) }
                                           else { Rect::from_center_size(pos_m[i], medida(&corto(&cb.miembros[i].id, 18), 12.0)) }).collect();
    let caja_c: Vec<Rect> = cb.comparten.iter().enumerate().map(|(i, x)| Rect::from_center_size(pos_c[i], medida(&corto(&x.ruta, 12), 10.5))).collect();
    let caja_j: Vec<Rect> = cb.juntan.iter().enumerate().map(|(i, x)| Rect::from_center_size(pos_j[i], medida(&corto(&x.ruta, 12), 10.5))).collect();
    let raton = resp.hover_pos();
    let bajo = raton.and_then(|q| {
        caja_m.iter().position(|r| r.contains(q)).map(Bajo::Miembro)
            .or_else(|| caja_c.iter().position(|r| r.contains(q)).map(Bajo::Comparte))
            .or_else(|| caja_j.iter().position(|r| r.contains(q)).map(Bajo::Junta))
    });
    // Con el raton encima, sus cables se encienden y el resto se apaga.
    let toca = |m: usize| match bajo { None => true, Some(Bajo::Miembro(x)) => x == m, _ => false };
    let alfa = |vivo: bool| if bajo.is_none() { 0.75 } else if vivo { 1.0 } else { 0.12 };

    // Puentes: curvas suaves desde cada miembro.
    let curva = |a: Pos2, b: Pos2, col: Color32, w: f32| {
        let dy = (b.y - a.y) * 0.5;
        p.add(CubicBezierShape::from_points_stroke([a, a + Vec2::new(0.0, dy), b - Vec2::new(0.0, dy), b], false, Color32::TRANSPARENT, Stroke::new(w, col)));
    };
    for (i, x) in cb.comparten.iter().enumerate() {
        for &m in &x.miembros {
            let vivo = toca(m) || bajo == Some(Bajo::Comparte(i));
            curva(caja_m[m].center_bottom(), caja_c[i].center_top(), c.acento.gamma_multiply(alfa(vivo) * 0.8), 1.2);
        }
    }
    for (i, x) in cb.juntan.iter().enumerate() {
        for &m in &x.miembros {
            let vivo = toca(m) || bajo == Some(Bajo::Junta(i));
            curva(caja_m[m].center_top(), caja_j[i].center_bottom(), c.verde.gamma_multiply(alfa(vivo) * 0.8), 1.2);
        }
    }
    // Directas: arco hacia el centro del anillo, con flecha en el que es usado.
    for &(a, b, veces) in &cb.directas {
        let vivo = toca(a) || toca(b) && bajo.is_some();
        let col = Color32::WHITE.gamma_multiply(alfa(vivo));
        let (pa, pb) = (pos_m[a], pos_m[b]);
        let medio = pa + (pb - pa) / 2.0;
        let hacia = (centro - medio) * 0.35;
        let (c1, c2) = (pa + (pb - pa) * 0.25 + hacia, pa + (pb - pa) * 0.75 + hacia);
        let w = 1.4 + (veces as f32).ln_1p();
        if c.brillo { p.add(CubicBezierShape::from_points_stroke([pa, c1, c2, pb], false, Color32::TRANSPARENT, Stroke::new(w + 4.0, c.acento.gamma_multiply(alfa(vivo) * 0.25)))); }
        p.add(CubicBezierShape::from_points_stroke([pa, c1, c2, pb], false, Color32::TRANSPARENT, Stroke::new(w, col)));
        // Flecha donde la curva entra en la caja del usado.
        let curva_en = |t: f32| { let u = 1.0 - t; pa.to_vec2() * u * u * u + c1.to_vec2() * 3.0 * u * u * t + c2.to_vec2() * 3.0 * u * t * t + pb.to_vec2() * t * t * t };
        let mut t = 1.0;
        while t > 0.5 && caja_m[b].expand(2.0).contains(curva_en(t).to_pos2()) { t -= 0.02; }
        let punta = curva_en(t).to_pos2();
        let dir = (curva_en(t) - curva_en(t - 0.03)).normalized();
        let lado = Vec2::new(-dir.y, dir.x);
        p.add(egui::Shape::convex_polygon(vec![punta, punta - dir * 9.0 + lado * 4.5, punta - dir * 9.0 - lado * 4.5], col, Stroke::NONE));
    }
    // Nodos. Los rotulados cuando son muchos: los mas unidos y los que tienen Seguridad.
    let rotular: Vec<usize> = if denso {
        let mut v: Vec<usize> = (0..n).collect();
        v.sort_by_key(|i| (std::cmp::Reverse(k.ejes.get(*i).map(|x| x[0] > 0).unwrap_or(false)), std::cmp::Reverse(cb.miembros[*i].lazos)));
        v.truncate(ROTULOS);
        v
    } else { Vec::new() };
    let mut rotulos: Vec<Rect> = Vec::new();
    let pastilla = |r: Rect, texto: &str, tam: f32, borde: Color32, relleno: Color32, txt: Color32| {
        p.rect(r, 5, relleno, Stroke::new(1.3, borde), StrokeKind::Inside);
        p.text(r.center(), Align2::CENTER_CENTER, texto, FontId::proportional(tam), txt);
    };
    for (i, x) in cb.comparten.iter().enumerate() {
        let vivo = bajo == Some(Bajo::Comparte(i)) || x.miembros.iter().any(|m| bajo == Some(Bajo::Miembro(*m)));
        pastilla(caja_c[i], &corto(&x.ruta, 12), 10.5, c.acento.gamma_multiply(alfa(vivo)), c.panel, c.texto.gamma_multiply(alfa(vivo).max(0.4)));
    }
    for (i, x) in cb.juntan.iter().enumerate() {
        let vivo = bajo == Some(Bajo::Junta(i)) || x.miembros.iter().any(|m| bajo == Some(Bajo::Miembro(*m)));
        pastilla(caja_j[i], &corto(&x.ruta, 12), 10.5, c.verde.gamma_multiply(alfa(vivo)), c.panel, c.texto.gamma_multiply(alfa(vivo).max(0.4)));
    }
    for i in 0..n {
        let ejes = k.ejes.get(i).copied().unwrap_or_default();
        let seguridad = ejes[0] > 0;
        let suelto = cb.sueltos.contains(&i) && n >= 2;
        let borde = if seguridad { c.error } else if suelto { c.tenue } else { c.acento };
        if c.brillo && seguridad { for (g, a) in [(3.0, 0.35), (6.0, 0.15)] { p.rect_stroke(caja_m[i].expand(g), 6, Stroke::new(2.0, c.error.gamma_multiply(a)), StrokeKind::Outside); } }
        let encima = bajo == Some(Bajo::Miembro(i));
        let relleno = if encima { borde.gamma_multiply(0.30) } else { c.superficie };
        if denso {
            p.circle(pos_m[i], 5.5, if encima || !suelto { borde } else { c.superficie }, Stroke::new(1.3, borde));
            if rotular.contains(&i) || encima {
                let fuera = (pos_m[i] - centro).normalized();
                let (ancla, alin) = if fuera.x >= 0.0 { (pos_m[i] + Vec2::new(9.0, 0.0), Align2::LEFT_CENTER) } else { (pos_m[i] - Vec2::new(9.0, 0.0), Align2::RIGHT_CENTER) };
                let g = p.layout_no_wrap(corto(&cb.miembros[i].id, 22), FontId::proportional(10.5), Color32::WHITE);
                let r = alin.anchor_size(ancla, g.size()).expand2(Vec2::new(3.0, 1.0));
                // Un rotulo que pisaria a otro se calla (el de debajo del raton, nunca).
                if !encima && rotulos.iter().any(|o| o.intersects(r)) { continue; }
                rotulos.push(r);
                p.rect_filled(r, 3, Color32::from_black_alpha(200));
                p.galley(r.min + Vec2::new(3.0, 1.0), g, if suelto { c.tenue } else { Color32::WHITE });
            }
        } else {
            pastilla(caja_m[i], &corto(&cb.miembros[i].id, 18), 12.0, borde, relleno, if suelto { c.tenue } else { Color32::WHITE });
            if ejes.iter().any(|x| *x > 0) { crate::pintar_grafo::triangulo(&p, c, caja_m[i].right_top() + Vec2::new(2.0, -2.0), 11.0, ejes); }
        }
    }
    // Ficha, clics.
    if let (Some(q), Some(b)) = (raton, bajo) {
        let t = match b {
            Bajo::Miembro(i) => {
                let m = &cb.miembros[i];
                let ejes = k.ejes.get(i).copied().unwrap_or_default();
                format!("{}\n{} · se toca con {} del grupo\nSeguridad {} · Rendimiento {} · Elegancia {}\nclic: mostrar · clic derecho: sacar del grupo",
                        m.id, if m.archivos == 1 { "1 archivo".to_string() } else { format!("{} archivos", m.archivos) }, m.lazos, ejes[0], ejes[1], ejes[2])
            }
            Bajo::Comparte(i) => format!("{}\nlo usan {} del grupo\nclic: mostrar · Ctrl+clic: meterlo al grupo", cb.comparten[i].ruta, cb.comparten[i].miembros.len()),
            Bajo::Junta(i) => format!("{}\nusa a {} del grupo\nclic: mostrar · Ctrl+clic: meterlo al grupo", cb.juntan[i].ruta, cb.juntan[i].miembros.len()),
        };
        let g = p.layout(t, FontId::proportional(11.5), c.texto, 300.0);
        let mut r = Rect::from_min_size(q + Vec2::new(12.0, 12.0), g.size() + Vec2::new(12.0, 8.0));
        if r.max.x > caja.max.x { r = r.translate(Vec2::new(caja.max.x - r.max.x - 4.0, 0.0)); }
        if r.max.y > caja.max.y { r = r.translate(Vec2::new(0.0, -(r.height() + 24.0))); }
        p.rect(r, 4, c.panel, Stroke::new(1.0, c.acento), StrokeKind::Inside);
        p.galley(r.min + Vec2::new(6.0, 4.0), g, c.texto);
    }
    let ruta = |b: Bajo| match b { Bajo::Miembro(i) => cb.miembros[i].id.clone(), Bajo::Comparte(i) => cb.comparten[i].ruta.clone(), Bajo::Junta(i) => cb.juntan[i].ruta.clone() };
    if resp.clicked() {
        if let Some(b) = bajo {
            let sumar = ui.input(|i| i.modifiers.command);
            if sumar && !matches!(b, Bajo::Miembro(_)) { acciones.push(Accion::Grupo(AccionGrupo::Alternar(ruta(b)))); }
            else { acciones.push(Accion::Revelar { id: ruta(b), editor: false }); }
        }
    }
    if resp.secondary_clicked() {
        if let Some(Bajo::Miembro(i)) = bajo {
            if k.del_grafo { acciones.push(Accion::Pulsar { nombre: cb.miembros[i].id.clone(), sumar: true }); }
            else { acciones.push(Accion::Grupo(AccionGrupo::Quitar(cb.miembros[i].id.clone()))); }
        }
    }
}
