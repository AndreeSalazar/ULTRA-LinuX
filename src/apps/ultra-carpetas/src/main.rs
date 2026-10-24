// SPDX-License-Identifier: GPL-2.0-only
//! ULTRA Carpetas para Linux: la capa fina sobre `carpetas-nucleo` (no_std). Diseno: Eddi Salazar.
//!
//! Como React + Redux: este archivo solo traduce eventos a `Accion`, ejecuta los
//! `Efecto` (disco, abrir archivos) y dibuja la `Vista` con egui sobre wgpu (Vulkan).
//! Lo que el programa decide esta en el nucleo, que BMO-X puede usar tal cual.
//! Solo ordena la vista: nunca mueve, renombra ni borra archivos.

mod almacen;
mod entradas;

use carpetas_nucleo::{atajo, reducir, vista, Accion, AccionPaleta, Comando, Efecto, Estado, Nodo, Orden, Rgb, Tecla, Tema, Tipo};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use entradas::{legible, listar};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

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
    /// Ultimo tema aplicado a egui: se vuelve a aplicar solo si cambia.
    tema_aplicado: Option<Tema>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, carpeta: PathBuf) -> Self {
        let estado = Estado { acento: acento(), ..Estado::default() };
        fuentes(&cc.egui_ctx);
        let mut app = App { estado, carpeta: PathBuf::new(), atras: Vec::new(), error: None, revisado: Instant::now(), mtime: None,
                            texto_paleta: String::new(), tema_aplicado: None };
        let ajustes = almacen::cargar_ajustes(&almacen::archivo_ajustes(&almacen::base()));
        app.despachar(Accion::CargarAjustes(ajustes));
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
            Efecto::GuardarAjustes(json) => {
                if let Err(e) = almacen::guardar(&almacen::archivo_ajustes(&almacen::base()), &json) {
                    self.error = Some(format!("No se pudo guardar los ajustes: {e}"));
                }
            }
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
        const MAPA: [(Key, Tecla); 15] = [(Key::T, Tecla::T), (Key::Comma, Tecla::Coma), (Key::G, Tecla::G), (Key::L, Tecla::L), (Key::H, Tecla::H), (Key::Z, Tecla::Z), (Key::A, Tecla::A),
            (Key::P, Tecla::P), (Key::Backspace, Tecla::Retroceso), (Key::Enter, Tecla::Enter), (Key::Escape, Tecla::Escape),
            (Key::F5, Tecla::F5), (Key::ArrowUp, Tecla::Arriba), (Key::ArrowDown, Tecla::Abajo), (Key::ArrowLeft, Tecla::Izquierda)];
        let (m, pulsadas) = ctx.input(|i| (i.modifiers, MAPA.iter().filter(|(k, _)| i.key_pressed(*k)).map(|(_, t)| *t).collect::<Vec<_>>()));
        for t in pulsadas {
            if let Some(a) = atajo(t, m.command, m.shift, m.alt, self.estado.paleta.is_some()) { self.despachar(a); }
        }
    }

    fn colores(&self) -> Colores { Colores::de(&self.estado.ajustes.tema(self.estado.acento)) }

    fn barra(&mut self, ui: &mut egui::Ui) {
        let c = self.colores();
        let mut acciones = Vec::new();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("BMO-X // CARPETAS").color(c.acento).strong().size(16.0));
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
                if ui.add(egui::Button::new(egui::RichText::new(texto).color(c.texto)).frame(false)).clicked() { destino = Some(acumulada.clone()); }
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
            if ui.selectable_label(self.estado.panel_ajustes, "Ajustes y temas [Ctrl+,]").clicked() { acciones.push(Accion::Comando(Comando::Ajustes)); }
        });
        for a in acciones { self.despachar(a); }
    }

    fn lienzo(&mut self, ui: &mut egui::Ui) {
        let ancho = ui.available_width().max(1.0);
        if (ancho - self.estado.ancho).abs() > 0.5 { self.despachar(Accion::Ancho(ancho)); }
        let v = vista(&self.estado);
        let c = Colores::de(&v.tema);
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
                    while y < tam.y { let mut x = 0.0; while x < tam.x { pintor.circle_filled(origen + Vec2::new(x, y), 1.2, c.linea); x += paso[0]; } y += paso[1]; }
                }
                Nodo::Icono { nombre, pos, tam, tipo, color: col, seleccionado, etiqueta } => {
                    let rect = Rect::from_min_size(origen + Vec2::new(pos[0], pos[1]), Vec2::new(tam[0], tam[1]));
                    let r = ui.interact(rect, ui.make_persistent_id(("icono", &self.carpeta, nombre)), Sense::click_and_drag());
                    if r.clicked() { acciones.push(Accion::Pulsar { nombre: nombre.clone(), sumar: ctrl }); }
                    if r.double_clicked() { acciones.push(Accion::Abrir(nombre.clone())); }
                    if r.drag_started() { acciones.push(Accion::EmpezarArrastre(nombre.clone())); }
                    if r.dragged() { let d = r.drag_delta(); acciones.push(Accion::Arrastrar([d.x, d.y])); }
                    if r.drag_stopped() { acciones.push(Accion::Soltar); }
                    icono(&pintor, &c, rect, nombre, *tipo, color(*col), *seleccionado, r.hovered(), etiqueta);
                    let tam_txt = self.estado.entradas.iter().find(|e| &e.nombre == nombre).map(|e| e.tam).unwrap_or(0);
                    r.on_hover_text(format!("{nombre}\n{}", if *tipo == Tipo::Carpeta { "Carpeta".into() } else { legible(tam_txt) }));
                }
                Nodo::Banda { desde, hasta, color: col } => {
                    let banda = Rect::from_two_pos(origen + Vec2::new(desde[0], desde[1]), origen + Vec2::new(hasta[0], hasta[1]));
                    pintor.rect(banda, c.radio(), color(*col).gamma_multiply(0.08), Stroke::new(1.0, color(*col)), StrokeKind::Inside);
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
        let v = vista(&self.estado);
        let Some(vp) = v.paleta else { self.texto_paleta.clear(); return };
        let c = Colores::de(&v.tema);
        let mut acciones = Vec::new();
        egui::Area::new(egui::Id::unique("ultra-carpetas-paleta")).anchor(Align2::CENTER_TOP, [0.0, 70.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.acento)).corner_radius(c.radio()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(520.0);
                ui.label(egui::RichText::new("COMANDOS //").color(c.acento).strong());
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto_paleta).hint_text("escribe para buscar (ej. rej, tema drac)").desired_width(500.0));
                r.request_focus();
                if r.changed() { acciones.push(Accion::Paleta(AccionPaleta::Escribir(self.texto_paleta.clone()))); }
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (i, (titulo, tecla, elegido)) in vp.filas.iter().enumerate() {
                        let fila = ui.horizontal(|ui| {
                            let texto = egui::RichText::new(*titulo).color(if *elegido { c.sobre_acento } else { c.texto });
                            let r = ui.add(egui::Button::new(texto).fill(if *elegido { c.acento } else { Color32::TRANSPARENT }).min_size(Vec2::new(380.0, 22.0)));
                            ui.label(egui::RichText::new(*tecla).color(c.tenue).monospace());
                            r
                        }).inner;
                        if *elegido { fila.scroll_to_me(None); }
                        if fila.clicked() { acciones.push(Accion::Paleta(AccionPaleta::Elegir(i))); }
                    }
                });
                if vp.filas.is_empty() { ui.label(egui::RichText::new("Ningun comando coincide").color(c.tenue)); }
            });
        });
        for a in acciones { self.despachar(a); }
    }

    /// Ajustes como en VS Code (Ctrl+,): tarjetas de tema con vista previa y opciones.
    fn ajustes(&mut self, ctx: &egui::Context) {
        let v = vista(&self.estado);
        let Some(va) = v.ajustes else { return };
        let c = Colores::de(&v.tema);
        let mut acciones = Vec::new();
        egui::Area::new(egui::Id::unique("ultra-carpetas-ajustes")).anchor(Align2::CENTER_TOP, [0.0, 70.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.acento)).corner_radius(c.radio()).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(760.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("AJUSTES // TEMA").color(c.acento).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cerrar [Esc]").clicked() { acciones.push(Accion::Comando(Comando::Ajustes)); }
                    });
                });
                ui.label(egui::RichText::new("Elige con clic, con T (siguiente) o desde la paleta: «tema …». Se guarda solo.").color(c.tenue));
                ui.add_space(8.0);
                egui::Grid::new("temas").num_columns(3).spacing([10.0, 10.0]).show(ui, |ui| {
                    for (i, (_, nombre, descripcion, muestras, elegido)) in va.temas.iter().enumerate() {
                        let (rect, r) = ui.allocate_exact_size(Vec2::new(240.0, 86.0), Sense::click());
                        let p = ui.painter_at(rect);
                        // La tarjeta se pinta con los colores del propio tema: vista previa real.
                        p.rect(rect, c.radio(), color(muestras[1]), Stroke::new(if *elegido { 2.0 } else { 1.0 }, if *elegido { color(muestras[2]) } else if r.hovered() { c.acento } else { c.linea }), StrokeKind::Inside);
                        for (k, m) in muestras.iter().enumerate() {
                            let caja = Rect::from_min_size(rect.min + Vec2::new(10.0 + k as f32 * 22.0, 10.0), Vec2::new(18.0, 18.0));
                            p.rect(caja, CornerRadius::same(2), color(*m), Stroke::new(1.0, Color32::from_white_alpha(30)), StrokeKind::Inside);
                        }
                        p.text(rect.min + Vec2::new(10.0, 38.0), Align2::LEFT_TOP, *nombre, FontId::proportional(14.0), color(muestras[2]));
                        let desc = p.layout(descripcion.to_string(), FontId::proportional(10.5), c.tenue, 220.0);
                        p.galley(rect.min + Vec2::new(10.0, 56.0), desc, c.tenue);
                        if *elegido { p.text(rect.right_top() + Vec2::new(-10.0, 10.0), Align2::RIGHT_TOP, "EN USO", FontId::monospace(10.0), color(muestras[2])); }
                        if r.clicked() { acciones.push(Accion::Comando(Comando::Tema(i as u8))); }
                        if (i + 1) % 3 == 0 { ui.end_row(); }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut brillo = va.brillo;
                    if ui.checkbox(&mut brillo, "Brillo neon en lo seleccionado").changed() { acciones.push(Accion::Comando(Comando::Brillo)); }
                    let mut propio = va.acento_escritorio;
                    if ui.checkbox(&mut propio, "Usar el acento del escritorio en cualquier tema").changed() { acciones.push(Accion::Comando(Comando::AcentoEscritorio)); }
                });
            });
        });
        for a in acciones { self.despachar(a); }
    }
}

/// Los colores del tema ya convertidos para egui.
struct Colores { fondo: Color32, panel: Color32, superficie: Color32, linea: Color32, texto: Color32, tenue: Color32,
                 acento: Color32, sobre_acento: Color32, error: Color32, esquinas: u8, brillo: bool }

impl Colores {
    fn de(t: &Tema) -> Colores {
        Colores { fondo: color(t.fondo), panel: color(t.panel), superficie: color(t.superficie), linea: color(t.linea), texto: color(t.texto),
                  tenue: color(t.tenue), acento: color(t.acento), sobre_acento: color(t.sobre_acento), error: color(t.error),
                  esquinas: t.esquinas, brillo: t.brillo }
    }
    fn radio(&self) -> CornerRadius { CornerRadius::same(self.esquinas) }
}

#[allow(clippy::too_many_arguments)]
fn icono(pintor: &egui::Painter, c: &Colores, rect: Rect, nombre: &str, tipo: Tipo, col: Color32, seleccionado: bool, encima: bool, etiqueta: &str) {
    if encima || seleccionado {
        if c.brillo && seleccionado {
            // Brillo neon: tres contornos cada vez mas anchos y tenues.
            for (k, alfa) in [(2.0, 0.35), (4.0, 0.18), (7.0, 0.08)] {
                pintor.rect_stroke(rect.expand(k), c.radio(), Stroke::new(2.0, c.acento.gamma_multiply(alfa)), StrokeKind::Outside);
            }
        }
        pintor.rect(rect, c.radio(), if seleccionado { c.acento.gamma_multiply(0.16) } else { c.superficie.gamma_multiply(0.6) },
                    Stroke::new(1.0, if seleccionado { c.acento } else { c.linea }), StrokeKind::Inside);
    }
    let centro = rect.center_top() + Vec2::new(0.0, 30.0);
    let trazo = Stroke::new(2.0, col);
    if tipo == Tipo::Carpeta {
        let pts = vec![centro + Vec2::new(-26.0, -16.0), centro + Vec2::new(-8.0, -16.0), centro + Vec2::new(-3.0, -10.0), centro + Vec2::new(26.0, -10.0),
                       centro + Vec2::new(26.0, 18.0), centro + Vec2::new(-26.0, 18.0)];
        pintor.add(egui::Shape::convex_polygon(pts, col.gamma_multiply(0.18), trazo));
        pintor.line_segment([centro + Vec2::new(-26.0, -4.0), centro + Vec2::new(26.0, -4.0)], Stroke::new(1.0, col.gamma_multiply(0.6)));
    } else {
        let pts = vec![centro + Vec2::new(-18.0, -22.0), centro + Vec2::new(8.0, -22.0), centro + Vec2::new(18.0, -12.0), centro + Vec2::new(18.0, 22.0), centro + Vec2::new(-18.0, 22.0)];
        pintor.add(egui::Shape::convex_polygon(pts, col.gamma_multiply(0.12), trazo));
        pintor.line_segment([centro + Vec2::new(8.0, -22.0), centro + Vec2::new(8.0, -12.0)], trazo);
        pintor.line_segment([centro + Vec2::new(8.0, -12.0), centro + Vec2::new(18.0, -12.0)], trazo);
        if !etiqueta.is_empty() { pintor.text(centro + Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, etiqueta, FontId::monospace(10.0), col); }
        if tipo == Tipo::Enlace { pintor.text(centro + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, "enlace", FontId::monospace(8.0), col); }
    }
    let mut texto = nombre.to_string();
    if texto.chars().count() > 26 { texto = texto.chars().take(25).collect::<String>() + "…"; }
    let galeria = pintor.layout(texto, FontId::proportional(12.5), if seleccionado { c.texto } else { c.tenue }, rect.width() - 6.0);
    pintor.galley(rect.center_top() + Vec2::new(-galeria.size().x / 2.0, 58.0), galeria, c.texto);
}

/// Aplica el tema a egui (botones, menus, campos de texto).
fn estilo(ctx: &egui::Context, t: &Tema) {
    let c = Colores::de(t);
    let mut v = egui::Visuals::dark();
    v.panel_fill = c.fondo;
    v.window_fill = c.panel;
    v.extreme_bg_color = c.fondo;
    v.faint_bg_color = c.superficie;
    v.override_text_color = Some(c.texto);
    v.selection.bg_fill = c.acento.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, c.acento);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.linea);
    v.widgets.inactive.weak_bg_fill = c.panel;
    v.widgets.inactive.bg_fill = c.superficie;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.linea);
    v.widgets.hovered.weak_bg_fill = c.acento.gamma_multiply(0.15);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.acento);
    v.widgets.active.weak_bg_fill = c.acento.gamma_multiply(0.3);
    v.widgets.active.bg_stroke = Stroke::new(1.0, c.acento);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = c.radio();
    }
    v.window_corner_radius = c.radio();
    v.menu_corner_radius = c.radio();
    ctx.set_visuals(v);
}

fn fuentes(ctx: &egui::Context) {
    for ruta in ["/usr/share/fonts/ultra-neon/ChakraPetch-Medium.ttf", "/usr/share/fonts/ultra-neon/ChakraPetch-Regular.ttf"] {
        if let Ok(datos) = std::fs::read(ruta) {
            let mut f = egui::FontDefinitions::default();
            f.font_data.insert("chakra".into(), std::sync::Arc::new(egui::FontData::from_owned(datos)));
            f.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "chakra".into());
            ctx.set_fonts(f);
            break;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.vigilar();
        self.teclado(&ctx);
        if self.estado.panel_ajustes && ctx.input(|i| i.key_pressed(Key::Escape)) { self.despachar(Accion::Comando(Comando::Ajustes)); }
        let tema = self.estado.ajustes.tema(self.estado.acento);
        if self.tema_aplicado != Some(tema) { estilo(&ctx, &tema); self.tema_aplicado = Some(tema); }
        let c = Colores::de(&tema);
        egui::Panel::top("barra").frame(egui::Frame::new().fill(c.panel).inner_margin(8.0).stroke(Stroke::new(1.0, c.linea))).show(ui, |ui| self.barra(ui));
        let estado = format!("{} · tema {}", vista(&self.estado).estado, tema.nombre);
        egui::Panel::bottom("estado").frame(egui::Frame::new().fill(c.panel).inner_margin(6.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(estado).color(c.tenue));
                if let Some(e) = &self.error { ui.label(egui::RichText::new(format!("· {e}")).color(c.error)); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Solo ordena la vista: tus archivos no se mueven").color(c.tenue).italics());
                });
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(c.fondo)).show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.lienzo(ui));
        });
        self.paleta(&ctx);
        self.ajustes(&ctx);
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
