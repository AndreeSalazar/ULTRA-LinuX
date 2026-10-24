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
use carpetas_nucleo::editor::{lenguaje, resaltar, Lenguaje, Token};
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

/// El texto como LayoutJob coloreado por linea (sin ajuste de linea: es codigo).
fn trabajo(texto: &str, len: Lenguaje, c: &Colores) -> LayoutJob {
    let fuente = FontId::monospace(LETRA);
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let formato = |col: Color32| TextFormat::simple(fuente.clone(), col);
    if texto.len() > MAX_RESALTADO || len == Lenguaje::Otro {
        job.append(texto, 0.0, formato(c.texto));
        return job;
    }
    for linea in texto.split_inclusive('\n') {
        let cuerpo = linea.strip_suffix('\n').unwrap_or(linea);
        for (a, z, t) in resaltar(cuerpo, len) { job.append(&cuerpo[a..z], 0.0, formato(color_token(t, c))); }
        if cuerpo.len() < linea.len() { job.append("\n", 0.0, formato(c.texto)); }
    }
    job
}

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
    pub fn pintar(&mut self, ui: &mut egui::Ui, e: &Estado, c: &Colores, acciones: &mut Vec<Accion>) -> u64 {
        let inicio = Instant::now();
        let Some(ed) = &e.editor else { self.foco = false; return 0 };
        if ed.carga != self.carga || ed.id != self.id {
            self.texto = ed.texto.clone();
            self.carga = ed.carga;
            self.id = ed.id.clone();
            self.galeria = None;
            self.saltos = 0;
        }
        if ed.saltos != self.saltos && ed.cargado { self.saltos = ed.saltos; self.saltar = ed.ir_a; }
        let diags = e.errores.de(&ed.id);
        // Cabecera: archivo, si hay cambios, guardar y cerrar.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("EDITOR").color(c.acento).strong());
            ui.label(egui::RichText::new(&ed.id).color(c.texto).monospace());
            if ed.sucio { ui.label(egui::RichText::new("sin guardar").color(c.aviso)); }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cerrar [Esc]").clicked() { acciones.push(Accion::Comando(Comando::CerrarEditor)); }
                if ui.add_enabled(ed.sucio, egui::Button::new("Guardar [Ctrl+S]")).clicked() { acciones.push(Accion::Comando(Comando::Guardar)); }
            });
        });
        if let Some(a) = &ed.aviso { ui.label(egui::RichText::new(a).color(if a.starts_with("Guardado") { c.verde } else { c.aviso })); }
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
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _ancho: f32| {
            let t = buf.as_str();
            let clave = fnv(t.as_bytes(), clave_tema) ^ t.len() as u64;
            if let Some((k, g)) = galeria { if *k == clave { return g.clone(); } }
            let g = ui.fonts_mut(|f| f.layout_job(trabajo(t, len, c)));
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
            for fila in &salida.galley.rows {
                let r = fila.rect().translate(salida.galley_pos.to_vec2());
                if nueva && Some(linea) == saltar { fila_salto = Some(r); }
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
            if salida.response.changed() { acciones.push(Accion::Editar(self.texto.clone())); }
            foco = salida.response.has_focus();
        });
        self.foco = foco;
        inicio.elapsed().as_micros() as u64
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
