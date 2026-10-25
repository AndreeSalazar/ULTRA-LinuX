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
use carpetas_nucleo::analisis::{ResultadoAnalisis, MARCA_ESTUDIO};
use carpetas_nucleo::dependencias::{referencias, resolver, sigue, Motivo, Referencia, Rutas};
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
            aristas.push((a, b, r.linea, r.texto.clone(), r.clase));
        }
        if !h.sospechas.is_empty() { sospechas.push((desde.to_string(), h.sospechas)); }
    }
    // Una arista por par; su motivo es la primera linea que la escribe.
    aristas.sort_unstable_by(|x, y| (x.0, x.1, x.2).cmp(&(y.0, y.1, y.2)));
    aristas.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
    let motivos = aristas.iter().map(|(_, _, linea, texto, clase)| Motivo { linea: *linea, texto: texto.clone(), clase: *clase }).collect();
    let aristas: Vec<(u32, u32)> = aristas.into_iter().map(|(a, b, ..)| (a, b)).collect();
    // Casos de estudio: una carpeta (o una de arriba) con CASOS-MUSEO.txt. Sin seguir enlaces.
    let mut marcadas: HashMap<String, bool> = HashMap::new();
    let mut estudio = Vec::new();
    for (ruta, _) in &sospechas {
        let mut d = ruta.as_str();
        while let Some((arriba, _)) = d.rsplit_once('/') {
            d = arriba;
            let marcada = *marcadas.entry(d.to_string())
                .or_insert_with(|| std::fs::symlink_metadata(base.join(d).join(MARCA_ESTUDIO)).is_ok_and(|m| m.is_file()));
            if marcada { estudio.push(ruta.clone()); break; }
        }
    }
    ResultadoAnalisis { rutas: nombres, aristas, motivos, sospechas, archivos, bytes, truncado: truncado.load(Ordering::Relaxed), micros: t.elapsed().as_micros() as u64, estudio }
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
        // Cada arista lleva su motivo: la linea donde se escribio.
        let mut d = carpetas_nucleo::analisis::EstadoAnalisis::default();
        d.llegar(r.clone());
        let (a, b) = (d.dep.id("src/apps/ultra-carpetas/src/main.rs").unwrap(), d.dep.id("src/apps/ultra-carpetas/src/analisis.rs").unwrap());
        let m = d.dep.motivo(a, b).expect("motivo de mod analisis;");
        let linea = std::fs::read_to_string(raiz.join("src/apps/ultra-carpetas/src/main.rs")).unwrap().lines().nth(m.linea as usize - 1).unwrap().to_string();
        assert_eq!(m.escrito("main.rs"), "mod analisis;");
        assert!(linea.contains("mod analisis;"), "linea {}: {linea}", m.linea);
        // Las minas del laboratorio del guardian son a proposito: el museo las ve.
        assert!(r.sospechas.iter().any(|(ruta, v)| ruta.ends_with("minas-compilacion/division-constante.c") && v.iter().any(|s| s.ficha == "UB-002")));
        assert!(r.sospechas.iter().any(|(ruta, v)| ruta == "ASM/laboratorio/abi.S" && v.iter().any(|s| s.ficha == "ASM-002")));
        // ...y saben que son a proposito (CASOS-MUSEO.txt).
        assert!(r.estudio.iter().any(|x| x == "ASM/laboratorio/abi.S"));
        assert!(r.estudio.iter().any(|x| x.starts_with("src/apps/carpetas-nucleo/tests/datos/museo/")));
        assert!(!r.truncado);
    }

    /// Medida sobre una carpeta real: ULTRA_MUSEO_DIR=/ruta cargo test --release medir -- --ignored --nocapture
    #[test]
    #[ignore]
    fn medir_en_carpeta() {
        let raiz = std::path::PathBuf::from(std::env::var("ULTRA_MUSEO_DIR").expect("ULTRA_MUSEO_DIR"));
        let t = Instant::now();
        let (i, _) = crate::indice::indexar(&raiz, false);
        let ti = t.elapsed();
        let r = analizar(&i, &raiz, &|| true);
        let mut d = carpetas_nucleo::analisis::EstadoAnalisis::default();
        let (n_rutas, n_aristas) = (r.rutas.len(), r.aristas.len());
        let t2 = Instant::now();
        d.llegar(r.clone());
        let c = carpetas_nucleo::analisis::conectar_estado(&Default::default(), &d, None);
        eprintln!("indice {} rutas en {ti:?}; museo {} archivos, {:.1} MiB en {} ms; {n_rutas} rutas con {n_aristas} aristas; {} sospechas (+{} a proposito); conectar {:?}: {} causas, {} papeles",
                  i.len(), r.archivos, r.bytes as f64 / 1048576.0, r.micros / 1000, d.total, d.total_estudio, t2.elapsed(), c.1.causas.len(), c.1.roles.len());
        for causa in c.1.causas.iter().take(5) { eprintln!("  causa {} explica {} lo usan {}", causa.ruta, causa.explica, causa.arrastra); }
        // Caminos entre pares repartidos por todo el proyecto: cuanto tardan y de que clase son.
        use carpetas_nucleo::camino::{camino, SinPaso, Union};
        let n = d.dep.len();
        let (mut peor, mut total, mut clases, mut con_corte) = (std::time::Duration::ZERO, std::time::Duration::ZERO, [0u32; 5], 0u32);
        let mut ejemplo: Option<carpetas_nucleo::camino::Camino> = None;
        let pares = 300;
        for k in 0..pares {
            let (a, b) = ((k * 7919) % n, (k * 104729 + 13) % n);
            if a == b { continue; }
            let t = Instant::now();
            let c = camino(&d.dep, &d.dep.rutas[a], &d.dep.rutas[b]);
            let e = t.elapsed();
            total += e;
            peor = peor.max(e);
            clases[match c.union { Some(Union::Usa) => 0, Some(Union::EsUsado) => 1, Some(Union::Comparten) => 2, Some(Union::Juntan) => 3, None => 4 }] += 1;
            if c.pasos.iter().any(|p| p.sin == SinPaso::Imprescindible) { con_corte += 1; }
            if matches!(c.union, Some(Union::Usa | Union::EsUsado)) && c.pasos.len() >= 3 && ejemplo.as_ref().is_none_or(|x| c.pasos.len() > x.pasos.len()) { ejemplo = Some(c); }
        }
        eprintln!("caminos: {pares} pares en {total:?} (peor {peor:?}); usa {} · es usado {} · comparten {} · los junta {} · nada {}; con paso imprescindible {con_corte}",
                  clases[0], clases[1], clases[2], clases[3], clases[4]);
        if let Some(c) = ejemplo {
            eprintln!("  ejemplo: {}", c.por_que);
            for p in &c.pasos { eprintln!("    {}:{} {}  [{:?}] lo usan {}", p.desde, p.motivo.as_ref().map(|m| m.linea).unwrap_or(0), p.motivo.as_ref().map(|m| m.escrito(&p.desde)).unwrap_or_default(), p.sin, p.lo_usan); }
        }
    }
}
