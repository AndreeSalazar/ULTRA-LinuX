// SPDX-License-Identifier: GPL-2.0-only
//! Errores atrapados en los nodos: lee la salida de rustc/cargo, gcc/clang y Python
//! linea a linea y devuelve archivo, linea, columna y mensaje. Asi el nodo que falla se
//! pinta en rojo, en vez de buscar numeros en la terminal. Puro (no_std), con pruebas.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const MAX_DIAGNOSTICOS: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostico {
    pub linea: u32,
    pub columna: u32,
    pub mensaje: String,
    /// true = error; false = aviso.
    pub grave: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lector {
    /// Ultima cabecera "error[...]: ..." o "warning: ..." de rustc, esperando su "--> archivo".
    pendiente: Option<(String, bool)>,
    /// Python: frames del traceback en curso, a la espera del "XxxError: mensaje".
    traceback: Vec<(String, u32)>,
    /// Rust en marcha: "thread 'main' panicked at src/main.rs:5:9:" y el mensaje en la linea siguiente.
    panico: Option<(String, u32, u32)>,
}

fn numero(t: &str) -> Option<u32> { t.trim().parse().ok().filter(|n: &u32| *n > 0 && *n < 10_000_000) }

/// "archivo:linea:columna" (la columna es opcional).
fn posicion(t: &str) -> Option<(String, u32, u32)> {
    let mut partes = t.trim().rsplitn(3, ':');
    let ultimo = partes.next()?;
    let medio = partes.next()?;
    match partes.next() {
        Some(archivo) if numero(medio).is_some() => Some((archivo.to_string(), numero(medio)?, numero(ultimo).unwrap_or(1))),
        _ => Some((medio.to_string(), numero(ultimo)?, 1)),
    }
}

/// Lee una linea de salida. Puede devolver uno o varios diagnosticos (archivo, diagnostico).
pub fn leer(lector: &mut Lector, linea: &str) -> Vec<(String, Diagnostico)> {
    let t = linea.trim_end();
    let limpio = t.trim_start();
    let mut salida = Vec::new();
    if let Some((archivo, l, c)) = lector.panico.take() {
        let msg: String = limpio.chars().take(300).collect();
        salida.push((archivo, Diagnostico { linea: l, columna: c, mensaje: alloc::format!("panic: {msg}"), grave: true }));
        return salida;
    }
    if let Some((_, resto)) = limpio.split_once("' panicked at ").filter(|_| limpio.starts_with("thread '")) {
        if let Some(donde) = resto.strip_suffix(':') {
            lector.panico = posicion(donde);
        } else if let Some((msg, donde)) = resto.rsplit_once("', ") {
            // Formato antiguo: panicked at 'mensaje', src/main.rs:5:9
            if let Some((archivo, l, c)) = posicion(donde) {
                salida.push((archivo, Diagnostico { linea: l, columna: c, mensaje: alloc::format!("panic: {}", msg.trim_start_matches('\'')), grave: true }));
            }
        }
        return salida;
    }
    // rustc: cabecera y luego "  --> src/main.rs:3:5".
    if limpio.starts_with("error") && (limpio.starts_with("error:") || limpio.starts_with("error[")) {
        lector.pendiente = Some((limpio.chars().take(300).collect(), true));
        return salida;
    }
    if limpio.starts_with("warning:") || limpio.starts_with("warning[") {
        lector.pendiente = Some((limpio.chars().take(300).collect(), false));
        return salida;
    }
    if let Some(resto) = limpio.strip_prefix("--> ") {
        if let (Some((msg, grave)), Some((archivo, l, c))) = (lector.pendiente.take(), posicion(resto)) {
            salida.push((archivo, Diagnostico { linea: l, columna: c, mensaje: msg, grave }));
        }
        return salida;
    }
    // Python: File "ruta", line N, in funcion ... y al final "TipoError: mensaje".
    if let Some(resto) = limpio.strip_prefix("File \"") {
        if let Some((archivo, cola)) = resto.split_once('"') {
            if let Some(n) = cola.trim_start_matches(',').trim().strip_prefix("line ").and_then(|x| numero(x.split(',').next().unwrap_or(""))) {
                if lector.traceback.len() < 32 { lector.traceback.push((archivo.to_string(), n)); }
            }
        }
        return salida;
    }
    if !lector.traceback.is_empty() && !t.starts_with(' ') {
        let es_error = limpio.split_once(':').is_some_and(|(tipo, _)| (tipo.ends_with("Error") || tipo.ends_with("Exception")) && !tipo.contains(' '));
        if es_error {
            for (archivo, n) in lector.traceback.drain(..) {
                salida.push((archivo, Diagnostico { linea: n, columna: 1, mensaje: limpio.chars().take(300).collect(), grave: true }));
            }
        }
        return salida;
    }
    // gcc/clang: "ruta:linea:columna: error: mensaje" o "warning:".
    for (marca, grave) in [(": error: ", true), (": fatal error: ", true), (": warning: ", false)] {
        if let Some((donde, msg)) = limpio.split_once(marca) {
            if let Some((archivo, l, c)) = posicion(donde) {
                if !archivo.contains(' ') {
                    salida.push((archivo, Diagnostico { linea: l, columna: c, mensaje: msg.chars().take(300).collect(), grave }));
                }
            }
            break;
        }
    }
    salida
}

/// Donde apunta una linea suelta de la terminal ("--> src/main.rs:3:5", un frame de
/// Python, un panic o "main.c:12:7: error"), para poder hacer clic en ella.
pub fn ubicacion(linea: &str) -> Option<(String, u32, u32)> {
    let t = linea.trim();
    if let Some(r) = t.strip_prefix("--> ") { return posicion(r); }
    if let Some(r) = t.strip_prefix("File \"") {
        let (archivo, cola) = r.split_once('"')?;
        let n = cola.trim_start_matches(',').trim().strip_prefix("line ").and_then(|x| numero(x.split(',').next().unwrap_or("")))?;
        return Some((archivo.to_string(), n, 1));
    }
    if let Some((_, r)) = t.split_once("' panicked at ").filter(|_| t.starts_with("thread '")) {
        return posicion(r.trim_end_matches(':').rsplit(", ").next()?);
    }
    let mut l = Lector::default();
    leer(&mut l, t).into_iter().next().map(|(a, d)| (a, d.linea, d.columna))
}

/// "1 error", "3 errores": la cuenta con su singular o plural.
pub fn contar(n: u32, uno: &str, varios: &str) -> String { alloc::format!("{n} {}", if n == 1 { uno } else { varios }) }

/// Fin del proceso: lo que quedo a medias (un panic sin mensaje) sale igual.
pub fn cerrar(lector: &mut Lector) -> Vec<(String, Diagnostico)> {
    let mut salida = Vec::new();
    if let Some((archivo, l, c)) = lector.panico.take() {
        salida.push((archivo, Diagnostico { linea: l, columna: c, mensaje: String::from("panic"), grave: true }));
    }
    *lector = Lector::default();
    salida
}

/// Ruta de un diagnostico -> id de nodo (relativo a la carpeta abierta). `base` es la
/// carpeta donde corrio el proceso (relativa) y `raiz` la ruta absoluta de la abierta.
/// Rechaza lo que sale de la carpeta abierta.
pub fn resolver(ruta: &str, base: &str, raiz: &str) -> Option<String> {
    let relativa = if let Some(r) = ruta.strip_prefix(raiz).filter(|_| !raiz.is_empty()) {
        r.trim_start_matches('/').to_string()
    } else if ruta.starts_with('/') {
        return None;
    } else if base.is_empty() {
        ruta.to_string()
    } else {
        alloc::format!("{base}/{ruta}")
    };
    let mut partes: Vec<&str> = Vec::new();
    for tramo in relativa.split('/') {
        match tramo {
            "" | "." => {}
            ".." => { partes.pop()?; }
            t => partes.push(t),
        }
    }
    (!partes.is_empty()).then(|| partes.join("/"))
}

/// Errores de la ultima ejecucion, por nodo (ruta relativa a la carpeta abierta).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Errores {
    pub lector: Lector,
    /// Carpeta donde corre el proceso (relativa): las rutas de rustc son relativas a ella.
    pub base: String,
    pub por_nodo: BTreeMap<String, Vec<Diagnostico>>,
    pub total: usize,
    /// F8: por donde va el recorrido de errores.
    pub cursor: usize,
    /// Carpetas que se desplegaron solas por un error (solo una vez por ejecucion).
    pub desplegadas: alloc::collections::BTreeSet<String>,
    /// Ya se pidio parar el proceso por un error.
    pub parado: bool,
}

impl Errores {
    pub fn empezar(&mut self, base: &str) { *self = Errores { base: base.to_string(), ..Errores::default() }; }

    /// Olvida los diagnosticos anotados (lista, cuenta y recorrido F8), sin tocar el lector
    /// ni la carpeta base: para volver a anotarlos de cero desde otra fuente.
    pub fn olvidar_diagnosticos(&mut self) {
        self.por_nodo.clear();
        self.total = 0;
        self.cursor = 0;
    }

    /// Anota; devuelve true si entro algo nuevo (y si alguno es grave).
    pub fn anotar(&mut self, lista: Vec<(String, Diagnostico)>, raiz: &str) -> (bool, bool) {
        let (mut nuevo, mut grave) = (false, false);
        for (ruta, d) in lista {
            if self.total >= MAX_DIAGNOSTICOS { break; }
            let Some(id) = resolver(&ruta, &self.base, raiz) else { continue };
            let v = self.por_nodo.entry(id).or_default();
            if v.iter().any(|x| x.linea == d.linea && x.columna == d.columna && x.mensaje == d.mensaje) { continue; }
            grave |= d.grave;
            v.push(d);
            self.total += 1;
            nuevo = true;
        }
        (nuevo, grave)
    }

    /// El nodo y todo lo que cuelga de el ("" = todo). Las claves con el mismo prefijo
    /// estan juntas en el BTreeMap; luego se descarta "src-x" al buscar "src".
    fn debajo<'a>(&'a self, id: &'a str) -> impl Iterator<Item = (&'a String, &'a Vec<Diagnostico>)> + 'a {
        self.por_nodo.range::<str, _>((core::ops::Bound::Included(id), core::ops::Bound::Unbounded))
            .take_while(move |(k, _)| k.starts_with(id))
            .filter(move |(k, _)| id.is_empty() || k.len() == id.len() || k.as_bytes()[id.len()] == b'/')
    }

    /// (errores, avisos) del nodo; en una carpeta cuenta todo lo de dentro.
    pub fn conteo(&self, id: &str) -> (u32, u32) {
        self.debajo(id).flat_map(|(_, v)| v.iter()).fold((0, 0), |(e, a), d| if d.grave { (e + 1, a) } else { (e, a + 1) })
    }

    /// El primer error (o aviso si no hay errores) del nodo o de lo que tiene dentro.
    pub fn primero<'a>(&'a self, id: &'a str) -> Option<(&'a String, &'a Diagnostico)> {
        let mut todos = self.debajo(id).flat_map(|(k, v)| v.iter().map(move |d| (k, d)));
        let primero = todos.next()?;
        if primero.1.grave { return Some(primero); }
        Some(todos.find(|(_, d)| d.grave).unwrap_or(primero))
    }

    pub fn de(&self, id: &str) -> &[Diagnostico] { self.por_nodo.get(id).map(|v| v.as_slice()).unwrap_or(&[]) }
}
