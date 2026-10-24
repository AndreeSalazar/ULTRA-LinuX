// SPDX-License-Identifier: GPL-2.0-only
//! Organizar carpetas grandes sin tocar el disco (solo la vista), con las reglas que
//! usa un proyecto enorme como Chromium (medido sobre sus 511 951 archivos):
//!
//! 1. **Unidades:** `foo.h`, `foo.cc`, `foo_unittest.cc`, `foo_win.cc`, `foo_mac.mm`...
//!    son una sola pieza de codigo: un nodo `foo` (base/ pasa de 300 archivos a 139 nodos).
//! 2. **Build y metadatos:** `BUILD.gn`, `*.gni`, `DEPS`, `OWNERS`, `DIR_METADATA`, README...
//! 3. **Prefijos:** si aun quedan muchos sueltos (carpetas generadas con 3 514 archivos
//!    `IDS_*.png.sha1`), se agrupan por el comienzo del nombre, en niveles.
//!
//! Los grupos son nodos virtuales: su id lleva un byte NUL, que no puede estar en un
//! nombre de archivo, asi que nunca chocan con una ruta real.

use crate::estado::Entrada;
use crate::tipos::Tipo;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Mas archivos que esto en una carpeta: se organiza (modo automatico).
pub const UMBRAL: usize = 24;
/// Mas sueltos que esto tras sacar unidades: se agrupan por prefijo.
pub const MAX_SUELTOS: usize = 40;
pub const MARCA: char = '\0';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Organizar { #[default] Auto, Siempre, Nunca }

impl Organizar {
    pub fn siguiente(self) -> Self { match self { Organizar::Auto => Organizar::Siempre, Organizar::Siempre => Organizar::Nunca, Organizar::Nunca => Organizar::Auto } }
    pub fn nombre(self) -> &'static str {
        match self { Organizar::Auto => "automatico (mas de 24 archivos)", Organizar::Siempre => "siempre", Organizar::Nunca => "nunca (como en disco)" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase { Unidad, Meta, Prefijo }

#[derive(Debug, Clone, PartialEq)]
pub struct Grupo<'a> {
    /// "u:foo", "m", "p:IDS_A11Y".
    pub clave: String,
    pub clase: Clase,
    pub nombre: String,
    /// "h · cc · unittest · win" o "3 514 archivos".
    pub resumen: String,
    pub miembros: Vec<&'a Entrada>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Hijo<'a> { Real(&'a Entrada), Grupo(Grupo<'a>) }

/// Sufijos de Chromium (y de muchos proyectos C/C++) que no cambian de que pieza se trata.
const SUFIJOS: &[&str] = &["_unittests", "_unittest", "_browsertests", "_browsertest", "_interactive_uitest", "_uitest", "_apitest",
    "_perftest", "_fuzzer", "_nocompile", "_test_utils", "_test_util", "_testing", "_tests", "_test", "_mock", "_fake",
    "_win", "_mac", "_linux", "_posix", "_android", "_ios", "_chromeos", "_fuchsia", "_apple", "_nacl", "_x11", "_ozone",
    "_wayland", "_aura", "_cocoa", "_gtk", "_stub", "_unix", "_windows", "_macos",
    // Implementaciones por mecanismo del sistema (base/files/file_path_watcher_inotify.cc...).
    "_inotify", "_kqueue", "_fsevents", "_epoll", "_freebsd", "_openbsd", "_bsd", "_impl"];

const PRUEBAS: &[&str] = &["_unittests", "_unittest", "_browsertests", "_browsertest", "_interactive_uitest", "_uitest", "_apitest",
    "_perftest", "_fuzzer", "_nocompile", "_test_utils", "_test_util", "_testing", "_tests", "_test", "_mock", "_fake"];

/// "base_paths_android.cc" -> "base_paths"; "IDS_X.png.sha1" -> "IDS_X".
pub fn raiz_unidad(nombre: &str) -> &str {
    let sin_ext = match nombre.find('.') { Some(0) | None => nombre, Some(i) => &nombre[..i] };
    let mut b = sin_ext;
    loop {
        match SUFIJOS.iter().find(|s| b.len() > s.len() && b.ends_with(*s)) {
            Some(s) => b = &b[..b.len() - s.len()],
            None => return b,
        }
    }
}

/// Archivos de build y metadatos (Chromium, GN, Bazel, git, licencias).
pub fn es_meta(nombre: &str) -> bool {
    let n = nombre.to_ascii_lowercase();
    ["build.gn", "build", "build.bazel", "deps", "owners", "dir_metadata", "common_metadata", "metadata", "presubmit.py",
     ".gitignore", ".gitattributes", "codereview.settings", "authors", "patents", "readme", "license", "copying"].contains(&n.as_str())
        || n.ends_with("owners") || n.ends_with(".gni") || n.ends_with(".md") || n.starts_with("readme.") || n.starts_with("license")
        || n.starts_with("readme.chromium")
}

/// Orden dentro de una unidad: cabecera, codigo, variantes de plataforma, pruebas.
fn rango(nombre: &str) -> u8 {
    let sin_ext = match nombre.find('.') { Some(0) | None => nombre, Some(i) => &nombre[..i] };
    let ext = nombre.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    if PRUEBAS.iter().any(|s| sin_ext.contains(s)) { return 3; }
    if raiz_unidad(nombre) != sin_ext { return 2; }
    match ext { "h" | "hpp" | "hh" | "hxx" => 0, "cc" | "cpp" | "c" | "cxx" | "mm" | "m" | "rs" => 1, _ => 4 }
}

/// Etiquetas cortas de lo que tiene una unidad: "h · cc · mm · unittest · win".
fn resumen_unidad(miembros: &[&Entrada]) -> String {
    let mut etiquetas: Vec<String> = Vec::new();
    let anadir = |t: &str, v: &mut Vec<String>| { if !t.is_empty() && !v.iter().any(|x| x == t) { v.push(t.to_string()); } };
    for m in miembros {
        let ext = m.nombre.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        anadir(ext, &mut etiquetas);
    }
    for m in miembros {
        let sin_ext = match m.nombre.find('.') { Some(0) | None => m.nombre.as_str(), Some(i) => &m.nombre[..i] };
        let mut b = sin_ext;
        while let Some(s) = SUFIJOS.iter().find(|s| b.len() > s.len() && b.ends_with(*s)) {
            anadir(s.trim_start_matches('_'), &mut etiquetas);
            b = &b[..b.len() - s.len()];
        }
    }
    let mut r = etiquetas.iter().take(6).cloned().collect::<Vec<_>>().join(" · ");
    if etiquetas.len() > 6 { r.push_str(" …"); }
    r
}

fn es_separador(c: char) -> bool { matches!(c, '_' | '-' | '.' | ' ') }

/// Los primeros `k` trozos del nombre ("IDS_A11Y_ALERT.png" con k=2 -> "IDS_A11Y"), si tiene mas.
fn prefijo(nombre: &str, k: usize) -> Option<&str> {
    let mut trozos = 0;
    let mut dentro = false;
    for (i, c) in nombre.char_indices() {
        if es_separador(c) {
            if dentro { trozos += 1; if trozos == k { return (i + c.len_utf8() < nombre.len()).then(|| &nombre[..i]); } }
            dentro = false;
        } else { dentro = true; }
    }
    None
}

fn miles(n: usize) -> String {
    let s = n.to_string();
    let mut r = String::new();
    for (i, c) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { r.push(' '); } r.push(c); }
    r
}

/// Agrupa por prefijo de `k` trozos; sube k mientras todo caiga en un solo grupo.
fn por_prefijo<'a>(sueltos: Vec<&'a Entrada>, mut k: usize, salida: &mut Vec<Hijo<'a>>) {
    let mut grupos: BTreeMap<&'a str, Vec<&'a Entrada>> = BTreeMap::new();
    let mut solos = Vec::new();
    let tope = k + 6;
    loop {
        grupos.clear();
        solos.clear();
        for e in &sueltos {
            match prefijo(&e.nombre, k) { Some(p) => grupos.entry(p).or_default().push(*e), None => solos.push(*e) }
        }
        let un_solo_grupo = grupos.len() == 1 && grupos.values().next().is_some_and(|g| g.len() * 10 >= sueltos.len() * 9);
        if !un_solo_grupo || k >= tope { break; }
        k += 1;
    }
    // Si los prefijos casi no agrupan (nombres con numeros: X0, X1...), rangos alfabeticos.
    let agrupados: usize = grupos.values().filter(|g| g.len() >= 2).map(|g| g.len()).sum();
    if agrupados * 2 < sueltos.len() {
        let mut todos = sueltos;
        todos.sort_by(|a, b| a.nombre.cmp(&b.nombre));
        return por_rangos(todos, salida);
    }
    for (p, miembros) in grupos {
        if miembros.len() >= 2 {
            let n = miembros.len();
            salida.push(Hijo::Grupo(Grupo { clave: format!("p{k}:{p}"), clase: Clase::Prefijo, nombre: format!("{p}…"),
                                            resumen: format!("{} archivos", miles(n)), miembros }));
        } else {
            solos.extend(miembros);
        }
    }
    salida.extend(solos.into_iter().map(Hijo::Real));
}

/// Trozos alfabeticos de como mucho MAX_SUELTOS (en ordenes de 40, 1 600...): "X0 … X135".
/// La clave son el primer y el ultimo nombre, asi el grupo sigue siendo el mismo al releer.
fn por_rangos<'a>(ordenados: Vec<&'a Entrada>, salida: &mut Vec<Hijo<'a>>) {
    if ordenados.len() <= MAX_SUELTOS { salida.extend(ordenados.into_iter().map(Hijo::Real)); return; }
    let mut tam = MAX_SUELTOS;
    while ordenados.len().div_ceil(tam) > MAX_SUELTOS { tam *= MAX_SUELTOS; }
    for trozo in ordenados.chunks(tam) {
        let (a, z) = (&trozo[0].nombre, &trozo[trozo.len() - 1].nombre);
        let comun = a.chars().zip(z.chars()).take_while(|(x, y)| x == y).count();
        let corto = |n: &str| -> String { let r: String = n.chars().skip(comun.saturating_sub(4)).take(18).collect(); r };
        salida.push(Hijo::Grupo(Grupo { clave: format!("r:{a}\u{1}{z}"), clase: Clase::Prefijo, nombre: format!("{} … {}", corto(a), corto(z)),
                                        resumen: format!("{} archivos", miles(trozo.len())), miembros: trozo.to_vec() }));
    }
}

/// Lo que se ve dentro de una carpeta (o dentro de uno de sus grupos, `dentro`).
pub fn organizar<'a>(entradas: &'a [Entrada], modo: Organizar, dentro: Option<&str>) -> Vec<Hijo<'a>> {
    let carpetas = entradas.iter().filter(|e| e.tipo == Tipo::Carpeta);
    let archivos: Vec<&Entrada> = entradas.iter().filter(|e| e.tipo != Tipo::Carpeta).collect();
    let activo = match modo { Organizar::Nunca => false, Organizar::Siempre => true, Organizar::Auto => archivos.len() > UMBRAL };
    if !activo {
        return if dentro.is_some() { Vec::new() } else { entradas.iter().map(Hijo::Real).collect() };
    }
    // Clasificacion de primer nivel: meta, unidades y sueltos.
    let (meta, resto): (Vec<&Entrada>, Vec<&Entrada>) = archivos.into_iter().partition(|e| es_meta(&e.nombre));
    let mut unidades: BTreeMap<&str, Vec<&Entrada>> = BTreeMap::new();
    for e in resto { unidades.entry(raiz_unidad(&e.nombre)).or_default().push(e); }
    let mut grupos: Vec<Grupo<'a>> = Vec::new();
    let mut sueltos: Vec<&Entrada> = Vec::new();
    for (raiz, mut miembros) in unidades {
        if miembros.len() >= 2 {
            miembros.sort_by(|a, b| (rango(&a.nombre), &a.nombre).cmp(&(rango(&b.nombre), &b.nombre)));
            let resumen = resumen_unidad(&miembros);
            grupos.push(Grupo { clave: format!("u:{raiz}"), clase: Clase::Unidad, nombre: raiz.to_string(), resumen, miembros });
        } else {
            sueltos.extend(miembros);
        }
    }
    let meta_grupo = if meta.len() >= 2 {
        let resumen = meta.iter().take(4).map(|e| e.nombre.as_str()).collect::<Vec<_>>().join(" · ") + if meta.len() > 4 { " …" } else { "" };
        Some(Grupo { clave: String::from("m"), clase: Clase::Meta, nombre: String::from("build y metadatos"), resumen, miembros: meta })
    } else { sueltos.extend(meta); None };
    sueltos.sort_by(|a, b| a.nombre.cmp(&b.nombre));

    match dentro {
        None => {
            let mut salida: Vec<Hijo<'a>> = carpetas.map(Hijo::Real).collect();
            let mut medio: Vec<Hijo<'a>> = grupos.into_iter().map(Hijo::Grupo).collect();
            if sueltos.len() > MAX_SUELTOS { por_prefijo(sueltos, 1, &mut medio); } else { medio.extend(sueltos.into_iter().map(Hijo::Real)); }
            medio.sort_by_key(|h| match h { Hijo::Real(e) => e.nombre.to_lowercase(), Hijo::Grupo(g) => g.nombre.to_lowercase() });
            salida.extend(medio);
            salida.extend(meta_grupo.map(Hijo::Grupo));
            salida
        }
        Some("m") => meta_grupo.map(|g| g.miembros.into_iter().map(Hijo::Real).collect()).unwrap_or_default(),
        Some(c) if c.starts_with("u:") => grupos.into_iter().find(|g| g.clave == c).map(|g| g.miembros.into_iter().map(Hijo::Real).collect()).unwrap_or_default(),
        Some(c) if c.starts_with("r:") => {
            let Some((a, z)) = c[2..].split_once('\u{1}') else { return Vec::new() };
            let mut miembros: Vec<&Entrada> = sueltos.into_iter().filter(|e| e.nombre.as_str() >= a && e.nombre.as_str() <= z).collect();
            miembros.sort_by(|x, y| x.nombre.cmp(&y.nombre));
            let mut salida = Vec::new();
            por_rangos(miembros, &mut salida);
            salida
        }
        Some(c) => {
            // "p3:IDS_A11Y": los sueltos con ese prefijo de 3 trozos; si son muchos, otro nivel.
            let Some((k, p)) = c.strip_prefix('p').and_then(|r| r.split_once(':')).and_then(|(k, p)| Some((k.parse::<usize>().ok()?, p))) else { return Vec::new() };
            let miembros: Vec<&Entrada> = sueltos.into_iter().filter(|e| prefijo(&e.nombre, k) == Some(p)).collect();
            let mut salida = Vec::new();
            if miembros.len() > MAX_SUELTOS { por_prefijo(miembros, k + 1, &mut salida); } else { salida.extend(miembros.into_iter().map(Hijo::Real)); }
            salida
        }
    }
}

/// Id de un grupo dentro de una carpeta ("" = la raiz).
pub fn id_grupo(carpeta: &str, clave: &str) -> String {
    if carpeta.is_empty() { format!("{MARCA}{clave}") } else { format!("{carpeta}/{MARCA}{clave}") }
}

pub fn es_grupo(id: &str) -> bool { id.contains(MARCA) }

/// (carpeta real, clave) de un id de grupo.
pub fn partes_grupo(id: &str) -> Option<(&str, &str)> {
    let i = id.find(MARCA)?;
    let carpeta = id[..i].strip_suffix('/').unwrap_or(&id[..i]);
    Some((carpeta, &id[i + 1..]))
}

/// Grupos que hay que abrir para llegar a `nombre` dentro de su carpeta (para revelarlo).
pub fn camino_a<'a>(entradas: &'a [Entrada], modo: Organizar, nombre: &str) -> Vec<String> {
    let mut claves = Vec::new();
    let mut dentro: Option<String> = None;
    for _ in 0..8 {
        let hijos = organizar(entradas, modo, dentro.as_deref());
        let mut siguiente = None;
        for h in hijos {
            match h {
                Hijo::Real(e) if e.nombre == nombre => return claves,
                Hijo::Grupo(g) if g.miembros.iter().any(|m| m.nombre == nombre) => { siguiente = Some(g.clave); break; }
                _ => {}
            }
        }
        match siguiente { Some(c) => { claves.push(c.clone()); dentro = Some(c); } None => return claves }
    }
    claves
}
