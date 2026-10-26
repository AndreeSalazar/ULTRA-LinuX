// SPDX-License-Identifier: GPL-2.0-only
//! Estado de evidencia por contenido. Solo el motor instalado, nunca scripts del proyecto.
use crate::Colores;
use eframe::egui::{self, RichText};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const ENGINE: &str = "/usr/share/ultra/revision-engine/revisar-proyecto.py";
const MAX_BYTES: u64 = 1048576;

fn pending(reason: &str) -> Value { json!({"schema":1,"state":"pendiente","units":[],"counts":{"revisado":0,"caducado":0,"pendiente":0,"diagnostico":0},"production_certified":false,"reason":reason}) }

fn checked(v: Value, limit: usize) -> Result<Value,String> {
    if v["schema"]!=1 || v["production_certified"]!=false || !["revisado","caducado","pendiente","diagnostico"].contains(&v["state"].as_str().unwrap_or("")) {
        return Err("respuesta del motor desconocida".into());
    }
    let rows=v["units"].as_array().ok_or("faltan unidades")?;
    if rows.len()>1000 {return Err("demasiadas unidades".into());}
    let mut clean=Vec::new();
    for row in rows {
        let file=row["file"].as_str().ok_or("ruta ausente")?;
        let state=row["state"].as_str().ok_or("estado ausente")?;
        if file.len()>512 || file.is_empty() || Path::new(file).components().any(|c| !matches!(c,Component::Normal(_)))
            || !["revisado","caducado","pendiente","diagnostico"].contains(&state) || row["runtime_failure_confirmed"]!=false {
            return Err("unidad fuera del contrato".into());
        }
        let reasons:Vec<String>=row["reasons"].as_array().ok_or("motivos ausentes")?.iter().take(8)
            .filter_map(Value::as_str).map(|s|s.chars().take(240).collect()).collect();
        clean.push(json!({"file":file,"state":state,"reasons":reasons,"runtime_failure_confirmed":false}));
    }
    if v["state"]=="revisado" && (rows.is_empty() || rows.iter().any(|r|r["state"]!="revisado")) {
        return Err("aprobacion incompatible con las unidades".into());
    }
    clean.sort_by_key(|r|r["state"]=="revisado");let total=clean.len();clean.truncate(limit.clamp(1,100));
    let counts=json!({"revisado":rows.iter().filter(|r|r["state"]=="revisado").count(),
        "caducado":rows.iter().filter(|r|r["state"]=="caducado").count(),"pendiente":rows.iter().filter(|r|r["state"]=="pendiente").count(),
        "diagnostico":rows.iter().filter(|r|r["state"]=="diagnostico").count()});
    Ok(json!({"schema":1,"state":v["state"],"counts":counts,"units":clean,"remaining":total.saturating_sub(limit.clamp(1,100)),
        "production_certified":false,"reason":v["reason"].as_str().unwrap_or("").chars().take(400).collect::<String>(),
        "scope":"Comprobacion de contenido actual; revision estatica seleccionada, no ausencia de UB."}))
}

pub fn consultar(root: &Path, limit: usize) -> Result<Value,String> {
    if !Path::new(ENGINE).is_file() {return Ok(pending("Falta el motor instalado de ULTRA; no se ejecuta codigo del proyecto"));}
    let mut command=Command::new("/usr/bin/python3");
    command.args(["-I","-S",ENGINE,"estado-json","--raiz"]).arg(root).env_clear().env("PATH","/usr/bin:/bin").env("LC_ALL","C")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(unix)] {use std::os::unix::process::CommandExt;command.process_group(0);}
    let mut child=command.spawn().map_err(|e|e.to_string())?;
    let output=child.stdout.take().ok_or("sin salida del motor")?;
    let reader=std::thread::spawn(move || {let mut bytes=Vec::new();output.take(MAX_BYTES+1).read_to_end(&mut bytes).map(|_|bytes)});
    let start=Instant::now();
    loop {
        if let Some(status)=child.try_wait().map_err(|e|e.to_string())? {
            let bytes=reader.join().map_err(|_|"lector del motor fallo")?.map_err(|e|e.to_string())?;
            if !status.success() || bytes.len() as u64>MAX_BYTES {return Err("motor incompleto o salida excesiva".into());}
            return checked(serde_json::from_slice(&bytes).map_err(|e|e.to_string())?,limit);
        }
        if start.elapsed()>Duration::from_secs(40) {
            #[cfg(unix)] {let _=Command::new("/usr/bin/kill").args(["-KILL","--",&format!("-{}",child.id())]).status();}
            let _=child.kill();let _=child.wait();let _=reader.join();return Err("motor de revision excedio 40 s".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[derive(Default)]
pub struct Panel {root:PathBuf,version:u64,result:Option<Value>,receiver:Option<(PathBuf,u64,Receiver<Result<Value,String>>)>,checked:Option<Instant>,page:usize}

impl Panel {
    pub fn show(&mut self,ui:&mut egui::Ui,c:&Colores,root:&Path,version:u64)->Option<String> {
        if self.root!=root || self.version!=version {
            self.root=root.to_path_buf();self.version=version;self.result=None;self.checked=None;self.page=0;
        }
        if let Some((job_root,job_version,rx))=&self.receiver {
            if let Ok(result)=rx.try_recv(){
                if *job_root==self.root && *job_version==self.version {self.result=Some(result.unwrap_or_else(|e|pending(&e)));self.checked=Some(Instant::now());}
                self.receiver=None;
            }
        }
        let mut refresh=false;let mut selected=None;
        egui::CollapsingHeader::new(RichText::new("EVIDENCIA // CONTENIDO Y VIGENCIA").color(c.acento).strong()).default_open(true).show(ui,|ui| {
            ui.label(RichText::new("Revisado no significa libre de UB. Los diagnosticos requieren comprobar su causa.").size(10.5).color(c.tenue));
            refresh=ui.add_enabled(self.receiver.is_none(),egui::Button::new("Comprobar contenido actual")).clicked();
            if self.receiver.is_some(){ui.label("Comprobando en otro hilo…");}
            if let Some(result)=&self.result {
                let state=result["state"].as_str().unwrap_or("pendiente");
                let color=match state {"revisado"=>c.verde,"caducado"=>c.aviso,"diagnostico"=>c.error,_=>c.tenue};
                ui.label(RichText::new(format!("{} · {} revisados · {} caducados · {} pendientes · {} diagnosticos",state,
                    result["counts"]["revisado"],result["counts"]["caducado"],result["counts"]["pendiente"],result["counts"]["diagnostico"])).color(color).size(11.0));
                if let Some(when)=self.checked {ui.label(RichText::new(format!("Comprobado hace {} s; revalidar antes de publicar",when.elapsed().as_secs())).color(c.tenue).size(10.0));}
                if let Some(reason)=result["reason"].as_str().filter(|s|!s.is_empty()){ui.label(RichText::new(reason).color(c.aviso).size(10.5));}
                let rows=result["units"].as_array();let total=rows.map_or(0,Vec::len);let pages=total.div_ceil(6).max(1);
                self.page=self.page.min(pages-1);
                ui.horizontal(|ui|{if ui.add_enabled(self.page>0,egui::Button::new("Anterior")).clicked(){self.page-=1;}
                    ui.label(format!("{} / {}",self.page+1,pages));
                    if ui.add_enabled(self.page+1<pages,egui::Button::new("Siguiente")).clicked(){self.page+=1;}});
                for row in rows.into_iter().flatten().skip(self.page*6).take(6) {
                    let file=row["file"].as_str().unwrap_or("");
                    let title=format!("{} · {}",row["state"].as_str().unwrap_or("pendiente"),file.rsplit('/').next().unwrap_or(file));
                    if ui.small_button(title).on_hover_text(file).clicked(){selected=Some(file.into());}
                    if let Some(reason)=row["reasons"].as_array().and_then(|r|r.first()){
                        ui.add(egui::Label::new(RichText::new(reason.as_str().unwrap_or("")).color(c.tenue).size(10.0)).wrap());
                    }
                }
                if result["remaining"].as_u64().unwrap_or(0)>0{ui.label("Mas unidades: usar la consulta revision_estado con mayor limite");}
            }
        });
        // No polling scan every frame: first use, project analysis changes or explicit recheck.
        if self.receiver.is_none() && (refresh || self.result.is_none()) {
            let (tx,rx)=mpsc::channel();let job_root=root.to_path_buf();let context_root=job_root.clone();let ctx=ui.ctx().clone();
            std::thread::spawn(move ||{let _=tx.send(consultar(&context_root,100));ctx.request_repaint();});self.receiver=Some((job_root,version,rx));
        }
        if self.receiver.is_some(){ui.ctx().request_repaint_after(Duration::from_millis(150));}
        selected
    }
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]fn refuses_runtime_confirmation_and_outside_paths(){
        let mut v=json!({"schema":1,"state":"diagnostico","production_certified":false,"units":[{"file":"a.c","state":"diagnostico","reasons":[],"runtime_failure_confirmed":true}]});
        assert!(checked(v.clone(),10).is_err());v["units"][0]["runtime_failure_confirmed"]=json!(false);v["units"][0]["file"]=json!("../a.c");assert!(checked(v,10).is_err());
    }
    #[test]fn caps_rows_and_preserves_pending(){
        let v=json!({"schema":1,"state":"pendiente","production_certified":false,"units":[{"file":"a.c","state":"pendiente","reasons":["falta comprobacion"],"runtime_failure_confirmed":false},{"file":"b.c","state":"revisado","reasons":[],"runtime_failure_confirmed":false}]});
        let result=checked(v,1).unwrap();assert_eq!(result["remaining"],1);assert_eq!(result["units"][0]["state"],"pendiente");
    }
    #[test]fn refuses_green_summary_with_pending_rows(){
        let v=json!({"schema":1,"state":"revisado","production_certified":false,"units":[{"file":"a.c","state":"pendiente","reasons":[],"runtime_failure_confirmed":false}]});
        assert!(checked(v,10).is_err());
    }
}
