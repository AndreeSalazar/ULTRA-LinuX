// SPDX-License-Identifier: GPL-2.0-only
//! ULTRA Carpetas para Linux: la capa fina sobre `carpetas-nucleo` (no_std). Diseno: Eddi Salazar.
//!
//! Como React + Redux: este archivo solo traduce eventos a `Accion`, ejecuta los
//! `Efecto` (disco, abrir archivos) y dibuja la `Vista` con egui sobre wgpu (Vulkan).
//! Lo que el programa decide esta en el nucleo, que BMO-X puede usar tal cual.
//! Solo ordena la vista: nunca mueve, renombra ni borra archivos.

mod almacen;
mod entradas;

use carpetas_nucleo::{atajo, reducir, vista, Accion, AccionPaleta, Comando, Efecto, Estado, Nodo, Orden, Rgb, Tecla, Tipo};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use entradas::{legible, listar};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

const NEGRO: Color32 = Color32::from_rgb(6, 5, 10);
const PANEL: Color32 = Color32::from_rgb(15, 14, 22);
const LINEA: Color32 = Color32::from_rgb(42, 39, 51);
const TEXTO: Color32 = Color32::from_rgb(236, 233, 220);
const TENUE: Color32 = Color32::from_rgb(161, 158, 174);
const ROJO: Color32 = Color32::from_rgb(255, 42, 79);

fn color(c: Rgb) -> Color32 { Color32::from_rgb(c[0], c[1], c[2]) }

fn acento() -> Rgb {
    // El mismo acento que el resto del escritorio (Personalizar); hielo si no hay.
    let salida = std::process::Command::new("ultra-personalizar").arg("acento").stdin(std::process::Stdio::null())
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    if salida.len() == 8 && salida.chars().all(|c| c.is_ascii_hexdigit()) {
        let v = |i: usize| u8::from_str_radix(&salida[i..i + 2], 16).unwrap_or(0);
        return [v(0), v(2), v(4)];
    }
    carpetas_nucleo::tipos::HIELO
}

struct App {
    estado: Estado,
    carpeta: PathBuf,
    atras: Vec<PathBuf>,
    error: Option<String>,
    revisado: Instant,
    mtime: Option<SystemTime>,
    texto_paleta: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, carpeta: PathBuf) -> Self {
        let estado = Estado { acento: acento(), ..Estado::default() };
        estilo(&cc.egui_ctx, color(estado.acento));
        let mut app = App { estado, carpeta: PathBuf::new(), atras: Vec::new(), error: None, revisado: Instant::now(), mtime: None, texto_paleta: String::new() };
        app.abrir(carpeta, false);
        app
    }

    /// El unico camino para cambiar el estado: accion -> reducer -> efectos.
    fn despachar(&mut self, accion: Accion) {
        for efecto in reducir(&mut self.estado, accion) {
            self.ejecutar(efecto);
        }
    }

    fn ejecutar(&mut self, efecto: Efecto) {
        match efecto {
            Efecto::Guardar(json) => {
                if let Err(e) = almacen::guardar(&almacen::archivo_para(&almacen::base(), &self.carpeta), &json) {
                    self.error = Some(format!("No se pudo guardar el orden: {e}"));
                }
            }
            // El nucleo solo pide nombres que existen en la carpeta; read_dir nunca da nombres con '/'.
            Efecto::AbrirArchivo(n) => {
                if let Err(e) = std::process::Command::new("xdg-open").arg(self.carpeta.join(&n)).stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
                    self.error = Some(format!("No se pudo abrir {n}: {e}"));
                }
            }
            Efecto::EntrarCarpeta(n) => { let destino = self.carpeta.join(n); self.abrir(destino, true); }
            Efecto::Subir => { if let Some(p) = self.carpeta.parent().map(Path::to_path_buf) { self.abrir(p, true); } }
            Efecto::Atras => { if let Some(p) = self.atras.pop() { self.abrir(p, false); } }
            Efecto::Releer { .. } => self.releer(),
        }
    }

    fn abrir(&mut self, carpeta: PathBuf, recordar: bool) {
        let carpeta = carpeta.canonicalize().unwrap_or(carpeta);
        match listar(&carpeta, self.estado.ocultos) {
            Ok(entradas) => {
                if recordar && !self.carpeta.as_os_str().is_empty() { self.atras.push(self.carpeta.clone()); }
                self.carpeta = carpeta;
                self.error = None;
                self.mtime = std::fs::metadata(&self.carpeta).and_then(|m| m.modified()).ok();
                let disposicion = almacen::cargar(&almacen::archivo_para(&almacen::base(), &self.carpeta));
                self.despachar(Accion::Cargar { entradas, disposicion });
            }
            Err(e) => self.error = Some(format!("No se pudo abrir {}: {e}", carpeta.display())),
        }
    }

    fn releer(&mut self) {
        match listar(&self.carpeta, self.estado.ocultos) {
            Ok(entradas) => self.despachar(Accion::Refrescar(entradas)),
            Err(e) => self.error = Some(format!("No se pudo leer la carpeta: {e}")),
        }
    }

    /// Si la carpeta cambio en disco, se vuelve a leer. Una vez por segundo.
    fn vigilar(&mut self) {
        if self.revisado.elapsed() < Duration::from_secs(1) { return; }
        self.revisado = Instant::now();
        let ahora = std::fs::metadata(&self.carpeta).and_then(|m| m.modified()).ok();
        if ahora != self.mtime { self.mtime = ahora; self.releer(); }
    }

    /// Teclas de egui -> teclas del nucleo -> accion (el "keybindings.json").
    fn teclado(&mut self, ctx: &egui::Context) {
        const MAPA: [(Key, Tecla); 13] = [(Key::G, Tecla::G), (Key::L, Tecla::L), (Key::H, Tecla::H), (Key::Z, Tecla::Z), (Key::A, Tecla::A),
            (Key::P, Tecla::P), (Key::Backspace, Tecla::Retroceso), (Key::Enter, Tecla::Enter), (Key::Escape, Tecla::Escape),
            (Key::F5, Tecla::F5), (Key::ArrowUp, Tecla::Arriba), (Key::ArrowDown, Tecla::Abajo), (Key::ArrowLeft, Tecla::Izquierda)];
        let (m, pulsadas) = ctx.input(|i| (i.modifiers, MAPA.iter().filter(|(k, _)| i.key_pressed(*k)).map(|(_, t)| *t).collect::<Vec<_>>()));
        for t in pulsadas {
            if let Some(a) = atajo(t, m.command, m.shift, m.alt, self.estado.paleta.is_some()) { self.despachar(a); }
        }
    }

    fn barra(&mut self, ui: &mut egui::Ui) {
        let mut acciones = Vec::new();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("BMO-X // CARPETAS").color(color(self.estado.acento)).strong().size(16.0));
            ui.add_space(10.0);
            if ui.button("Atrás").clicked() { acciones.push(Accion::Comando(Comando::Atras)); }
            if ui.button("Arriba").on_hover_text("Retroceso").clicked() { acciones.push(Accion::Comando(Comando::Subir)); }
            ui.add_space(6.0);
            let mut acumulada = PathBuf::new();
            let mut destino = None;
            for parte in self.carpeta.components() {
                acumulada.push(parte);
                let texto = parte.as_os_str().to_string_lossy();
                let texto = if texto == "/" { "/".to_string() } else { format!("{texto} /") };
                if ui.add(egui::Button::new(egui::RichText::new(texto).color(TEXTO)).frame(false)).clicked() { destino = Some(acumulada.clone()); }
            }
            if let Some(d) = destino { self.abrir(d, true); }
        });
        ui.horizontal(|ui| {
            let d = &self.estado.disp;
            if ui.selectable_label(d.rejilla, "Rejilla [G]").clicked() { acciones.push(Accion::Comando(Comando::Rejilla)); }
            if ui.selectable_label(d.bloqueada, if d.bloqueada { "Candado puesto [L]" } else { "Candado [L]" }).clicked() { acciones.push(Accion::Comando(Comando::Candado)); }
            if ui.selectable_label(self.estado.ocultos, "Ocultos [H]").clicked() { acciones.push(Accion::Comando(Comando::Ocultos)); }
            ui.separator();
            ui.add_enabled_ui(!d.bloqueada, |ui| {
                ui.menu_button("Ordenar", |ui| {
                    for (texto, o) in [("Por nombre", Orden::Nombre), ("Por tipo", Orden::Tipo), ("Por tamano", Orden::Tamano)] {
                        if ui.button(texto).clicked() { acciones.push(Accion::Comando(Comando::Ordenar(o))); ui.close(); }
                    }
                });
                if ui.button("Deshacer [Ctrl+Z]").clicked() { acciones.push(Accion::Comando(Comando::Deshacer)); }
            });
            ui.separator();
            if ui.button("Comandos [Ctrl+Shift+P]").clicked() { acciones.push(Accion::Comando(Comando::Paleta)); }
        });
        for a in acciones { self.despachar(a); }
    }

    fn lienzo(&mut self, ui: &mut egui::Ui) {
        let ancho = ui.available_width().max(1.0);
        if (ancho - self.estado.ancho).abs() > 0.5 { self.despachar(Accion::Ancho(ancho)); }
        let v = vista(&self.estado);
        let tam = Vec2::new(v.tam[0].max(ui.available_width()), (v.tam[1] + 200.0).max(ui.available_height()));
        let (lienzo, fondo) = ui.allocate_exact_size(tam, Sense::click_and_drag());
        let pintor = ui.painter_at(lienzo);
        let origen = lienzo.min + Vec2::splat(16.0);
        let local = |p: Pos2| { let d = p - origen; [d.x, d.y] };
        let ctrl = ui.input(|i| i.modifiers.command);
        let mut acciones = Vec::new();
        for nodo in &v.nodos {
            match nodo {
                Nodo::Rejilla { paso } => {
                    let mut y = 0.0;
                    while y < tam.y { let mut x = 0.0; while x < tam.x { pintor.circle_filled(origen + Vec2::new(x, y), 1.2, LINEA); x += paso[0]; } y += paso[1]; }
                }
                Nodo::Icono { nombre, pos, tam, tipo, color: c, seleccionado, etiqueta } => {
                    let rect = Rect::from_min_size(origen + Vec2::new(pos[0], pos[1]), Vec2::new(tam[0], tam[1]));
                    let r = ui.interact(rect, ui.make_persistent_id(("icono", &self.carpeta, nombre)), Sense::click_and_drag());
                    if r.clicked() { acciones.push(Accion::Pulsar { nombre: nombre.clone(), sumar: ctrl }); }
                    if r.double_clicked() { acciones.push(Accion::Abrir(nombre.clone())); }
                    if r.drag_started() { acciones.push(Accion::EmpezarArrastre(nombre.clone())); }
                    if r.dragged() { let d = r.drag_delta(); acciones.push(Accion::Arrastrar([d.x, d.y])); }
                    if r.drag_stopped() { acciones.push(Accion::Soltar); }
                    icono(&pintor, rect, nombre, *tipo, color(*c), color(self.estado.acento), *seleccionado, r.hovered(), etiqueta);
                    let tam_txt = self.estado.entradas.iter().find(|e| &e.nombre == nombre).map(|e| e.tam).unwrap_or(0);
                    r.on_hover_text(format!("{nombre}\n{}", if *tipo == Tipo::Carpeta { "Carpeta".into() } else { legible(tam_txt) }));
                }
                Nodo::Banda { desde, hasta, color: c } => {
                    let banda = Rect::from_two_pos(origen + Vec2::new(desde[0], desde[1]), origen + Vec2::new(hasta[0], hasta[1]));
                    pintor.rect(banda, CornerRadius::ZERO, color(*c).gamma_multiply(0.08), Stroke::new(1.0, color(*c)), StrokeKind::Inside);
                }
            }
        }
        if fondo.drag_started() { if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::EmpezarBanda { punto: local(p), sumar: ctrl }); } }
        if fondo.dragged() { if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::MoverBanda(local(p))); } }
        if fondo.drag_stopped() { acciones.push(Accion::TerminarBanda); }
        if fondo.clicked() && !ctrl { acciones.push(Accion::Comando(Comando::LimpiarSeleccion)); }
        for a in acciones { self.despachar(a); }
    }

    /// Paleta de comandos como en VS Code: escribe, flechas y Enter.
    fn paleta(&mut self, ctx: &egui::Context) {
        let Some(vp) = vista(&self.estado).paleta else { self.texto_paleta.clear(); return };
        let acento = color(self.estado.acento);
        let mut acciones = Vec::new();
        egui::Area::new(egui::Id::unique("ultra-carpetas-paleta")).anchor(Align2::CENTER_TOP, [0.0, 70.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, acento)).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(520.0);
                ui.label(egui::RichText::new("COMANDOS //").color(acento).strong());
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto_paleta).hint_text("escribe para buscar (ej. rej, ordtam)").desired_width(500.0));
                r.request_focus();
                if r.changed() { acciones.push(Accion::Paleta(AccionPaleta::Escribir(self.texto_paleta.clone()))); }
                ui.add_space(4.0);
                for (i, (titulo, tecla, elegido)) in vp.filas.iter().enumerate() {
                    let fila = ui.horizontal(|ui| {
                        let texto = egui::RichText::new(*titulo).color(if *elegido { Color32::BLACK } else { TEXTO });
                        let r = ui.add(egui::Button::new(texto).fill(if *elegido { acento } else { Color32::TRANSPARENT }).min_size(Vec2::new(380.0, 22.0)));
                        ui.label(egui::RichText::new(*tecla).color(TENUE).monospace());
                        r
                    }).inner;
                    if fila.clicked() { acciones.push(Accion::Paleta(AccionPaleta::Elegir(i))); }
                }
                if vp.filas.is_empty() { ui.label(egui::RichText::new("Ningun comando coincide").color(TENUE)); }
            });
        });
        for a in acciones { self.despachar(a); }
    }
}

#[allow(clippy::too_many_arguments)]
fn icono(pintor: &egui::Painter, rect: Rect, nombre: &str, tipo: Tipo, c: Color32, acento: Color32, seleccionado: bool, encima: bool, etiqueta: &str) {
    if encima || seleccionado {
        pintor.rect(rect, CornerRadius::same(2), if seleccionado { acento.gamma_multiply(0.16) } else { Color32::from_white_alpha(6) },
                    Stroke::new(1.0, if seleccionado { acento } else { LINEA }), StrokeKind::Inside);
    }
    let centro = rect.center_top() + Vec2::new(0.0, 30.0);
    let trazo = Stroke::new(2.0, c);
    if tipo == Tipo::Carpeta {
        let pts = vec![centro + Vec2::new(-26.0, -16.0), centro + Vec2::new(-8.0, -16.0), centro + Vec2::new(-3.0, -10.0), centro + Vec2::new(26.0, -10.0),
                       centro + Vec2::new(26.0, 18.0), centro + Vec2::new(-26.0, 18.0)];
        pintor.add(egui::Shape::convex_polygon(pts, c.gamma_multiply(0.18), trazo));
        pintor.line_segment([centro + Vec2::new(-26.0, -4.0), centro + Vec2::new(26.0, -4.0)], Stroke::new(1.0, c.gamma_multiply(0.6)));
    } else {
        let pts = vec![centro + Vec2::new(-18.0, -22.0), centro + Vec2::new(8.0, -22.0), centro + Vec2::new(18.0, -12.0), centro + Vec2::new(18.0, 22.0), centro + Vec2::new(-18.0, 22.0)];
        pintor.add(egui::Shape::convex_polygon(pts, c.gamma_multiply(0.12), trazo));
        pintor.line_segment([centro + Vec2::new(8.0, -22.0), centro + Vec2::new(8.0, -12.0)], trazo);
        pintor.line_segment([centro + Vec2::new(8.0, -12.0), centro + Vec2::new(18.0, -12.0)], trazo);
        if !etiqueta.is_empty() { pintor.text(centro + Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, etiqueta, FontId::monospace(10.0), c); }
        if tipo == Tipo::Enlace { pintor.text(centro + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, "enlace", FontId::monospace(8.0), c); }
    }
    let mut texto = nombre.to_string();
    if texto.chars().count() > 26 { texto = texto.chars().take(25).collect::<String>() + "…"; }
    let galeria = pintor.layout(texto, FontId::proportional(12.5), if seleccionado { TEXTO } else { TENUE }, rect.width() - 6.0);
    pintor.galley(rect.center_top() + Vec2::new(-galeria.size().x / 2.0, 58.0), galeria, TEXTO);
}

fn estilo(ctx: &egui::Context, acento: Color32) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = NEGRO;
    v.window_fill = PANEL;
    v.extreme_bg_color = NEGRO;
    v.selection.bg_fill = acento.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, acento);
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
        let estado = vista(&self.estado).estado;
        egui::Panel::bottom("estado").frame(egui::Frame::new().fill(PANEL).inner_margin(6.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(estado).color(TENUE));
                if let Some(e) = &self.error { ui.label(egui::RichText::new(format!("· {e}")).color(ROJO)); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Solo ordena la vista: tus archivos no se mueven").color(TENUE).italics());
                });
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEGRO)).show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.lienzo(ui));
        });
        self.paleta(&ctx);
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
