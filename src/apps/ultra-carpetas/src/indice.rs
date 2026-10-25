// SPDX-License-Identifier: GPL-2.0-only
//! Todo el proyecto en dos hilos propios (carriles Indice y Busqueda), para que la
//! interfaz nunca espere al disco:
//!
//! - **ultra-indice** recorre la carpeta abierta entera (sin seguir enlaces) y guarda las
//!   rutas en un `Indice` compacto; responde a Ctrl+P puntuando nombres.
//! - **ultra-buscar** lee los archivos de texto y busca dentro (Ctrl+Shift+F). Cada
//!   busqueda nueva cancela la anterior (numero de generacion compartido).
//!
//! Se saltan las carpetas de build generadas: las que tienen `CACHEDIR.TAG` (cargo,
//! y el estandar de carpetas de cache), `build.ninja` o `CMakeCache.txt` (out/ de
//! Chromium, CMake), y `.git`, `.hg`, `.svn`, `node_modules`.

use carpetas_nucleo::ciudad::{juntar, ArchivoCiudad, MAX_EDIFICIOS};
use carpetas_nucleo::editor::buscar;
use carpetas_nucleo::estado::InfoIndice;
use carpetas_nucleo::proyecto::{buscar_en_tramo, ordenar, coincidencias, pasa_filtro, Coincidencia, Indice, MAX_COINCIDENCIAS, MAX_POR_ARCHIVO, MAX_RESULTADOS_IR_A};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SALTAR: &[&str] = &[".git", ".hg", ".svn", "node_modules", "__pycache__", ".cache"];
const MARCAS_DE_BUILD: &[&str] = &["CACHEDIR.TAG", "build.ninja", "CMakeCache.txt"];
const MAX_TEXTO: u64 = 2 * 1024 * 1024;

pub enum Pedido {
    Reindexar { base: PathBuf, ocultos: bool },
    /// Archivos de `raiz` con tamano y lineas, juntados en bloques si son demasiados.
    Ciudad { generacion: u64, raiz: String },
    /// (interno) El hilo de lineas termino: (version del indice, lineas por ruta, total).
    Lineas(u64, Arc<Vec<u32>>, u64, u64),
    Nombres { generacion: u64, consulta: String },
    Texto { generacion: u64, base: PathBuf, carpeta: String, patron: String, mayusculas: bool, palabra: bool, filtro: String },
    Cancelar,
}

pub enum Respuesta {
    Indice(InfoIndice),
    Nombres { generacion: u64, resultados: Vec<(String, bool)>, micros: u64 },
    Texto { generacion: u64, lote: Vec<(String, Vec<Coincidencia>)>, vistos: u64, terminada: bool, truncada: bool, micros: u64 },
    Ciudad { generacion: u64, raiz: String, archivos: Vec<ArchivoCiudad>, micros: u64 },
    Lineas { contadas: u64 },
    /// Carril Museo: sospechas y dependencias de todo el proyecto.
    Analisis(carpetas_nucleo::analisis::ResultadoAnalisis),
}

/// Lineas desconocidas (binario, demasiado grande, fuera del presupuesto).
pub const SIN_LINEAS: u32 = u32::MAX;
/// Presupuesto para contar lineas: hasta 32 GiB leidos o 5 min (Chromium real son varios GiB).
const LINEAS_BYTES: u64 = 32 << 30;
const LINEAS_TIEMPO: Duration = Duration::from_secs(300);

pub struct Proyecto { tx: Sender<Pedido>, rx: Receiver<Respuesta> }

/// Recorre `base` sin seguir enlaces. Devuelve el indice y si se corto por el limite.
pub fn indexar(base: &Path, ocultos: bool) -> (Indice, bool) {
    let mut indice = Indice::default();
    let mut pila = vec![String::new()];
    while let Some(rel) = pila.pop() {
        let dir = if rel.is_empty() { base.to_path_buf() } else { base.join(&rel) };
        let Ok(lectura) = std::fs::read_dir(&dir) else { continue };
        let mut hijos: Vec<(String, bool, u64)> = Vec::new();
        for e in lectura.flatten() {
            let Ok(tipo) = e.file_type() else { continue };
            if tipo.is_symlink() { continue; } // nunca se sigue un enlace (ni bucles ni salir de la carpeta)
            let Some(nombre) = e.file_name().to_str().map(str::to_string) else { continue };
            if !ocultos && nombre.starts_with('.') { continue; }
            // El tamano (para la ciudad): metadata de la entrada, sin seguir enlaces.
            let tam = if tipo.is_dir() { 0 } else { e.metadata().map(|m| m.len()).unwrap_or(0) };
            hijos.push((nombre, tipo.is_dir(), tam));
        }
        // Una carpeta de build generada no se indexa (ni ella ni lo de dentro).
        if !rel.is_empty() && hijos.iter().any(|(n, d, _)| !d && MARCAS_DE_BUILD.contains(&n.as_str())) { continue; }
        hijos.sort();
        for (nombre, es_dir, _) in hijos.iter().rev() {
            if *es_dir && SALTAR.contains(&nombre.as_str()) { continue; }
            let ruta = if rel.is_empty() { nombre.clone() } else { format!("{rel}/{nombre}") };
            if *es_dir { pila.push(ruta); }
        }
        for (nombre, es_dir, tam) in hijos {
            if es_dir && SALTAR.contains(&nombre.as_str()) { continue; }
            let ruta = if rel.is_empty() { nombre } else { format!("{rel}/{nombre}") };
            if !indice.agregar_con(&ruta, es_dir, tam) { return (indice, true); }
        }
    }
    (indice, false)
}

/// Extensiones que nunca son texto: ni se abren (la busqueda no gasta E/S en ellas).
const BINARIOS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico", "icns", "wav", "ogg", "opus", "mp3", "flac", "mp4", "mkv",
    "webm", "mov", "zip", "gz", "xz", "zst", "bz2", "7z", "tar", "rar", "pdf", "so", "a", "o", "obj", "dll", "exe", "bin", "img", "iso", "ttf",
    "otf", "woff", "woff2", "pyc", "class", "jar", "wasm", "pak", "dat", "db", "sqlite"];

/// Lee un archivo de texto (no enlace, hasta 2 MiB). Mira primero 8 KB: si hay un NUL es
/// binario y no se lee el resto.
pub fn leer_texto(ruta: &Path) -> Option<String> {
    if ruta.extension().and_then(|e| e.to_str()).is_some_and(|e| BINARIOS.iter().any(|b| b.eq_ignore_ascii_case(e))) { return None; }
    let meta = std::fs::symlink_metadata(ruta).ok()?;
    if !meta.is_file() || meta.len() > MAX_TEXTO { return None; }
    let mut f = std::fs::File::open(ruta).ok()?;
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    Read::by_ref(&mut f).take(8192).read_to_end(&mut bytes).ok()?;
    if bytes.contains(&0) { return None; }
    Read::by_ref(&mut f).take(MAX_TEXTO).read_to_end(&mut bytes).ok()?;
    String::from_utf8(bytes).ok()
}

/// Cuenta las lineas de los archivos de texto del indice (presupuesto de bytes y tiempo).
/// Devuelve una linea por ruta (SIN_LINEAS si no se conto) y el total contado.
pub fn contar_lineas(indice: &Indice, base: &Path, sigue: &dyn Fn() -> bool) -> (Vec<u32>, u64) {
    let inicio = Instant::now();
    let mut v = vec![SIN_LINEAS; indice.len()];
    let (mut leidos, mut total) = (0u64, 0u64);
    for (i, l) in v.iter_mut().enumerate() {
        if indice.es_carpeta(i) || indice.tam(i) > MAX_TEXTO { continue; }
        if leidos > LINEAS_BYTES || inicio.elapsed() > LINEAS_TIEMPO || (i % 256 == 0 && !sigue()) { break; }
        if indice.tam(i) == 0 { *l = 0; continue; }
        let Some(texto) = leer_texto(&base.join(indice.ruta(i))) else { continue };
        leidos += texto.len() as u64;
        let n = texto.bytes().filter(|b| *b == b'\n').count() as u32 + u32::from(!texto.ends_with('\n'));
        *l = n;
        total += n as u64;
    }
    (v, total)
}

#[allow(clippy::too_many_arguments)]
fn buscar_texto(indice: &Indice, base: &Path, carpeta: &str, patron: &str, mayusculas: bool, palabra: bool, filtro: &str, generacion: u64,
                vigente: &AtomicU64, tx: &Sender<Respuesta>, avisar: &(dyn Fn() + Send)) {
    let inicio = Instant::now();
    let prefijo = if carpeta.is_empty() { String::new() } else { format!("{carpeta}/") };
    let mut lote = Vec::new();
    let (mut vistos, mut total, mut ultimo_envio) = (0u64, 0usize, Instant::now());
    let mut truncada = false;
    for i in 0..indice.len() {
        if vigente.load(Ordering::Relaxed) != generacion { return; } // cancelada: hay otra
        if indice.es_carpeta(i) { continue; }
        let ruta = indice.ruta(i);
        if !ruta.starts_with(&prefijo) || !pasa_filtro(filtro, ruta) { continue; }
        vistos += 1;
        let Some(texto) = leer_texto(&base.join(ruta)) else { continue };
        let hallazgos = buscar(&texto, patron, mayusculas, palabra, MAX_POR_ARCHIVO);
        if !hallazgos.is_empty() {
            total += hallazgos.len();
            lote.push((ruta.to_string(), coincidencias(&texto, &hallazgos)));
            if total >= MAX_COINCIDENCIAS { truncada = true; break; }
        }
        if ultimo_envio.elapsed() > Duration::from_millis(100) {
            let micros = inicio.elapsed().as_micros() as u64;
            if tx.send(Respuesta::Texto { generacion, lote: std::mem::take(&mut lote), vistos, terminada: false, truncada: false, micros }).is_err() { return; }
            avisar();
            ultimo_envio = Instant::now();
        }
    }
    let micros = inicio.elapsed().as_micros() as u64;
    let _ = tx.send(Respuesta::Texto { generacion, lote, vistos, terminada: true, truncada, micros });
    avisar();
}

/// Los archivos de `raiz` para la ciudad (con lineas si ya se contaron), en bloques si son muchos.
fn ciudad(indice: &Indice, lineas: &[u32], generacion: u64, raiz: &str) -> Respuesta {
    let t = Instant::now();
    let prefijo = if raiz.is_empty() { String::new() } else { format!("{raiz}/") };
    let archivos: Vec<ArchivoCiudad> = (0..indice.len()).filter(|i| !indice.es_carpeta(*i) && indice.ruta(*i).starts_with(&prefijo))
        .map(|i| ArchivoCiudad { ruta: indice.ruta(i).to_string(), tam: indice.tam(i), lineas: lineas.get(i).copied().filter(|l| *l != SIN_LINEAS), archivos: 1, bloque: false })
        .collect();
    let archivos = juntar(archivos, raiz, MAX_EDIFICIOS);
    Respuesta::Ciudad { generacion, raiz: raiz.to_string(), archivos, micros: t.elapsed().as_micros() as u64 }
}

/// Ctrl+P repartido entre los nucleos (cada hilo puntua un tramo del indice).
fn nombres_en_paralelo(indice: &Indice, consulta: &str) -> Vec<usize> {
    let hilos = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    let n = indice.len();
    let tramo = n.div_ceil(hilos).max(1);
    let mut todos: Vec<(i32, usize)> = std::thread::scope(|s| {
        let manos: Vec<_> = (0..n).step_by(tramo).map(|a| s.spawn(move || buscar_en_tramo(indice, consulta, a..(a + tramo).min(n), MAX_RESULTADOS_IR_A))).collect();
        manos.into_iter().flat_map(|m| m.join().unwrap_or_default()).collect()
    });
    ordenar(indice, &mut todos);
    todos.truncate(MAX_RESULTADOS_IR_A);
    todos.into_iter().map(|(_, i)| i).collect()
}

impl Proyecto {
    pub fn nuevo(avisar: impl Fn() + Send + Sync + Clone + 'static) -> Proyecto {
        let (tx, rx_pedidos) = mpsc::channel::<Pedido>();
        let tx_interno = tx.clone();
        // Hilo de lineas: cuenta en segundo plano y avisa al hilo del indice.
        let (tx_contar, rx_contar) = mpsc::channel::<(u64, Arc<Indice>, PathBuf)>();
        let version_lineas = Arc::new(AtomicU64::new(0));
        {
            let version_lineas = version_lineas.clone();
            std::thread::Builder::new().name("ultra-lineas".into()).spawn(move || {
                while let Ok((version, indice, base)) = rx_contar.recv() {
                    if version_lineas.load(Ordering::Relaxed) != version { continue; }
                    let t = Instant::now();
                    let sigue = || version_lineas.load(Ordering::Relaxed) == version;
                    let (lineas, total) = contar_lineas(&indice, &base, &sigue);
                    if !sigue() { continue; }
                    if tx_interno.send(Pedido::Lineas(version, Arc::new(lineas), total, t.elapsed().as_micros() as u64)).is_err() { return; }
                }
            }).expect("hilo de lineas");
        }
        let (tx_resp, rx) = mpsc::channel();
        // Hilo del museo: analiza cada indice nuevo; uno viejo se abandona.
        let (tx_museo, rx_museo) = mpsc::channel::<(u64, Arc<Indice>, PathBuf)>();
        let version_museo = Arc::new(AtomicU64::new(0));
        {
            let (version_museo, tx_resp, avisar) = (version_museo.clone(), tx_resp.clone(), avisar.clone());
            std::thread::Builder::new().name("ultra-museo".into()).spawn(move || {
                while let Ok((version, indice, base)) = rx_museo.recv() {
                    if version_museo.load(Ordering::Relaxed) != version { continue; }
                    let vivo = || version_museo.load(Ordering::Relaxed) == version;
                    let r = crate::analisis::analizar(&indice, &base, &vivo);
                    if !vivo() { continue; }
                    if tx_resp.send(Respuesta::Analisis(r)).is_err() { return; }
                    avisar();
                }
            }).expect("hilo del museo");
        }
        let vigente = Arc::new(AtomicU64::new(0));
        // Hilo de busqueda de texto: recibe trabajos con su copia del indice.
        type Trabajo = (Arc<Indice>, PathBuf, String, String, bool, bool, String, u64);
        let (tx_trabajo, rx_trabajo) = mpsc::channel::<Trabajo>();
        {
            let (tx_resp, vigente, avisar) = (tx_resp.clone(), vigente.clone(), avisar.clone());
            std::thread::Builder::new().name("ultra-buscar".into()).spawn(move || {
                while let Ok((indice, base, carpeta, patron, mayus, palabra, filtro, generacion)) = rx_trabajo.recv() {
                    if vigente.load(Ordering::Relaxed) != generacion { continue; }
                    buscar_texto(&indice, &base, &carpeta, &patron, mayus, palabra, &filtro, generacion, &vigente, &tx_resp, &avisar);
                }
            }).expect("hilo de busqueda");
        }
        std::thread::Builder::new().name("ultra-indice".into()).spawn(move || {
            let mut indice = Arc::new(Indice::default());
            let mut pendiente_texto: Option<Trabajo> = None;
            let mut version = 0u64;
            let mut lineas: Arc<Vec<u32>> = Arc::new(Vec::new());
            let mut pendiente_ciudad: Option<(u64, String)> = None;
            while let Ok(primero) = rx_pedidos.recv() {
                // De varias consultas de nombres en cola (se teclea rapido) solo vale la ultima.
                let mut cola: Vec<Pedido> = std::iter::once(primero).chain(rx_pedidos.try_iter()).collect();
                if let Some(ultima) = cola.iter().rposition(|p| matches!(p, Pedido::Nombres { .. })) {
                    let mut k = 0;
                    cola.retain(|p| { k += 1; !matches!(p, Pedido::Nombres { .. }) || k - 1 == ultima });
                }
                for p in cola {
                match p {
                    Pedido::Reindexar { base, ocultos } => {
                        let t = Instant::now();
                        let (nuevo, truncado) = indexar(&base, ocultos);
                        let carpetas = (0..nuevo.len()).filter(|i| nuevo.es_carpeta(*i)).count() as u64;
                        let info = InfoIndice { archivos: nuevo.len() as u64 - carpetas, carpetas, truncado, micros: t.elapsed().as_micros() as u64, listo: true, ..InfoIndice::default() };
                        indice = Arc::new(nuevo);
                        version += 1;
                        lineas = Arc::new(Vec::new());
                        version_lineas.store(version, Ordering::Relaxed);
                        let _ = tx_contar.send((version, indice.clone(), base.clone()));
                        version_museo.store(version, Ordering::Relaxed);
                        let _ = tx_museo.send((version, indice.clone(), base.clone()));
                        if tx_resp.send(Respuesta::Indice(info)).is_err() { return; }
                        avisar();
                        if let Some((g, r)) = pendiente_ciudad.take() { let _ = tx_resp.send(ciudad(&indice, &lineas, g, &r)); avisar(); }
                        // Una busqueda pedida mientras se indexaba se lanza ahora.
                        if let Some(mut tr) = pendiente_texto.take() { tr.0 = indice.clone(); let _ = tx_trabajo.send(tr); }
                    }
                    Pedido::Nombres { generacion, consulta } => {
                        let t = Instant::now();
                        let resultados = nombres_en_paralelo(&indice, &consulta).into_iter()
                            .map(|i| (indice.ruta(i).to_string(), indice.es_carpeta(i))).collect();
                        if tx_resp.send(Respuesta::Nombres { generacion, resultados, micros: t.elapsed().as_micros() as u64 }).is_err() { return; }
                        avisar();
                    }
                    Pedido::Texto { generacion, base, carpeta, patron, mayusculas, palabra, filtro } => {
                        vigente.store(generacion, Ordering::Relaxed);
                        let tr = (indice.clone(), base, carpeta, patron, mayusculas, palabra, filtro, generacion);
                        if indice.is_empty() { pendiente_texto = Some(tr); } else { let _ = tx_trabajo.send(tr); }
                    }
                    Pedido::Cancelar => { vigente.fetch_add(1 << 40, Ordering::Relaxed); pendiente_texto = None; }
                    Pedido::Ciudad { generacion, raiz } => {
                        if indice.is_empty() { pendiente_ciudad = Some((generacion, raiz)); continue; }
                        if tx_resp.send(ciudad(&indice, &lineas, generacion, &raiz)).is_err() { return; }
                        avisar();
                    }
                    Pedido::Lineas(v, l, total, _micros) => {
                        if v != version { continue; } // de un indice viejo
                        lineas = l;
                        if tx_resp.send(Respuesta::Lineas { contadas: total }).is_err() { return; }
                        avisar();
                    }
                }
                }
            }
        }).expect("hilo del indice");
        Proyecto { tx, rx }
    }

    pub fn pedir(&self, p: Pedido) { let _ = self.tx.send(p); }

    /// Lo que llego; de las respuestas de nombres solo importa la ultima.
    pub fn recoger(&self) -> Vec<Respuesta> { self.rx.try_iter().take(256).collect() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn arbol(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ultra-indice-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (r, t) in [("src/main.rs", "fn main() { let vga = 1; }\n"), ("src/vga.c", "int vga;\n// VGA\nvga = 2;\n"),
                       ("third_party/x/vga.c", "vga\n"), ("target/debug/basura.rs", "vga"), (".git/config", "vga"), ("bin.dat", "vga\0\0"),
                       ("out/Default/build.ninja", ""), ("out/Default/gen.cc", "vga"), ("docs/n.md", "nada")] {
            let p = d.join(r);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, t).unwrap();
        }
        std::fs::write(d.join("target/CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55").unwrap();
        std::os::unix::fs::symlink("/etc", d.join("enlace")).unwrap();
        d
    }

    #[test]
    fn indexa_sin_enlaces_ni_builds() {
        let d = arbol("ix");
        let (i, truncado) = indexar(&d, false);
        let rutas: Vec<&str> = (0..i.len()).map(|k| i.ruta(k)).collect();
        assert!(!truncado);
        for r in ["src", "src/main.rs", "src/vga.c", "third_party/x/vga.c", "docs/n.md", "bin.dat"] { assert!(rutas.contains(&r), "{r} en {rutas:?}"); }
        for r in ["target/debug/basura.rs", ".git/config", "out/Default/gen.cc", "enlace"] { assert!(!rutas.iter().any(|x| x.starts_with(r)), "{r} no"); }
        assert!(!rutas.contains(&"target") || !rutas.iter().any(|x| x.starts_with("target/")), "dentro de target nada");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn busca_texto_en_hilo_y_cancela() {
        let d = arbol("bt");
        let p = Proyecto::nuevo(|| {});
        p.pedir(Pedido::Reindexar { base: d.clone(), ocultos: false });
        p.pedir(Pedido::Nombres { generacion: 1, consulta: "vgac".into() });
        p.pedir(Pedido::Texto { generacion: 7, base: d.clone(), carpeta: String::new(), patron: "vga".into(), mayusculas: true, palabra: true,
                                filtro: "!third_party/".into() });
        let limite = Instant::now() + Duration::from_secs(10);
        let (mut nombres, mut texto, mut fin) = (None, Vec::new(), false);
        while Instant::now() < limite && !(fin && nombres.is_some()) {
            for r in p.recoger() {
                match r {
                    Respuesta::Nombres { resultados, .. } => nombres = Some(resultados),
                    Respuesta::Texto { generacion: 7, lote, terminada, .. } => { texto.extend(lote); fin |= terminada; }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let nombres = nombres.unwrap();
        assert!(nombres[0].0.ends_with("vga.c"), "{nombres:?}");
        texto.sort_by(|a, b| a.0.cmp(&b.0));
        let resumen: Vec<(&str, Vec<u32>)> = texto.iter().map(|(r, v)| (r.as_str(), v.iter().map(|c| c.linea).collect())).collect();
        assert_eq!(resumen, [("src/main.rs", vec![1]), ("src/vga.c", vec![1, 3])], "palabra entera, mayusculas, sin third_party, sin binarios ni builds");
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[cfg(test)]
mod medida {
    /// Cuanto tarda buscar en este mismo repositorio (no es una prueba de velocidad fija:
    /// imprime el tiempo; solo exige que termine).
    #[test]
    fn buscar_en_el_repositorio() {
        use super::*;
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let t = Instant::now();
        let (i, _) = indexar(&raiz, false);
        let ti = t.elapsed();
        let (tx, rx) = mpsc::channel();
        let vigente = AtomicU64::new(1);
        let t = Instant::now();
        buscar_texto(&i, &raiz, "", "ruta_segura", true, false, "", 1, &vigente, &tx, &|| {});
        let tb = t.elapsed();
        let n: usize = rx.try_iter().map(|r| match r { Respuesta::Texto { lote, .. } => lote.len(), _ => 0 }).sum();
        eprintln!("indice {} rutas en {ti:?}; busqueda en {tb:?}; {n} archivos con resultados", i.len());
        assert!(n >= 2);
    }
}
