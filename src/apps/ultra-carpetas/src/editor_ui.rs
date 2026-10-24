// SPDX-License-Identifier: GPL-2.0-only
//! Panel del editor de codigo (clic derecho en un nodo, F8 o clic en una linea de error).
//! El estado vive en el nucleo (Editor); aqui solo la copia que edita egui, el dibujo y
//! la cache del texto ya coloreado.
//!
//! Rendimiento: el texto coloreado (galley) se guarda en cache por huella del texto y
//! del tema, asi un cuadro sin teclear no vuelve a colorear nada. Los numeros de linea y
//! las marcas solo se pintan en las filas que se ven. Archivos de mas de 256 KiB se
//! muestran sin colores (siguen editables) para que el carril Editor siga en verde.

use crate::Colores;
use carpetas_nucleo::editor::{lenguaje, resaltar, AccionBuscar, Lenguaje, Token};
use carpetas_nucleo::{Accion, Comando, Estado};
use eframe::egui::{self, text::LayoutJob, Align, Align2, Color32, FontId, Galley, Pos2, Rect, Sense, Stroke, TextFormat, Vec2};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

const MAX_RESALTADO: usize = 256 * 1024;
const LETRA: f32 = 13.0;

#[derive(Default)]
pub struct PanelEditor {
    texto: String,
    id: String,
    carga: u64,
    saltos: u64,
    /// Hay que saltar a esta linea en cuanto haya galeria.
    saltar: Option<u32>,
    galeria: Option<(u64, Arc<Galley>)>,
    /// El cuadro de texto tiene el foco (Esc cierra el editor en vez de quitar seleccion).
    pub foco: bool,
    /// El cuadro de buscar o reemplazar tiene el foco.
    pub foco_busqueda: bool,
    texto_busqueda: String,
    texto_reemplazo: String,
    foco_pedido: u64,
}

fn fnv(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes { h ^= *b as u64; h = h.wrapping_mul(0x100_0000_01b3); }
    h
}

fn color_token(t: Token, c: &Colores) -> Color32 {
    match t {
        Token::Normal => c.texto,
        Token::Clave => c.acento,
        Token::Texto => c.verde,
        Token::Comentario => c.tenue,
        Token::Numero => c.aviso,
        Token::Simbolo => c.texto.gamma_multiply(0.75),
    }
}

/// Coincidencias de la busqueda a marcar: (tramos en bytes, cual es la actual).
pub struct Marcas<'a> { pub hallazgos: &'a [(usize, usize)], pub actual: usize }

/// El texto como LayoutJob coloreado por linea (sin ajuste de linea: es codigo), con las
/// coincidencias de la busqueda de fondo (la actual mas fuerte).
fn trabajo(texto: &str, len: Lenguaje, c: &Colores, marcas: &Marcas) -> LayoutJob {
    let fuente = FontId::monospace(LETRA);
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let fondo_hallazgo = c.aviso.gamma_multiply(0.30);
    let fondo_actual = c.acento.gamma_multiply(0.55);
    // Puntero a la siguiente coincidencia: las secciones se cortan en sus bordes.
    let mut k = 0;
    let tramo = |job: &mut LayoutJob, a: usize, z: usize, col: Color32, k: &mut usize| {
        let mut i = a;
        while i < z {
            while *k < marcas.hallazgos.len() && marcas.hallazgos[*k].1 <= i { *k += 1; }
            let (fin, fondo) = match marcas.hallazgos.get(*k) {
                Some(&(ha, hz)) if ha <= i => (hz.min(z), if *k == marcas.actual { fondo_actual } else { fondo_hallazgo }),
                Some(&(ha, _)) => (ha.min(z), Color32::TRANSPARENT),
                None => (z, Color32::TRANSPARENT),
            };
            let mut f = TextFormat::simple(fuente.clone(), col);
            f.background = fondo;
            job.append(&texto[i..fin], 0.0, f);
            i = fin;
        }
    };
    if texto.len() > MAX_RESALTADO || len == Lenguaje::Otro {
        tramo(&mut job, 0, texto.len(), c.texto, &mut k);
        return job;
    }
    let mut base = 0;
    for linea in texto.split_inclusive('\n') {
        let cuerpo = linea.strip_suffix('\n').unwrap_or(linea);
        for (a, z, t) in resaltar(cuerpo, len) { tramo(&mut job, base + a, base + z, color_token(t, c), &mut k); }
        if cuerpo.len() < linea.len() { tramo(&mut job, base + cuerpo.len(), base + linea.len(), c.texto, &mut k); }
        base += linea.len();
    }
    job
}

/// Indice de caracter de un byte del texto (egui cuenta caracteres).
fn caracter(texto: &str, byte: usize) -> usize { texto[..byte.min(texto.len())].chars().count() }

/// Indice de caracter del inicio de una linea (1 = la primera).
fn inicio_de_linea(texto: &str, linea: u32) -> usize {
    if linea <= 1 { return 0; }
    let mut n = 1;
    for (i, ch) in texto.chars().enumerate() {
        if ch == '\n' { n += 1; if n == linea { return i + 1; } }
    }
    texto.chars().count()
}

impl PanelEditor {
    /// Pinta el panel; devuelve los microsegundos gastados (carril Editor).
    pub fn pintar(&mut self, ui: &mut egui::Ui, e: &Estado, c: &Colores, iconos: &crate::svg::Iconos, acciones: &mut Vec<Accion>) -> u64 {
        let inicio = Instant::now();
        let Some(ed) = e.editor.activa() else { self.foco = false; self.foco_busqueda = false; return 0 };
        if ed.carga != self.carga || ed.id != self.id {
            self.texto = ed.texto.clone();
            self.carga = ed.carga;
            self.id = ed.id.clone();
            self.galeria = None;
            self.saltos = 0;
        }
        let mut saltar_rango = None;
        if ed.saltos != self.saltos && ed.cargado {
            self.saltos = ed.saltos;
            match ed.rango { Some(r) => saltar_rango = Some(r), None => self.saltar = ed.ir_a }
        }
        let diags = e.errores.de(&ed.id);
        self.pestanas(ui, e, c, iconos, acciones);
        // Cabecera: archivo, si hay cambios, guardar y cerrar.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&ed.id).color(c.tenue).monospace().size(11.5));
            if ed.sucio { ui.label(egui::RichText::new("sin guardar").color(c.aviso).size(11.5)); }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("Cerrar [Ctrl+W]").clicked() { acciones.push(Accion::Comando(Comando::CerrarEditor)); }
                if ui.add_enabled(ed.sucio, egui::Button::new("Guardar [Ctrl+S]").small()).clicked() { acciones.push(Accion::Comando(Comando::Guardar)); }
                if ui.small_button("Buscar [Ctrl+F]").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Abrir { reemplazar: false })); }
            });
        });
        if let Some(a) = &ed.aviso { ui.label(egui::RichText::new(a).color(if a.starts_with("Guardado") || a.ends_with("reemplazos") { c.verde } else { c.aviso })); }
        self.barra_busqueda(ui, e, c, acciones);
        // Lista de errores de este archivo: clic = saltar.
        if !diags.is_empty() {
            egui::ScrollArea::vertical().id_salt("editor-errores").max_height(92.0).show(ui, |ui| {
                for d in diags.iter().take(100) {
                    let texto = format!("{:>5}:{:<3} {}", d.linea, d.columna, d.mensaje);
                    let col = if d.grave { c.error } else { c.aviso };
                    if ui.add(egui::Label::new(egui::RichText::new(texto).monospace().size(11.5).color(col)).sense(Sense::click()).truncate())
                        .on_hover_text("clic: ir a la linea").clicked() { self.saltar = Some(d.linea); }
                }
            });
            ui.separator();
        }
        if !ed.cargado {
            ui.label(egui::RichText::new(if ed.aviso.is_some() { "no se pudo abrir" } else { "leyendo…" }).color(c.tenue));
            return inicio.elapsed().as_micros() as u64;
        }
        // Lineas marcadas: la peor gana (error sobre aviso).
        let mut marcadas: BTreeMap<u32, (bool, String)> = BTreeMap::new();
        for d in diags {
            let m = marcadas.entry(d.linea).or_insert((d.grave, String::new()));
            m.0 |= d.grave;
            if !m.1.is_empty() { m.1.push('\n'); }
            m.1.push_str(&d.mensaje);
        }
        let len = lenguaje(&ed.id);
        let lineas_total = self.texto.matches('\n').count() + 1;
        let margen = 18.0 + 8.0 * (lineas_total.max(10).ilog10() + 1) as f32;
        let id_texto = ui.make_persistent_id(("editor-texto", &ed.id));
        let clave_tema = fnv(&[c.texto.r(), c.acento.r(), c.acento.g(), c.acento.b(), c.verde.g(), c.tenue.r()], 0xcbf2_9ce4_8422_2325);
        let galeria = &mut self.galeria;
        let vacio: &[(usize, usize)] = &[];
        let marcas = match &e.editor.busqueda {
            Some(b) => Marcas { hallazgos: &b.hallazgos, actual: b.actual },
            None => Marcas { hallazgos: vacio, actual: 0 },
        };
        // La huella de las marcas entra en la clave: buscar otra cosa recolorea, teclear igual no.
        let clave_marcas = marcas.hallazgos.iter().fold(fnv(&marcas.actual.to_le_bytes(), clave_tema), |h, (a, z)| fnv(&(*a as u64 ^ (*z as u64) << 32).to_le_bytes(), h));
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _ancho: f32| {
            let t = buf.as_str();
            let clave = fnv(t.as_bytes(), clave_marcas) ^ t.len() as u64;
            if let Some((k, g)) = galeria { if *k == clave { return g.clone(); } }
            // Si el texto de egui aun no es el del nucleo (tecla a medio camino), sin marcas.
            let m = if t.len() == ed.texto.len() { &marcas } else { &Marcas { hallazgos: vacio, actual: 0 } };
            let g = ui.fonts_mut(|f| f.layout_job(trabajo(t, len, c, m)));
            *galeria = Some((clave, g.clone()));
            g
        };
        let saltar = self.saltar.take();
        let mut foco = false;
        egui::ScrollArea::both().id_salt("editor-codigo").auto_shrink([false, false]).show(ui, |ui| {
            // Hueco a la izquierda para los numeros de linea (sin marco, el margen del TextEdit no cuenta).
            let salida = ui.horizontal_top(|ui| {
                ui.add_space(margen);
                egui::TextEdit::multiline(&mut self.texto).id(id_texto).code_editor().font(FontId::monospace(LETRA))
                    .desired_width(f32::INFINITY).desired_rows(30).lock_focus(true)
                    .frame(egui::Frame::NONE).layouter(&mut layouter).show(ui)
            }).inner;
            let pintor = ui.painter();
            let visible = ui.clip_rect();
            // Numeros de linea y marcas: solo las filas que se ven.
            let mut linea = 1u32;
            let mut nueva = true;
            let x_num = salida.galley_pos.x - 10.0;
            let mut fila_salto = None;
            let mut fila_rango = None;
            let mut chars_vistos = 0usize;
            let destino_rango = saltar_rango.map(|(ra, _)| caracter(&self.texto, ra));
            for fila in &salida.galley.rows {
                let r = fila.rect().translate(salida.galley_pos.to_vec2());
                if nueva && Some(linea) == saltar { fila_salto = Some(r); }
                let fin_fila = chars_vistos + fila.char_count_including_newline().0;
                if fila_rango.is_none() && destino_rango.is_some_and(|d| fin_fila > d) { fila_rango = Some(r); }
                chars_vistos = fin_fila;
                if nueva && r.max.y >= visible.min.y && r.min.y <= visible.max.y {
                    let marca = marcadas.get(&linea);
                    let col = match marca { Some((true, _)) => c.error, Some((false, _)) => c.aviso, None => c.tenue.gamma_multiply(0.7) };
                    if let Some((grave, msg)) = marca {
                        let franja = Rect::from_min_max(Pos2::new(salida.galley_pos.x - margen, r.min.y), Pos2::new(visible.max.x, r.max.y));
                        pintor.rect_filled(franja, 0.0, (if *grave { c.error } else { c.aviso }).gamma_multiply(0.10));
                        pintor.circle_filled(Pos2::new(salida.galley_pos.x - margen + 6.0, r.center().y), 3.5, col);
                        let gut = Rect::from_min_max(Pos2::new(salida.galley_pos.x - margen, r.min.y), Pos2::new(salida.galley_pos.x, r.max.y));
                        ui.interact(gut, ui.make_persistent_id(("editor-marca", linea)), Sense::hover()).on_hover_text(msg.as_str());
                    }
                    pintor.text(Pos2::new(x_num, r.center().y), Align2::RIGHT_CENTER, linea.to_string(), FontId::monospace(LETRA - 1.5), col);
                }
                nueva = fila.ends_with_newline;
                if nueva { linea += 1; }
            }
            pintor.line_segment([Pos2::new(salida.galley_pos.x - 4.0, visible.min.y), Pos2::new(salida.galley_pos.x - 4.0, visible.max.y)],
                                Stroke::new(1.0, c.linea));
            if let (Some(l), Some(r)) = (saltar, fila_salto) {
                // Cursor al inicio de la linea y la linea al centro de la vista.
                let i = inicio_de_linea(&self.texto, l);
                if let Some(mut estado) = egui::TextEdit::load_state(ui.ctx(), id_texto) {
                    estado.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(i))));
                    estado.store(ui.ctx(), id_texto);
                }
                ui.memory_mut(|m| m.request_focus(id_texto));
                ui.scroll_to_rect(r.expand2(Vec2::new(0.0, 80.0)), Some(Align::Center));
            }
            if let (Some((ra, rz)), Some(r)) = (saltar_rango, fila_rango) {
                // Coincidencia de la busqueda: se selecciona y se centra, sin quitar el foco al cuadro de buscar.
                if let Some(mut estado) = egui::TextEdit::load_state(ui.ctx(), id_texto) {
                    let (a, z) = (caracter(&self.texto, ra), caracter(&self.texto, rz));
                    estado.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(a), egui::text::CCursor::new(z))));
                    estado.store(ui.ctx(), id_texto);
                }
                ui.scroll_to_rect(r.expand2(Vec2::new(0.0, 80.0)), Some(Align::Center));
            }
            if salida.response.changed() { acciones.push(Accion::Editar(self.texto.clone())); }
            foco = salida.response.has_focus();
        });
        self.foco = foco;
        inicio.elapsed().as_micros() as u64
    }

    /// Barra de pestanas: nombre, punto si hay cambios, insignia de errores y cerrar.
    fn pestanas(&self, ui: &mut egui::Ui, e: &Estado, c: &Colores, iconos: &crate::svg::Iconos, acciones: &mut Vec<Accion>) {
        egui::ScrollArea::horizontal().id_salt("editor-pestanas").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let tema = e.ajustes.tema(e.acento);
                for (i, ed) in e.editor.lista.iter().enumerate() {
                    let activa = i == e.editor.activa;
                    let (err, av) = e.errores.conteo(&ed.id);
                    let col_nombre = if err > 0 { c.error } else if av > 0 { c.aviso } else if activa { c.texto } else { c.tenue };
                    let fuente = FontId::proportional(12.5);
                    let g = ui.painter().layout_no_wrap(ed.nombre().to_string(), fuente.clone(), col_nombre);
                    let ancho = g.size().x + 64.0 + if err > 0 { 18.0 } else { 0.0 };
                    let (rect, r) = ui.allocate_exact_size(Vec2::new(ancho, 26.0), Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, if activa { c.fondo } else if r.hovered() { c.superficie } else { c.panel });
                    if activa { p.rect_filled(Rect::from_min_size(rect.min, Vec2::new(rect.width(), 2.0)), 0.0, c.acento); }
                    // Logo del lenguaje (o el icono dibujado), como en el explorador.
                    let tipo = carpetas_nucleo::tipo_por_extension(ed.nombre());
                    let caja_icono = Rect::from_center_size(Pos2::new(rect.min.x + 17.0, rect.center().y), Vec2::splat(15.0));
                    let clave = carpetas_nucleo::iconos::icono(ed.nombre(), tipo, e.ajustes.iconos);
                    let [r_, g_, b_] = tema.color_tipo(tipo);
                    let col_tipo = Color32::from_rgb(r_, g_, b_);
                    if clave.is_empty() || !iconos.pintar(ui, clave, caja_icono, col_tipo) {
                        crate::pintar_grafo::icono_dibujado(&p, caja_icono, tipo, col_tipo, false);
                    }
                    p.galley(Pos2::new(rect.min.x + 30.0, rect.center().y - g.size().y / 2.0), g, col_nombre);
                    if err > 0 {
                        let centro = Pos2::new(rect.max.x - 36.0, rect.center().y);
                        p.circle_filled(centro, 7.0, c.error);
                        p.text(centro, Align2::CENTER_CENTER, err.min(99).to_string(), FontId::proportional(9.5), Color32::WHITE);
                    }
                    // Cerrar: una X dibujada; con cambios sin guardar, un punto (como VS Code) que pasa a X al pasar el raton.
                    let caja = Rect::from_center_size(Pos2::new(rect.max.x - 14.0, rect.center().y), Vec2::splat(16.0));
                    let rc = ui.interact(caja, ui.make_persistent_id(("pestana-cerrar", &ed.id)), Sense::click());
                    if ed.sucio && !rc.hovered() {
                        p.circle_filled(caja.center(), 4.0, c.aviso);
                    } else if rc.hovered() || activa || r.hovered() {
                        if rc.hovered() { p.rect_filled(caja, 3, c.superficie); }
                        let k = 4.0;
                        let trazo = Stroke::new(1.4, if rc.hovered() { c.texto } else { c.tenue });
                        p.line_segment([caja.center() + Vec2::new(-k, -k), caja.center() + Vec2::new(k, k)], trazo);
                        p.line_segment([caja.center() + Vec2::new(-k, k), caja.center() + Vec2::new(k, -k)], trazo);
                    }
                    if rc.clicked() || r.middle_clicked() { acciones.push(Accion::CerrarPestana(i)); continue; }
                    if r.clicked() { acciones.push(Accion::Pestana(i)); }
                    r.on_hover_text(format!("{}{}", ed.id, if ed.sucio { "\nsin guardar" } else { "" }));
                }
            });
        });
        ui.add_space(2.0);
    }

    /// Buscar (Ctrl+F) y reemplazar (Ctrl+H): Enter siguiente, Shift+Enter anterior, Esc cierra.
    fn barra_busqueda(&mut self, ui: &mut egui::Ui, e: &Estado, c: &Colores, acciones: &mut Vec<Accion>) {
        let Some(b) = &e.editor.busqueda else { self.foco_busqueda = false; return };
        if b.foco != self.foco_pedido { self.texto_busqueda = b.patron.clone(); self.texto_reemplazo = b.reemplazo.clone(); }
        let mut foco = false;
        egui::Frame::new().fill(c.superficie).corner_radius(4).inner_margin(6.0).stroke(Stroke::new(1.0, c.linea)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto_busqueda).hint_text("buscar").desired_width(200.0).font(egui::TextStyle::Monospace));
                if b.foco != self.foco_pedido { self.foco_pedido = b.foco; r.request_focus(); }
                if r.changed() { acciones.push(Accion::Buscar(AccionBuscar::Patron(self.texto_busqueda.clone()))); }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let atras = ui.input(|i| i.modifiers.shift);
                    acciones.push(Accion::Buscar(if atras { AccionBuscar::Anterior } else { AccionBuscar::Siguiente }));
                    r.request_focus();
                }
                foco |= r.has_focus();
                if ui.selectable_label(b.mayusculas, "Aa").on_hover_text("Distinguir mayusculas").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Mayusculas)); }
                if ui.selectable_label(b.palabra, "Palabra").on_hover_text("Solo palabras enteras").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Palabra)); }
                let n = b.hallazgos.len();
                let cuenta = if b.patron.is_empty() { String::new() } else if n == 0 { "sin resultados".into() }
                             else { format!("{} de {}{}", b.actual + 1, n, if b.demasiados { "+" } else { "" }) };
                ui.label(egui::RichText::new(cuenta).monospace().size(11.5).color(if n == 0 && !b.patron.is_empty() { c.error } else { c.tenue }));
                if ui.small_button("Anterior").on_hover_text("Shift+Enter o Shift+F3").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Anterior)); }
                if ui.small_button("Siguiente").on_hover_text("Enter o F3").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Siguiente)); }
                if !b.reemplazar && ui.small_button("Reemplazar…").on_hover_text("Ctrl+H").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Abrir { reemplazar: true })); }
                if ui.small_button("Cerrar").on_hover_text("Esc").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Cerrar)); }
            });
            if b.reemplazar {
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut self.texto_reemplazo).hint_text("reemplazar por").desired_width(200.0).font(egui::TextStyle::Monospace));
                    if r.changed() { acciones.push(Accion::Buscar(AccionBuscar::Reemplazo(self.texto_reemplazo.clone()))); }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { acciones.push(Accion::Buscar(AccionBuscar::Reemplazar)); r.request_focus(); }
                    foco |= r.has_focus();
                    let hay = !b.hallazgos.is_empty();
                    if ui.add_enabled(hay, egui::Button::new("Reemplazar").small()).on_hover_text("Esta coincidencia (Enter)").clicked() { acciones.push(Accion::Buscar(AccionBuscar::Reemplazar)); }
                    if ui.add_enabled(hay, egui::Button::new("Todo").small()).on_hover_text("Todas de una vez; Ctrl+Z en el editor lo deshace").clicked() { acciones.push(Accion::Buscar(AccionBuscar::ReemplazarTodo)); }
                    ui.label(egui::RichText::new("texto literal, sin expresiones regulares").size(10.5).color(c.tenue));
                });
            }
        });
        self.foco_busqueda = foco;
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn inicio_de_linea_en_caracteres() {
        let t = "añ\nb\n\nc";
        assert_eq!(inicio_de_linea(t, 1), 0);
        assert_eq!(inicio_de_linea(t, 2), 3, "en caracteres, no bytes");
        assert_eq!(inicio_de_linea(t, 4), 6);
        assert_eq!(inicio_de_linea(t, 99), t.chars().count());
    }
}
