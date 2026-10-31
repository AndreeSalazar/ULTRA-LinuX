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
    layout: String, options: String, rate: i32, delay: i32,
}
impl Default for Settings {
    fn default() -> Self { Self { speed: 0., scroll: 1., accel: String::new(), left: false,
        natural: false, layout: "us".into(), options: String::new(), rate: 25, delay: 600 } }
}
impl Settings {
    fn decode(v: &Value) -> Option<Self> { Some(Self {
        speed: v["sensitivity"].as_f64()?, scroll: v["scroll_factor"].as_f64()?,
        accel: v["accel_profile"].as_str()?.into(), left: v["left_handed"].as_bool()?,
        natural: v["natural_scroll"].as_bool()?, layout: v["kb_layout"].as_str()?.into(),
        options: v["kb_options"].as_str()?.into(), rate: v["repeat_rate"].as_i64()? as i32,
        delay: v["repeat_delay"].as_i64()? as i32,
    }) }
    fn encode(&self) -> Value { json!({"sensitivity":self.speed, "scroll_factor":self.scroll,
        "accel_profile":self.accel, "left_handed":self.left, "natural_scroll":self.natural,
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

struct Panel {
    form: Settings, pending: Option<Receiver<Result<Value, String>>>, loaded: bool,
    saved: bool, status: String, typing: String, error: bool,
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
        style.spacing.item_spacing = egui::vec2(12., 12.);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        let mut app = Self { form: Settings::default(), pending: None, loaded: false, saved: false,
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
                    self.actual = form.encode(); self.form = form; self.loaded = true; self.saved = v["guardado"].is_object();
                    self.status = if self.saved { "Ajustes verificados · perfil guardado" } else { "Ajustes actuales · sin perfil guardado" }.into();
                } else { self.status = "Todavia no hay un perfil guardado".into(); },
                Err(e) => { self.status = e; self.error = true; }
            }
        }
        if self.pending.is_some() { ui.ctx().request_repaint_after(Duration::from_millis(150)); }
        let cyan = Color32::from_rgb(60, 225, 240);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.);
            ui.heading(RichText::new("ENTRADA // Mouse y teclado").color(cyan));
            ui.label("ULTRA LinuX · tu velocidad, tus teclas");
            ui.separator();
            let enabled = self.loaded && self.pending.is_none();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.heading(RichText::new("Mouse").color(cyan));
                    ui.add(egui::Slider::new(&mut self.form.speed, -1.0..=1.0).text("Velocidad del puntero").step_by(0.05));
                    egui::ComboBox::from_id_salt("aceleracion").selected_text(match self.form.accel.as_str() {
                        "flat" => "Sin aceleracion", "adaptive" => "Aceleracion adaptativa", _ => "Perfil del sistema",
                    }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.form.accel, String::new(), "Perfil del sistema");
                        ui.selectable_value(&mut self.form.accel, "flat".into(), "Sin aceleracion");
                        ui.selectable_value(&mut self.form.accel, "adaptive".into(), "Aceleracion adaptativa");
                    });
                    ui.checkbox(&mut self.form.left, "Boton principal derecho (zurdo)");
                    ui.checkbox(&mut self.form.natural, "Invertir direccion de la rueda");
                    ui.add(egui::Slider::new(&mut self.form.scroll, 0.1..=5.0).text("Multiplicador de desplazamiento").step_by(0.1));
                    ui.horizontal(|ui| {
                        if ui.button("Normal").clicked() { self.form.speed = 0.; self.form.accel = "flat".into(); }
                        if ui.button("Rapido").clicked() { self.form.speed = 0.6; self.form.accel = "flat".into(); }
                        ui.label("Presets: se aplican al guardar.");
                    });
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 55.), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4., Color32::from_rgb(12, 34, 36));
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Mueve el puntero aqui para probar", egui::FontId::proportional(15.), cyan);
                    if response.hovered() { if let Some(p) = response.hover_pos() { ui.painter().circle_stroke(p, 12., egui::Stroke::new(2., cyan)); } }
                    ui.separator();
                    ui.heading(RichText::new("Teclado").color(Color32::from_rgb(192, 120, 245)));
                    let choices = [("us", "Ingles US"), ("es", "Espanol"), ("latam", "Latinoamericano"),
                                   ("us,latam", "US + Latinoamericano (Alt+Shift)"), ("us,es", "US + Espanol (Alt+Shift)")];
                    let title = choices.iter().find(|(v, _)| *v == self.form.layout).map(|(_, label)| *label).unwrap_or(self.form.layout.as_str());
                    let before = self.form.layout.clone();
                    egui::ComboBox::from_id_salt("teclado").selected_text(title).show_ui(ui, |ui| {
                        for (v, label) in choices { ui.selectable_value(&mut self.form.layout, v.into(), label); }
                    });
                    if before != self.form.layout { self.form.options = if self.form.layout.contains(',') { "grp:alt_shift_toggle".into() } else { String::new() }; }
                    ui.add(egui::Slider::new(&mut self.form.rate, 1..=80).text("Repeticiones por segundo"));
                    ui.add(egui::Slider::new(&mut self.form.delay, 100..=1500).text("Espera antes de repetir (ms)").step_by(50.));
                    ui.text_edit_singleline(&mut self.typing).on_hover_text("Prueba letras, simbolos y repeticion. No se guarda este texto.");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(RichText::new("Aplicar y guardar").color(cyan)).clicked() { self.start("guardar", Some(self.form.encode()), ui.ctx()); }
                        if ui.button("Recuperar guardado").clicked() { self.start("restaurar", None, ui.ctx()); }
                        if ui.button("Valores iniciales").clicked() { self.form = Settings::default(); }
                    });
                });
                if ui.button("Volver a consultar").clicked() { self.start("estado", None, ui.ctx()); }
                ui.colored_label(if self.error { Color32::from_rgb(255, 100, 115) } else { Color32::from_rgb(100, 230, 160) }, &self.status);
                if self.loaded && !self.form.matches(&self.actual) { ui.colored_label(Color32::from_rgb(245, 210, 100), "Cambios sin aplicar: pulsa Aplicar y guardar."); }
                ui.separator();
                ui.label("Los cambios se aplican al guardar y se recuperan al iniciar el escritorio.");
                ui.small("Esto controla el escritorio. La sensibilidad con entrada cruda y el DPI de hardware se ajustan en el juego o dispositivo.");
                if ui.button("Cerrar").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions { renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title("ULTRA Entrada · Mouse y teclado")
            .with_app_id("ultra-entrada").with_inner_size([720., 660.]).with_min_inner_size([520., 500.]),
        ..Default::default() };
    eframe::run_native("ultra-entrada", options, Box::new(|cc| Ok(Box::new(Panel::new(cc)))))
}
