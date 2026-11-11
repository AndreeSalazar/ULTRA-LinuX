// SPDX-License-Identifier: GPL-2.0-only
//! Small event-driven input panel; Hyprland calls run outside the drawing thread.
use eframe::egui::{self, Color32, RichText};
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[derive(Clone)]
struct Settings {
    speed: f64, scroll: f64, accel: String, left: bool, natural: bool,
    follow_mouse: i32, hide_on_key_press: bool,
    layout: String, options: String, rate: i32, delay: i32,
}
impl Default for Settings {
    fn default() -> Self { Self { speed: 0., scroll: 1., accel: "adaptive".into(), left: false,
        natural: false, follow_mouse: 1, hide_on_key_press: false,
        layout: "us".into(), options: String::new(), rate: 25, delay: 600 } }
}
impl Settings {
    fn decode(v: &Value) -> Option<Self> { Some(Self {
        speed: v["sensitivity"].as_f64()?, scroll: v["scroll_factor"].as_f64()?,
        accel: v["accel_profile"].as_str()?.into(), left: v["left_handed"].as_bool()?,
        follow_mouse: v["follow_mouse"].as_i64().filter(|value| (0..=3).contains(value))? as i32,
        hide_on_key_press: v["hide_on_key_press"].as_bool()?,
        natural: v["natural_scroll"].as_bool()?, layout: v["kb_layout"].as_str()?.into(),
        options: v["kb_options"].as_str()?.into(), rate: v["repeat_rate"].as_i64()? as i32,
        delay: v["repeat_delay"].as_i64()? as i32,
    }) }
    fn encode(&self) -> Value { json!({"sensitivity":self.speed, "scroll_factor":self.scroll,
        "accel_profile":self.accel, "left_handed":self.left, "natural_scroll":self.natural,
        "follow_mouse":self.follow_mouse, "hide_on_key_press":self.hide_on_key_press,
        "kb_layout":self.layout, "kb_options":self.options, "repeat_rate":self.rate, "repeat_delay":self.delay}) }
    fn matches(&self, value: &Value) -> bool {
        let Some(other) = Self::decode(value) else { return false; };
        let mut current = self.encode();
        current["sensitivity"] = value["sensitivity"].clone();
        current["scroll_factor"] = value["scroll_factor"].clone();
        (self.speed - other.speed).abs() < 0.0001
            && (self.scroll - other.scroll).abs() < 0.0001 && current == *value
    }
}

const ACCENT: Color32 = Color32::from_rgb(109, 224, 242);
const CARD: Color32 = Color32::from_rgb(20, 32, 46);
const MUTED: Color32 = Color32::from_rgb(157, 176, 195);

fn setting_row(ui: &mut egui::Ui, title: &str, hint: &str, control: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(CARD).corner_radius(8).inner_margin(12).show(ui, |ui| {
        ui.horizontal(|ui| {
            let control_width = 194.;
            let label_width = ui.available_width() - control_width - ui.spacing().item_spacing.x;
            ui.allocate_ui_with_layout(egui::vec2(label_width, 0.), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(label_width);
                ui.label(RichText::new(title).strong());
                ui.label(RichText::new(hint).small().color(MUTED));
            });
            ui.allocate_ui_with_layout(egui::vec2(control_width, 0.), egui::Layout::right_to_left(egui::Align::Center), control);
        });
    });
}

#[derive(PartialEq)]
enum Tab { Mouse, Keyboard }

struct Panel {
    form: Settings, pending: Option<Receiver<Result<Value, String>>>, loaded: bool,
    tab: Tab, status: String, typing: String, error: bool,
    actual: Value,
}
impl Panel {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = Color32::from_rgb(7, 13, 22);
        style.visuals.selection.bg_fill = Color32::from_rgb(20, 95, 112);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(19, 114, 125);
        style.spacing.item_spacing = egui::vec2(10., 7.);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        let mut app = Self { form: Settings::default(), pending: None, loaded: false, tab: Tab::Mouse,
            status: "Consultando ajustes activos…".into(), typing: String::new(), error: false, actual: Value::Null };
        app.start("estado", None, &cc.egui_ctx); app
    }
    fn start(&mut self, action: &'static str, data: Option<Value>, ctx: &egui::Context) {
        if self.pending.is_some() { return; }
        let (tx, rx) = mpsc::channel(); self.pending = Some(rx); self.error = false;
        self.status = if action == "guardar" { "Aplicando y verificando…" } else { "Leyendo perfil…" }.into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<Value, String> {
                let mut child = Command::new("/usr/local/bin/ultra-entrada").arg(action)
                    .stdin(if data.is_some() { Stdio::piped() } else { Stdio::null() })
                    .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
                if let Some(data) = data {
                    let bytes = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
                    child.stdin.take().ok_or("Sin canal de ajustes")?.write_all(&bytes).map_err(|e| e.to_string())?;
                }
                let out = child.wait_with_output().map_err(|e| e.to_string())?;
                if !out.status.success() { return Err(String::from_utf8_lossy(&out.stderr).chars().take(400).collect()); }
                if out.stdout.len() > 16384 { return Err("Respuesta demasiado grande".into()); }
                serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
            })();
            let _ = tx.send(result); ctx.request_repaint();
        });
    }

    fn mouse(&mut self, ui: &mut egui::Ui) {
        ui.heading("Puntero");
        setting_row(ui, "Botón principal", "Elige el botón que hace clic y selecciona.", |ui| {
            egui::ComboBox::from_id_salt("boton").width(170.)
                .selected_text(if self.form.left { "Derecho" } else { "Izquierdo" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.form.left, false, "Izquierdo");
                    ui.selectable_value(&mut self.form.left, true, "Derecho");
                });
        });
        setting_row(ui, "Velocidad del puntero", "Ajusta la sensibilidad entre −1 y +1.", |ui| {
            ui.add(egui::Slider::new(&mut self.form.speed, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Ajustes rápidos").color(MUTED));
            for (label, speed) in [("Lento", -0.5), ("Normal", 0.0), ("Rápido", 0.6)] {
                let selected = self.form.accel == "flat" && (self.form.speed - speed).abs() < 0.0001;
                if ui.selectable_label(selected, label).clicked() {
                    self.form.speed = speed;
                    self.form.accel = "flat".into();
                }
            }
        });
        setting_row(ui, "Aceleración del puntero", "La adaptativa responde a la rapidez de tu movimiento.", |ui| {
            egui::ComboBox::from_id_salt("aceleracion").width(170.)
                .selected_text(match self.form.accel.as_str() {
                    "flat" => "Sin aceleración",
                    _ => "Adaptativa",
                }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.form.accel, "flat".into(), "Sin aceleración");
                    ui.selectable_value(&mut self.form.accel, "adaptive".into(), "Adaptativa");
                });
        });

        ui.add_space(5.);
        ui.heading("Desplazamiento");
        setting_row(ui, "Velocidad de la rueda", "Multiplica la distancia de desplazamiento.", |ui| {
            ui.add(egui::Slider::new(&mut self.form.scroll, 0.1..=5.0).step_by(0.1).fixed_decimals(1).suffix("×"));
        });
        setting_row(ui, "Dirección de la rueda", "Elige el sentido en que se mueve el contenido.", |ui| {
            egui::ComboBox::from_id_salt("rueda").width(170.)
                .selected_text(if self.form.natural { "Invertida" } else { "Estándar" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.form.natural, false, "Estándar");
                    ui.selectable_value(&mut self.form.natural, true, "Invertida");
                });
        });
        let focus_modes = [
            (0, "Clic para enfocar"),
            (1, "Enfoque bajo el puntero"),
            (2, "Rueda sobre la inactiva"),
            (3, "Focos independientes"),
        ];
        setting_row(ui, "Ventanas inactivas", "Elige cómo interactúa el puntero con otras ventanas.", |ui| {
            egui::ComboBox::from_id_salt("foco").width(170.)
                .selected_text(focus_modes[self.form.follow_mouse as usize].1)
                .show_ui(ui, |ui| {
                    for (value, label) in focus_modes {
                        ui.selectable_value(&mut self.form.follow_mouse, value, label);
                    }
                });
        });

        ui.add_space(5.);
        ui.heading("Visibilidad");
        setting_row(ui, "Ocultar el puntero al escribir", "El puntero reaparece cuando mueves el mouse.", |ui| {
            ui.checkbox(&mut self.form.hide_on_key_press, "Activado");
        });
        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 60.), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8., Color32::from_rgb(13, 41, 50));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Zona de prueba del puntero",
            egui::FontId::proportional(14.), ACCENT);
        if let Some(pos) = response.hover_pos() {
            ui.painter().with_clip_rect(rect).circle_stroke(pos, 12., egui::Stroke::new(2., ACCENT));
        }
        ui.small("Si la velocidad cambia solo dentro de un juego, ajusta su sensibilidad o desactiva su entrada cruda si ofrece esa opción.");
    }

    fn keyboard(&mut self, ui: &mut egui::Ui) {
        let choices = [("us", "Inglés US"), ("es", "Español"), ("latam", "Latinoamericano"),
                       ("us,latam", "US + Latinoamericano"), ("us,es", "US + Español")];
        ui.heading("Distribución y repetición");
        setting_row(ui, "Distribución", "Con dos distribuciones, usa Alt + Mayús para alternar.", |ui| {
            let title = choices.iter().find(|(value, _)| *value == self.form.layout)
                .map(|(_, label)| *label).unwrap_or(self.form.layout.as_str());
            let before = self.form.layout.clone();
            egui::ComboBox::from_id_salt("teclado").width(170.).selected_text(title).show_ui(ui, |ui| {
                for (value, label) in choices {
                    ui.selectable_value(&mut self.form.layout, value.into(), label);
                }
            });
            if before != self.form.layout {
                self.form.options = if self.form.layout.contains(',') { "grp:alt_shift_toggle".into() } else { String::new() };
            }
        });
        setting_row(ui, "Velocidad de repetición", "Repeticiones por segundo al mantener pulsada una tecla.", |ui| {
            ui.add(egui::Slider::new(&mut self.form.rate, 1..=80));
        });
        setting_row(ui, "Retardo de repetición", "Tiempo antes de que empiece a repetirse la tecla.", |ui| {
            ui.add(egui::Slider::new(&mut self.form.delay, 100..=1500).step_by(50.).suffix(" ms"));
        });
        ui.add(egui::TextEdit::singleline(&mut self.typing).hint_text("Prueba el teclado aquí"));
    }
}

impl eframe::App for Panel {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let received = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v), Err(mpsc::TryRecvError::Disconnected) => Some(Err("Se cerro la consulta".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = received {
            self.pending = None;
            match result {
                Ok(v) => if let Some(form) = Settings::decode(&v["actual"]) {
                    self.actual = form.encode(); self.form = form; self.loaded = true;
                    self.status = if v["guardado"].is_object() { "Perfil guardado · se recupera al iniciar" } else { "Ajustes actuales · sin perfil guardado" }.into();
                } else { self.status = "Todavía no hay un perfil guardado".into(); },
                Err(e) => { self.status = e; self.error = true; }
            }
        }
        if self.pending.is_some() { ui.ctx().request_repaint_after(Duration::from_millis(150)); }
        let enabled = self.loaded && self.pending.is_none();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label(RichText::new("ULTRA  /  CONFIGURACIÓN").small().strong().color(ACCENT));
            ui.heading(RichText::new("Mouse y teclado").size(28.));
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Mouse, "Mouse");
                ui.selectable_value(&mut self.tab, Tab::Keyboard, "Teclado");
            });
            ui.separator();
            egui::Panel::bottom("acciones").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let save = egui::Button::new(RichText::new("Aplicar y guardar").color(Color32::from_rgb(7, 13, 22)).strong()).fill(ACCENT);
                    if ui.add_enabled(enabled, save).clicked() {
                        self.start("guardar", Some(self.form.encode()), ui.ctx());
                    }
                    if ui.add_enabled(enabled, egui::Button::new("Recuperar guardado")).clicked() {
                        self.start("restaurar", None, ui.ctx());
                    }
                    if ui.add_enabled(enabled, egui::Button::new("Valores iniciales")).clicked() {
                        self.form = Settings::default();
                    }
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Volver a consultar")).clicked() {
                        self.start("estado", None, ui.ctx());
                    }
                });
                let (status, color) = if self.error {
                    (self.status.as_str(), Color32::from_rgb(255, 118, 134))
                } else if enabled && !self.form.matches(&self.actual) {
                    ("Cambios pendientes de guardar", Color32::from_rgb(245, 210, 100))
                } else {
                    (self.status.as_str(), MUTED)
                };
                ui.colored_label(color, status);
            });
            egui::ScrollArea::vertical().id_salt(self.tab == Tab::Mouse).show(ui, |ui| {
                ui.add_enabled_ui(self.loaded && self.pending.is_none(), |ui| match self.tab {
                    Tab::Mouse => self.mouse(ui),
                    Tab::Keyboard => self.keyboard(ui),
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions { renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title("ULTRA Entrada · Mouse y teclado")
            .with_app_id("ultra-entrada").with_inner_size([780., 740.]).with_min_inner_size([540., 500.]),
        ..Default::default() };
    eframe::run_native("ultra-entrada", options, Box::new(|cc| Ok(Box::new(Panel::new(cc)))))
}
