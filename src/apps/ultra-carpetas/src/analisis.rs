// SPDX-License-Identifier: GPL-2.0-only
//! Carril Museo: lee el codigo del proyecto (C, C++, ASM, Rust y Python) en su propio
//! hilo, repartido entre los nucleos, y saca dos cosas:
//!
//! - las sospechas del museo (patrones de las fichas de C, C++ y ASM), y
//! - quien usa a quien (`#include`, `mod`/`use`, `import`), resuelto contra el indice.
//!
//! Solo lectura: abre archivos regulares (nunca enlaces) de hasta 2 MiB, como mucho
//! 2 GiB o 60 s en total. Si corta, lo dice (`truncado`).

use crate::indice::leer_texto;
use carpetas_nucleo::analisis::ResultadoAnalisis;
use carpetas_nucleo::dependencias::{referencias, resolver, sigue, Referencia, Rutas};
use carpetas_nucleo::museo::{analizar as museo, idioma, Sospecha};
use carpetas_nucleo::proyecto::Indice;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const BYTES: u64 = 2 << 30;
const TIEMPO: Duration = Duration::from_secs(60);

/// Las rutas del indice para resolver referencias.
struct PorIndice<'a> { todas: HashMap<&'a str, u32>, nombres: HashMap<&'a str, Vec<&'a str>> }

impl<'a> PorIndice<'a> {
    fn nuevo(i: &'a Indice) -> PorIndice<'a> {
        let mut todas = HashMap::with_capacity(i.len());
        let mut nombres: HashMap<&str, Vec<&str>> = HashMap::new();
        for k in 0..i.len() {
            if i.es_carpeta(k) { continue; }
            let r = i.ruta(k);
            todas.insert(r, k as u32);
            nombres.entry(r.rsplit('/').next().unwrap_or(r)).or_default().push(r);
        }
        PorIndice { todas, nombres }
    }
}

impl Rutas for PorIndice<'_> {
    fn existe(&self, ruta: &str) -> bool { self.todas.contains_key(ruta) }
    fn por_nombre(&self, nombre: &str) -> Vec<&str> { self.nombres.get(nombre).cloned().unwrap_or_default() }
}

/// Lo que sale de leer un archivo.
struct Hoja { ruta: u32, refs: Vec<Referencia>, sospechas: Vec<Sospecha>, bytes: u64 }

/// Analiza todo el indice. `sigue()` false cancela (llego otro indice).
pub fn analizar(indice: &Indice, base: &Path, sigue_vivo: &(dyn Fn() -> bool + Sync)) -> ResultadoAnalisis {
    let t = Instant::now();
    let candidatos: Vec<usize> = (0..indice.len())
        .filter(|k| !indice.es_carpeta(*k) && indice.tam(*k) <= 2 << 20 && { let r = indice.ruta(*k); sigue(r) || idioma(r).is_some() })
        .collect();
    let leidos = AtomicU64::new(0);
    let truncado = std::sync::atomic::AtomicBool::new(false);
    let hilos = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let tramo = candidatos.len().div_ceil(hilos).max(1);
    let hojas: Vec<Hoja> = std::thread::scope(|s| {
        let manos: Vec<_> = candidatos.chunks(tramo).map(|parte| {
            let (leidos, truncado) = (&leidos, &truncado);
            s.spawn(move || {
                let mut v = Vec::new();
                for (n, &k) in parte.iter().enumerate() {
                    if leidos.load(Ordering::Relaxed) > BYTES || t.elapsed() > TIEMPO { truncado.store(true, Ordering::Relaxed); break; }
                    if n % 64 == 0 && !sigue_vivo() { break; }
                    let ruta = indice.ruta(k);
                    let Some(texto) = leer_texto(&base.join(ruta)) else { continue };
                    leidos.fetch_add(texto.len() as u64, Ordering::Relaxed);
                    let refs = if sigue(ruta) { referencias(ruta, &texto) } else { Vec::new() };
                    let sospechas = museo(ruta, &texto);
                    v.push(Hoja { ruta: k as u32, refs, sospechas, bytes: texto.len() as u64 });
                }
                v
            })
        }).collect();
        manos.into_iter().flat_map(|m| m.join().unwrap_or_default()).collect()
    });
    // Resolver contra el indice: a usa b.
    let rutas = PorIndice::nuevo(indice);
    let mut ids: HashMap<u32, u32> = HashMap::new();
    let mut nombres: Vec<String> = Vec::new();
    let mut id = |k: u32, nombres: &mut Vec<String>| -> u32 {
        *ids.entry(k).or_insert_with(|| { nombres.push(indice.ruta(k as usize).to_string()); nombres.len() as u32 - 1 })
    };
    let mut aristas = Vec::new();
    let mut sospechas = Vec::new();
    let (mut archivos, mut bytes) = (0u64, 0u64);
    for h in hojas {
        archivos += 1;
        bytes += h.bytes;
        let desde = indice.ruta(h.ruta as usize);
        for r in &h.refs {
            let Some(destino) = resolver(desde, r, &rutas) else { continue };
            let Some(&kb) = rutas.todas.get(destino.as_str()) else { continue };
            let a = id(h.ruta, &mut nombres);
            let b = id(kb, &mut nombres);
            aristas.push((a, b));
        }
        if !h.sospechas.is_empty() { sospechas.push((desde.to_string(), h.sospechas)); }
    }
    aristas.sort_unstable();
    aristas.dedup();
    ResultadoAnalisis { rutas: nombres, aristas, sospechas, archivos, bytes, truncado: truncado.load(Ordering::Relaxed), micros: t.elapsed().as_micros() as u64 }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn analiza_este_repositorio() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let (i, _) = crate::indice::indexar(&raiz, false);
        let r = analizar(&i, &raiz, &|| true);
        eprintln!("{} archivos, {} KiB, {} rutas con aristas, {} aristas, {} con sospechas, {} ms", r.archivos, r.bytes / 1024, r.rutas.len(),
                  r.aristas.len(), r.sospechas.len(), r.micros / 1000);
        let arista = |a: &str, b: &str| r.aristas.iter().any(|(x, y)| r.rutas[*x as usize] == a && r.rutas[*y as usize] == b);
        assert!(arista("src/apps/ultra-carpetas/src/main.rs", "src/apps/ultra-carpetas/src/analisis.rs"), "mod analisis;");
        assert!(arista("src/apps/ultra-carpetas/src/analisis.rs", "src/apps/carpetas-nucleo/src/museo.rs"), "use carpetas_nucleo::museo");
        // Las minas del laboratorio del guardian son a proposito: el museo las ve.
        assert!(r.sospechas.iter().any(|(ruta, v)| ruta.ends_with("minas-compilacion/division-constante.c") && v.iter().any(|s| s.ficha == "UB-002")));
        assert!(r.sospechas.iter().any(|(ruta, v)| ruta == "ASM/laboratorio/abi.S" && v.iter().any(|s| s.ficha == "ASM-002")));
        assert!(!r.truncado);
    }
}
