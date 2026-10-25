// SPDX-License-Identifier: GPL-2.0-only
//! Pasa el museo por una carpeta real y cuenta: archivos, tiempo y sospechas por ficha.
//! Uso: cargo run --release --example medir_museo -- <carpeta> [muestras]
use carpetas_nucleo::museo::{analizar, idioma, Fuerza};
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let base = std::path::PathBuf::from(args.next().expect("carpeta"));
    let muestras: usize = args.next().and_then(|x| x.parse().ok()).unwrap_or(3);
    let mut pila = vec![base.clone()];
    let (mut archivos, mut bytes) = (0u64, 0u64);
    let mut por: BTreeMap<&str, (u32, u32, Vec<String>)> = BTreeMap::new();
    let t = Instant::now();
    while let Some(d) = pila.pop() {
        let Ok(l) = std::fs::read_dir(&d) else { continue };
        for e in l.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() { if !e.file_name().to_string_lossy().starts_with('.') { pila.push(p); } continue; }
            if !ft.is_file() { continue; }
            let rel = p.strip_prefix(&base).unwrap().to_string_lossy().to_string();
            if idioma(&rel).is_none() { continue; }
            let Ok(texto) = std::fs::read_to_string(&p) else { continue };
            archivos += 1;
            bytes += texto.len() as u64;
            for s in analizar(&rel, &texto) {
                let e = por.entry(s.ficha).or_default();
                if s.fuerza == Fuerza::Exacto { e.0 += 1 } else { e.1 += 1 }
                if e.2.len() < muestras { e.2.push(format!("{rel}:{} {} | {}", s.linea, s.visto, s.motivo)); }
            }
        }
    }
    let ms = t.elapsed().as_millis();
    println!("{archivos} archivos C/C++/ASM, {:.1} MiB en {ms} ms", bytes as f64 / 1048576.0);
    for (f, (ex, rev, m)) in &por {
        println!("{f}: {ex} exactas, {rev} revisar");
        for x in m { println!("    {x}"); }
    }
}
