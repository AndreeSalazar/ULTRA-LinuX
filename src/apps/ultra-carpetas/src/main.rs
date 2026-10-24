// SPDX-License-Identifier: GPL-2.0-only
//! ULTRA Carpetas: gestor de carpetas de BMO-X. Diseno: Eddi Salazar.
//!
//! Los iconos van donde tu los dejes, como en el Windows antiguo, y cada carpeta
//! recuerda su orden. Con control: rejilla, candado, deshacer y ordenar.
//! Solo ordena la vista: nunca mueve, renombra ni borra archivos.
//! Dibuja con wgpu: Vulkan en Linux y Windows, Metal en macOS.

mod disposicion;
mod entradas;

use disposicion::{archivo_para, Disposicion, CELDA};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use entradas::{legible, listar, Entrada, Tipo};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

const ICONO: Vec2 = Vec2::new(96.0, 92.0);
const NEGRO: Color32 = Color32::from_rgb(6, 5, 10);
const PANEL: Color32 = Color32::from_rgb(15, 14, 22);
const LINEA: Color32 = Color32::from_rgb(42, 39, 51);
const TEXTO: Color32 = Color32::from_rgb(236, 233, 220);
const TENUE: Color32 = Color32::from_rgb(161, 158, 174);
const CYAN: Color32 = Color32::from_rgb(0, 240, 255);
const ROJO: Color32 = Color32::from_rgb(255, 42, 79);
const VERDE: Color32 = Color32::from_rgb(61, 255, 154);
const VIOLETA: Color32 = Color32::from_rgb(192, 59, 255);
const AMARILLO: Color32 = Color32::from_rgb(252, 238, 10);
const CORAL: Color32 = Color32::from_rgb(255, 122, 144);

fn acento() -> Color32 {
    // El mismo acento que el resto del escritorio (Personalizar); hielo si no hay.
    let salida = std::process::Command::new("ultra-personalizar").arg("acento").stdin(std::process::Stdio::null())
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    if salida.len() == 8 && salida.chars().all(|c| c.is_ascii_hexdigit()) {
        let v = |i: usize| u8::from_str_radix(&salida[i..i + 2], 16).unwrap_or(0);
        return Color32::from_rgb(v(0), v(2), v(4));
    }
    Color32::from_rgb(90, 216, 255)
}

fn color_tipo(tipo: Tipo, acento: Color32) -> Color32 {
    match tipo {
        Tipo::Carpeta => acento, Tipo::Imagen => VIOLETA, Tipo::Video => ROJO, Tipo::Audio => AMARILLO,
        Tipo::Codigo => CYAN, Tipo::Comprimido => CORAL, Tipo::Programa => VERDE, Tipo::Documento => TEXTO,
        Tipo::Enlace => TENUE, Tipo::Otro => TENUE,
    }
}

fn config() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(x).join("ultra/carpetas");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/home/ultra".into())).join(".config/ultra/carpetas")
}

#[derive(Clone, Copy, PartialEq)]
enum Orden { Nombre, Tipo, Tamano }

struct App {
    carpeta: PathBuf,
    atras: Vec<PathBuf>,
    entradas: Vec<Entrada>,
    disp: Disposicion,
    seleccion: BTreeSet<String>,
    ocultos: bool,
    acento: Color32,
    arrastrando: bool,
    banda: Option<Pos2>,
    error: Option<String>,
    revisado: Instant,
    mtime: Option<SystemTime>,
    ancho_vista: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, carpeta: PathBuf) -> Self {
        let acento = acento();
        estilo(&cc.egui_ctx, acento);
        let mut app = App {
            carpeta: PathBuf::new(), atras: Vec::new(), entradas: Vec::new(), disp: Disposicion::default(),
            seleccion: BTreeSet::new(), ocultos: false, acento, arrastrando: false, banda: None, error: None,
            revisado: Instant::now(), mtime: None, ancho_vista: 1100.0,
        };
        app.abrir(carpeta, false);
        app
    }

    fn ruta_disp(&self) -> PathBuf { archivo_para(&config(), &self.carpeta) }

    fn guardar(&mut self) {
        if let Err(e) = self.disp.guardar(&self.ruta_disp()) {
            self.error = Some(format!("No se pudo guardar el orden: {e}"));
        }
    }

    fn abrir(&mut self, carpeta: PathBuf, recordar: bool) {
        let carpeta = carpeta.canonicalize().unwrap_or(carpeta);
        match listar(&carpeta, self.ocultos) {
            Ok(entradas) => {
                if recordar && !self.carpeta.as_os_str().is_empty() { self.atras.push(self.carpeta.clone()); }
                self.carpeta = carpeta;
                self.entradas = entradas;
                self.disp = Disposicion::cargar(&self.ruta_disp());
                self.seleccion.clear();
                self.error = None;
                self.mtime = std::fs::metadata(&self.carpeta).and_then(|m| m.modified()).ok();
                self.sincronizar();
            }
            Err(e) => self.error = Some(format!("No se pudo abrir {}: {e}", carpeta.display())),
        }
    }

    fn recargar(&mut self) {
        if let Ok(entradas) = listar(&self.carpeta, self.ocultos) {
            self.entradas = entradas;
            let nombres: BTreeSet<_> = self.entradas.iter().map(|e| e.nombre.clone()).collect();
            self.seleccion.retain(|n| nombres.contains(n));
            self.sincronizar();
        }
    }

    fn sincronizar(&mut self) {
        let nombres: Vec<String> = self.entradas.iter().map(|e| e.nombre.clone()).collect();
        if self.disp.sincronizar(&nombres, self.ancho_vista) { self.guardar(); }
    }

    /// Si la carpeta cambio en disco (algo nuevo o borrado), se vuelve a leer. Una vez por segundo.
    fn vigilar(&mut self) {
        if self.revisado.elapsed() < Duration::from_secs(1) { return; }
        self.revisado = Instant::now();
        let ahora = std::fs::metadata(&self.carpeta).and_then(|m| m.modified()).ok();
        if ahora != self.mtime { self.mtime = ahora; self.recargar(); }
    }

    fn abrir_entrada(&mut self, nombre: &str) {
        let Some(e) = self.entradas.iter().find(|e| e.nombre == nombre).cloned() else { return };
        if e.tipo == Tipo::Carpeta {
            self.abrir(e.ruta, true);
        } else if let Err(err) = std::process::Command::new("xdg-open").arg(&e.ruta).stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
            self.error = Some(format!("No se pudo abrir {}: {err}", e.nombre));
        }
    }

    fn ordenar(&mut self, orden: Orden) {
        let mut lista: Vec<&Entrada> = self.entradas.iter().collect();
        match orden {
            Orden::Nombre => lista.sort_by_key(|e| (e.tipo != Tipo::Carpeta, e.nombre.to_lowercase())),
            Orden::Tipo => lista.sort_by_key(|e| (e.tipo, e.nombre.to_lowercase())),
            Orden::Tamano => lista.sort_by_key(|e| (e.tipo != Tipo::Carpeta, std::cmp::Reverse(e.tam))),
        }
        let nombres: Vec<String> = lista.iter().map(|e| e.nombre.clone()).collect();
        self.disp.ordenar(&nombres, self.ancho_vista);
        self.guardar();
    }

    fn teclado(&mut self, ctx: &egui::Context) {
        let (mods, teclas) = ctx.input(|i| (i.modifiers, [Key::G, Key::L, Key::H, Key::Z, Key::A, Key::Backspace, Key::Enter, Key::Escape, Key::F5]
            .map(|k| i.key_pressed(k))));
        let [g, l, h, z, a, atras, enter, esc, f5] = teclas;
        if mods.ctrl && z { if self.disp.deshacer() { self.guardar(); } }
        else if mods.ctrl && a { self.seleccion = self.entradas.iter().map(|e| e.nombre.clone()).collect(); }
        else if g { self.disp.rejilla = !self.disp.rejilla; self.guardar(); }
        else if l { self.disp.bloqueada = !self.disp.bloqueada; self.guardar(); }
        else if h { self.ocultos = !self.ocultos; self.recargar(); }
        else if atras || (mods.alt && ctx.input(|i| i.key_pressed(Key::ArrowUp))) { self.subir(); }
        else if enter && self.seleccion.len() == 1 { let n = self.seleccion.iter().next().cloned().unwrap_or_default(); self.abrir_entrada(&n); }
        else if esc { self.seleccion.clear(); }
        else if f5 { self.recargar(); }
    }

    fn subir(&mut self) {
        if let Some(padre) = self.carpeta.parent().map(Path::to_path_buf) { self.abrir(padre, true); }
    }

    fn barra(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("BMO-X // CARPETAS").color(self.acento).strong().size(16.0));
            ui.add_space(10.0);
            if ui.button("Atrás").on_hover_text("Carpeta anterior").clicked() {
                if let Some(p) = self.atras.pop() { self.abrir(p, false); }
            }
            if ui.button("Arriba").on_hover_text("Carpeta de arriba (Retroceso)").clicked() { self.subir(); }
            ui.add_space(6.0);
            // Migas: cada parte de la ruta es un boton.
            let mut acumulada = PathBuf::new();
            let mut destino = None;
            for parte in self.carpeta.components() {
                acumulada.push(parte);
                let texto = parte.as_os_str().to_string_lossy();
                let texto = if texto == "/" { "/".to_string() } else { format!("{texto} /") };
                if ui.add(egui::Button::new(egui::RichText::new(texto).color(TEXTO)).frame(false)).clicked() {
                    destino = Some(acumulada.clone());
                }
            }
            if let Some(d) = destino { self.abrir(d, true); }
        });
        ui.horizontal(|ui| {
            let mut rejilla = self.disp.rejilla;
            if ui.toggle_value(&mut rejilla, "Rejilla [G]").changed() { self.disp.rejilla = rejilla; self.guardar(); }
            let mut candado = self.disp.bloqueada;
            let texto = if candado { "Candado puesto [L]" } else { "Candado [L]" };
            if ui.toggle_value(&mut candado, texto).changed() {
                self.disp.bloqueada = candado; self.guardar();
            }
            if ui.toggle_value(&mut self.ocultos, "Ocultos [H]").changed() { self.recargar(); }
            ui.separator();
            ui.add_enabled_ui(!self.disp.bloqueada, |ui| {
                ui.menu_button("Ordenar", |ui| {
                    if ui.button("Por nombre").clicked() { self.ordenar(Orden::Nombre); ui.close(); }
                    if ui.button("Por tipo").clicked() { self.ordenar(Orden::Tipo); ui.close(); }
                    if ui.button("Por tamano").clicked() { self.ordenar(Orden::Tamano); ui.close(); }
                });
                if ui.button("Deshacer [Ctrl+Z]").clicked() && self.disp.deshacer() { self.guardar(); }
            });
        });
    }

    fn lienzo(&mut self, ui: &mut egui::Ui) {
        self.ancho_vista = ui.available_width().max(CELDA[0]);
        let mut alto = 0.0f32;
        let mut ancho = 0.0f32;
        for p in self.disp.posiciones.values() { ancho = ancho.max(p[0] + CELDA[0]); alto = alto.max(p[1] + CELDA[1]); }
        let tam = Vec2::new(ancho.max(ui.available_width()), (alto + 200.0).max(ui.available_height()));
        let (lienzo, fondo) = ui.allocate_exact_size(tam, Sense::click_and_drag());
        let pintor = ui.painter_at(lienzo);
        let origen = lienzo.min + Vec2::splat(16.0);
        // Rejilla visible solo cuando esta activa: puntos tenues en cada celda.
        if self.disp.rejilla {
            let mut y = 0.0;
            while y < tam.y {
                let mut x = 0.0;
                while x < tam.x { pintor.circle_filled(origen + Vec2::new(x, y), 1.2, LINEA); x += CELDA[0]; }
                y += CELDA[1];
            }
        }
        let modificadores = ui.input(|i| i.modifiers);
        let mut abrir = None;
        let entradas = self.entradas.clone();
        for e in &entradas {
            let Some(p) = self.disp.posiciones.get(&e.nombre).copied() else { continue };
            let rect = Rect::from_min_size(origen + Vec2::new(p[0], p[1]), ICONO);
            let r = ui.interact(rect, ui.make_persistent_id(("icono", &self.carpeta, &e.nombre)), Sense::click_and_drag());
            let seleccionado = self.seleccion.contains(&e.nombre);
            if r.clicked() {
                if modificadores.ctrl {
                    if !self.seleccion.remove(&e.nombre) { self.seleccion.insert(e.nombre.clone()); }
                } else {
                    self.seleccion = [e.nombre.clone()].into();
                }
            }
            if r.double_clicked() { abrir = Some(e.nombre.clone()); }
            if r.drag_started() && !self.disp.bloqueada {
                if !seleccionado { self.seleccion = [e.nombre.clone()].into(); }
                self.disp.recordar();
                self.arrastrando = true;
            }
            if r.dragged() && self.arrastrando {
                let d = r.drag_delta();
                let grupo = self.seleccion.clone();
                self.disp.mover(&grupo, [d.x, d.y]);
            }
            if r.drag_stopped() && self.arrastrando {
                let grupo = self.seleccion.clone();
                self.disp.soltar(&grupo);
                self.arrastrando = false;
                self.guardar();
            }
            let resaltado = r.hovered() || seleccionado;
            self.icono(&pintor, rect, e, seleccionado, resaltado);
            r.on_hover_text(format!("{}\n{}", e.nombre, if e.tipo == Tipo::Carpeta { "Carpeta".into() } else { legible(e.tam) }));
        }
        // Banda de seleccion arrastrando sobre el fondo.
        if fondo.drag_started() { self.banda = fondo.interact_pointer_pos(); if !modificadores.ctrl { self.seleccion.clear(); } }
        if fondo.clicked() && !modificadores.ctrl { self.seleccion.clear(); }
        if let (Some(inicio), Some(actual)) = (self.banda, ui.input(|i| i.pointer.interact_pos())) {
            let banda = Rect::from_two_pos(inicio, actual);
            pintor.rect(banda, CornerRadius::ZERO, self.acento.gamma_multiply(0.08), Stroke::new(1.0, self.acento), StrokeKind::Inside);
            for (n, p) in &self.disp.posiciones {
                if banda.intersects(Rect::from_min_size(origen + Vec2::new(p[0], p[1]), ICONO)) { self.seleccion.insert(n.clone()); }
            }
            if fondo.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) { self.banda = None; }
        }
        if let Some(n) = abrir { self.abrir_entrada(&n); }
    }

    fn icono(&self, pintor: &egui::Painter, rect: Rect, e: &Entrada, seleccionado: bool, resaltado: bool) {
        let color = color_tipo(e.tipo, self.acento);
        if resaltado {
            pintor.rect(rect, CornerRadius::same(2), if seleccionado { self.acento.gamma_multiply(0.16) } else { Color32::from_white_alpha(6) },
                        Stroke::new(1.0, if seleccionado { self.acento } else { LINEA }), StrokeKind::Inside);
        }
        let c = rect.center_top() + Vec2::new(0.0, 30.0);
        let trazo = Stroke::new(2.0, color);
        if e.tipo == Tipo::Carpeta {
            // Carpeta neon: pestana y cuerpo con un corte inclinado.
            let pts = vec![c + Vec2::new(-26.0, -16.0), c + Vec2::new(-8.0, -16.0), c + Vec2::new(-3.0, -10.0), c + Vec2::new(26.0, -10.0),
                           c + Vec2::new(26.0, 18.0), c + Vec2::new(-26.0, 18.0)];
            pintor.add(egui::Shape::convex_polygon(pts.clone(), color.gamma_multiply(0.18), trazo));
            pintor.line_segment([c + Vec2::new(-26.0, -4.0), c + Vec2::new(26.0, -4.0)], Stroke::new(1.0, color.gamma_multiply(0.6)));
        } else {
            // Hoja con la esquina doblada y la extension.
            let pts = vec![c + Vec2::new(-18.0, -22.0), c + Vec2::new(8.0, -22.0), c + Vec2::new(18.0, -12.0), c + Vec2::new(18.0, 22.0), c + Vec2::new(-18.0, 22.0)];
            pintor.add(egui::Shape::convex_polygon(pts, color.gamma_multiply(0.12), trazo));
            pintor.line_segment([c + Vec2::new(8.0, -22.0), c + Vec2::new(8.0, -12.0)], trazo);
            pintor.line_segment([c + Vec2::new(8.0, -12.0), c + Vec2::new(18.0, -12.0)], trazo);
            let ext = e.nombre.rsplit_once('.').map(|(_, x)| x.to_uppercase()).unwrap_or_default();
            if !ext.is_empty() && ext.len() <= 5 {
                pintor.text(c + Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, ext, FontId::monospace(10.0), color);
            }
            if e.tipo == Tipo::Enlace { pintor.text(c + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, "enlace", FontId::monospace(8.0), color); }
        }
        // Nombre: hasta dos lineas, recortado con "…".
        let mut nombre = e.nombre.clone();
        if nombre.chars().count() > 26 { nombre = nombre.chars().take(25).collect::<String>() + "…"; }
        let galeria = pintor.layout(nombre, FontId::proportional(12.5), if seleccionado { TEXTO } else { TENUE }, ICONO.x - 6.0);
        let pos = rect.center_top() + Vec2::new(-galeria.size().x / 2.0, 58.0);
        pintor.galley(pos, galeria, TEXTO);
    }
}

fn estilo(ctx: &egui::Context, acento: Color32) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = NEGRO;
    v.window_fill = PANEL;
    v.extreme_bg_color = NEGRO;
    v.selection.bg_fill = acento.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, acento);
    v.hyperlink_color = acento;
    v.widgets.inactive.weak_bg_fill = PANEL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINEA);
    v.widgets.hovered.weak_bg_fill = acento.gamma_multiply(0.15);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, acento);
    v.widgets.active.weak_bg_fill = acento.gamma_multiply(0.3);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::ZERO;
    }
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXTO);
    ctx.set_visuals(v);
    // Fuente del tema si esta instalada (Chakra Petch); si no, la de egui.
    for ruta in ["/usr/share/fonts/ultra-neon/ChakraPetch-Medium.ttf", "/usr/share/fonts/ultra-neon/ChakraPetch-Regular.ttf"] {
        if let Ok(datos) = std::fs::read(ruta) {
            let mut fuentes = egui::FontDefinitions::default();
            fuentes.font_data.insert("chakra".into(), std::sync::Arc::new(egui::FontData::from_owned(datos)));
            fuentes.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "chakra".into());
            ctx.set_fonts(fuentes);
            break;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.vigilar();
        self.teclado(&ctx);
        egui::Panel::top("barra").frame(egui::Frame::new().fill(PANEL).inner_margin(8.0).stroke(Stroke::new(1.0, LINEA))).show(ui, |ui| self.barra(ui));
        egui::Panel::bottom("estado").frame(egui::Frame::new().fill(PANEL).inner_margin(6.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} elementos · {} seleccionados", self.entradas.len(), self.seleccion.len())).color(TENUE));
                if self.disp.rejilla { ui.label(egui::RichText::new("· rejilla").color(self.acento)); }
                if self.disp.bloqueada { ui.label(egui::RichText::new("· candado puesto").color(ROJO)); }
                if let Some(e) = &self.error { ui.label(egui::RichText::new(format!("· {e}")).color(ROJO)); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Solo ordena la vista: tus archivos no se mueven").color(TENUE).italics());
                });
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEGRO)).show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.lienzo(ui));
        });
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

fn main() -> eframe::Result {
    let carpeta = std::env::args_os().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into())));
    let opciones = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title("ULTRA Carpetas").with_app_id("ultra-carpetas").with_inner_size([1120.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("ultra-carpetas", opciones, Box::new(move |cc| Ok(Box::new(App::new(cc, carpeta)))))
}
