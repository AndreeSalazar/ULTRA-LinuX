// SPDX-License-Identifier: GPL-2.0-only
//! Barra de actividad (como VS Code), "Ir a archivo" (Ctrl+P) y "Buscar en el proyecto"
//! (Ctrl+Shift+F). Solo dibujan el Estado y traducen clics a Acciones.

use crate::{color, pintar_grafo, App, Colores};
use carpetas_nucleo::diagnosticos::contar;
use carpetas_nucleo::estado::Lateral;
use carpetas_nucleo::proyecto::{letras_marcadas, AccionIrA, AccionProyecto, Coincidencia};
use carpetas_nucleo::{Accion, Comando, Modo, Tipo};
use eframe::egui::{self, text::LayoutJob, Align, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, TextFormat, Vec2};

/// Iconos de la barra de actividad, dibujados con formas (nitidos y sin fuentes).
#[derive(Clone, Copy, PartialEq)]
pub enum Icono { Explorador, Buscar, IrA, Grafo, Ciudad, Museo, Casm, Terminal, Carriles, Ajustes }

pub fn dibujar(p: &egui::Painter, caja: Rect, i: Icono, col: Color32) {
    let c = caja.center();
    let t = Stroke::new(1.7, col);
    match i {
        Icono::Explorador => {
            p.rect_stroke(Rect::from_center_size(c + Vec2::new(-3.0, -2.0), Vec2::new(12.0, 15.0)), 1, t, StrokeKind::Middle);
            p.rect_stroke(Rect::from_center_size(c + Vec2::new(3.0, 3.0), Vec2::new(12.0, 15.0)), 1, t, StrokeKind::Middle);
        }
        Icono::Buscar => {
            p.circle_stroke(c + Vec2::new(-2.0, -2.0), 6.0, t);
            p.line_segment([c + Vec2::new(2.5, 2.5), c + Vec2::new(8.0, 8.0)], Stroke::new(2.4, col));
        }
        Icono::IrA => {
            p.rect_stroke(Rect::from_center_size(c + Vec2::new(2.0, 0.0), Vec2::new(12.0, 16.0)), 1, t, StrokeKind::Middle);
            p.line_segment([c + Vec2::new(-9.0, 0.0), c + Vec2::new(3.0, 0.0)], t);
            p.line_segment([c + Vec2::new(-1.0, -4.0), c + Vec2::new(3.0, 0.0)], t);
            p.line_segment([c + Vec2::new(-1.0, 4.0), c + Vec2::new(3.0, 0.0)], t);
        }
        Icono::Grafo => {
            let (a, b, d) = (c + Vec2::new(-7.0, 0.0), c + Vec2::new(6.0, -6.0), c + Vec2::new(6.0, 6.0));
            p.line_segment([a, b], t);
            p.line_segment([a, d], t);
            for q in [a, b, d] { p.circle_filled(q, 3.2, col); }
        }
        Icono::Ciudad => {
            // Tres edificios de alturas distintas sobre el suelo.
            let suelo = c.y + 8.0;
            for (x, h, w) in [(-8.0, 9.0, 5.0), (-2.0, 15.0, 5.0), (4.0, 11.0, 5.0)] {
                p.rect_stroke(Rect::from_min_max(Pos2::new(c.x + x, suelo - h), Pos2::new(c.x + x + w, suelo)), 0, t, StrokeKind::Middle);
            }
            p.line_segment([Pos2::new(c.x - 10.0, suelo), Pos2::new(c.x + 11.0, suelo)], t);
        }
        Icono::Museo => {
            // Un museo: frontón y tres columnas sobre la base.
            p.line_segment([c + Vec2::new(-10.0, -4.0), c + Vec2::new(0.0, -10.0)], t);
            p.line_segment([c + Vec2::new(0.0, -10.0), c + Vec2::new(10.0, -4.0)], t);
            p.line_segment([c + Vec2::new(-10.0, -4.0), c + Vec2::new(10.0, -4.0)], t);
            for x in [-6.5, 0.0, 6.5] { p.line_segment([c + Vec2::new(x, -2.0), c + Vec2::new(x, 6.0)], Stroke::new(2.2, col)); }
            p.line_segment([c + Vec2::new(-11.0, 8.5), c + Vec2::new(11.0, 8.5)], Stroke::new(2.0, col));
        }
        Icono::Casm => {
            // Una katana: hoja en diagonal, guarda y empunadura.
            let (punta, guarda) = (c + Vec2::new(9.0, -9.0), c + Vec2::new(-3.5, 3.5));
            p.line_segment([guarda, punta], Stroke::new(2.2, col));
            p.line_segment([guarda + Vec2::new(-3.0, -3.0), guarda + Vec2::new(3.0, 3.0)], Stroke::new(2.4, col));
            p.line_segment([guarda + Vec2::new(-1.0, 1.0), c + Vec2::new(-9.0, 9.0)], Stroke::new(3.0, col));
        }
        Icono::Terminal => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(20.0, 15.0)), 2, t, StrokeKind::Middle);
            p.line_segment([c + Vec2::new(-6.0, -3.0), c + Vec2::new(-2.0, 0.0)], t);
            p.line_segment([c + Vec2::new(-6.0, 3.0), c + Vec2::new(-2.0, 0.0)], t);
            p.line_segment([c + Vec2::new(0.0, 4.0), c + Vec2::new(5.0, 4.0)], t);
        }
        Icono::Carriles => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(10.0, 20.0)), 3, t, StrokeKind::Middle);
            for (k, f) in [(-5.5, Color32::from_rgb(255, 42, 79)), (0.0, Color32::from_rgb(252, 238, 10)), (5.5, Color32::from_rgb(61, 255, 154))] {
                p.circle_filled(c + Vec2::new(0.0, k), 2.2, f.gamma_multiply(0.9));
            }
        }
        Icono::Ajustes => {
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = Vec2::new(a.cos(), a.sin());
                p.line_segment([c + d * 5.5, c + d * 9.0], Stroke::new(2.6, col));
            }
            p.circle_stroke(c, 5.5, t);
            p.circle_filled(c, 2.0, col);
        }
    }
}

/// Una fila de la lista de resultados (aplanada para dibujar solo lo visible).
enum FilaB<'a> { Archivo(&'a str, usize, bool), Linea(&'a str, usize, &'a Coincidencia) }

impl App {
    /// Barra de actividad vertical: explorador, buscar, ir a, grafo, terminal, carriles, ajustes.
    pub(crate) fn actividad(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        let e = &self.estado;
        let lateral = e.ajustes.explorador;
        let modo = e.ajustes.modo;
        let pasos = |destino: Modo| -> usize { let i = |m: Modo| match m { Modo::Iconos => 0, Modo::Grafo => 1, Modo::Ciudad => 2 }; (i(destino) + 3 - i(modo)) % 3 };
        let museo = lateral && e.lateral == Lateral::Museo;
        let taller = lateral && e.lateral == Lateral::Casm;
        let botones: [(Icono, bool, &str, Accion); 8] = [
            (Icono::Explorador, lateral && e.lateral == Lateral::Explorador, "Explorador [Ctrl+B]",
             if lateral && e.lateral == Lateral::Explorador { Accion::Comando(Comando::Explorador) } else { Accion::Lateral(Lateral::Explorador) }),
            (Icono::Buscar, lateral && e.lateral == Lateral::Buscar, "Buscar en el proyecto [Ctrl+Shift+F]",
             if lateral && e.lateral == Lateral::Buscar { Accion::Comando(Comando::Explorador) } else { Accion::Proyecto(AccionProyecto::Abrir) }),
            (Icono::IrA, e.ir_a.is_some(), "Ir a archivo o carpeta [Ctrl+P]: lo busca en todo el proyecto y lo muestra en el grafo y el explorador",
             Accion::Comando(Comando::IrA)),
            (Icono::Grafo, modo == Modo::Grafo, "Grafo de nodos [Tab]", if modo == Modo::Grafo { Accion::Comando(Comando::Encuadrar) } else { Accion::Comando(Comando::Modo) }),
            (Icono::Ciudad, modo == Modo::Ciudad, "Ciudad 3D: carpetas = distritos, archivos = edificios (huella = tamano, altura = lineas) [Tab]",
             Accion::Comando(Comando::Ciudad)),
            (Icono::Museo, museo, "SUPER GRAFO // Museo: fallo conectado, causa probable y las fichas de C, C++ y ASM que aparecen en tu codigo [A: aislar]",
             if museo { Accion::Comando(Comando::Explorador) } else { Accion::Lateral(Lateral::Museo) }),
            (Icono::Casm, taller, "Taller CASM // SAMURAI: como se genera el .hex, las katanas (donde corta cada regla) y las compilaciones",
             if taller { Accion::Comando(Comando::Explorador) } else { Accion::Lateral(Lateral::Casm) }),
            (Icono::Terminal, e.terminal.abierta, "Terminal [Ctrl+`]", Accion::Comando(Comando::Terminal)),
        ];
        ui.add_space(4.0);
        let hay_fallo = e.conectado.fallos > 0;
        for (icono, activo, ayuda, accion) in botones {
            let r = self.boton_actividad(ui, c, icono, activo);
            if icono == Icono::Museo && hay_fallo {
                // Punto rojo con nucleo blanco: algo falla.
                let q = r.rect.right_top() + Vec2::new(-11.0, 10.0);
                ui.painter().circle_filled(q, 4.5, c.error);
                ui.painter().circle_filled(q, 1.8, Color32::WHITE);
            }
            if r.on_hover_text(ayuda).clicked() {
                // El boton de grafo lleva a esa vista desde cualquiera (Tab va en orden).
                if icono == Icono::Grafo && modo != Modo::Grafo { for _ in 0..pasos(Modo::Grafo) { acciones.push(Accion::Comando(Comando::Modo)); } } else { acciones.push(accion); }
            }
        }
        ui.with_layout(egui::Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(6.0);
            if self.boton_actividad(ui, c, Icono::Ajustes, self.estado.panel_ajustes).on_hover_text("Ajustes y temas [Ctrl+,]").clicked() {
                acciones.push(Accion::Comando(Comando::Ajustes));
            }
            if self.boton_actividad(ui, c, Icono::Carriles, self.estado.carriles).on_hover_text("Carriles con semaforo [F12]").clicked() {
                acciones.push(Accion::Comando(Comando::Carriles));
            }
        });
        for a in acciones { self.despachar(a); }
    }

    fn boton_actividad(&self, ui: &mut egui::Ui, c: &Colores, icono: Icono, activo: bool) -> egui::Response {
        let (caja, r) = ui.allocate_exact_size(Vec2::new(44.0, 42.0), Sense::click());
        let p = ui.painter_at(caja);
        if r.hovered() { p.rect_filled(caja.shrink(3.0), 4, c.superficie); }
        if activo { p.rect_filled(Rect::from_min_size(caja.min, Vec2::new(2.5, caja.height())), 0.0, c.acento); }
        dibujar(&p, caja, icono, if activo { c.acento } else if r.hovered() { c.texto } else { c.tenue });
        r
    }

    /// Ctrl+P: buscar por nombre en todo el proyecto y mostrarlo en las dos vistas.
    pub(crate) fn ir_a(&mut self, ctx: &egui::Context, c: &Colores) {
        let Some(ir) = &self.estado.ir_a else { self.texto_ir_a.clear(); return };
        let mut acciones = Vec::new();
        let info = self.estado.indice;
        egui::Area::new(egui::Id::unique("ultra-carpetas-ir-a")).anchor(Align2::CENTER_TOP, [0.0, 64.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.acento)).corner_radius(c.radio()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(660.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("IR A ARCHIVO O CARPETA").color(c.acento).strong());
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let estado = if !info.listo { "indexando el proyecto…".to_string() }
                                     else { format!("{} archivos · {} carpetas{}", miles(info.archivos), miles(info.carpetas), if info.truncado { " (limite)" } else { "" }) };
                        ui.label(egui::RichText::new(estado).color(c.tenue).size(11.0));
                    });
                });
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto_ir_a).hint_text("nombre, iniciales (fpw = file_path_watcher) o ruta con /")
                    .desired_width(640.0).font(egui::TextStyle::Monospace));
                r.request_focus();
                if r.changed() { acciones.push(Accion::IrA(AccionIrA::Texto(self.texto_ir_a.clone()))); }
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(420.0).min_scrolled_height(if ir.resultados.len() > 14 { 400.0 } else { 0.0 }).show(ui, |ui| {
                    for (i, (ruta, carpeta)) in ir.resultados.iter().enumerate() {
                        let elegido = i == ir.elegido;
                        let (fila, rf) = ui.allocate_exact_size(Vec2::new(640.0, 26.0), Sense::click());
                        let p = ui.painter_at(fila);
                        if elegido { p.rect_filled(fila, 3, c.acento.gamma_multiply(0.22)); } else if rf.hovered() { p.rect_filled(fila, 3, c.superficie); }
                        let (dir, nombre) = ruta.rsplit_once('/').unwrap_or(("", ruta));
                        let tipo = if *carpeta { Tipo::Carpeta } else { carpetas_nucleo::tipo_por_extension(nombre) };
                        let ic = Rect::from_center_size(Pos2::new(fila.min.x + 14.0, fila.center().y), Vec2::splat(16.0));
                        let clave = carpetas_nucleo::iconos::icono(nombre, tipo, self.estado.ajustes.iconos);
                        let col_tipo = color(self.estado.ajustes.tema(self.estado.acento).color_tipo(tipo));
                        if clave.is_empty() || !self.iconos.pintar(ui, clave, ic, col_tipo) { pintar_grafo::icono_dibujado(&p, ic, tipo, col_tipo, false); }
                        // Letras que casan, en el color del acento.
                        let marcadas = letras_marcadas(&self.texto_ir_a, nombre);
                        let mut job = LayoutJob::default();
                        for (k, ch) in nombre.chars().enumerate() {
                            let mut f = TextFormat::simple(FontId::proportional(13.5), if marcadas.contains(&k) { c.acento } else { c.texto });
                            if marcadas.contains(&k) { f.underline = Stroke::new(1.0, c.acento); }
                            job.append(&ch.to_string(), 0.0, f);
                        }
                        job.append(&format!("   {dir}"), 0.0, TextFormat::simple(FontId::proportional(11.5), c.tenue));
                        let g = ui.fonts_mut(|f| f.layout_job(job));
                        p.galley(Pos2::new(ic.max.x + 8.0, fila.center().y - g.size().y / 2.0), g, c.texto);
                        if *carpeta { p.text(fila.right_center() - Vec2::new(8.0, 0.0), Align2::RIGHT_CENTER, "carpeta · Ctrl+Enter enfoca", FontId::proportional(10.5), c.tenue); }
                        if elegido { rf.scroll_to_me(None); }
                        if rf.clicked() { acciones.push(Accion::IrA(AccionIrA::Mover(i as i32 - ir.elegido as i32))); acciones.push(Accion::IrA(AccionIrA::Aceptar { editor: ui.input(|i| i.modifiers.command) })); }
                    }
                });
                let pie = if ir.buscando { "buscando…".to_string() }
                          else if ir.resultados.is_empty() && !self.texto_ir_a.trim().is_empty() { "nada con ese nombre".to_string() }
                          else { "Enter: mostrar en el grafo y el explorador · Ctrl+Enter: abrir en el editor (o enfocar la carpeta) · Esc".to_string() };
                ui.label(egui::RichText::new(pie).color(c.tenue).size(11.0));
            });
        });
        for a in acciones { self.despachar(a); }
    }

    /// Panel lateral "Buscar": texto en todos los archivos, agrupado por archivo.
    pub(crate) fn panel_buscar(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        let b = &self.estado.proyecto;
        let raiz = carpetas_nucleo::grafo::raiz_vista(&self.estado).to_string();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("BUSCAR EN EL PROYECTO").color(c.acento).strong());
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if !b.patron.is_empty() && ui.small_button("Limpiar").clicked() { acciones.push(Accion::Proyecto(AccionProyecto::Limpiar)); }
            });
        });
        let donde = if raiz.is_empty() { format!("en todo {}", self.estado.nombre_raiz) } else { format!("solo en {raiz} (enfocada)") };
        ui.label(egui::RichText::new(donde).color(c.tenue).size(11.0));
        if b.foco != self.foco_buscar_visto { self.texto_buscar = b.patron.clone(); self.texto_filtro = b.filtro.clone(); }
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.texto_buscar).hint_text("texto a buscar").desired_width(ui.available_width() - 92.0)
                .font(egui::TextStyle::Monospace));
            if b.foco != self.foco_buscar_visto { self.foco_buscar_visto = b.foco; r.request_focus(); }
            if r.changed() { acciones.push(Accion::Proyecto(AccionProyecto::Patron(self.texto_buscar.clone()))); }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { acciones.push(Accion::Proyecto(AccionProyecto::Lanzar)); r.request_focus(); }
            if ui.selectable_label(b.mayusculas, "Aa").on_hover_text("Distinguir mayusculas").clicked() { acciones.push(Accion::Proyecto(AccionProyecto::Mayusculas)); }
            if ui.selectable_label(b.palabra, "ab").on_hover_text("Solo palabras enteras").clicked() { acciones.push(Accion::Proyecto(AccionProyecto::Palabra)); }
        });
        let rf = ui.add(egui::TextEdit::singleline(&mut self.texto_filtro).hint_text("archivos: *.cc, *.h, src/, !third_party/")
            .desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
        if rf.changed() { acciones.push(Accion::Proyecto(AccionProyecto::Filtro(self.texto_filtro.clone()))); }
        let archivos = b.por_archivo.len() as u32;
        let estado = if !self.estado.indice.listo { "indexando el proyecto…".to_string() }
            else if b.patron.is_empty() { "escribe para buscar dentro de los archivos (desde 2 letras)".to_string() }
            else if b.buscando { format!("buscando… {} archivos leidos · {}", miles(b.archivos_vistos), contar(b.total as u32, "resultado", "resultados")) }
            else if b.total == 0 { format!("sin resultados en {} archivos", miles(b.archivos_vistos)) }
            else { format!("{} en {} de {} leidos{}", contar(b.total as u32, "resultado", "resultados"), contar(archivos, "archivo", "archivos"),
                           miles(b.archivos_vistos), if b.truncada { " · limite: afina la busqueda" } else { "" }) };
        ui.label(egui::RichText::new(estado).color(if b.truncada { c.aviso } else { c.tenue }).size(11.0));
        ui.separator();
        // Lista aplanada: solo se dibujan las filas que se ven (hasta 20 000 resultados).
        let mut filas: Vec<FilaB> = Vec::new();
        for (id, v) in &b.por_archivo {
            let plegado = b.plegados.contains(id);
            filas.push(FilaB::Archivo(id, v.len(), plegado));
            if !plegado { filas.extend(v.iter().enumerate().map(|(i, co)| FilaB::Linea(id, i, co))); }
        }
        let alto = 22.0;
        let tema = self.estado.ajustes.tema(self.estado.acento);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, alto, filas.len(), |ui, rango| {
            for f in &filas[rango] {
                let (fila, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), alto), Sense::click());
                let p = ui.painter_at(fila);
                if r.hovered() { p.rect_filled(fila, 0.0, c.superficie); }
                match f {
                    FilaB::Archivo(id, n, plegado) => {
                        let (dir, nombre) = id.rsplit_once('/').unwrap_or(("", id));
                        let m = Pos2::new(fila.min.x + 7.0, fila.center().y);
                        let tri = if *plegado { vec![m + Vec2::new(-2.0, -4.0), m + Vec2::new(3.0, 0.0), m + Vec2::new(-2.0, 4.0)] }
                                  else { vec![m + Vec2::new(-4.0, -2.0), m + Vec2::new(4.0, -2.0), m + Vec2::new(0.0, 3.0)] };
                        p.add(egui::Shape::convex_polygon(tri, c.tenue, Stroke::NONE));
                        let tipo = carpetas_nucleo::tipo_por_extension(nombre);
                        let ic = Rect::from_center_size(Pos2::new(fila.min.x + 24.0, fila.center().y), Vec2::splat(15.0));
                        let clave = carpetas_nucleo::iconos::icono(nombre, tipo, self.estado.ajustes.iconos);
                        if clave.is_empty() || !self.iconos.pintar(ui, clave, ic, color(tema.color_tipo(tipo))) { pintar_grafo::icono_dibujado(&p, ic, tipo, color(tema.color_tipo(tipo)), false); }
                        let mut job = LayoutJob::default();
                        job.append(nombre, 0.0, TextFormat::simple(FontId::proportional(13.0), c.texto));
                        job.append(&format!("  {dir}"), 0.0, TextFormat::simple(FontId::proportional(11.0), c.tenue));
                        job.wrap.max_width = fila.width() - 80.0;
                        job.wrap.max_rows = 1;
                        job.wrap.break_anywhere = true;
                        let g = ui.fonts_mut(|fo| fo.layout_job(job));
                        p.galley(Pos2::new(ic.max.x + 6.0, fila.center().y - g.size().y / 2.0), g, c.texto);
                        let cuenta = p.layout_no_wrap(n.to_string(), FontId::proportional(10.5), c.sobre_acento);
                        let caja = Rect::from_min_size(Pos2::new(fila.max.x - cuenta.size().x - 16.0, fila.center().y - 8.0), Vec2::new(cuenta.size().x + 10.0, 16.0));
                        p.rect_filled(caja, 8, c.acento);
                        p.galley(caja.center() - cuenta.size() / 2.0, cuenta, c.sobre_acento);
                        if r.clicked() { acciones.push(Accion::Proyecto(AccionProyecto::Plegar(id.to_string()))); }
                        if r.double_clicked() || r.secondary_clicked() { acciones.push(Accion::Revelar { id: id.to_string(), editor: r.secondary_clicked() }); }
                        r.on_hover_text(format!("{id}\nclic: plegar · doble clic: mostrar en el grafo · clic derecho: editor"));
                    }
                    FilaB::Linea(id, i, co) => {
                        let mut job = LayoutJob::default();
                        job.append(&format!("{:>5}  ", co.linea), 0.0, TextFormat::simple(FontId::monospace(11.0), c.tenue));
                        let (a, z) = co.en_vista;
                        let mono = |col: Color32| TextFormat::simple(FontId::monospace(11.5), col);
                        // Como mucho ~14 letras antes de la coincidencia: que se vea en un panel estrecho.
                        let antes = &co.vista[..a];
                        let corte = antes.char_indices().rev().nth(13).map(|(k, _)| k).unwrap_or(0);
                        if corte > 0 { job.append("…", 0.0, mono(c.tenue)); }
                        job.append(&antes[corte..], 0.0, mono(c.texto.gamma_multiply(0.85)));
                        let mut f = mono(c.texto);
                        f.background = c.aviso.gamma_multiply(0.35);
                        job.append(&co.vista[a..z], 0.0, f);
                        job.append(&co.vista[z..], 0.0, mono(c.texto.gamma_multiply(0.85)));
                        job.wrap.max_width = fila.width() - 24.0;
                        job.wrap.max_rows = 1;
                        job.wrap.break_anywhere = true;
                        let g = ui.fonts_mut(|fo| fo.layout_job(job));
                        p.galley(Pos2::new(fila.min.x + 18.0, fila.center().y - g.size().y / 2.0), g, c.texto);
                        if r.clicked() { acciones.push(Accion::Proyecto(AccionProyecto::Ir { id: id.to_string(), i: *i })); }
                    }
                }
            }
        });
        for a in acciones { self.despachar(a); }
    }
}

pub fn miles(n: u64) -> String {
    let s = n.to_string();
    let mut r = String::new();
    for (i, ch) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { r.push(' '); } r.push(ch); }
    r
}
