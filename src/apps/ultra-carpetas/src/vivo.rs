// SPDX-License-Identifier: GPL-2.0-only
//! La ventana en vivo para el MCP: publica lo que se ve (`ventana.json`) y aplica lo que el
//! MCP pide ensenar (`peticion-*.json`). Ver `puente.rs`.
//!
//! Un hilo vigila las peticiones cada 300 ms y despierta a la ventana (que no repinta si
//! nadie la toca); cada 5 s la despierta tambien para el latido.

use crate::{puente, App};
use carpetas_nucleo::estado::Lateral;
use carpetas_nucleo::{Accion, AccionGrupo, Comando, Modo};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Lo que la ventana recuerda del puente.
pub struct Vivo {
    /// La carpeta del puente de la carpeta abierta (la comparte con el hilo).
    dir: Arc<Mutex<Option<PathBuf>>>,
    de: PathBuf,
    rx: mpsc::Receiver<Value>,
    publicado: Option<(u64, Instant)>,
}

/// Cada cuanto se reescribe aunque nada cambie (para que el MCP sepa que la ventana sigue).
const LATIDO: Duration = Duration::from_secs(5);

impl Vivo {
    pub fn nuevo(ctx: egui_ctx::Context) -> Vivo {
        let dir: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::channel();
        let d = dir.clone();
        std::thread::Builder::new().name("puente-mcp".into()).spawn(move || {
            let mut ultimo_latido = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(300));
                let actual = d.lock().ok().and_then(|g| g.clone());
                if let Some(dir) = actual {
                    let p = puente::recoger(&dir);
                    if !p.is_empty() {
                        for x in p { if tx.send(x).is_err() { return; } }
                        ctx.request_repaint();
                    }
                }
                if ultimo_latido.elapsed() >= LATIDO { ultimo_latido = Instant::now(); ctx.request_repaint(); }
            }
        }).ok();
        Vivo { dir, de: PathBuf::new(), rx, publicado: None }
    }
}

/// egui por su nombre corto.
mod egui_ctx { pub use eframe::egui::Context; }

/// Una ruta relativa sin `..` ni raiz (las peticiones vienen de fuera).
fn relativa(r: &str) -> Option<String> {
    let p = std::path::Path::new(r);
    if r.is_empty() || p.components().any(|c| !matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir)) { return None; }
    Some(r.trim_start_matches("./").to_string())
}

fn rutas(p: &Value, campo: &str) -> Vec<String> {
    p[campo].as_array().into_iter().flatten().filter_map(|x| x.as_str().and_then(relativa)).take(64).collect()
}

impl App {
    /// Cada cuadro: aplicar lo que pidio el MCP y publicar lo que se ve si cambio.
    pub(crate) fn puente_vivo(&mut self) {
        // La carpeta abierta pudo cambiar (Atras, Arriba, otra carpeta): otro puente.
        if self.vivo.de != self.carpeta {
            self.vivo.de = self.carpeta.clone();
            let d = puente::carpeta(&self.carpeta);
            if let Ok(mut g) = self.vivo.dir.lock() { *g = d; }
            self.vivo.publicado = None;
        }
        let pedidas: Vec<Value> = self.vivo.rx.try_iter().collect();
        for p in pedidas { self.aplicar_peticion(&p); }
        let version = self.estado.version;
        let toca = match self.vivo.publicado { None => true, Some((v, t)) => v != version || t.elapsed() >= LATIDO };
        if !toca { return; }
        let Some(dir) = self.vivo.dir.lock().ok().and_then(|g| g.clone()) else { return };
        let vista = self.vista_para_mcp();
        if puente::publicar(&dir, &vista) { self.vivo.publicado = Some((version, Instant::now())); }
    }

    /// Lo que se ve, en pocas cifras (con tope).
    fn vista_para_mcp(&mut self) -> Value {
        // El camino y la combinacion se rehacen aqui tambien (el panel quiza no se pinto aun).
        self.combinar.al_dia(&self.estado);
        let e = &self.estado;
        let aislado = e.aislar.as_deref().map(|a| match a {
            "" => "los fallos".to_string(),
            x if x.starts_with(carpetas_nucleo::estado::MARCA_VARIOS) => format!("{} elegidos", x.split(carpetas_nucleo::estado::MARCA_VARIOS).filter(|s| !s.is_empty()).count()),
            x => x.to_string(),
        });
        let editor = e.editor.activa().map(|x| json!({ "archivo": x.id, "linea": x.ir_a, "sin_guardar": x.sucio }));
        let camino = self.combinar.camino.as_ref().filter(|_| e.grupo.len() >= 2 || e.seleccion.len() >= 2)
            .map(|c| json!({ "a": c.a, "b": c.b, "por_que": c.por_que, "saltos": c.pasos.len() }));
        json!({
            "carpeta": self.carpeta.display().to_string(),
            "modo": match e.ajustes.modo { Modo::Iconos => "iconos", Modo::Grafo => "grafo", Modo::Ciudad => "ciudad" },
            "panel": if e.ajustes.explorador { format!("{:?}", e.lateral).to_lowercase() } else { "cerrado".into() },
            "foco": e.disp.foco,
            "seleccion": e.seleccion.iter().take(64).collect::<Vec<_>>(),
            "grupo": e.grupo,
            "aislado": aislado,
            "filtro_eje": e.filtro_eje.map(|x| x.nombre()),
            "editor": editor,
            "ultimo_revelado": e.revelado.0,
            "camino": camino,
            "aviso": e.aviso,
        })
    }

    /// Lo que pide el MCP: solo mover la vista (grupo, revelar, camino, aislar, limpiar).
    fn aplicar_peticion(&mut self, p: &Value) {
        let accion = p["accion"].as_str().unwrap_or("");
        let grupo = |s: &mut App, r: Vec<String>| {
            s.despachar(Accion::Grupo(AccionGrupo::Limpiar));
            s.despachar(Accion::Grupo(AccionGrupo::Sumar(r)));
        };
        match accion {
            "grupo" | "aislar" => {
                let r = rutas(p, "rutas");
                if r.is_empty() { return; }
                grupo(self, r);
                match p["modo"].as_str() {
                    Some("grafo") => self.despachar(Accion::Grupo(AccionGrupo::AlGrafo)),
                    Some("ciudad") => self.despachar(Accion::Comando(Comando::Ciudad)),
                    _ => {}
                }
                if accion == "aislar" && self.estado.aislar.is_none() { self.despachar(Accion::Grupo(AccionGrupo::Aislar)); }
                self.despachar(Accion::Lateral(Lateral::Combinar));
            }
            "revelar" => {
                let Some(r) = p["ruta"].as_str().and_then(relativa) else { return };
                match p["linea"].as_u64() {
                    Some(l) => self.despachar(Accion::IrALinea { id: r, linea: l.max(1) as u32 }),
                    None => self.despachar(Accion::Revelar { id: r, editor: false }),
                }
            }
            "camino" => {
                let (Some(a), Some(b)) = (p["a"].as_str().and_then(relativa), p["b"].as_str().and_then(relativa)) else { return };
                grupo(self, vec![a.clone(), b.clone()]);
                self.despachar(Accion::Grupo(AccionGrupo::Extremos(a, b)));
                self.despachar(Accion::Lateral(Lateral::Combinar));
            }
            "limpiar" => {
                self.despachar(Accion::Grupo(AccionGrupo::Limpiar));
                self.despachar(Accion::Aislar(None));
            }
            _ => return,
        }
        self.estado.aviso = Some(format!("el SUPER GRAFO (MCP) te ensena: {accion}"));
    }
}
