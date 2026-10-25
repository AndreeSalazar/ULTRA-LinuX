// SPDX-License-Identifier: GPL-2.0-only
//! Quien usa a quien (SUPER GRAFO): `#include` de C/C++/ASM, `mod`/`use` de Rust e
//! `import` de Python, resueltos contra las rutas del proyecto. Con eso, cuando algo
//! falla se conecta todo lo que lo arrastra y se aisla la causa probable.
//!
//! Es lectura de texto, no un compilador: no evalua `#ifdef`, macros en includes, rutas
//! de `-I` ni `#[cfg]`. Una referencia que no se encuentra en el proyecto (la libc, std,
//! un paquete de pip) simplemente no crea arista.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaseRef {
    /// `#include "x.h"`, `%include "x"`, `.include "x"`: primero junto al archivo.
    Comillas,
    /// `#include <x.h>`: desde la raiz del proyecto (o nada si es del sistema).
    Angulos,
    /// `mod x;` de Rust (`ruta` trae `#[path]` si lo habia).
    ModRust,
    /// `use crate::a::b` o `use otro_crate::a` (texto con `::`).
    UsoRust,
    /// `import a.b` / `from .m import x`: cuantos puntos delante.
    Python(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Referencia { pub texto: String, pub linea: u32, pub clase: ClaseRef }

fn ext(ruta: &str) -> &str { ruta.rsplit('/').next().unwrap_or(ruta).rsplit_once('.').map(|(_, e)| e).unwrap_or("") }

/// Lenguajes con referencias que se siguen.
pub fn sigue(ruta: &str) -> bool {
    matches!(ext(ruta), "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "ipp" | "inl" | "tpp" | "cu" | "cuh" | "m" | "mm" | "S" | "s" | "asm" | "nasm" | "inc" | "rs" | "py" | "pyi")
}

/// Las referencias de un archivo, linea a linea (barato: no tokeniza).
pub fn referencias(ruta: &str, texto: &str) -> Vec<Referencia> {
    let e = ext(ruta);
    let mut v = Vec::new();
    let rust = e == "rs";
    let python = e == "py" || e == "pyi";
    let mut path_attr: Option<String> = None;
    let mut en_comentario = false;
    for (k, crudo) in texto.lines().enumerate() {
        let linea = k as u32 + 1;
        let mut l = crudo.trim();
        if rust || !python {
            if en_comentario { match l.find("*/") { Some(p) => { l = l[p + 2..].trim(); en_comentario = false; } None => continue } }
            if l.starts_with("/*") && !l.contains("*/") { en_comentario = true; continue; }
        }
        if v.len() >= 4096 { break; }
        if rust {
            if let Some(p) = l.strip_prefix("#[path").and_then(|r| r.split('"').nth(1)) { path_attr = Some(p.to_string()); continue; }
            let sin_pub = quitar_pub(l);
            if let Some(resto) = sin_pub.strip_prefix("mod ") {
                let nombre = resto.trim_end().strip_suffix(';').map(str::trim);
                if let Some(n) = nombre.filter(|n| es_nombre(n)) {
                    let texto = path_attr.take().map(|p| format!("#{p}")).unwrap_or_else(|| n.to_string());
                    v.push(Referencia { texto, linea, clase: ClaseRef::ModRust });
                }
                continue;
            }
            if !l.starts_with("#[") { path_attr = None; }
            if let Some(resto) = sin_pub.strip_prefix("use ") {
                // use a::b::{c, d::e}; -> a::b::c, a::b::d::e (un nivel de llaves)
                let resto = resto.trim_end().trim_end_matches(';').trim();
                for camino in expandir_use(resto) { v.push(Referencia { texto: camino, linea, clase: ClaseRef::UsoRust }); }
            }
            continue;
        }
        if python {
            if let Some(resto) = l.strip_prefix("from ") {
                let (modulo, nombres) = resto.split_once(" import ").unwrap_or((resto, ""));
                let modulo = modulo.trim();
                let puntos = modulo.bytes().take_while(|b| *b == b'.').count() as u8;
                let m = &modulo[puntos as usize..];
                if m.is_empty() {
                    for n in nombres.trim_matches(|c| c == '(' || c == ')' || c == ' ').split(',') {
                        let n = n.split_whitespace().next().unwrap_or("");
                        if es_nombre(n) { v.push(Referencia { texto: n.to_string(), linea, clase: ClaseRef::Python(puntos) }); }
                    }
                } else if m.split('.').all(es_nombre) {
                    // from pkg.sub import b: b puede ser un submodulo (pkg/sub/b.py); si no, vale pkg/sub.
                    let antes = v.len();
                    for n in nombres.trim_matches(|c| c == '(' || c == ')' || c == ' ').split(',') {
                        let n = n.split_whitespace().next().unwrap_or("");
                        if es_nombre(n) { v.push(Referencia { texto: format!("{m}.{n}"), linea, clase: ClaseRef::Python(puntos) }); }
                    }
                    if v.len() == antes { v.push(Referencia { texto: m.to_string(), linea, clase: ClaseRef::Python(puntos) }); }
                }
            } else if let Some(resto) = l.strip_prefix("import ") {
                for m in resto.split(',') {
                    let m = m.split_whitespace().next().unwrap_or("");
                    if !m.is_empty() && m.split('.').all(es_nombre) { v.push(Referencia { texto: m.to_string(), linea, clase: ClaseRef::Python(0) }); }
                }
            }
            continue;
        }
        // C, C++ y ASM
        let directiva = l.strip_prefix('#').map(str::trim_start).and_then(|r| r.strip_prefix("include").or_else(|| r.strip_prefix("import")))
            .or_else(|| l.strip_prefix("%include")).or_else(|| l.strip_prefix(".include"));
        let Some(resto) = directiva else { continue };
        let resto = resto.trim_start();
        if let Some(r) = resto.strip_prefix('"') {
            if let Some((x, _)) = r.split_once('"') { if !x.is_empty() { v.push(Referencia { texto: x.to_string(), linea, clase: ClaseRef::Comillas }); } }
        } else if let Some(r) = resto.strip_prefix('<') {
            if let Some((x, _)) = r.split_once('>') { if !x.is_empty() { v.push(Referencia { texto: x.to_string(), linea, clase: ClaseRef::Angulos }); } }
        }
    }
    v
}

fn quitar_pub(l: &str) -> &str {
    let Some(r) = l.strip_prefix("pub") else { return l };
    let r = r.trim_start();
    if let Some(r2) = r.strip_prefix('(') { return r2.split_once(')').map(|(_, x)| x.trim_start()).unwrap_or(l); }
    r
}

fn es_nombre(n: &str) -> bool { !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') && !n.as_bytes()[0].is_ascii_digit() }

fn expandir_use(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    match s.split_once('{') {
        Some((base, resto)) => {
            let dentro = resto.rsplit_once('}').map(|(x, _)| x).unwrap_or(resto);
            let mut nivel = 0;
            let mut actual = String::new();
            for c in dentro.chars().chain(core::iter::once(',')) {
                match c { '{' => { nivel += 1; actual.push(c); } '}' => { nivel -= 1; actual.push(c); }
                          ',' if nivel == 0 => { let a = actual.trim(); if !a.is_empty() && !a.contains('{') { v.push(format!("{base}{a}")); } actual.clear(); }
                          _ => actual.push(c) }
            }
        }
        None => v.push(s.split(" as ").next().unwrap_or(s).trim().to_string()),
    }
    v.into_iter().filter(|x| x.contains("::")).take(32).collect()
}

/// Une y normaliza "a/b" + "../c" sin salir de la raiz (None si sale).
pub fn unir(dir: &str, rel: &str) -> Option<String> {
    let mut partes: Vec<&str> = if dir.is_empty() { Vec::new() } else { dir.split('/').collect() };
    for p in rel.split('/') {
        match p { "" | "." => {} ".." => { partes.pop()?; } x => partes.push(x) }
    }
    Some(partes.join("/"))
}

fn dir_de(ruta: &str) -> &str { ruta.rsplit_once('/').map(|(d, _)| d).unwrap_or("") }

/// Lo que la plataforma sabe del proyecto para resolver: si existe una ruta y que rutas
/// terminan en un nombre (para `#include "base/files/file.h"` desde cualquier sitio).
pub trait Rutas {
    fn existe(&self, ruta: &str) -> bool;
    /// Rutas cuyo ultimo componente es `nombre`.
    fn por_nombre(&self, nombre: &str) -> Vec<&str>;
}

/// A que archivo del proyecto apunta una referencia hecha desde `desde`.
pub fn resolver(desde: &str, r: &Referencia, rutas: &dyn Rutas) -> Option<String> {
    let dir = dir_de(desde);
    let probar = |c: Option<String>| c.filter(|x| rutas.existe(x) && x != desde);
    match r.clase {
        ClaseRef::Comillas | ClaseRef::Angulos => {
            if r.clase == ClaseRef::Comillas { if let Some(x) = probar(unir(dir, &r.texto)) { return Some(x); } }
            if let Some(x) = probar(unir("", &r.texto)) { return Some(x); }
            // Por el final: "linux/kernel.h" -> include/linux/kernel.h; el mas cercano al que incluye.
            let nombre = r.texto.rsplit('/').next()?;
            let cola = format!("/{}", r.texto.trim_start_matches("./"));
            let mut mejor: Option<(usize, &str)> = None;
            for c in rutas.por_nombre(nombre) {
                if c == desde || !(c.ends_with(&cola) || c == r.texto) { continue; }
                let comun = c.bytes().zip(desde.bytes()).take_while(|(a, b)| a == b).count();
                if mejor.is_none_or(|(m, x)| comun > m || (comun == m && c.len() < x.len())) { mejor = Some((comun, c)); }
            }
            mejor.map(|(_, c)| c.to_string())
        }
        ClaseRef::ModRust => {
            if let Some(p) = r.texto.strip_prefix('#') { return probar(unir(dir, p)); }
            let archivo = desde.rsplit('/').next().unwrap_or(desde);
            // En main.rs, lib.rs y mod.rs los modulos van al lado; en x.rs, dentro de x/. Una
            // raiz de crate suelta (tests/a.rs, examples/b.rs) tambien los tiene al lado.
            let propio = format!("{}{}", if dir.is_empty() { String::new() } else { format!("{dir}/") }, archivo.trim_end_matches(".rs"));
            let bases: &[&str] = if matches!(archivo, "main.rs" | "lib.rs" | "mod.rs") { &[dir] } else { &[&propio, dir] };
            bases.iter().find_map(|b| probar(unir(b, &format!("{}.rs", r.texto))).or_else(|| probar(unir(b, &format!("{}/mod.rs", r.texto)))))
        }
        ClaseRef::UsoRust => {
            let mut tramos: Vec<&str> = r.texto.split("::").map(str::trim).collect();
            let primero = tramos.remove(0);
            let src = match primero {
                "crate" => raiz_crate(desde, rutas)?,
                "self" | "super" | "std" | "core" | "alloc" => return None,
                otro => crate_hermano(otro, rutas)?,
            };
            // a::b::c -> src/a/b/c.rs, src/a/b.rs, src/a.rs (el mas largo que exista).
            let tramos: Vec<&str> = tramos.into_iter().filter(|t| es_nombre(t)).collect();
            for n in (1..=tramos.len()).rev() {
                let camino = tramos[..n].join("/");
                if let Some(x) = probar(unir(&src, &format!("{camino}.rs"))).or_else(|| probar(unir(&src, &format!("{camino}/mod.rs")))) { return Some(x); }
            }
            if primero != "crate" { return ["lib.rs", "main.rs"].iter().find_map(|f| probar(unir(&src, f))); }
            None
        }
        ClaseRef::Python(puntos) => {
            let camino = r.texto.replace('.', "/");
            let mut bases: Vec<String> = Vec::new();
            if puntos > 0 {
                let mut d = dir.to_string();
                for _ in 1..puntos { d = dir_de(&d).to_string(); }
                bases.push(d);
            } else {
                bases.push(String::new());
                bases.push(dir.to_string());
            }
            for b in &bases {
                let partes: Vec<&str> = camino.split('/').collect();
                for n in (1..=partes.len()).rev() {
                    let c = partes[..n].join("/");
                    if let Some(x) = probar(unir(b, &format!("{c}.py"))).or_else(|| probar(unir(b, &format!("{c}/__init__.py")))) { return Some(x); }
                }
            }
            None
        }
    }
}

/// La carpeta `src` del crate de un archivo de Rust (la que tiene lib.rs o main.rs).
fn raiz_crate(desde: &str, rutas: &dyn Rutas) -> Option<String> {
    let mut d = dir_de(desde);
    loop {
        if rutas.existe(&unir(d, "lib.rs")?) || rutas.existe(&unir(d, "main.rs")?) { return Some(d.to_string()); }
        if d.is_empty() { return None; }
        d = dir_de(d);
    }
}

/// `use carpetas_nucleo::x` desde otro crate del mismo proyecto: .../carpetas-nucleo/src.
fn crate_hermano(nombre: &str, rutas: &dyn Rutas) -> Option<String> {
    let guion = nombre.replace('_', "-");
    rutas.por_nombre("lib.rs").into_iter().find_map(|c| {
        let src = dir_de(c);
        let carpeta = dir_de(src);
        let ultimo = carpeta.rsplit('/').next().unwrap_or(carpeta);
        (src.ends_with("src") && (ultimo == guion || ultimo == nombre)).then(|| src.to_string())
    })
}

// ====================================================================================
// El grafo de dependencias y la conexion de fallos.
// ====================================================================================

/// Grafo dirigido: `a usa b` (a incluye, importa o declara b).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dependencias {
    pub rutas: Vec<String>,
    indice: BTreeMap<String, u32>,
    usa: Vec<Vec<u32>>,
    usado_por: Vec<Vec<u32>>,
    pub aristas: u64,
}

impl Dependencias {
    pub fn nuevo(rutas: Vec<String>, aristas: &[(u32, u32)]) -> Dependencias {
        let n = rutas.len();
        let mut usa = alloc::vec![Vec::new(); n];
        let mut usado_por = alloc::vec![Vec::new(); n];
        let mut total = 0;
        for &(a, b) in aristas {
            if (a as usize) < n && (b as usize) < n && a != b { usa[a as usize].push(b); usado_por[b as usize].push(a); total += 1; }
        }
        for v in usa.iter_mut().chain(usado_por.iter_mut()) { v.sort_unstable(); v.dedup(); }
        let indice = rutas.iter().enumerate().map(|(i, r)| (r.clone(), i as u32)).collect();
        Dependencias { rutas, indice, usa, usado_por, aristas: total }
    }
    pub fn id(&self, ruta: &str) -> Option<u32> { self.indice.get(ruta).copied() }
    pub fn ruta(&self, id: u32) -> &str { &self.rutas[id as usize] }
    pub fn usa(&self, ruta: &str) -> impl Iterator<Item = &str> { self.id(ruta).into_iter().flat_map(move |i| self.usa[i as usize].iter().map(move |j| self.ruta(*j))) }
    pub fn usado_por(&self, ruta: &str) -> impl Iterator<Item = &str> { self.id(ruta).into_iter().flat_map(move |i| self.usado_por[i as usize].iter().map(move |j| self.ruta(*j))) }
    pub fn len(&self) -> usize { self.rutas.len() }
    pub fn is_empty(&self) -> bool { self.rutas.is_empty() }

    /// Recorrido en anchura desde `origen`: hacia lo que usan (`arriba` = false) o hacia
    /// quien los usa (`arriba` = true). Devuelve id -> distancia (como mucho `max`).
    pub fn alcance(&self, origen: &[u32], arriba: bool, max: usize) -> (BTreeMap<u32, u16>, bool) {
        let mut vistos: BTreeMap<u32, u16> = BTreeMap::new();
        let mut cola: VecDeque<u32> = VecDeque::new();
        for o in origen { if vistos.insert(*o, 0).is_none() { cola.push_back(*o); } }
        while let Some(x) = cola.pop_front() {
            let d = vistos[&x];
            let vecinos = if arriba { &self.usado_por[x as usize] } else { &self.usa[x as usize] };
            for y in vecinos {
                if vistos.contains_key(y) { continue; }
                if vistos.len() >= max { return (vistos, true); }
                vistos.insert(*y, d.saturating_add(1));
                cola.push_back(*y);
            }
        }
        (vistos, false)
    }
}

/// Papel de un archivo cuando se conecta un fallo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rol {
    /// Lo que usa la causa (o el archivo aislado): de donde puede venir el problema.
    Base,
    /// Usa (directa o indirectamente) algo que falla: se puede romper con el.
    Arrastra,
    /// Falla, pero lo que usa tambien falla: probablemente es consecuencia.
    Falla,
    /// Falla y nada de lo que usa falla: causa probable (o el archivo aislado).
    Causa,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Causa {
    pub ruta: String,
    /// Cuantos de los otros archivos que fallan dependen de este.
    pub explica: u32,
    /// Cuantos archivos del proyecto lo usan, directa o indirectamente.
    pub arrastra: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Conexion {
    pub roles: BTreeMap<String, Rol>,
    /// Causas probables, la que mas explica primero.
    pub causas: Vec<Causa>,
    /// Aristas entre archivos con papel (a usa b), como mucho MAX_ARISTAS.
    pub aristas: Vec<(String, String)>,
    pub truncada: bool,
}

pub const MAX_ALCANCE: usize = 20_000;
pub const MAX_ARISTAS: usize = 4_000;
pub const MAX_CAUSAS: usize = 300;

/// Conecta los fallos: quien es causa probable, quien es consecuencia y que arrastran.
/// `fallos`: archivos con errores del compilador o sospechas del museo. `propios`: los que
/// fallan por algo escrito en ellos mismos (el museo): siempre son causa, aunque usen otro
/// que falla. Un error del compilador si puede venir de lo que se incluye.
///
/// Un recorrido para todos: lo que usa (directa o indirectamente) algun archivo que falla
/// es "consecuencia" si tambien falla; los que fallan y no usan nada que falle son causas.
pub fn conectar(dep: &Dependencias, fallos: &BTreeSet<String>, propios: &BTreeSet<String>) -> Conexion {
    let mut c = Conexion::default();
    if fallos.is_empty() { return c; }
    let ids: BTreeSet<u32> = fallos.iter().filter_map(|f| dep.id(f)).collect();
    // Quien usa a alguien que falla (sin contar al propio origen): un recorrido multiorigen
    // que empieza en los que usan a los que fallan.
    let padres: Vec<u32> = ids.iter().flat_map(|i| dep.usado_por[*i as usize].iter().copied()).collect();
    let (usan_fallo, t) = dep.alcance(&padres, true, MAX_ALCANCE * 5);
    c.truncada |= t;
    let mut causas = Vec::new();
    for f in fallos {
        match dep.id(f) {
            Some(i) if usan_fallo.contains_key(&i) && !propios.contains(f) => { c.roles.insert(f.clone(), Rol::Falla); }
            _ => { c.roles.insert(f.clone(), Rol::Causa); causas.push(Causa { ruta: f.clone(), explica: 0, arrastra: 0 }); }
        }
    }
    // Cuanto explica cada causa (las primeras MAX_CAUSAS; el resto queda en 0).
    for causa in causas.iter_mut().take(MAX_CAUSAS) {
        let Some(i) = dep.id(&causa.ruta) else { continue };
        let (arriba, t) = dep.alcance(&[i], true, MAX_ALCANCE);
        c.truncada |= t;
        causa.explica = arriba.keys().filter(|x| **x != i && ids.contains(x)).count() as u32;
        causa.arrastra = arriba.len().saturating_sub(1) as u32;
    }
    // Un ciclo en el que todos fallan: no hay uno solo que explique al resto; todos son causa.
    if causas.is_empty() {
        for (r, rol) in c.roles.iter_mut() { *rol = Rol::Causa; causas.push(Causa { ruta: r.clone(), explica: 0, arrastra: 0 }); }
    }
    c.truncada |= causas.len() > MAX_CAUSAS;
    causas.sort_by(|a, b| b.explica.cmp(&a.explica).then(b.arrastra.cmp(&a.arrastra)).then(a.ruta.cmp(&b.ruta)));
    // Lo que arrastran las causas: quien las usa (sin pisar a los que fallan).
    let raices: Vec<u32> = causas.iter().filter_map(|x| dep.id(&x.ruta)).collect();
    let (arriba, t) = dep.alcance(&raices, true, MAX_ALCANCE);
    c.truncada |= t;
    for id in arriba.keys() { c.roles.entry(dep.ruta(*id).to_string()).or_insert(Rol::Arrastra); }
    c.causas = causas;
    c.aristas = aristas_entre(dep, &c.roles, &mut c.truncada);
    c
}

/// Aisla un archivo: lo que usa (Base) y quien lo usa (Arrastra), con el en el centro.
pub fn aislar(dep: &Dependencias, centro: &str) -> Conexion {
    let mut c = Conexion::default();
    c.roles.insert(centro.to_string(), Rol::Causa);
    if let Some(i) = dep.id(centro) {
        let (abajo, t1) = dep.alcance(&[i], false, MAX_ALCANCE);
        let (arriba, t2) = dep.alcance(&[i], true, MAX_ALCANCE);
        c.truncada = t1 || t2;
        for id in abajo.keys().filter(|x| **x != i) { c.roles.insert(dep.ruta(*id).to_string(), Rol::Base); }
        for id in arriba.keys().filter(|x| **x != i) { c.roles.insert(dep.ruta(*id).to_string(), Rol::Arrastra); }
        c.causas.push(Causa { ruta: centro.to_string(), explica: 0, arrastra: arriba.len().saturating_sub(1) as u32 });
    }
    c.aristas = aristas_entre(dep, &c.roles, &mut c.truncada);
    c
}

fn aristas_entre(dep: &Dependencias, roles: &BTreeMap<String, Rol>, truncada: &mut bool) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for a in roles.keys() {
        for b in dep.usa(a) {
            if roles.contains_key(b) {
                if v.len() >= MAX_ARISTAS { *truncada = true; return v; }
                v.push((a.clone(), b.to_string()));
            }
        }
    }
    v
}
