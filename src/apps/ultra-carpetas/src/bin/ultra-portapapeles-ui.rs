// SPDX-License-Identifier: GPL-2.0-only
//! Event-driven clipboard cards. Entries remain data; copying never executes them.
use eframe::egui::{self, Color32, RichText};
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

struct Card { id: i64, pinned: bool, preview: String, image: Option<egui::TextureHandle> }
struct Clipboard {
    cards: Vec<Card>, pending: Option<Receiver<Result<Value,String>>>, query: String,
    page: i64, more: bool, quota: i64, paused: bool, total: i64, pins: i64,
    used: f64, status: String, close_after: bool, actual_quota: i64,
}
impl Clipboard {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut style=(*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals.panel_fill=Color32::from_rgb(7,13,22);
        style.visuals.selection.bg_fill=Color32::from_rgb(20,95,112);
        style.spacing.item_spacing=egui::vec2(10.,10.);
        cc.egui_ctx.set_style_of(egui::Theme::Dark,style);
        let mut app=Self {cards:Vec::new(),pending:None,query:String::new(),page:0,more:false,
            quota:256,paused:false,total:0,pins:0,used:0.,status:"Cargando historial…".into(),close_after:false,actual_quota:256};
        app.start("listar",json!({}),&cc.egui_ctx);app
    }
    fn request(&self) -> Value { json!({"query":self.query,"page":self.page}) }
    fn start(&mut self, action: &'static str, data: Value, ctx: &egui::Context) {
        if self.pending.is_some(){return;}
        let (tx,rx)=mpsc::channel();self.pending=Some(rx);self.status="Consultando…".into();
        self.close_after=action=="copiar";let ctx=ctx.clone();
        std::thread::spawn(move || {
            let result=(|| -> Result<Value,String> {
                let mut cmd=Command::new("/usr/local/bin/ultra-portapapeles");cmd.arg(action);
                if action=="listar" {cmd.arg(data.to_string());}
                let mut child=cmd.stdin(if action=="listar" || action=="borrar" {Stdio::null()}else{Stdio::piped()})
                    .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
                if let Some(mut input)=child.stdin.take(){input.write_all(data.to_string().as_bytes()).map_err(|e|e.to_string())?;}
                let out=child.wait_with_output().map_err(|e|e.to_string())?;
                if !out.status.success(){return Err(String::from_utf8_lossy(&out.stderr).chars().take(400).collect());}
                if out.stdout.len()>1024*1024 {return Err("Respuesta demasiado grande".into());}
                serde_json::from_slice(&out.stdout).map_err(|e|e.to_string())
            })();let _=tx.send(result);ctx.request_repaint();
        });
    }
    fn receive(&mut self, v: Value, ctx: &egui::Context) {
        self.page=v["page"].as_i64().unwrap_or(0);self.more=v["more"].as_bool().unwrap_or(false);
        self.query=v["query"].as_str().unwrap_or("").into();self.quota=v["quota_mib"].as_i64().unwrap_or(256);
        self.actual_quota=self.quota;
        self.paused=v["paused"].as_bool().unwrap_or(false);self.total=v["total"].as_i64().unwrap_or(0);
        self.pins=v["pins"].as_i64().unwrap_or(0);self.used=v["used_bytes"].as_f64().unwrap_or(0.)/(1024.*1024.);
        self.cards.clear();
        if let Some(rows)=v["entries"].as_array(){for row in rows.iter().take(30){
            let Some(id)=row["id"].as_i64() else{continue;};
            let mut image=None;let t=&row["thumbnail"];
            if let (Some(w),Some(h),Some(bytes))=(t["width"].as_u64(),t["height"].as_u64(),t["rgba"].as_array()){
                if w>0 && w<=96 && h>0 && h<=64 && bytes.len()==(w*h*4) as usize{
                    let rgba:Option<Vec<u8>>=bytes.iter().map(|b|b.as_u64().and_then(|v|u8::try_from(v).ok())).collect();
                    if let Some(rgba)=rgba{image=Some(ctx.load_texture(format!("clip-{id}"),egui::ColorImage::from_rgba_unmultiplied([w as usize,h as usize],&rgba),egui::TextureOptions::LINEAR));}
                }
            }
            self.cards.push(Card{id,pinned:row["pinned"].as_bool().unwrap_or(false),preview:row["preview"].as_str().unwrap_or("Entrada").chars().take(384).collect(),image});
        }}
        self.status="Los pines se conservan al limpiar. Copiar cierra el panel; despues pulsa Ctrl+V.".into();
    }
}
impl eframe::App for Clipboard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let result=self.pending.as_ref().and_then(|rx|match rx.try_recv(){
            Ok(v)=>Some(v),Err(mpsc::TryRecvError::Disconnected)=>Some(Err("Se cerro la consulta".into())),
            Err(mpsc::TryRecvError::Empty)=>None});
        if let Some(result)=result{self.pending=None;match result{
            Ok(v)=>{self.receive(v,ui.ctx());if self.close_after{ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);}},
            Err(e)=>{self.status=e;self.close_after=false;}
        }}
        if self.pending.is_some(){ui.ctx().request_repaint_after(Duration::from_millis(150));}
        let cyan=Color32::from_rgb(60,225,240);let green=Color32::from_rgb(100,230,160);
        egui::CentralPanel::default().show(ui,|ui|{
            ui.heading(RichText::new("PORTAPAPELES // Pines").color(cyan));
            ui.label(format!("{} entradas · {} pines · ocupacion estimada {:.2} / {} MiB",self.total,self.pins,self.used,self.actual_quota));
            let mut action=None;
            ui.add_enabled_ui(self.pending.is_none(),|ui|{
                ui.horizontal(|ui|{
                    let field=ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Buscar en las vistas previas").char_limit(80).desired_width(270.));
                    if ui.button("Buscar").clicked() || (field.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter))){self.page=0;action=Some(("listar",self.request()));}
                    if ui.button("Actualizar").clicked(){action=Some(("listar",self.request()));}
                });
                ui.horizontal(|ui|{
                    if ui.checkbox(&mut self.paused,"Pausar nuevas copias").changed(){action=Some(("pausa",json!({"paused":self.paused,"query":self.query,"page":self.page})));}
                    if ui.button("Limpiar no fijados").clicked(){action=Some(("borrar",json!({})));}
                });
                ui.horizontal(|ui|{
                    ui.label("Presupuesto:");
                    egui::ComboBox::from_id_salt("presupuesto").selected_text(format!("{} MiB",self.quota)).show_ui(ui,|ui|{
                        for n in [32,64,128,256,512,1024,2048,4096]{ui.selectable_value(&mut self.quota,n,format!("{n} MiB"));}
                    });
                    if ui.button("Aplicar presupuesto").clicked(){action=Some(("presupuesto",json!({"mib":self.quota,"query":self.query,"page":self.page})));}
                });
                ui.small("Sin limite fijo de cantidad. Al llenar el presupuesto salen las copias no fijadas mas antiguas.");
                ui.separator();
                egui::ScrollArea::vertical().max_height((ui.available_height()-105.).max(100.)).show(ui,|ui|{
                    if self.cards.is_empty(){ui.label("Sin resultados. Copia texto o una imagen para comenzar.");}
                    for card in &self.cards{
                        egui::Frame::group(ui.style()).show(ui,|ui|{
                            if card.pinned{ui.colored_label(green,"FIJADO");}
                            ui.horizontal_wrapped(|ui|{
                                if let Some(image)=&card.image{ui.image((image.id(),image.size_vec2()));}
                                ui.add(egui::Label::new(&card.preview).wrap());
                            });
                            ui.horizontal(|ui|{
                                if ui.button(RichText::new("Copiar").color(cyan)).clicked(){action=Some(("copiar",json!({"id":card.id,"query":self.query,"page":self.page})));}
                                if ui.button(if card.pinned{"Desfijar"}else{"Fijar"}).clicked(){action=Some(("pin",json!({"id":card.id,"query":self.query,"page":self.page})));}
                                if ui.add_enabled(!card.pinned,egui::Button::new("Eliminar")).clicked(){action=Some(("eliminar",json!({"id":card.id,"query":self.query,"page":self.page})));}
                            });
                        });
                    }
                });
                ui.horizontal(|ui|{
                    if ui.add_enabled(self.page>0,egui::Button::new("Anterior")).clicked(){self.page-=1;action=Some(("listar",self.request()));}
                    ui.label(format!("Pagina {}",self.page+1));
                    if ui.add_enabled(self.more,egui::Button::new("Siguiente")).clicked(){self.page+=1;action=Some(("listar",self.request()));}
                    if ui.button("Cerrar").clicked(){ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);}
                });
            });
            ui.label(&self.status);
            if let Some((cmd,data))=action{self.start(cmd,data,ui.ctx());}
            if ui.input(|i|i.key_pressed(egui::Key::Escape)){ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);}
        });
    }
}
fn main() -> eframe::Result{
    let options=eframe::NativeOptions{renderer:eframe::Renderer::Wgpu,
        viewport:egui::ViewportBuilder::default().with_title("ULTRA Portapapeles · Pines").with_app_id("ultra-portapapeles")
            .with_inner_size([680.,720.]).with_min_inner_size([560.,500.]),..Default::default()};
    eframe::run_native("ultra-portapapeles",options,Box::new(|cc|Ok(Box::new(Clipboard::new(cc)))))
}
