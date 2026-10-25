// SPDX-License-Identifier: GPL-2.0-only
//! Historia de git en su propio hilo (carril Git). Solo lectura:
//! - `git log` sin shell, sin terminal (GIT_TERMINAL_PROMPT=0: nunca pide claves);
//! - GIT_OPTIONAL_LOCKS=0: no toma bloqueos ni reescribe el indice de git;
//! - firmas y fsmonitor apagados (-c log.showSignature=false, core.fsmonitor=false):
//!   un repositorio ajeno no puede hacer que se ejecute otro programa al leerlo;
//! - como mucho MAX_COMMITS commits, 30 s y 256 MiB de salida.

use carpetas_nucleo::historia::{leer_log, Historia, MAX_COMMITS};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const MAX_SALIDA: u64 = 256 * 1024 * 1024;
const MAX_TIEMPO: Duration = Duration::from_secs(30);

pub struct Git { tx: Sender<(u64, PathBuf)>, rx: Receiver<(u64, u64, Result<Historia, String>)> }

/// Lee la historia de `base` (relativa a ella: --relative).
pub fn leer(base: &Path) -> Result<Historia, String> {
    let mut hijo = Command::new("git").arg("-C").arg(base)
        .args(["-c", "core.quotepath=false", "-c", "log.showSignature=false", "-c", "core.fsmonitor=false",
               "log", "--no-merges", "--no-renames", "--name-only", "--relative", "--no-color", "--format=%x01%ct%x02%an"])
        .arg(format!("--max-count={MAX_COMMITS}"))
        .env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0").env("LC_ALL", "C")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("no se pudo ejecutar git: {e}"))?;
    let mut salida = hijo.stdout.take().ok_or("sin salida")?;
    let mut err = hijo.stderr.take().ok_or("sin salida")?;
    // Se lee en otro hilo para poder cortar por tiempo.
    let lector = std::thread::spawn(move || { let mut v = Vec::new(); let _ = Read::by_ref(&mut salida).take(MAX_SALIDA).read_to_end(&mut v); v });
    let lector_err = std::thread::spawn(move || { let mut v = Vec::new(); let _ = Read::by_ref(&mut err).take(64 * 1024).read_to_end(&mut v); v });
    let inicio = Instant::now();
    let mut cortada = false;
    let estado = loop {
        if let Some(s) = hijo.try_wait().map_err(|e| e.to_string())? { break Some(s); }
        if inicio.elapsed() > MAX_TIEMPO { let _ = hijo.kill(); let _ = hijo.wait(); cortada = true; break None; }
        std::thread::sleep(Duration::from_millis(20));
    };
    let bytes = lector.join().unwrap_or_default();
    let errores = String::from_utf8_lossy(&lector_err.join().unwrap_or_default()).to_string();
    if let Some(s) = estado {
        if !s.success() {
            return Err(if errores.contains("not a git repository") { "no es un repositorio git".into() }
                       else if errores.contains("dubious ownership") { "git no confia en este repositorio (safe.directory)".into() }
                       else if errores.contains("does not have any commits") { "el repositorio aun no tiene commits".into() }
                       else { format!("git: {}", errores.lines().next().unwrap_or("fallo")) });
        }
    }
    let mut h = leer_log(&String::from_utf8_lossy(&bytes), MAX_COMMITS);
    h.truncada |= cortada || bytes.len() as u64 >= MAX_SALIDA;
    Ok(h)
}

impl Git {
    pub fn nuevo(avisar: impl Fn() + Send + 'static) -> Git {
        let (tx, rx_pedidos) = mpsc::channel::<(u64, PathBuf)>();
        let (tx_resp, rx) = mpsc::channel();
        std::thread::Builder::new().name("ultra-git".into()).spawn(move || {
            while let Ok(mut p) = rx_pedidos.recv() {
                // Si se pidio varias veces seguidas (se cambio de carpeta rapido), solo la ultima.
                while let Ok(q) = rx_pedidos.try_recv() { p = q; }
                let t = Instant::now();
                let r = leer(&p.1);
                if tx_resp.send((p.0, t.elapsed().as_micros() as u64, r)).is_err() { return; }
                avisar();
            }
        }).expect("hilo de git");
        Git { tx, rx }
    }

    /// Pide la historia de una carpeta; `generacion` para descartar respuestas viejas.
    pub fn pedir(&self, generacion: u64, base: PathBuf) { let _ = self.tx.send((generacion, base)); }

    pub fn recoger(&self) -> Vec<(u64, u64, Result<Historia, String>)> { self.rx.try_iter().collect() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_la_historia_de_este_repositorio() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let h = match leer(&raiz) { Ok(h) => h, Err(e) => { eprintln!("sin git aqui: {e}"); return; } };
        assert!(h.commits > 0 && !h.por_ruta.is_empty());
        assert!(h.por_ruta.keys().all(|k| !k.starts_with('/') && !k.contains("..")));
        // Relativa a la carpeta pedida: desde src/apps, las rutas no empiezan por src/apps/.
        let sub = leer(&raiz.join("src/apps")).unwrap();
        assert!(sub.por_ruta.keys().any(|k| k.starts_with("ultra-carpetas/")), "relativa a la carpeta");
    }

    #[test]
    fn una_carpeta_sin_git_lo_dice() {
        let d = std::env::temp_dir().join(format!("ultra-sin-git-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let r = leer(&d);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(r.unwrap_err(), "no es un repositorio git");
    }
}
