// SPDX-License-Identifier: GPL-2.0-only
//! Pinta el modo grafo como los editores de nodos de Blender y Houdini. No decide nada:
//! dibuja la Vista y convierte lo que hace el raton en Acciones.
//!
//! Rendimiento por diseno: solo se pintan los nodos y cables que caen en pantalla, y con
//! poco zoom se dejan de pintar textos e iconos (nivel de detalle).

use crate::gpu::{uniforme, Lienzo, Vertice};
use crate::svg::Iconos;
use crate::Colores;
use carpetas_nucleo::{Accion, Camara, Direccion, Nodo, NodoGrafo, Tipo, Vista};
use std::sync::{atomic::AtomicU64, Arc};

/// Geometria retenida para la GPU: (clave, vertices, contador de subidas).
pub type Retenido = (u64, Arc<Vec<Vertice>>, Arc<AtomicU64>);
use eframe::egui::{self, epaint::CubicBezierShape, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

fn color(c: [u8; 3]) -> Color32 { Color32::from_rgb(c[0], c[1], c[2]) }

#[allow(clippy::too_many_arguments)]
pub fn grafo(ui: &mut egui::Ui, v: &Vista, cam: Camara, gpu: Option<Retenido>, iconos: &Iconos, c: &Colores, direccion: Direccion, lienzo: [f32; 2], acciones: &mut Vec<Accion>) {
    let (rect, fondo) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    if (rect.width() - lienzo[0]).abs() > 1.0 || (rect.height() - lienzo[1]).abs() > 1.0 {
        acciones.push(Accion::Lienzo([rect.width(), rect.height()]));
    }
    let z = cam.zoom;
    let en_gpu = gpu.is_some();
    let a_pantalla = |p: [f32; 2]| rect.min + Vec2::new((p[0] - cam.x) * z, (p[1] - cam.y) * z);
    let a_mundo = |p: Pos2| [cam.x + (p.x - rect.min.x) / z, cam.y + (p.y - rect.min.y) / z];
    let pintor = ui.painter_at(rect);
    pintor.rect_filled(rect, 0.0, c.fondo);
    if let Some((clave, geometria, subidas)) = gpu {
        // La GPU pinta rejilla, cables y cuerpos con su geometria retenida; solo viaja la camara.
        pintor.add(eframe::egui_wgpu::Callback::new_paint_callback(rect, Lienzo { clave, geometria, uniforme: uniforme(cam, rect.width(), rect.height()), subidas }));
    }

    // Rejilla como en Blender: lineas finas cada 40 y marcadas cada 200 (en el mundo).
    let paso = 40.0 * z;
    if paso >= 8.0 && !en_gpu {
        let inicio = [((cam.x / 40.0).floor()) * 40.0, ((cam.y / 40.0).floor()) * 40.0];
        let mut i = 0;
        let mut x = inicio[0];
        while a_pantalla([x, 0.0]).x < rect.max.x && i < 400 {
            let px = a_pantalla([x, 0.0]).x;
            let fuerte = (x / 40.0).round() as i64 % 5 == 0;
            pintor.line_segment([Pos2::new(px, rect.min.y), Pos2::new(px, rect.max.y)], Stroke::new(1.0, c.linea.gamma_multiply(if fuerte { 0.55 } else { 0.22 })));
            x += 40.0; i += 1;
        }
        let mut y = inicio[1];
        i = 0;
        while a_pantalla([0.0, y]).y < rect.max.y && i < 400 {
            let py = a_pantalla([0.0, y]).y;
            let fuerte = (y / 40.0).round() as i64 % 5 == 0;
            pintor.line_segment([Pos2::new(rect.min.x, py), Pos2::new(rect.max.x, py)], Stroke::new(1.0, c.linea.gamma_multiply(if fuerte { 0.55 } else { 0.22 })));
            y += 40.0; i += 1;
        }
    }

    // Raton sobre el fondo: rueda = zoom, boton central o derecho = mover, izquierdo = banda.
    if let Some(p) = fondo.hover_pos() {
        let rueda = ui.input(|i| i.smooth_scroll_delta().y);
        if rueda != 0.0 { acciones.push(Accion::Zoom { factor: 1.0015f32.powf(rueda), ancla: [p.x - rect.min.x, p.y - rect.min.y] }); }
    }
    if fondo.dragged_by(PointerButton::Middle) || fondo.dragged_by(PointerButton::Secondary) {
        let d = fondo.drag_delta();
        acciones.push(Accion::Desplazar([d.x, d.y]));
    }
    let ctrl = ui.input(|i| i.modifiers.command);
    if fondo.drag_started_by(PointerButton::Primary) {
        if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::EmpezarBanda { punto: a_mundo(p), sumar: ctrl }); }
    }
    if fondo.dragged_by(PointerButton::Primary) {
        if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::MoverBanda(a_mundo(p))); }
    }
    if fondo.drag_stopped_by(PointerButton::Primary) { acciones.push(Accion::TerminarBanda); }
    if fondo.clicked() && !ctrl { acciones.push(Accion::Comando(carpetas_nucleo::Comando::LimpiarSeleccion)); }

    let Some((nodos, cables)) = &v.grafo else { return };
    let visible = rect.expand(60.0);
    // Cables primero (quedan debajo de los nodos). Curvas como los "noodles" de Blender.
    for cable in cables.iter().filter(|_| !en_gpu) {
        let (a, b) = (a_pantalla(cable.desde), a_pantalla(cable.hasta));
        if !visible.contains(a) && !visible.contains(b) && !visible.intersects(Rect::from_two_pos(a, b)) { continue; }
        let (c1, c2) = match direccion {
            Direccion::Blender => { let d = ((b.x - a.x).abs() * 0.5).max(40.0 * z); (a + Vec2::new(d, 0.0), b - Vec2::new(d, 0.0)) }
            Direccion::Houdini => { let d = ((b.y - a.y).abs() * 0.5).max(30.0 * z); (a + Vec2::new(0.0, d), b - Vec2::new(0.0, d)) }
        };
        let col = color(cable.color).gamma_multiply(0.85);
        pintor.add(CubicBezierShape::from_points_stroke([a, c1, c2, b], false, Color32::TRANSPARENT, Stroke::new((2.0 * z).clamp(1.0, 3.0), col)));
    }
    for n in nodos {
        let r = Rect::from_min_size(a_pantalla(n.pos), Vec2::new(n.tam[0] * z, n.tam[1] * z));
        if !visible.intersects(r) { continue; }
        nodo(ui, &pintor, n, r, z, iconos, c, direccion, en_gpu, acciones);
    }
    // Banda de seleccion (en coordenadas del mundo).
    for n in &v.nodos {
        if let Nodo::Banda { desde, hasta, color: col } = n {
            let banda = Rect::from_two_pos(a_pantalla(*desde), a_pantalla(*hasta));
            pintor.rect(banda, 0.0, color(*col).gamma_multiply(0.08), Stroke::new(1.0, color(*col)), StrokeKind::Inside);
        }
    }
    // Ayuda de controles abajo, como la barra de estado de Blender.
    pintor.text(rect.left_bottom() + Vec2::new(10.0, -8.0), Align2::LEFT_BOTTOM,
                "rueda: zoom · central/derecho: mover · doble clic: desplegar/abrir · Inicio: encuadrar · D: Blender/Houdini",
                FontId::proportional(11.0), c.tenue);
}

#[allow(clippy::too_many_arguments)]
fn nodo(ui: &egui::Ui, pintor: &egui::Painter, n: &NodoGrafo, r: Rect, z: f32, iconos: &Iconos, c: &Colores, direccion: Direccion, en_gpu: bool, acciones: &mut Vec<Accion>) {
    let col = color(n.color);
    let resp = ui.interact(r, ui.make_persistent_id(("nodo", &n.id)), Sense::click_and_drag());
    if !n.id.is_empty() {
        let ctrl = ui.input(|i| i.modifiers.command);
        if resp.clicked() { acciones.push(Accion::Pulsar { nombre: n.id.clone(), sumar: ctrl }); }
        if resp.double_clicked() { acciones.push(Accion::Abrir(n.id.clone())); }
        if resp.drag_started_by(PointerButton::Primary) { acciones.push(Accion::EmpezarArrastre(n.id.clone())); }
        if resp.dragged_by(PointerButton::Primary) { let d = resp.drag_delta(); acciones.push(Accion::Arrastrar([d.x, d.y])); }
        if resp.drag_stopped_by(PointerButton::Primary) { acciones.push(Accion::Soltar); }
    }
    let radio = (c.esquinas as f32 * z).max(2.0) as u8;
    let alto_cab = (22.0 * z).max(4.0);
    let cab = Rect::from_min_size(r.min, Vec2::new(r.width(), alto_cab));
    if en_gpu {
        // La GPU ya pinto el cuerpo: aqui solo el resaltado al pasar el raton.
        if resp.hovered() && !n.seleccionado { pintor.rect_stroke(r, 0, Stroke::new(1.0, col), StrokeKind::Inside); }
    } else {
    // Brillo y borde de seleccion.
    if n.seleccionado && c.brillo {
        for (k, a) in [(2.0, 0.35), (5.0, 0.15)] { pintor.rect_stroke(r.expand(k), radio, Stroke::new(2.0, c.acento.gamma_multiply(a)), StrokeKind::Outside); }
    }
    pintor.rect(r, radio, c.superficie, Stroke::new(if n.seleccionado { 2.0 } else { 1.0 }, if n.seleccionado { c.acento } else if resp.hovered() { col } else { c.linea }), StrokeKind::Inside);
    // Cabecera de color por tipo, como las categorias de nodos de Blender.
    pintor.rect_filled(cab, egui::CornerRadius { nw: radio, ne: radio, sw: 0, se: 0 }, col.gamma_multiply(if n.id.is_empty() { 0.75 } else { 0.5 }));
    // Conectores: entrada (no en la raiz) y salida (carpetas desplegadas).
    let (entrada, salida) = match direccion {
        Direccion::Blender => (r.left_center(), r.right_center()),
        Direccion::Houdini => (r.center_top(), r.center_bottom()),
    };
    let rad = (4.5 * z).max(2.0);
    if !n.id.is_empty() { pintor.circle(entrada, rad, c.superficie, Stroke::new(1.5, col)); }
    if n.expandida == Some(true) { pintor.circle(salida, rad, col, Stroke::new(1.0, c.fondo)); }
    }
    if z < 0.45 { return; } // nivel de detalle: lejos, solo formas
    // Titulo, icono y boton de desplegar.
    let tam_letra = 12.5 * z;
    let icono_r = Rect::from_min_size(cab.min + Vec2::new(5.0 * z, 3.0 * z), Vec2::splat(16.0 * z));
    if !iconos.pintar(ui, n.icono, icono_r, col) {
        // Estilo Neon: un cuadrado (carpeta) o una hoja (archivo) dibujados.
        let k = icono_r.shrink(2.0 * z);
        if n.tipo == Tipo::Carpeta { pintor.rect(k, 1.0, col.gamma_multiply(0.3), Stroke::new(1.2, c.texto), StrokeKind::Inside); }
        else { pintor.rect(k.shrink2(Vec2::new(2.0 * z, 0.0)), 1.0, Color32::TRANSPARENT, Stroke::new(1.2, c.texto), StrokeKind::Inside); }
    }
    let mut titulo = n.nombre.clone();
    if titulo.chars().count() > 22 { titulo = titulo.chars().take(21).collect::<String>() + "…"; }
    pintor.text(cab.left_center() + Vec2::new(26.0 * z, 0.0), Align2::LEFT_CENTER, titulo, FontId::proportional(tam_letra), c.texto);
    if let (Some(abierta), false) = (n.expandida, n.id.is_empty()) {
        let boton = Rect::from_center_size(cab.right_center() - Vec2::new(12.0 * z, 0.0), Vec2::splat(16.0 * z));
        let rb = ui.interact(boton, ui.make_persistent_id(("desplegar", &n.id)), Sense::click());
        if rb.clicked() { acciones.push(Accion::Expandir(n.id.clone())); }
        pintor.rect(boton, 2, if rb.hovered() { col.gamma_multiply(0.5) } else { Color32::TRANSPARENT }, Stroke::new(1.0, c.texto.gamma_multiply(0.6)), StrokeKind::Inside);
        pintor.text(boton.center(), Align2::CENTER_CENTER, if abierta { "-" } else { "+" }, FontId::monospace(12.0 * z), c.texto);
    }
    // Play: lo que arranca este nodo (cargo run, make, python3...), como en VS Code.
    if let Some(titulo) = &n.play {
        let hueco = if n.expandida.is_some() && !n.id.is_empty() { 34.0 } else { 12.0 };
        let boton = Rect::from_center_size(cab.right_center() - Vec2::new(hueco * z, 0.0), Vec2::splat(16.0 * z));
        let rp = ui.interact(boton, ui.make_persistent_id(("play", &n.id)), Sense::click());
        triangulo_play(pintor, boton, if rp.hovered() { c.texto } else { c.verde });
        if rp.clicked() { acciones.push(Accion::Ejecutar(n.id.clone())); }
        rp.on_hover_text(format!("Ejecutar: {titulo}"));
    }
    // Cuerpo: detalle y barra de peso (para ver de un vistazo que ocupa mas espacio).
    pintor.text(r.min + Vec2::new(10.0 * z, alto_cab + 8.0 * z), Align2::LEFT_TOP, &n.detalle, FontId::proportional(11.0 * z), c.tenue);
    if en_gpu { resp.on_hover_text(if n.id.is_empty() { "Carpeta abierta".to_string() } else { format!("{}\n{}", n.id, n.detalle) }); return; }
    let barra = Rect::from_min_size(Pos2::new(r.min.x + 10.0 * z, r.max.y - 12.0 * z), Vec2::new(r.width() - 20.0 * z, 4.0 * z));
    pintor.rect_filled(barra, 1.0, c.linea);
    if n.peso > 0.0 {
        let lleno = Rect::from_min_size(barra.min, Vec2::new(barra.width() * n.peso.clamp(0.0, 1.0), barra.height()));
        pintor.rect_filled(lleno, 1.0, if n.peso > 0.6 { c.error } else { col });
    }
    resp.on_hover_text(if n.id.is_empty() { "Carpeta abierta".to_string() } else { format!("{}\n{}", n.id, n.detalle) });
}

/// Triangulo de play dibujado (no depende de que la fuente tenga el caracter).
pub fn triangulo_play(pintor: &egui::Painter, caja: Rect, color: Color32) {
    let c = caja.center();
    let (w, h) = (caja.width() * 0.32, caja.height() * 0.38);
    pintor.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-w * 0.8, -h), c + Vec2::new(w * 1.1, 0.0), c + Vec2::new(-w * 0.8, h)], color, Stroke::NONE));
}
