// SPDX-License-Identifier: GPL-2.0-only
//! Pinta el modo grafo como los editores de nodos de Blender y Houdini. No decide nada:
//! dibuja la Vista y convierte lo que hace el raton en Acciones.
//!
//! Rendimiento por diseno: solo se pintan los nodos y cables que caen en pantalla, y con
//! poco zoom se dejan de pintar textos e iconos (nivel de detalle).

use crate::gpu::{uniforme, Lienzo, Vertice};
use crate::svg::Iconos;
use crate::Colores;
use carpetas_nucleo::{Accion, Camara, ClaseCable, Direccion, Marcas, Nodo, NodoGrafo, ProcesoVivo, Rol, Tipo, Vista};
use std::sync::{atomic::AtomicU64, Arc};

/// Geometria retenida para la GPU: (clave, vertices, contador de subidas).
pub type Retenido = (u64, Arc<Vec<Vertice>>, Arc<AtomicU64>);

/// El proceso en marcha: de que nodo cuelga y su arbol (de /proc).
pub struct Vivos<'a> { pub id: &'a str, pub titulo: &'a str, pub procesos: &'a [ProcesoVivo] }
use eframe::egui::{self, epaint::CubicBezierShape, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

fn color(c: [u8; 3]) -> Color32 { Color32::from_rgb(c[0], c[1], c[2]) }

#[allow(clippy::too_many_arguments)]
pub fn grafo(ui: &mut egui::Ui, v: &Vista, cam: Camara, gpu: Option<Retenido>, iconos: &Iconos, c: &Colores, direccion: Direccion, lienzo: [f32; 2],
             vivos: Option<Vivos>, acciones: &mut Vec<Accion>) {
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
    migas(ui, &pintor, rect, &v.migas, c, acciones);
    let visible = rect.expand(60.0);
    // Cables primero (quedan debajo de los nodos). Curvas como los "noodles" de Blender.
    for cable in cables.iter().filter(|_| !en_gpu) {
        let (a, b) = (a_pantalla(cable.desde), a_pantalla(cable.hasta));
        if !visible.contains(a) && !visible.contains(b) && !visible.intersects(Rect::from_two_pos(a, b)) { continue; }
        let (c1, c2) = match (direccion, cable.vuelta) {
            (Direccion::Blender, false) => { let d = ((b.x - a.x).abs() * 0.5).max(40.0 * z); (a + Vec2::new(d, 0.0), b - Vec2::new(d, 0.0)) }
            (Direccion::Houdini, false) => { let d = ((b.y - a.y).abs() * 0.5).max(30.0 * z); (a + Vec2::new(0.0, d), b - Vec2::new(0.0, d)) }
            (Direccion::Blender, true) => { let x = a.x.max(b.x) + 50.0 * z + (b.y - a.y).abs() * 0.3; (Pos2::new(x, a.y), Pos2::new(x, b.y)) }
            (Direccion::Houdini, true) => { let y = a.y.max(b.y) + 50.0 * z + (b.x - a.x).abs() * 0.3; (Pos2::new(a.x, y), Pos2::new(b.x, y)) }
        };
        let capas: Vec<(f32, Color32)> = match cable.clase {
            ClaseCable::Arbol => vec![((2.0 * z).clamp(1.0, 3.0), color(cable.color).gamma_multiply(0.85))],
            ClaseCable::Fallo => vec![((8.0 * z).clamp(3.0, 10.0), c.error.gamma_multiply(0.16)), ((3.6 * z).clamp(1.5, 5.0), c.error), ((1.2 * z).clamp(0.8, 2.0), Color32::WHITE)],
            ClaseCable::Dependencia => vec![((5.0 * z).clamp(2.0, 7.0), color(cable.color).gamma_multiply(0.18)), ((2.0 * z).clamp(1.0, 3.0), color(cable.color))],
        };
        for (g, col) in capas { pintor.add(CubicBezierShape::from_points_stroke([a, c1, c2, b], false, Color32::TRANSPARENT, Stroke::new(g, col))); }
        if cable.clase != ClaseCable::Arbol {
            pintor.circle_filled(b, (6.0 * z).max(3.0), if cable.clase == ClaseCable::Fallo { c.error } else { color(cable.color) });
            pintor.circle_filled(b, (2.5 * z).max(1.5), Color32::WHITE);
        }
    }
    for n in nodos {
        let r = Rect::from_min_size(a_pantalla(n.pos), Vec2::new(n.tam[0] * z, n.tam[1] * z));
        if !visible.intersects(r) { continue; }
        nodo(ui, &pintor, n, r, z, iconos, c, direccion, en_gpu, acciones);
    }
    // El proceso en marcha: borde verde en su nodo y la tarjeta con el arbol de /proc.
    if let Some(vv) = &vivos {
        if let Some(n) = nodos.iter().find(|n| n.id == vv.id) {
            let r = Rect::from_min_size(a_pantalla(n.pos), Vec2::new(n.tam[0] * z, n.tam[1] * z));
            pintor.rect_stroke(r.expand(3.0), 2, Stroke::new(2.0, c.verde), StrokeKind::Outside);
            tarjeta_vivos(&pintor, c, Pos2::new(r.min.x, r.max.y + 10.0), vv, rect);
        }
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
                "rueda: zoom · central/derecho arrastrar: mover · clic derecho: editor · doble clic: desplegar/abrir · F8: siguiente error · Inicio: encuadrar",
                FontId::proportional(11.0), c.tenue);
}

#[allow(clippy::too_many_arguments)]
fn nodo(ui: &egui::Ui, pintor: &egui::Painter, n: &NodoGrafo, r: Rect, z: f32, iconos: &Iconos, c: &Colores, direccion: Direccion, en_gpu: bool, acciones: &mut Vec<Accion>) {
    if !en_gpu { anillos_de_rol(pintor, c, n, r); }
    nodo_dentro(ui, pintor, n, r, z, iconos, c, direccion, en_gpu, acciones);
    // Aislado y sin conexion: apagado (por encima del cuerpo de la GPU y del texto).
    if n.atenuado { pintor.rect_filled(r.expand(10.0 * z), 0, c.fondo.gamma_multiply(0.78)); }
}

/// Lo mismo que la GPU (gpu::anillos_de_rol) cuando se dibuja en la CPU.
fn anillos_de_rol(p: &egui::Painter, c: &Colores, n: &NodoGrafo, r: Rect) {
    let anillo = |k: f32, g: f32, col: Color32| { p.rect_stroke(r.expand(k), 0, Stroke::new(g, col), StrokeKind::Outside); };
    match n.rol {
        Some(Rol::Causa) => { anillo(14.0, 5.0, c.error.gamma_multiply(0.10)); anillo(8.0, 4.0, c.error.gamma_multiply(0.24)); anillo(3.0, 3.0, c.error.gamma_multiply(0.6));
                              anillo(0.5, 2.5, c.error); anillo(-2.0, 2.0, Color32::WHITE); }
        Some(Rol::Falla) => { anillo(4.0, 3.0, c.error.gamma_multiply(0.35)); anillo(0.5, 2.0, c.error); anillo(-1.0, 1.0, Color32::WHITE.gamma_multiply(0.9)); }
        Some(Rol::Arrastra) => { anillo(3.0, 3.0, c.error.gamma_multiply(0.14)); anillo(0.0, 1.5, c.error.gamma_multiply(0.8)); }
        Some(Rol::Base) => anillo(0.0, 1.5, Color32::WHITE.gamma_multiply(0.75)),
        None => {}
    }
}

/// Que significa el papel, para la ficha emergente.
pub fn texto_rol(rol: Rol) -> &'static str {
    match rol {
        Rol::Causa => "SUPER GRAFO: causa probable (falla y nada de lo que usa falla)",
        Rol::Falla => "SUPER GRAFO: falla, pero usa algo que tambien falla (probable consecuencia)",
        Rol::Arrastra => "SUPER GRAFO: usa algo que falla; se puede romper con el",
        Rol::Base => "SUPER GRAFO: lo usa lo aislado (de aqui puede venir el problema)",
    }
}

#[allow(clippy::too_many_arguments)]
fn nodo_dentro(ui: &egui::Ui, pintor: &egui::Painter, n: &NodoGrafo, r: Rect, z: f32, iconos: &Iconos, c: &Colores, direccion: Direccion, en_gpu: bool, acciones: &mut Vec<Accion>) {
    let col = color(n.color);
    let resp = ui.interact(r, ui.make_persistent_id(("nodo", &n.id)), Sense::click_and_drag());
    if !n.raiz {
        let ctrl = ui.input(|i| i.modifiers.command);
        if resp.clicked() { acciones.push(Accion::Pulsar { nombre: n.id.clone(), sumar: ctrl }); }
        if resp.double_clicked() { acciones.push(Accion::Abrir(n.id.clone())); }
        if resp.drag_started_by(PointerButton::Primary) { acciones.push(Accion::EmpezarArrastre(n.id.clone())); }
        if resp.dragged_by(PointerButton::Primary) { let d = resp.drag_delta(); acciones.push(Accion::Arrastrar([d.x, d.y])); }
        if resp.drag_stopped_by(PointerButton::Primary) { acciones.push(Accion::Soltar); }
    }
    // Clic derecho: editor de codigo (en una carpeta, en su primer error).
    if resp.secondary_clicked() { acciones.push(Accion::AbrirEditor { id: n.id.clone(), linea: None }); }
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
    if n.grupo.is_some() {
        for k in [8.0, 4.0] { pintor.rect(r.translate(Vec2::splat(k * z)), radio, c.superficie.gamma_multiply(0.85), Stroke::new(1.0, col.gamma_multiply(0.55)), StrokeKind::Inside); }
    }
    pintor.rect(r, radio, c.superficie, Stroke::new(if n.seleccionado { 2.0 } else { 1.0 }, if n.seleccionado { c.acento } else if resp.hovered() { col } else { c.linea }), StrokeKind::Inside);
    // Cabecera de color por tipo, como las categorias de nodos de Blender.
    pintor.rect_filled(cab, egui::CornerRadius { nw: radio, ne: radio, sw: 0, se: 0 }, col.gamma_multiply(if n.raiz { 0.75 } else { 0.5 }));
    // Conectores: entrada (no en la raiz) y salida (carpetas desplegadas).
    let (entrada, salida) = match direccion {
        Direccion::Blender => (r.left_center(), r.right_center()),
        Direccion::Houdini => (r.center_top(), r.center_bottom()),
    };
    let rad = (4.5 * z).max(2.0);
    if !n.raiz { pintor.circle(entrada, rad, c.superficie, Stroke::new(1.5, col)); }
    if n.expandida == Some(true) { pintor.circle(salida, rad, col, Stroke::new(1.0, c.fondo)); }
    }
    if !en_gpu && n.marcas.errores > 0 {
        pintor.rect_stroke(r.expand(3.0), radio, Stroke::new(2.5, c.error), StrokeKind::Outside);
    } else if !en_gpu && n.marcas.avisos > 0 {
        pintor.rect_stroke(r.expand(2.0), radio, Stroke::new(1.5, c.aviso), StrokeKind::Outside);
    }
    insignia(pintor, c, r.right_top() + Vec2::new(-4.0, -19.0), &n.marcas); // encima del borde: no tapa los botones
    // El clic derecho abre el editor en archivos, grupos y carpetas con errores; no prometerlo en el resto.
    let editable = n.tipo != Tipo::Carpeta || n.grupo.is_some() || n.marcas.errores + n.marcas.avisos > 0;
    let tooltip = |base: String| {
        let pista = if editable { "\nclic derecho: abrir en el editor" } else if n.tipo == Tipo::Carpeta && !n.raiz { "\nE o la diana: enfocar esta carpeta" } else { "" };
        let mut t = base;
        if let Some(rol) = n.rol { t.push_str("\n\n"); t.push_str(texto_rol(rol)); }
        if let Some(p) = &n.marcas.primero { t.push_str("\n\n"); t.push_str(p); }
        if let Some(m) = &n.marcas.museo {
            t.push_str("\n\n");
            t.push_str(m);
            if n.marcas.museo_total() > 1 { t.push_str(&format!("\n(+{} mas en el panel Museo)", n.marcas.museo_total() - 1)); }
        }
        t.push_str(pista);
        t
    };
    if z < 0.45 {
        // Lejos: sin iconos ni detalle, pero el nombre en letra fija y legible si cabe.
        if z >= 0.18 && r.width() > 40.0 {
            let mut t = n.nombre.clone();
            let cabe = ((r.width() - 8.0) / 6.0) as usize;
            if t.chars().count() > cabe { t = t.chars().take(cabe.saturating_sub(1)).collect::<String>() + "…"; }
            pintor.text(r.left_center() + Vec2::new(4.0, 0.0), Align2::LEFT_CENTER, t, FontId::proportional(10.5), c.texto);
        }
        resp.on_hover_text(tooltip(n.id.clone()));
        return;
    }
    // Titulo, icono y boton de desplegar.
    let tam_letra = 12.5 * z;
    let icono_r = Rect::from_min_size(cab.min + Vec2::new(5.0 * z, 3.0 * z), Vec2::splat(16.0 * z));
    if !iconos.pintar(ui, n.icono, icono_r, col) {
        // Estilo Neon: carpeta o pagina dibujadas (los lenguajes ya traen su logo).
        icono_dibujado(pintor, icono_r, n.tipo, c.texto, n.expandida == Some(true));
    }
    let mut titulo = n.nombre.clone();
    if titulo.chars().count() > 22 { titulo = titulo.chars().take(21).collect::<String>() + "…"; }
    pintor.text(cab.left_center() + Vec2::new(26.0 * z, 0.0), Align2::LEFT_CENTER, titulo, FontId::proportional(tam_letra), c.texto);
    if let (Some(abierta), false) = (n.expandida, n.raiz) {
        let boton = Rect::from_center_size(cab.right_center() - Vec2::new(12.0 * z, 0.0), Vec2::splat(16.0 * z));
        let rb = ui.interact(boton, ui.make_persistent_id(("desplegar", &n.id)), Sense::click());
        if rb.clicked() { acciones.push(Accion::Expandir(n.id.clone())); }
        pintor.rect(boton, 2, if rb.hovered() { col.gamma_multiply(0.5) } else { Color32::TRANSPARENT }, Stroke::new(1.0, c.texto.gamma_multiply(0.6)), StrokeKind::Inside);
        pintor.text(boton.center(), Align2::CENTER_CENTER, if abierta { "-" } else { "+" }, FontId::monospace(12.0 * z), c.texto);
    }
    // Play: lo que arranca este nodo (cargo run, make, python3...), como en VS Code.
    if let Some(titulo) = &n.play {
        let hueco = if n.expandida.is_some() && !n.raiz { 34.0 } else { 12.0 };
        let boton = Rect::from_center_size(cab.right_center() - Vec2::new(hueco * z, 0.0), Vec2::splat(16.0 * z));
        let rp = ui.interact(boton, ui.make_persistent_id(("play", &n.id)), Sense::click());
        triangulo_play(pintor, boton, if rp.hovered() { c.texto } else { c.verde });
        if rp.clicked() { acciones.push(Accion::Ejecutar(n.id.clone())); }
        rp.on_hover_text(format!("Ejecutar: {titulo}"));
    }
    // Enfocar (carpetas reales): la carpeta pasa a ser la raiz. En la raiz enfocada: subir.
    let es_carpeta_real = n.tipo == Tipo::Carpeta && n.grupo.is_none() && !n.raiz;
    let foco_raiz = n.raiz && !n.id.is_empty();
    if es_carpeta_real || foco_raiz {
        let hueco = if foco_raiz { if n.play.is_some() { 34.0 } else { 12.0 } } else if n.play.is_some() { 56.0 } else { 34.0 };
        let boton = Rect::from_center_size(cab.right_center() - Vec2::new(hueco * z, 0.0), Vec2::splat(16.0 * z));
        let rb = ui.interact(boton, ui.make_persistent_id(("enfocar", &n.id)), Sense::click());
        let tinta = if rb.hovered() { c.acento } else { c.texto.gamma_multiply(0.75) };
        if foco_raiz {
            flecha_arriba(pintor, boton, tinta);
            if rb.clicked() { acciones.push(Accion::Enfocar(n.id.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default())); }
            rb.on_hover_text("Subir el foco a la carpeta de arriba [U]");
        } else {
            diana(pintor, boton, tinta);
            if rb.clicked() { acciones.push(Accion::Enfocar(n.id.clone())); }
            rb.on_hover_text("Enfocar: esta carpeta pasa a ser la raiz del grafo y del explorador [E]");
        }
    }
    // Cuerpo: detalle y barra de peso (para ver de un vistazo que ocupa mas espacio).
    pintor.text(r.min + Vec2::new(10.0 * z, alto_cab + 8.0 * z), Align2::LEFT_TOP, &n.detalle, FontId::proportional(11.0 * z), c.tenue);
    if let Some(resumen) = &n.grupo {
        // Que piezas tiene la unidad: "h · cc · unittest · win".
        pintor.text(r.min + Vec2::new(10.0 * z, alto_cab + 24.0 * z), Align2::LEFT_TOP, resumen, FontId::monospace(10.0 * z), col.gamma_multiply(0.9));
    }
    let titulo_tip = if n.raiz { format!("{}\n{}", if n.id.is_empty() { "Carpeta abierta" } else { "Carpeta enfocada" }, n.detalle) }
                     else if let Some(r) = &n.grupo { format!("{} (grupo: {})\n{}\ndoble clic: abrir el grupo", n.nombre, r, n.detalle) }
                     else { format!("{}\n{}", n.id, n.detalle) };
    if en_gpu { resp.on_hover_text(tooltip(titulo_tip)); return; }
    let barra = Rect::from_min_size(Pos2::new(r.min.x + 10.0 * z, r.max.y - 12.0 * z), Vec2::new(r.width() - 20.0 * z, 4.0 * z));
    pintor.rect_filled(barra, 1.0, c.linea);
    if n.peso > 0.0 {
        let lleno = Rect::from_min_size(barra.min, Vec2::new(barra.width() * n.peso.clamp(0.0, 1.0), barra.height()));
        pintor.rect_filled(lleno, 1.0, if n.peso > 0.6 { c.error } else { col });
    }
    resp.on_hover_text(tooltip(titulo_tip));
}

/// Triangulo de play dibujado (no depende de que la fuente tenga el caracter).
pub fn triangulo_play(pintor: &egui::Painter, caja: Rect, color: Color32) {
    let c = caja.center();
    let (w, h) = (caja.width() * 0.32, caja.height() * 0.38);
    pintor.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-w * 0.8, -h), c + Vec2::new(w * 1.1, 0.0), c + Vec2::new(-w * 0.8, h)], color, Stroke::NONE));
}

/// Insignia de errores (roja) y avisos (amarilla), con la cuenta. Tamano fijo en pantalla
/// para que se lea a cualquier zoom. Se pinta hacia la izquierda desde `derecha`.
pub fn insignia(pintor: &egui::Painter, c: &Colores, derecha: Pos2, m: &Marcas) -> f32 {
    let mut x = derecha.x;
    for (n, fondo, letra, uno, varios) in [(m.coincidencias, c.acento, c.sobre_acento, "resultado", "resultados"), (m.avisos, c.aviso, c.fondo, "aviso", "avisos"),
                                           (m.museo_total(), Color32::WHITE, c.error, "museo", "museo"),
                                           (m.errores, c.error, Color32::WHITE, "error", "errores")] {
        if n == 0 { continue; }
        let t = carpetas_nucleo::diagnosticos::contar(n, uno, varios);
        let g = pintor.layout_no_wrap(t, FontId::proportional(11.0), letra);
        let caja = Rect::from_min_size(Pos2::new(x - g.size().x - 12.0, derecha.y), Vec2::new(g.size().x + 12.0, 17.0));
        pintor.rect_filled(caja, 8, fondo);
        pintor.galley(caja.center() - g.size() / 2.0, g, letra);
        x = caja.min.x - 4.0;
    }
    derecha.x - x
}

/// Tarjeta "en vivo": el arbol de procesos (cargo -> rustc -> programa) con CPU y memoria.
fn tarjeta_vivos(pintor: &egui::Painter, c: &Colores, arriba: Pos2, vv: &Vivos, lienzo: Rect) {
    let filas = carpetas_nucleo::procesos::arbol(vv.procesos);
    let (cpu, mem) = carpetas_nucleo::procesos::totales(vv.procesos);
    const MAX_FILAS: usize = 10;
    let alto = 30.0 + 18.0 * filas.len().clamp(1, MAX_FILAS + 1) as f32;
    let ancho = 300.0;
    // Que no se salga del lienzo.
    let x = arriba.x.clamp(lienzo.min.x + 4.0, (lienzo.max.x - ancho - 4.0).max(lienzo.min.x + 4.0));
    let y = arriba.y.min(lienzo.max.y - alto - 24.0).max(lienzo.min.y + 4.0);
    let caja = Rect::from_min_size(Pos2::new(x, y), Vec2::new(ancho, alto));
    pintor.rect(caja, 4, c.panel, Stroke::new(1.0, c.verde), StrokeKind::Inside);
    pintor.circle_filled(caja.min + Vec2::new(12.0, 14.0), 4.0, c.verde);
    let cab = format!("EN VIVO · {} · CPU {:.0}% · {}", vv.titulo, cpu, memoria(mem));
    pintor.text(caja.min + Vec2::new(22.0, 14.0), Align2::LEFT_CENTER, cab, FontId::proportional(12.0), c.texto);
    if filas.is_empty() {
        pintor.text(caja.min + Vec2::new(12.0, 36.0), Align2::LEFT_CENTER, "leyendo /proc…", FontId::proportional(11.0), c.tenue);
        return;
    }
    for (i, (nivel, p)) in filas.iter().take(MAX_FILAS).enumerate() {
        let fy = caja.min.y + 36.0 + i as f32 * 18.0;
        let fx = caja.min.x + 12.0 + *nivel as f32 * 12.0;
        if *nivel > 0 { pintor.line_segment([Pos2::new(fx - 8.0, fy - 6.0), Pos2::new(fx - 8.0, fy), ], Stroke::new(1.0, c.linea));
                        pintor.line_segment([Pos2::new(fx - 8.0, fy), Pos2::new(fx - 2.0, fy)], Stroke::new(1.0, c.linea)); }
        pintor.text(Pos2::new(fx, fy), Align2::LEFT_CENTER, format!("{} ({})", p.nombre, p.pid), FontId::monospace(11.0), c.texto);
        // Barra de CPU: llena = un nucleo entero.
        let barra = Rect::from_min_size(Pos2::new(caja.max.x - 130.0, fy - 4.0), Vec2::new(50.0, 8.0));
        pintor.rect_filled(barra, 2, c.linea);
        let lleno = (p.cpu / 100.0).clamp(0.0, 1.0);
        pintor.rect_filled(Rect::from_min_size(barra.min, Vec2::new(barra.width() * lleno, barra.height())), 2, if p.cpu > 90.0 { c.aviso } else { c.verde });
        pintor.text(Pos2::new(caja.max.x - 76.0, fy), Align2::LEFT_CENTER, format!("{:>3.0}%", p.cpu), FontId::monospace(10.5), c.tenue);
        pintor.text(Pos2::new(caja.max.x - 8.0, fy), Align2::RIGHT_CENTER, memoria(p.memoria_kb), FontId::monospace(10.5), c.tenue);
    }
    if filas.len() > MAX_FILAS {
        pintor.text(Pos2::new(caja.min.x + 12.0, caja.min.y + 36.0 + MAX_FILAS as f32 * 18.0), Align2::LEFT_CENTER,
                    format!("+{} procesos mas", filas.len() - MAX_FILAS), FontId::proportional(11.0), c.tenue);
    }
}

pub fn memoria(kb: u64) -> String {
    if kb >= 1024 * 1024 { format!("{:.1} GB", kb as f32 / 1048576.0) } else if kb >= 1024 { format!("{:.0} MB", kb as f32 / 1024.0) } else { format!("{kb} KB") }
}

/// Icono Neon dibujado con formas: carpeta con pestana (abierta si esta desplegada) o
/// pagina con la esquina doblada. Nitido a cualquier tamano y sin depender de la fuente.
pub fn icono_dibujado(p: &egui::Painter, caja: Rect, tipo: Tipo, col: Color32, abierta: bool) {
    let r = caja.shrink(caja.width() * 0.08);
    let (w, h) = (r.width(), r.height());
    let trazo = Stroke::new((w * 0.08).clamp(1.0, 2.0), col);
    if tipo == Tipo::Carpeta {
        let arriba = r.min.y + h * 0.18;
        let pestana = [r.min + Vec2::new(0.0, h * 0.18), r.min + Vec2::new(0.0, h * 0.05), r.min + Vec2::new(w * 0.38, h * 0.05),
                       r.min + Vec2::new(w * 0.48, h * 0.18)];
        p.add(egui::Shape::line(pestana.to_vec(), trazo));
        let cuerpo = Rect::from_min_max(Pos2::new(r.min.x, arriba), Pos2::new(r.max.x, r.max.y - h * 0.08));
        p.rect(cuerpo, 1.5, col.gamma_multiply(if abierta { 0.10 } else { 0.28 }), trazo, StrokeKind::Inside);
        if abierta {
            // Tapa abierta: un trapecio delante, como la carpeta abierta de los exploradores.
            let tapa = vec![Pos2::new(r.min.x + w * 0.12, arriba + h * 0.28), Pos2::new(r.max.x + w * 0.04, arriba + h * 0.28),
                            Pos2::new(r.max.x - w * 0.08, r.max.y - h * 0.08), Pos2::new(r.min.x, r.max.y - h * 0.08)];
            p.add(egui::Shape::convex_polygon(tapa, col.gamma_multiply(0.35), trazo));
        }
    } else {
        let pliegue = w * 0.32;
        let x0 = r.min.x + w * 0.12;
        let x1 = r.max.x - w * 0.12;
        let hoja = vec![Pos2::new(x0, r.min.y), Pos2::new(x1 - pliegue, r.min.y), Pos2::new(x1, r.min.y + pliegue), Pos2::new(x1, r.max.y), Pos2::new(x0, r.max.y)];
        p.add(egui::Shape::convex_polygon(hoja, col.gamma_multiply(0.10), trazo));
        p.add(egui::Shape::line(vec![Pos2::new(x1 - pliegue, r.min.y), Pos2::new(x1 - pliegue, r.min.y + pliegue), Pos2::new(x1, r.min.y + pliegue)], trazo));
        if tipo == Tipo::Enlace {
            p.line_segment([Pos2::new(x0 + w * 0.12, r.max.y - h * 0.2), Pos2::new(x1 - w * 0.12, r.min.y + h * 0.5)], trazo);
        }
    }
}

/// Diana dibujada (enfocar).
fn diana(p: &egui::Painter, caja: Rect, col: Color32) {
    let r = caja.width() * 0.36;
    p.circle_stroke(caja.center(), r, Stroke::new(1.3, col));
    p.circle_filled(caja.center(), r * 0.35, col);
}

/// Flecha hacia arriba dibujada (subir el foco).
pub fn flecha_arriba(p: &egui::Painter, caja: Rect, col: Color32) {
    let c = caja.center();
    let (w, h) = (caja.width() * 0.3, caja.height() * 0.32);
    let trazo = Stroke::new(1.6, col);
    p.line_segment([c + Vec2::new(0.0, h), c + Vec2::new(0.0, -h)], trazo);
    p.line_segment([c + Vec2::new(-w, -h + w), c + Vec2::new(0.0, -h)], trazo);
    p.line_segment([c + Vec2::new(w, -h + w), c + Vec2::new(0.0, -h)], trazo);
}

/// Migas de pan arriba a la izquierda del grafo: "chromium › net › http". Clic = enfocar ahi.
fn migas(ui: &egui::Ui, pintor: &egui::Painter, lienzo: Rect, migas: &[(String, String)], c: &Colores, acciones: &mut Vec<Accion>) {
    let mut x = lienzo.min.x + 10.0;
    let y = lienzo.min.y + 10.0;
    for (i, (id, nombre)) in migas.iter().enumerate() {
        let ultima = i + 1 == migas.len();
        let g = pintor.layout_no_wrap(nombre.clone(), FontId::proportional(12.5), if ultima { c.sobre_acento } else { c.texto });
        let caja = Rect::from_min_size(Pos2::new(x, y), Vec2::new(g.size().x + 16.0, 22.0));
        let r = ui.interact(caja, ui.make_persistent_id(("miga", id)), Sense::click());
        pintor.rect(caja, 4, if ultima { c.acento } else if r.hovered() { c.superficie } else { c.panel }, Stroke::new(1.0, c.linea), StrokeKind::Inside);
        pintor.galley(caja.min + Vec2::new(8.0, (22.0 - g.size().y) / 2.0), g, c.texto);
        if r.clicked() && !ultima { acciones.push(Accion::Enfocar(id.clone())); }
        r.on_hover_text(if ultima { "Raiz de la vista".to_string() } else { format!("Enfocar {nombre}") });
        x = caja.max.x + 4.0;
        if !ultima {
            // Chevron dibujado (no depende de la fuente).
            let m = Pos2::new(x + 6.0, y + 11.0);
            pintor.line_segment([m + Vec2::new(-2.5, -4.0), m + Vec2::new(2.0, 0.0)], Stroke::new(1.4, c.tenue));
            pintor.line_segment([m + Vec2::new(-2.5, 4.0), m + Vec2::new(2.0, 0.0)], Stroke::new(1.4, c.tenue));
            x += 14.0;
        }
    }
}
