// SPDX-License-Identifier: GPL-2.0-only
//! Carril de disco: un hilo propio lista y vigila carpetas. La interfaz nunca espera al
//! disco: manda pedidos y recoge respuestas cuando llegan. Una carpeta enorme o un disco
//! lento no congelan nada; como mucho, su respuesta tarda.

use crate::entradas::listar;
use carpetas_nucleo::Entrada;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime};

pub enum Pedido {
    /// Listar una carpeta (ruta relativa a `base`).
    Listar { generacion: u64, base: PathBuf, ruta: String, ocultos: bool },
    /// Que carpetas vigilar (se revisan una vez por segundo).
    Vigilar { generacion: u64, base: PathBuf, rutas: Vec<String>, ocultos: bool },
}

pub struct Listado {
    pub generacion: u64,
    pub base: PathBuf,
    pub ruta: String,
    pub resultado: Result<Vec<Entrada>, String>,
    pub micros: u64,
}

pub struct Disco {
    pedidos: Sender<Pedido>,
    respuestas: Receiver<Listado>,
}

/// Une base + ruta relativa solo si la ruta es limpia (sin "..", sin absolutas) y el
/// destino es una carpeta de verdad, no un enlace: no se sale de lo abierto ni hay bucles.
/// Estricto en el texto: cada tramo es un nombre (sin vacios, ".", ".." ni "/" inicial).
pub fn ruta_limpia(ruta: &str) -> bool {
    ruta.split('/').all(|t| !t.is_empty() && t != "." && t != "..")
        && Path::new(ruta).components().all(|c| matches!(c, Component::Normal(_)))
}

pub fn ruta_segura(base: &Path, ruta: &str) -> Option<PathBuf> {
    if !ruta.is_empty() && !ruta_limpia(ruta) { return None; }
    let relativa = Path::new(ruta);
    let final_ = base.join(relativa);
    if ruta.is_empty() { return Some(final_); }
    let meta = std::fs::symlink_metadata(&final_).ok()?;
    (meta.is_dir() && !meta.file_type().is_symlink()).then_some(final_)
}

fn listar_medido(base: &Path, ruta: &str, ocultos: bool) -> (Result<Vec<Entrada>, String>, u64) {
    let inicio = Instant::now();
    let resultado = match ruta_segura(base, ruta) {
        Some(p) => listar(&p, ocultos).map_err(|e| e.to_string()),
        None => Err(format!("ruta rechazada: {ruta}")),
    };
    (resultado, inicio.elapsed().as_micros() as u64)
}

impl Disco {
    /// `avisar` despierta a la interfaz cuando llega algo (egui no repinta solo).
    pub fn nuevo(avisar: impl Fn() + Send + 'static) -> Disco {
        let (pedidos, rx_pedidos) = mpsc::channel::<Pedido>();
        let (tx_respuestas, respuestas) = mpsc::channel::<Listado>();
        std::thread::Builder::new().name("ultra-carpetas-disco".into()).spawn(move || {
            let mut vigilando: Option<(u64, PathBuf, bool, BTreeMap<String, Option<SystemTime>>)> = None;
            let mut revisado = Instant::now();
            loop {
                // Sin carpetas vigiladas no hay nada que recalcular periodicamente.
                // Un pedido despierta el hilo de inmediato; vigilando, revisar a 1 Hz.
                match rx_pedidos.recv_timeout(if vigilando.is_some() { Duration::from_secs(1) } else { Duration::from_secs(3600) }) {
                    Ok(Pedido::Listar { generacion, base, ruta, ocultos }) => {
                        let (resultado, micros) = listar_medido(&base, &ruta, ocultos);
                        if tx_respuestas.send(Listado { generacion, base, ruta, resultado, micros }).is_err() { return; }
                        avisar();
                    }
                    Ok(Pedido::Vigilar { generacion, base, rutas, ocultos }) => {
                        let previas = vigilando.take().map(|v| v.3).unwrap_or_default();
                        let mapa = rutas.into_iter().take(carpetas_nucleo::carriles::MAX_DESPLEGADAS + 1)
                            .map(|r| { let m = previas.get(&r).copied().flatten().or_else(|| mtime(&base, &r)); (r, m) }).collect();
                        vigilando = Some((generacion, base, ocultos, mapa));
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                if revisado.elapsed() < Duration::from_secs(1) { continue; }
                revisado = Instant::now();
                let Some((generacion, base, ocultos, mapa)) = &mut vigilando else { continue };
                for (ruta, previo) in mapa.iter_mut() {
                    let ahora = mtime(base, ruta);
                    if ahora != *previo {
                        *previo = ahora;
                        let (resultado, micros) = listar_medido(base, ruta, *ocultos);
                        if tx_respuestas.send(Listado { generacion: *generacion, base: base.clone(), ruta: ruta.clone(), resultado, micros }).is_err() { return; }
                        avisar();
                    }
                }
            }
        }).expect("hilo del disco");
        Disco { pedidos, respuestas }
    }

    pub fn pedir(&self, p: Pedido) { let _ = self.pedidos.send(p); }

    pub fn recoger(&self) -> Vec<Listado> { self.respuestas.try_iter().take(256).collect() }
}

fn mtime(base: &Path, ruta: &str) -> Option<SystemTime> {
    ruta_segura(base, ruta).and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn ruta_segura_no_sale_ni_sigue_enlaces() {
        let base = std::env::temp_dir().join(format!("ultra-carpetas-disco-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("src/app")).unwrap();
        std::os::unix::fs::symlink("/etc", base.join("atajo")).unwrap();
        assert_eq!(ruta_segura(&base, "src/app"), Some(base.join("src/app")));
        assert_eq!(ruta_segura(&base, ""), Some(base.clone()));
        for mala in ["../etc", "/etc", "src/../../etc", "atajo", "no-existe", "src/./app"] {
            assert_eq!(ruta_segura(&base, mala), None, "{mala}");
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn hilo_lista_y_avisa_cambios() {
        let base = std::env::temp_dir().join(format!("ultra-carpetas-hilo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::write(base.join("sub/a.txt"), b"x").unwrap();
        let disco = Disco::nuevo(|| {});
        disco.pedir(Pedido::Listar { generacion: 7, base: base.clone(), ruta: "sub".into(), ocultos: false });
        let espera = |disco: &Disco| { for _ in 0..60 { let r = disco.recoger(); if !r.is_empty() { return r; } std::thread::sleep(Duration::from_millis(50)); } Vec::new() };
        let r = espera(&disco);
        assert_eq!(r[0].generacion, 7);
        assert_eq!(r[0].resultado.as_ref().unwrap()[0].nombre, "a.txt");
        disco.pedir(Pedido::Vigilar { generacion: 7, base: base.clone(), rutas: vec!["sub".into()], ocultos: false });
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(base.join("sub/b.txt"), b"y").unwrap();
        let r = espera(&disco);
        assert_eq!(r.last().unwrap().resultado.as_ref().unwrap().len(), 2, "un archivo nuevo llega solo, sin pedirlo");
        let _ = std::fs::remove_dir_all(&base);
    }
}
