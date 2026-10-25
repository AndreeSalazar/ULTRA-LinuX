// SPDX-License-Identifier: GPL-2.0-only
//! El camino entre dos elegidos, dibujado (panel Combinar y Ciudad).
//!
//! La animacion no es adorno: es la **onda de recompilacion**. Sale de lo que cambiaria
//! (lo usado) y viaja hacia quien lo usa, paso a paso:
//! - A usa B: la onda va de B hacia A.
//! - Comparten X: sale de X y se abre hacia los dos.
//! - Los junta X: entra en X desde los dos.
//!
//! El paso **imprescindible** (sin el, se separan) late en rojo neon: ahi se corta.

use crate::Colores;
use carpetas_nucleo::camino::{Camino, SinPaso, Union};
use carpetas_nucleo::{Accion, AccionGrupo};
use eframe::egui::{self, epaint::CubicBezierShape, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

/// Cuando le llega la onda a cada nodo (0: de donde sale). La onda va de `hasta` a `desde`.
pub fn tiempos(c: &Camino) -> Vec<u32> {
    let n = c.nodos.len();
    let mut t = vec![0u32; n];
    // nodo i del paso: el que usa (desde) y el usado (hasta) son i e i+1, en un orden u otro.
    let usuario = |i: usize| -> (usize, usize) { if c.pasos[i].desde == c.nodos[i] { (i, i + 1) } else { (i + 1, i) } };
    for _ in 0..n {
        for i in 0..c.pasos.len() {
            let (u, d) = usuario(i);
            t[u] = t[u].max(t[d] + 1);
        }
    }
    t
}

/// Donde va la onda en el paso i (0 = en el usado, 1 = en el que usa), o None si no esta ahi.
pub fn onda(c: &Camino, t: &[u32], i: usize, ahora: f64) -> Option<f32> {
    let total = t.iter().copied().max().unwrap_or(0) as f64 + 1.6;
    let fase = (ahora * 1.1) % total;
    let d = if c.pasos[i].desde == c.nodos[i] { i + 1 } else { i };
    let x = fase - t[d] as f64;
    (0.0..1.0).contains(&x).then_some(x as f32)
}

/// El paso i va de nodos[i] a nodos[i+1] en la direccion real (desde -> hasta)?
fn hacia_delante(c: &Camino, i: usize) -> bool { c.pasos[i].desde == c.nodos[i] }

fn corto(ruta: &str, max: usize) -> String {
    let n = ruta.rsplit('/').next().unwrap_or(ruta);
    if n.chars().count() <= max { n.to_string() } else { format!("{}…", n.chars().take(max - 1).collect::<String>()) }
}

/// Selector de extremos, el porque, el dibujo animado y los pasos con su motivo.
pub fn seccion(ui: &mut egui::Ui, c: &Colores, cam: &Camino, miembros: &[String], animar: &mut bool, acciones: &mut Vec<Accion>) {
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("CAMINO").color(Color32::WHITE).strong());
        if miembros.len() > 2 {
            let mut elegir = |ui: &mut egui::Ui, id: &str, actual: &str, otro: &str, es_a: bool| {
                egui::ComboBox::from_id_salt(id).selected_text(corto(actual, 22)).width(150.0).show_ui(ui, |ui| {
                    for m in miembros.iter().filter(|m| m.as_str() != otro) {
                        if ui.selectable_label(m == actual, m.as_str()).clicked() {
                            let (a, b) = if es_a { (m.clone(), otro.to_string()) } else { (otro.to_string(), m.clone()) };
                            acciones.push(Accion::Grupo(AccionGrupo::Extremos(a, b)));
                        }
                    }
                });
            };
            elegir(ui, "camino-a", &cam.a, &cam.b, true);
            ui.label("a");
            elegir(ui, "camino-b", &cam.b, &cam.a, false);
        } else {
            ui.label(RichText::new(format!("{}  ·  {}", corto(&cam.a, 26), corto(&cam.b, 26))).color(c.texto));
        }
        if ui.small_button("Invertir").on_hover_text("Cambiar A por B").clicked() { acciones.push(Accion::Grupo(AccionGrupo::Invertir)); }
        ui.checkbox(animar, "onda").on_hover_text("La onda de recompilacion: sale de lo que cambiaria y llega a quien lo usa");
    });
    let union = match cam.union { Some(u) => u.nombre(), None => "sin union" };
    ui.label(RichText::new(format!("{} · {} saltos · {} recorridos{}", union, cam.pasos.len(), cam.explorados,
                                   if cam.recortado { " · recortado" } else { "" })).color(c.tenue).size(11.0));
    ui.label(RichText::new(&cam.por_que).color(c.texto).size(12.0));
    if cam.nodos.is_empty() { return; }
    lienzo(ui, c, cam, *animar, acciones);
    // Pasos, cada uno con su motivo y que pasaria sin el.
    ui.add_space(4.0);
    for (i, p) in cam.pasos.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            let (col, chip) = match p.sin {
                SinPaso::Imprescindible => (c.error, "imprescindible: cortar aqui los separa".to_string()),
                SinPaso::Desvio(u, s) => (c.acento, format!("sin el: otro camino de {s} saltos ({})", u.nombre())),
                SinPaso::SinAnalizar => (c.tenue, "sin analizar".to_string()),
            };
            ui.label(RichText::new(format!("{}.", i + 1)).color(c.tenue).monospace());
            let donde = match &p.motivo { Some(m) => format!("{}:{}", corto(&p.desde, 30), m.linea), None => corto(&p.desde, 30) };
            let r = ui.add(egui::Label::new(RichText::new(donde).monospace().color(Color32::WHITE).size(12.0)).wrap().sense(Sense::click()))
                .on_hover_text(format!("{}\nclic: abrir el editor en esa linea", p.desde));
            if r.clicked() {
                acciones.push(match &p.motivo { Some(m) => Accion::IrALinea { id: p.desde.clone(), linea: m.linea }, None => Accion::Revelar { id: p.desde.clone(), editor: true } });
            }
            // Una ruta larga sin espacios no se parte y ensancharia el panel: en la fila va su
            // final; entera, al pasar el raton.
            let (escrito, entero) = match &p.motivo {
                Some(m) => {
                    let entero = m.escrito(&p.desde);
                    let fila = if entero.chars().count() > 40 && m.texto.contains('/') { entero.replace(&m.texto, &format!("…/{}", corto(&m.texto, 30))) } else { entero.clone() };
                    (fila, entero)
                }
                None => (format!("usa {}", corto(&p.hasta, 30)), p.hasta.clone()),
            };
            ui.add(egui::Label::new(RichText::new(escrito).monospace().color(c.aviso).size(12.0)).wrap()).on_hover_text(entero);
            ui.label(RichText::new(chip).color(col).size(11.0));
            ui.label(RichText::new(format!("· {} usan {}", p.lo_usan, corto(&p.hasta, 24))).color(c.tenue).size(11.0)).on_hover_text("Cuantos archivos del proyecto usan a ese: el peso del eslabon");
        });
    }
    // La pista para optimizar: entre los imprescindibles, el que menos arrastra.
    if let Some(p) = cam.pasos.iter().filter(|p| p.sin == SinPaso::Imprescindible).min_by_key(|p| p.lo_usan) {
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Para desacoplarlos: el paso imprescindible con menos peso es {} usa {} ({} archivos usan {}). \
                                        Quitar o cambiar esa referencia los separa; comprobarlo compilando.",
                                       corto(&p.desde, 30), corto(&p.hasta, 30), p.lo_usan, corto(&p.hasta, 30))).color(c.error).size(11.5));
    }
}

/// Los nodos en serpiente (de A a B), los pasos con su flecha real y la onda.
fn lienzo(ui: &mut egui::Ui, c: &Colores, cam: &Camino, animar: bool, acciones: &mut Vec<Accion>) {
    let n = cam.nodos.len();
    let ancho = ui.available_width().max(220.0);
    let por_fila = ((ancho - 20.0) / 150.0).floor().max(2.0) as usize;
    let filas = n.div_ceil(por_fila);
    let alto = 30.0 + filas as f32 * 70.0;
    let (caja, resp) = ui.allocate_exact_size(Vec2::new(ancho, alto), Sense::click());
    let p = ui.painter_at(caja);
    p.rect(caja, 6, c.fondo, Stroke::new(1.0, c.linea), StrokeKind::Inside);
    let paso_x = (ancho - 20.0) / por_fila as f32;
    let pos: Vec<Pos2> = (0..n).map(|i| {
        let (f, k) = (i / por_fila, i % por_fila);
        let k = if f % 2 == 1 { por_fila - 1 - k } else { k };
        Pos2::new(caja.left() + 10.0 + paso_x * (k as f32 + 0.5), caja.top() + 40.0 + f as f32 * 70.0)
    }).collect();
    // Cuantas letras caben en una columna (los nombres no se pisan con el vecino).
    let letras = (((paso_x - 22.0) / 6.6) as usize).clamp(6, 18);
    let cajas: Vec<Rect> = (0..n).map(|i| {
        let g = p.layout_no_wrap(corto(&cam.nodos[i], letras), FontId::proportional(11.5), Color32::WHITE);
        Rect::from_center_size(pos[i], g.size() + Vec2::new(14.0, 10.0))
    }).collect();
    let ahora = ui.input(|i| i.time);
    let t = tiempos(cam);
    let raton = resp.hover_pos();
    let mut bajo_paso = None;
    for (i, paso) in cam.pasos.iter().enumerate() {
        let (a, b) = (pos[i], pos[i + 1]);
        let misma_fila = (a.y - b.y).abs() < 1.0;
        let (c1, c2) = if misma_fila { (a + Vec2::new(0.0, -26.0), b + Vec2::new(0.0, -26.0)) } else { (a + Vec2::new(60.0 * (a.x - caja.center().x).signum(), 30.0), b + Vec2::new(60.0 * (b.x - caja.center().x).signum(), -30.0)) };
        let punto = |s: f32| { let u = 1.0 - s; (a.to_vec2() * u * u * u + c1.to_vec2() * 3.0 * u * u * s + c2.to_vec2() * 3.0 * u * s * s + b.to_vec2() * s * s * s).to_pos2() };
        let cerca = raton.is_some_and(|q| (0..=20).any(|k| (punto(k as f32 / 20.0) - q).length() < 7.0));
        if cerca { bajo_paso = Some(i); }
        let imprescindible = paso.sin == SinPaso::Imprescindible;
        let base = if imprescindible { c.error } else { Color32::WHITE };
        let latido = if imprescindible && animar { 0.65 + 0.35 * ((ahora * 4.0).sin() as f32).abs() } else { 1.0 };
        let w = if cerca { 3.2 } else { 2.0 };
        if c.brillo || imprescindible {
            p.add(CubicBezierShape::from_points_stroke([a, c1, c2, b], false, Color32::TRANSPARENT, Stroke::new(w + 6.0, base.gamma_multiply(0.18 * latido))));
        }
        p.add(CubicBezierShape::from_points_stroke([a, c1, c2, b], false, Color32::TRANSPARENT, Stroke::new(w, base.gamma_multiply(latido))));
        // Flecha en el usado (la direccion real del #include).
        let (s_fin, s_antes): (f32, f32) = if hacia_delante(cam, i) { (1.0, 0.94) } else { (0.0, 0.06) };
        let caja_usado = if hacia_delante(cam, i) { cajas[i + 1] } else { cajas[i] };
        let mut s = s_fin;
        while (s - s_fin).abs() < 0.5 && caja_usado.expand(2.0).contains(punto(s)) { s += if s_fin > 0.5 { -0.02 } else { 0.02 }; }
        let punta = punto(s);
        let dir = (punta - punto(s + (s_antes - s_fin))).normalized();
        let lado = Vec2::new(-dir.y, dir.x);
        p.add(egui::Shape::convex_polygon(vec![punta, punta - dir * 9.0 + lado * 4.5, punta - dir * 9.0 - lado * 4.5], base.gamma_multiply(latido), Stroke::NONE));
        // Marca de corte en el imprescindible: dos rayas cruzando el cable.
        if imprescindible {
            let m = punto(0.5);
            let d = (punto(0.55) - punto(0.45)).normalized();
            let l = Vec2::new(-d.y, d.x);
            for k in [-3.0, 3.0] { p.line_segment([m + d * k - l * 7.0, m + d * k + l * 7.0], Stroke::new(1.6, c.error)); }
        }
        // La onda: un punto brillante que viaja del usado al que usa.
        if animar {
            if let Some(x) = onda(cam, &t, i, ahora) {
                let s = if hacia_delante(cam, i) { 1.0 - x } else { x };
                let q = punto(s);
                let col = if imprescindible { c.error } else { c.acento };
                for (r, a) in [(9.0, 0.12), (6.0, 0.3), (3.5, 1.0)] { p.circle_filled(q, r, col.gamma_multiply(a)); }
                p.circle_filled(q, 1.6, Color32::WHITE);
            }
        }
    }
    // Nodos: A y B en blanco, el punto de encuentro con anillo, el resto en el acento.
    let mut bajo_nodo = None;
    for i in 0..n {
        let r = cajas[i];
        if raton.is_some_and(|q| r.contains(q)) { bajo_nodo = Some(i); }
        let extremo = i == 0 || i == n - 1;
        let es_punto = cam.punto == Some(i);
        let borde = if extremo { Color32::WHITE } else if es_punto { c.verde } else { c.acento };
        if es_punto && animar {
            let k = ((ahora * 2.0) % 1.0) as f32;
            p.rect_stroke(r.expand(2.0 + k * 10.0), 8, Stroke::new(1.5, c.verde.gamma_multiply(1.0 - k)), StrokeKind::Outside);
        }
        p.rect(r, 5, if bajo_nodo == Some(i) { borde.gamma_multiply(0.3) } else { c.superficie }, Stroke::new(if extremo { 2.0 } else { 1.3 }, borde), StrokeKind::Inside);
        p.text(r.center(), Align2::CENTER_CENTER, corto(&cam.nodos[i], letras), FontId::proportional(11.5), if extremo { Color32::WHITE } else { c.texto });
        let etiqueta = if i == 0 { Some("A") } else if i == n - 1 { Some("B") } else if es_punto { Some(if cam.union == Some(Union::Juntan) { "los junta" } else { "comparten" }) } else { None };
        if let Some(e) = etiqueta { p.text(r.center_top() - Vec2::new(0.0, if es_punto { 14.0 } else { 3.0 }), Align2::CENTER_BOTTOM, e, FontId::proportional(10.0), borde); }
    }
    // Ficha y clics.
    if let Some(q) = raton {
        let texto = if let Some(i) = bajo_nodo {
            Some(format!("{}\nclic: mostrar (la Ciudad lleva la camara)", cam.nodos[i]))
        } else {
            bajo_paso.map(|i| {
                let p = &cam.pasos[i];
                let motivo = p.motivo.as_ref().map(|m| format!("{}:{}  {}", p.desde, m.linea, m.escrito(&p.desde))).unwrap_or_else(|| format!("{} usa {}", p.desde, p.hasta));
                let sin = match p.sin {
                    SinPaso::Imprescindible => "sin este paso se separan: es donde cortar".to_string(),
                    SinPaso::Desvio(u, s) => format!("sin este paso siguen unidos ({}) por otro camino de {s} saltos", u.nombre()),
                    SinPaso::SinAnalizar => "no analizado".to_string(),
                };
                format!("{motivo}\n{sin}\n{} archivos usan {}\nclic: abrir el editor en esa linea", p.lo_usan, p.hasta)
            })
        };
        if let Some(t) = texto {
            let g = p.layout(t, FontId::proportional(11.5), c.texto, 340.0);
            let mut r = Rect::from_min_size(q + Vec2::new(12.0, 12.0), g.size() + Vec2::new(12.0, 8.0));
            if r.max.x > caja.max.x { r = r.translate(Vec2::new(caja.max.x - r.max.x - 4.0, 0.0)); }
            p.rect(r, 4, c.panel, Stroke::new(1.0, c.acento), StrokeKind::Inside);
            p.galley(r.min + Vec2::new(6.0, 4.0), g, c.texto);
        }
    }
    if resp.clicked() {
        if let Some(i) = bajo_nodo { acciones.push(Accion::Revelar { id: cam.nodos[i].clone(), editor: false }); }
        else if let Some(i) = bajo_paso {
            let p = &cam.pasos[i];
            acciones.push(match &p.motivo { Some(m) => Accion::IrALinea { id: p.desde.clone(), linea: m.linea }, None => Accion::Revelar { id: p.desde.clone(), editor: true } });
        }
    }
    if animar { ui.ctx().request_repaint_after(std::time::Duration::from_millis(33)); }
}
