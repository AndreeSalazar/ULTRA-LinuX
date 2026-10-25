// SPDX-License-Identifier: GPL-2.0-only
//! Vigia al instante (Linux inotify): el kernel avisa cuando cambia un archivo, sin sondear.
//!
//! Se vigila cada carpeta del indice (las mismas reglas: sin ocultas ni enlaces). Una
//! carpeta nueva se anade sola. Solo cuenta lo que el SUPER GRAFO lee (C, C++, ASM, Rust,
//! Python y compile_commands.json): el resto de cambios no despierta a nadie.
//!
//! Si inotify no esta (o se pasan las vigilancias del sistema), devuelve None y quien
//! llama vuelve a sondear.

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Como mucho tantas carpetas vigiladas (Chromium tiene decenas de miles). Si el sistema no
/// deja tantas (`/proc/sys/fs/inotify/max_user_watches`), no hay inotify y se mira cada poco.
const MAX_CARPETAS: usize = 500_000;

fn relevante(nombre: &str) -> bool {
    carpetas_nucleo::dependencias::sigue(nombre) || carpetas_nucleo::museo::idioma(nombre).is_some() || nombre == "compile_commands.json"
}

/// Empieza a vigilar `raiz`; cada cambio que importa manda un `()`. None: sin inotify.
pub fn iniciar(raiz: &Path) -> Option<mpsc::Receiver<()>> {
    let raiz = raiz.canonicalize().ok()?;
    let mut ino = Inotify::init().ok()?;
    let mascara = WatchMask::CLOSE_WRITE | WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE
        | WatchMask::MOVED_FROM | WatchMask::MOVED_TO | WatchMask::DONT_FOLLOW | WatchMask::ONLYDIR;
    let (indice, _) = crate::indice::indexar(&raiz, false);
    let mut carpetas: HashMap<WatchDescriptor, PathBuf> = HashMap::new();
    let dirs = std::iter::once(raiz.clone()).chain((0..indice.len()).filter(|k| indice.es_carpeta(*k)).map(|k| raiz.join(indice.ruta(k))));
    for d in dirs {
        if carpetas.len() >= MAX_CARPETAS { return None; }
        match ino.add_watch(&d, mascara) { Ok(w) => { carpetas.insert(w, d); } Err(_) => return None }
    }
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new().name("vigia-inotify".into()).spawn(move || {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let Ok(eventos) = ino.read_events_blocking(&mut buf) else { return };
            let mut avisar = false;
            let mut nuevas: Vec<PathBuf> = Vec::new();
            for e in eventos {
                let nombre = e.name.and_then(|n| n.to_str()).unwrap_or("");
                if nombre.starts_with('.') { continue; }
                if e.mask.contains(EventMask::ISDIR) {
                    if e.mask.intersects(EventMask::CREATE | EventMask::MOVED_TO) {
                        if let Some(p) = carpetas.get(&e.wd) { nuevas.push(p.join(nombre)); }
                    }
                    avisar |= e.mask.intersects(EventMask::DELETE | EventMask::MOVED_FROM | EventMask::MOVED_TO);
                } else if relevante(nombre) {
                    avisar = true;
                }
            }
            for d in nuevas {
                if carpetas.len() < MAX_CARPETAS { if let Ok(w) = ino.add_watch(&d, mascara) { carpetas.insert(w, d); avisar = true; } }
            }
            if avisar && tx.send(()).is_err() { return; }
        }
    }).ok()?;
    Some(rx)
}

/// Espera el primer aviso (o `maximo`) y despues junta la rafaga: hasta que haya `calma`
/// sin avisos (como mucho 1 s). true si hubo algun aviso.
pub fn esperar(rx: &mpsc::Receiver<()>, maximo: Duration, calma: Duration) -> bool {
    if rx.recv_timeout(maximo).is_err() { return false; }
    let inicio = std::time::Instant::now();
    while inicio.elapsed() < Duration::from_secs(1) {
        if rx.recv_timeout(calma).is_err() { break; }
    }
    true
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn avisa_al_instante_y_ve_carpetas_nuevas() {
        let dir = std::env::temp_dir().join(format!("ultra-vigia-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.c"), "int a;\n").unwrap();
        let Some(rx) = iniciar(&dir) else { return }; // sin inotify (otro sistema): nada que probar
        // Un .txt no importa.
        std::fs::write(dir.join("src/notas.txt"), "hola").unwrap();
        assert!(!esperar(&rx, Duration::from_millis(300), Duration::from_millis(50)));
        // Un .c si, y enseguida.
        let t = std::time::Instant::now();
        std::fs::write(dir.join("src/a.c"), "int a = 1;\n").unwrap();
        assert!(esperar(&rx, Duration::from_secs(2), Duration::from_millis(50)));
        assert!(t.elapsed() < Duration::from_millis(500), "{:?}", t.elapsed());
        // Una carpeta nueva se vigila sola.
        std::fs::create_dir_all(dir.join("nueva")).unwrap();
        let _ = esperar(&rx, Duration::from_millis(300), Duration::from_millis(50));
        std::fs::write(dir.join("nueva/b.h"), "int b;\n").unwrap();
        assert!(esperar(&rx, Duration::from_secs(2), Duration::from_millis(50)), "la carpeta nueva tambien avisa");
        std::fs::remove_dir_all(&dir).ok();
    }
}
