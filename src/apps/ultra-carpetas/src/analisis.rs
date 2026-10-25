// SPDX-License-Identifier: GPL-2.0-only
//! Carril Museo: lee el codigo del proyecto (C, C++, ASM, Rust y Python) en su propio
//! hilo, repartido entre los nucleos, y saca dos cosas:
//!
//! - las sospechas del museo (patrones de las fichas de C, C++ y ASM), y
//! - quien usa a quien (`#include`, `mod`/`use`, `import`), resuelto contra el indice.
//!
//! Solo lectura: abre archivos regulares (nunca enlaces) de hasta 2 MiB, como mucho
//! 2 GiB o 60 s en total. Si corta, lo dice (`truncado`).
//!
//! **Incremental:** lo que saca de cada archivo (sus referencias y sospechas) se guarda en
//! la cache del usuario (`$XDG_CACHE_HOME/ultra-carpetas`), con su fecha y su tamano. La
//! vez siguiente solo se relee lo que cambio. La cache se invalida sola si cambia el
//! programa (otro detector del museo). `ULTRA_SIN_CACHE=1` la apaga.
//!
//! Si hay `compile_commands.json`, sus `-I` entran en el resolvedor (donde de verdad busca
//! el compilador).

use crate::indice::leer_texto;
use carpetas_nucleo::analisis::{ResultadoAnalisis, MARCA_ESTUDIO};
use carpetas_nucleo::dependencias::{referencias, resolver, sigue, ClaseRef, Motivo, Referencia, Rutas};
use carpetas_nucleo::museo::{analizar as museo, ficha, idioma, Fuerza, Sospecha};
use serde_json::{json, Value};
use carpetas_nucleo::proyecto::Indice;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const BYTES: u64 = 2 << 30;
const TIEMPO: Duration = Duration::from_secs(60);

/// Las rutas del indice para resolver referencias.
struct PorIndice<'a> { todas: HashMap<&'a str, u32>, nombres: HashMap<&'a str, Vec<&'a str>>, bases: Vec<String> }

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
        PorIndice { todas, nombres, bases: Vec::new() }
    }
}

impl Rutas for PorIndice<'_> {
    fn existe(&self, ruta: &str) -> bool { self.todas.contains_key(ruta) }
    fn por_nombre(&self, nombre: &str) -> Vec<&str> { self.nombres.get(nombre).cloned().unwrap_or_default() }
    fn bases(&self) -> &[String] { &self.bases }
}

/// Lo que sale de leer un archivo (y su fecha y tamano, para la cache).
#[derive(Clone)]
struct Hoja { ruta: u32, refs: Vec<Referencia>, sospechas: Vec<Sospecha>, bytes: u64, fecha: u128, tam: u64, reusada: bool }

// ---------------- Cache incremental ----------------

/// Sube si cambia lo que se guarda por archivo.
const VERSION_CACHE: u32 = 1;

/// Donde va la cache de una carpeta (None: apagada).
pub fn ruta_cache(base: &Path) -> Option<std::path::PathBuf> {
    if std::env::var_os("ULTRA_SIN_CACHE").is_some() { return None; }
    let dir = std::env::var_os("ULTRA_CACHE_DIR").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(|x| std::path::PathBuf::from(x).join("ultra-carpetas")))
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache/ultra-carpetas")))?;
    let raiz = base.canonicalize().ok()?;
    // FNV-1a de la ruta: un archivo por carpeta analizada.
    let h = raiz.to_string_lossy().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    Some(dir.join(format!("analisis-{h:016x}.json")))
}

/// La huella del programa: si cambia (otro binario, otros detectores), la cache no vale.
fn huella() -> String {
    let exe = std::env::current_exe().ok().and_then(|e| std::fs::metadata(e).ok());
    let (tam, fecha) = exe.map(|m| (m.len(), fecha_de(&m))).unwrap_or((0, 0));
    format!("{VERSION_CACHE}|{}|{tam}|{fecha}", carpetas_nucleo::fichas::FICHAS.len())
}

fn fecha_de(m: &std::fs::Metadata) -> u128 { m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0) }

fn clase_a(c: ClaseRef) -> String {
    match c { ClaseRef::Comillas => "c".into(), ClaseRef::Angulos => "a".into(), ClaseRef::ModRust => "m".into(), ClaseRef::UsoRust => "u".into(), ClaseRef::Python(n) => format!("p{n}") }
}
fn clase_de(s: &str) -> Option<ClaseRef> {
    Some(match s { "c" => ClaseRef::Comillas, "a" => ClaseRef::Angulos, "m" => ClaseRef::ModRust, "u" => ClaseRef::UsoRust, p => ClaseRef::Python(p.strip_prefix('p')?.parse().ok()?) })
}

/// Lo guardado de un archivo: (fecha, tamano, bytes, referencias, sospechas).
type Guardado = (u128, u64, u64, Vec<Referencia>, Vec<Sospecha>);

fn cargar_cache(ruta: &Path) -> HashMap<String, Guardado> {
    let Some(doc) = std::fs::read(ruta).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return HashMap::new() };
    if doc["huella"].as_str() != Some(huella().as_str()) { return HashMap::new(); }
    let mut m = HashMap::new();
    for (k, v) in doc["archivos"].as_object().into_iter().flatten() {
        let leer = || -> Option<Guardado> {
            let fecha = v[0].as_str()?.parse().ok()?;
            let refs = v[3].as_array()?.iter().map(|r| Some(Referencia { texto: r[0].as_str()?.into(), linea: r[1].as_u64()? as u32, clase: clase_de(r[2].as_str()?)?,
                                                                       condicion: r[3].as_str().map(str::to_string) })).collect::<Option<Vec<_>>>()?;
            let sosp = v[4].as_array()?.iter().map(|s| Some(Sospecha { ficha: ficha(s[0].as_str()?)?.id, linea: s[1].as_u64()? as u32, columna: s[2].as_u64()? as u32,
                                                                      fuerza: if s[3].as_bool()? { Fuerza::Exacto } else { Fuerza::Revisar },
                                                                      visto: s[4].as_str()?.into(), motivo: s[5].as_str()?.into() })).collect::<Option<Vec<_>>>()?;
            Some((fecha, v[1].as_u64()?, v[2].as_u64()?, refs, sosp))
        };
        if let Some(g) = leer() { m.insert(k.clone(), g); }
    }
    m
}

fn guardar_cache(ruta: &Path, indice: &Indice, hojas: &[Hoja]) {
    let archivos: serde_json::Map<String, Value> = hojas.iter().map(|h| (indice.ruta(h.ruta as usize).to_string(), json!([
        h.fecha.to_string(), h.tam, h.bytes,
        h.refs.iter().map(|r| json!([r.texto, r.linea, clase_a(r.clase), r.condicion])).collect::<Vec<_>>(),
        h.sospechas.iter().map(|s| json!([s.ficha, s.linea, s.columna, s.fuerza == Fuerza::Exacto, s.visto, s.motivo])).collect::<Vec<_>>(),
    ]))).collect();
    let doc = json!({ "huella": huella(), "archivos": archivos });
    let Some(dir) = ruta.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() { return; }
    // Escritura atomica: a un temporal y despues se renombra.
    let tmp = ruta.with_extension(format!("tmp{}", std::process::id()));
    if std::fs::write(&tmp, serde_json::to_vec(&doc).unwrap_or_default()).is_ok() { let _ = std::fs::rename(&tmp, ruta); } else { let _ = std::fs::remove_file(&tmp); }
}

/// Analiza todo el indice, con la cache del usuario. `sigue()` false cancela.
pub fn analizar(indice: &Indice, base: &Path, sigue_vivo: &(dyn Fn() -> bool + Sync)) -> ResultadoAnalisis {
    analizar_con(indice, base, sigue_vivo, ruta_cache(base).as_deref())
}

/// Analiza todo el indice; con `cache`, solo relee lo que cambio desde la ultima vez.
pub fn analizar_con(indice: &Indice, base: &Path, sigue_vivo: &(dyn Fn() -> bool + Sync), cache: Option<&Path>) -> ResultadoAnalisis {
    let t = Instant::now();
    let guardado = cache.map(cargar_cache).unwrap_or_default();
    let candidatos: Vec<usize> = (0..indice.len())
        .filter(|k| !indice.es_carpeta(*k) && indice.tam(*k) <= 2 << 20 && { let r = indice.ruta(*k); sigue(r) || idioma(r).is_some() })
        .collect();
    let leidos = AtomicU64::new(0);
    let truncado = std::sync::atomic::AtomicBool::new(false);
    let hilos = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let tramo = candidatos.len().div_ceil(hilos).max(1);
    let hojas: Vec<Hoja> = std::thread::scope(|s| {
        let manos: Vec<_> = candidatos.chunks(tramo).map(|parte| {
            let (leidos, truncado, guardado) = (&leidos, &truncado, &guardado);
            s.spawn(move || {
                let mut v = Vec::new();
                for (n, &k) in parte.iter().enumerate() {
                    if leidos.load(Ordering::Relaxed) > BYTES || t.elapsed() > TIEMPO { truncado.store(true, Ordering::Relaxed); break; }
                    if n % 64 == 0 && !sigue_vivo() { break; }
                    let ruta = indice.ruta(k);
                    let camino = base.join(ruta);
                    let Ok(meta) = std::fs::symlink_metadata(&camino) else { continue };
                    if !meta.is_file() { continue; }
                    let (fecha, tam) = (fecha_de(&meta), meta.len());
                    // Igual que la ultima vez (fecha y tamano): no se relee.
                    if let Some((f, s, b, refs, sosp)) = guardado.get(ruta) {
                        if *f == fecha && *s == tam && fecha != 0 {
                            v.push(Hoja { ruta: k as u32, refs: refs.clone(), sospechas: sosp.clone(), bytes: *b, fecha, tam, reusada: true });
                            continue;
                        }
                    }
                    let Some(texto) = leer_texto(&camino) else { continue };
                    leidos.fetch_add(texto.len() as u64, Ordering::Relaxed);
                    let refs = if sigue(ruta) { referencias(ruta, &texto) } else { Vec::new() };
                    let sospechas = museo(ruta, &texto);
                    v.push(Hoja { ruta: k as u32, refs, sospechas, bytes: texto.len() as u64, fecha, tam, reusada: false });
                }
                v
            })
        }).collect();
        manos.into_iter().flat_map(|m| m.join().unwrap_or_default()).collect()
    });
    let reutilizados = hojas.iter().filter(|h| h.reusada).count() as u64;
    if let Some(c) = cache { if !truncado.load(Ordering::Relaxed) && sigue_vivo() { guardar_cache(c, indice, &hojas); } }
    // Resolver contra el indice: a usa b (con las -I de compile_commands.json si lo hay).
    let mut rutas = PorIndice::nuevo(indice);
    let cc = crate::compilacion::leer(base);
    if let Some(c) = &cc { rutas.bases = c.bases.clone(); }
    let compilacion = cc.map(|c| (c.origen, c.bases.len()));
    let mut ids: HashMap<u32, u32> = HashMap::new();
    let mut nombres: Vec<String> = Vec::new();
    let mut id = |k: u32, nombres: &mut Vec<String>| -> u32 {
        *ids.entry(k).or_insert_with(|| { nombres.push(indice.ruta(k as usize).to_string()); nombres.len() as u32 - 1 })
    };
    let mut aristas = Vec::new();
    let mut sospechas = Vec::new();
    let (mut archivos, mut bytes) = (0u64, 0u64);
    // Los #include con carpeta que no se encontraron: quiza son cabeceras exportadas.
    let mut sin_resolver: Vec<(u32, Referencia)> = Vec::new();
    for h in hojas {
        archivos += 1;
        bytes += h.bytes;
        let desde = indice.ruta(h.ruta as usize);
        for r in &h.refs {
            let Some(destino) = resolver(desde, r, &rutas) else {
                if matches!(r.clase, ClaseRef::Comillas | ClaseRef::Angulos) && r.texto.contains('/') { sin_resolver.push((h.ruta, r.clone())); }
                continue;
            };
            let Some(&kb) = rutas.todas.get(destino.as_str()) else { continue };
            let a = id(h.ruta, &mut nombres);
            let b = id(kb, &mut nombres);
            aristas.push((a, b, r.linea, Motivo::de(r)));
        }
        if !h.sospechas.is_empty() { sospechas.push((desde.to_string(), h.sospechas)); }
    }
    // Segunda pasada: los prefijos exportados que se aprenden del proyecto entero.
    let prefijos = carpetas_nucleo::dependencias::prefijos_exportados(sin_resolver.iter().map(|(_, r)| r.texto.as_str()), &rutas);
    let mut exportadas: BTreeMap<String, u32> = BTreeMap::new();
    if !prefijos.is_empty() {
        for (k, r) in &sin_resolver {
            let Some((p, _)) = r.texto.trim_start_matches("./").split_once('/') else { continue };
            if !prefijos.contains_key(p) { continue; }
            let desde = indice.ruta(*k as usize);
            let Some(destino) = carpetas_nucleo::dependencias::exportada(&r.texto, &rutas) else { continue };
            if destino == desde { continue; }
            let Some(&kb) = rutas.todas.get(destino.as_str()) else { continue };
            let a = id(*k, &mut nombres);
            let b = id(kb, &mut nombres);
            let mut m = Motivo::de(r);
            m.exportada = true;
            aristas.push((a, b, r.linea, m));
            *exportadas.entry(format!("{p}/")).or_default() += 1;
        }
    }
    // Una arista por par; su motivo es la primera linea que la escribe.
    // Si la misma arista se escribe sin condicion en otra linea, manda esa (existe siempre).
    aristas.sort_unstable_by(|x, y| (x.0, x.1, x.3.condicion.is_some(), x.2).cmp(&(y.0, y.1, y.3.condicion.is_some(), y.2)));
    aristas.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
    let motivos = aristas.iter().map(|(_, _, _, m)| m.clone()).collect();
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
    ResultadoAnalisis { rutas: nombres, aristas, motivos, sospechas, archivos, bytes, truncado: truncado.load(Ordering::Relaxed), micros: t.elapsed().as_micros() as u64, estudio,
                        reutilizados, compilacion, exportadas: exportadas.into_iter().collect() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn analiza_este_repositorio() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let (i, _) = crate::indice::indexar(&raiz, false);
        let r = analizar_con(&i, &raiz, &|| true, None);
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

    #[test]
    fn la_cache_solo_relee_lo_que_cambio_y_las_i_de_compile_commands_resuelven() {
        let dir = std::env::temp_dir().join(format!("ultra-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/inc/lib")).unwrap();
        std::fs::create_dir_all(dir.join("vendor/x/lib")).unwrap();
        std::fs::write(dir.join("vendor/x/lib/util.h"), "#pragma once\nint util(void);\n").unwrap();
        std::fs::write(dir.join("src/main.c"), "#include <lib/util.h>\nint main(void) { return 0; }\n").unwrap();
        std::fs::write(dir.join("src/inc/lib/util.h"), "#pragma once\nint util(void);\n").unwrap();
        std::fs::write(dir.join("src/libera.c"), "#include <stdlib.h>\nvoid g(char *p) {\n    free(p + 1);\n}\n").unwrap();
        let cache = dir.join("cache.json");
        let pasada = || { let (i, _) = crate::indice::indexar(&dir, false); analizar_con(&i, &dir, &|| true, Some(&cache)) };
        let destino = |r: &ResultadoAnalisis| r.aristas.first().map(|(_, b)| r.rutas[*b as usize].clone());
        // Sin compile_commands.json, <lib/util.h> se adivina por el final: el mas cercano.
        let r = pasada();
        assert_eq!(r.reutilizados, 0);
        assert_eq!(destino(&r).as_deref(), Some("src/inc/lib/util.h"));
        assert!(r.sospechas.iter().any(|(k, v)| k == "src/libera.c" && v[0].ficha == "C-001"));
        // Con compile_commands.json, la -I real (../vendor/x) manda. Y nada se relee.
        let raiz = dir.canonicalize().unwrap();
        std::fs::write(dir.join("compile_commands.json"), serde_json::json!([{ "directory": raiz.join("src").display().to_string(), "file": "main.c", "arguments": ["gcc", "-I../vendor/x", "-c", "main.c"] }]).to_string()).unwrap();
        let r = pasada();
        assert_eq!(r.reutilizados, 4, "los cuatro, de la cache");
        assert_eq!(r.compilacion, Some(("compile_commands.json".into(), 1)));
        assert_eq!(destino(&r).as_deref(), Some("vendor/x/lib/util.h"), "la -I del compilador, no la adivinanza");
        assert!(r.sospechas.iter().any(|(k, v)| k == "src/libera.c" && v[0].ficha == "C-001"), "las sospechas tambien vuelven de la cache");
        // Cambia un archivo: solo ese se relee, y se ve el cambio.
        std::fs::write(dir.join("src/libera.c"), "void g(char *p) { (void)p; }\n").unwrap();
        let r = pasada();
        assert_eq!(r.reutilizados, 3);
        assert!(!r.sospechas.iter().any(|(k, _)| k == "src/libera.c"), "arreglado: ya no hay sospecha");
        std::fs::remove_dir_all(&dir).ok();
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
        eprintln!("cabeceras exportadas: {:?}", r.exportadas);
        // Para compararlas con la verdad del proyecto: desde, texto y a donde se resolvio.
        if let Some(salida) = std::env::var_os("ULTRA_EXPORTADAS_SALIDA") {
            let lineas: String = r.aristas.iter().zip(&r.motivos).filter(|(_, m)| m.exportada)
                .map(|((a, b), m)| format!("{}\t{}\t{}\n", r.rutas[*a as usize], m.texto, r.rutas[*b as usize])).collect();
            std::fs::write(salida, lineas).unwrap();
        }
        let mut d = carpetas_nucleo::analisis::EstadoAnalisis::default();
        let (n_rutas, n_aristas) = (r.rutas.len(), r.aristas.len());
        let t2 = Instant::now();
        d.llegar(r.clone());
        let c = carpetas_nucleo::analisis::conectar_estado(&Default::default(), &d, None);
        eprintln!("indice {} rutas en {ti:?}; museo {} archivos, {:.1} MiB en {} ms; {n_rutas} rutas con {n_aristas} aristas; {} sospechas (+{} a proposito); conectar {:?}: {} causas, {} papeles",
                  i.len(), r.archivos, r.bytes as f64 / 1048576.0, r.micros / 1000, d.total, d.total_estudio, t2.elapsed(), c.1.causas.len(), c.1.roles.len());
        for causa in c.1.causas.iter().take(5) { eprintln!("  causa {} explica {} lo usan {}", causa.ruta, causa.explica, causa.arrastra); }
        let condicionales = r.motivos.iter().filter(|m| m.condicion.is_some()).count();
        eprintln!("aristas dentro de un #if: {condicionales} de {}", r.motivos.len());
        // Ciclos de cabeceras: cuantos, de que tamano y la vuelta del mayor.
        let t3 = Instant::now();
        let ci = carpetas_nucleo::ciclos::ciclos(&d.dep);
        eprintln!("ciclos: {} ({} archivos) en {:?}; tamanos {:?}", ci.total, ci.archivos, t3.elapsed(), ci.lista.iter().take(8).map(|c| c.archivos.len()).collect::<Vec<_>>());
        for (k, c) in ci.lista.iter().take(3).enumerate() {
            eprintln!("  ciclo {k}: {} archivos, vuelta {}, corte {:?}, condicional {}", c.archivos.len(), c.vuelta.len(), c.corte, c.condicional);
            for e in &c.vuelta { eprintln!("    {}:{} {}  lo usan {}", e.desde, e.motivo.as_ref().map(|m| m.linea).unwrap_or(0), e.motivo.as_ref().map(|m| m.escrito(&e.desde)).unwrap_or_default(), e.lo_usan); }
        }
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
