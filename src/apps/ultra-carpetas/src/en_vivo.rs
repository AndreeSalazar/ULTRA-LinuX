// SPDX-License-Identifier: GPL-2.0-only
//! 4D en vivo: el trafico de un programa mientras corre, pintado sobre la Ciudad.
//!
//! El programa escribe en la raiz del proyecto `.super-grafo-vivo.jsonl`, una linea por
//! tramo de trabajo: `{"t":1234.5,"ruta":"web/estilo","us":5400,"desde":"web/dom"}` (`ruta`
//! es la carpeta que trabajo; `desde`, de donde venia el trabajo; `us`, cuanto). QUIMERA lo
//! hace con `QUIMERA_VIVO=.super-grafo-vivo.jsonl`; cualquier otro programa puede hacerlo.
//!
//! Un hilo sigue el archivo mientras crece (cada 100 ms, desde donde se quedo; si se vacia,
//! empieza otra vez). La Ciudad enciende las carpetas que trabajan segun sus ms por segundo
//! y dibuja el trafico entre ellas. El archivo empieza por punto: el indice no lo ve.

use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// El archivo que se sigue, en la raiz del proyecto.
pub const ARCHIVO: &str = ".super-grafo-vivo.jsonl";
/// Lo que cuenta como «ahora».
pub const VENTANA: Duration = Duration::from_secs(2);
/// Como mucho tantas lineas por lectura (un archivo enorme no se lee entero de golpe).
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Traza { pub t: f64, pub ruta: String, pub us: u64, pub desde: Option<String> }

/// Una linea del archivo (None si no es una traza).
pub fn leer_linea(l: &str) -> Option<Traza> {
    let v: Value = serde_json::from_str(l.trim()).ok()?;
    let ruta = v["ruta"].as_str()?.trim_matches('/').to_string();
    if ruta.is_empty() || ruta.contains("..") { return None; }
    Some(Traza { t: v["t"].as_f64().unwrap_or(0.0), ruta, us: v["us"].as_u64().unwrap_or(0), desde: v["desde"].as_str().map(|d| d.trim_matches('/').to_string()) })
}

/// Lo que paso en una ventana de tiempo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resumen {
    /// (carpeta, ms de trabajo por segundo), de mas a menos.
    pub por_ruta: Vec<(String, f64)>,
    /// (desde, hasta, veces), de mas a menos.
    pub trafico: Vec<(String, String, u32)>,
    pub trazas: usize,
}

/// Junta las trazas de `segundos` de tiempo.
pub fn resumir<'a>(trazas: impl IntoIterator<Item = &'a Traza>, segundos: f64) -> Resumen {
    let mut por: BTreeMap<&str, u64> = BTreeMap::new();
    let mut tr: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    let mut n = 0;
    for t in trazas {
        n += 1;
        *por.entry(&t.ruta).or_default() += t.us;
        if let Some(d) = t.desde.as_deref().filter(|d| *d != t.ruta) { *tr.entry((d, &t.ruta)).or_default() += 1; }
    }
    let s = segundos.max(0.001);
    let mut por_ruta: Vec<(String, f64)> = por.into_iter().map(|(r, us)| (r.to_string(), us as f64 / 1000.0 / s)).collect();
    por_ruta.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut trafico: Vec<(String, String, u32)> = tr.into_iter().map(|((a, b), k)| (a.to_string(), b.to_string(), k)).collect();
    trafico.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    Resumen { por_ruta, trafico, trazas: n }
}

/// Las trazas de los ultimos `segundos` del archivo (por su propio reloj `t`): para la consulta.
pub fn ultimas(raiz: &Path, segundos: f64) -> Result<(Vec<Traza>, f64), String> {
    let ruta = raiz.join(ARCHIVO);
    let mut f = std::fs::File::open(&ruta).map_err(|_| format!("no hay {ARCHIVO}: arranca el programa con QUIMERA_VIVO={ARCHIVO} (desde esta carpeta)"))?;
    let largo = f.metadata().map(|m| m.len()).unwrap_or(0);
    let desde = largo.saturating_sub(MAX_BYTES);
    f.seek(SeekFrom::Start(desde)).map_err(|e| e.to_string())?;
    let mut texto = String::new();
    f.take(MAX_BYTES).read_to_string(&mut texto).map_err(|e| e.to_string())?;
    let todas: Vec<Traza> = texto.lines().skip(usize::from(desde > 0)).filter_map(leer_linea).collect();
    let fin = todas.iter().map(|t| t.t).fold(0.0, f64::max);
    Ok((todas.into_iter().filter(|t| t.t >= fin - segundos * 1000.0).collect(), fin))
}

/// Sigue el archivo en su hilo; manda las trazas nuevas.
struct Lector { rx: Receiver<Vec<Traza>> }

impl Lector {
    fn nuevo(ruta: PathBuf, avisar: impl Fn() + Send + 'static) -> Lector {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new().name("super-grafo-vivo".into()).spawn(move || {
            let mut pos: u64 = 0;
            let mut resto = String::new();
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let Ok(mut f) = std::fs::File::open(&ruta) else { continue };
                let largo = f.metadata().map(|m| m.len()).unwrap_or(0);
                if largo < pos { pos = 0; resto.clear(); } // se vacio: el programa volvio a arrancar
                if largo == pos { continue; }
                // Si se llega tarde a un archivo enorme, solo lo ultimo.
                if largo - pos > MAX_BYTES { pos = largo - MAX_BYTES; resto.clear(); }
                if f.seek(SeekFrom::Start(pos)).is_err() { continue; }
                let mut nuevo = String::new();
                let Ok(n) = f.take(MAX_BYTES).read_to_string(&mut nuevo) else { continue };
                pos += n as u64;
                resto.push_str(&nuevo);
                // Solo las lineas completas; la ultima, si esta a medias, espera.
                let completas = match resto.rfind('\n') { Some(i) => resto.drain(..=i).collect::<String>(), None => continue };
                let v: Vec<Traza> = completas.lines().filter_map(leer_linea).collect();
                if !v.is_empty() {
                    if tx.send(v).is_err() { return; }
                    avisar();
                }
            }
        }).ok();
        Lector { rx }
    }
}

/// El estado de la vista en vivo.
#[derive(Default)]
pub struct Vivo {
    pub activo: bool,
    lector: Option<(String, Lector)>,
    reciente: VecDeque<(Instant, Traza)>,
    pub total: u64,
    pub ultima: Option<Instant>,
    /// Sube con cada traza nueva (para rehacer los colores).
    pub version: u64,
}

impl Vivo {
    pub fn encender(&mut self, raiz: &str, ctx: &eframe::egui::Context) {
        self.activo = true;
        if self.lector.as_ref().is_some_and(|(r, _)| r == raiz) { return; }
        let ctx = ctx.clone();
        self.lector = Some((raiz.to_string(), Lector::nuevo(Path::new(raiz).join(ARCHIVO), move || ctx.request_repaint())));
        self.reciente.clear();
        self.total = 0;
    }

    /// Recoge lo nuevo y olvida lo que salio de la ventana. true si algo cambio.
    pub fn recoger(&mut self) -> bool {
        let Some((_, l)) = &self.lector else { return false };
        let ahora = Instant::now();
        let mut cambio = false;
        for v in l.rx.try_iter() {
            self.total += v.len() as u64;
            self.reciente.extend(v.into_iter().map(|t| (ahora, t)));
            self.ultima = Some(ahora);
            cambio = true;
        }
        while self.reciente.front().is_some_and(|(t, _)| ahora.duration_since(*t) > VENTANA) { self.reciente.pop_front(); cambio = true; }
        if cambio { self.version += 1; }
        cambio
    }

    pub fn resumen(&self) -> Resumen { resumir(self.reciente.iter().map(|(_, t)| t), VENTANA.as_secs_f64()) }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_trazas_y_rechaza_lo_raro() {
        let t = leer_linea(r#"{"t":12.5,"ruta":"web/estilo/","us":5400,"desde":"web/dom"}"#).unwrap();
        assert_eq!((t.ruta.as_str(), t.us, t.desde.as_deref()), ("web/estilo", 5400, Some("web/dom")));
        assert!(leer_linea("no es json").is_none());
        assert!(leer_linea(r#"{"t":1,"ruta":"../fuera","us":1}"#).is_none());
        assert!(leer_linea(r#"{"t":1,"us":1}"#).is_none());
    }

    #[test]
    fn el_resumen_da_ms_por_segundo_y_el_trafico() {
        let v: Vec<Traza> = [r#"{"t":0,"ruta":"web/estilo","us":4000,"desde":"web/dom"}"#, r#"{"t":1,"ruta":"web/maquetacion","us":2000,"desde":"web/estilo"}"#,
                             r#"{"t":2,"ruta":"web/estilo","us":2000,"desde":"web/dom"}"#, r#"{"t":3,"ruta":"red/red","us":1000}"#]
            .iter().filter_map(|l| leer_linea(l)).collect();
        let r = resumir(&v, 2.0);
        assert_eq!(r.trazas, 4);
        assert_eq!(r.por_ruta[0], ("web/estilo".to_string(), 3.0), "6 ms en 2 s");
        assert_eq!(r.trafico[0], ("web/dom".into(), "web/estilo".into(), 2));
        assert_eq!(r.trafico.len(), 2, "sin desde no hay trafico");
    }

    #[test]
    fn la_consulta_lee_los_ultimos_segundos() {
        let dir = std::env::temp_dir().join(format!("sg-vivo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(ultimas(&dir, 5.0).is_err(), "sin archivo, dice como crearlo");
        std::fs::write(dir.join(ARCHIVO), "{\"t\":100,\"ruta\":\"a\",\"us\":1}\n{\"t\":9000,\"ruta\":\"b\",\"us\":2}\n{\"t\":10000,\"ruta\":\"c\",\"us\":3}\n").unwrap();
        let (v, fin) = ultimas(&dir, 5.0).unwrap();
        assert_eq!(fin, 10000.0);
        assert_eq!(v.iter().map(|t| t.ruta.as_str()).collect::<Vec<_>>(), ["b", "c"]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
