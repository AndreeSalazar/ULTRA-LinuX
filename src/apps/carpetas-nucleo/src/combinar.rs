// SPDX-License-Identifier: GPL-2.0-only
//! Seleccion grupal: como se combinan unos archivos (o carpetas) entre si.
//!
//! Con varios edificios de la Ciudad (o varios nodos del grafo) elegidos, el panel
//! «Combinar» ensena, en vivo:
//! - **Directas:** uno del grupo usa a otro (`#include`, `use`, `import`…).
//! - **Lo que comparten:** archivos de fuera que usan dos o mas del grupo.
//! - **Quien los junta:** archivos de fuera que usan a dos o mas del grupo.
//! - **Sueltos:** los del grupo que no se tocan con ningun otro.
//!
//! Un miembro puede ser un archivo o una carpeta (un bloque de la Ciudad o una carpeta del
//! grafo): una carpeta cuenta con todos sus archivos.

use crate::dependencias::Dependencias;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

/// Como mucho tantos miembros en un grupo (mas no se lee en un panel).
pub const MAX_GRUPO: usize = 64;
/// Cuantos puentes (compartidos o que los juntan) se ensenan de cada clase.
pub const MAX_PUENTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Miembro {
    pub id: String,
    /// Archivos suyos con dependencias (1 si es un archivo; los de dentro si es carpeta).
    pub archivos: u32,
    /// Con cuantos del grupo se toca (directa o por algo compartido).
    pub lazos: u32,
}

/// Un archivo de fuera del grupo que toca a dos o mas miembros.
#[derive(Debug, Clone, PartialEq)]
pub struct Puente {
    pub ruta: String,
    /// Indices en `miembros`, en orden.
    pub miembros: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Combinacion {
    pub miembros: Vec<Miembro>,
    /// (a, b, n): el miembro a usa al b por n aristas entre sus archivos.
    pub directas: Vec<(usize, usize, u32)>,
    /// Lo que usan dos o mas: los mas compartidos primero.
    pub comparten: Vec<Puente>,
    pub total_comparten: u32,
    /// Quien usa a dos o mas: los que mas juntan primero.
    pub juntan: Vec<Puente>,
    pub total_juntan: u32,
    /// Piezas: miembros que quedan unidos por directas o por algo compartido.
    pub piezas: u32,
    /// Miembros que no se tocan con ningun otro.
    pub sueltos: Vec<usize>,
    /// Lo elegido que no tiene dependencias conocidas (no es C, C++, ASM, Rust ni Python,
    /// o el analisis aun no llego).
    pub sin_datos: Vec<String>,
    /// Se pidieron mas de MAX_GRUPO.
    pub recortado: bool,
}

impl Combinacion {
    /// Indice del miembro con ese id.
    pub fn miembro(&self, id: &str) -> Option<usize> { self.miembros.iter().position(|m| m.id == id) }
}

/// Los archivos de un miembro: el propio archivo, o los de dentro de la carpeta.
fn archivos_de(dep: &Dependencias, id: &str) -> Vec<u32> {
    if let Some(i) = dep.id(id) { return alloc::vec![i]; }
    if id.is_empty() { return Vec::new(); }
    let prefijo = alloc::format!("{id}/");
    dep.con_prefijo(&prefijo).collect()
}

/// Como se combinan los elegidos. `grupo` en el orden en que se quieren ver.
pub fn combinar<'a>(dep: &Dependencias, grupo: impl IntoIterator<Item = &'a String>) -> Combinacion {
    let mut c = Combinacion::default();
    let mut dueno: BTreeMap<u32, usize> = BTreeMap::new();
    let mut vistos: BTreeSet<&str> = BTreeSet::new();
    for id in grupo {
        if !vistos.insert(id.as_str()) { continue; }
        if c.miembros.len() >= MAX_GRUPO { c.recortado = true; break; }
        let archivos = archivos_de(dep, id);
        if archivos.is_empty() { c.sin_datos.push(id.clone()); continue; }
        let m = c.miembros.len();
        // Si una carpeta y un archivo suyo estan los dos, el archivo es del primero que llego.
        let mut propios = 0;
        for f in archivos { if let alloc::collections::btree_map::Entry::Vacant(v) = dueno.entry(f) { v.insert(m); propios += 1; } }
        c.miembros.push(Miembro { id: id.clone(), archivos: propios, lazos: 0 });
    }
    let n = c.miembros.len();
    let mut directas: BTreeMap<(usize, usize), u32> = BTreeMap::new();
    let mut comparten: BTreeMap<u32, BTreeSet<usize>> = BTreeMap::new();
    let mut juntan: BTreeMap<u32, BTreeSet<usize>> = BTreeMap::new();
    for (&f, &m) in &dueno {
        for &g in dep.usa_id(f) {
            match dueno.get(&g) {
                Some(&o) if o != m => *directas.entry((m, o)).or_insert(0) += 1,
                Some(_) => {}
                None => { comparten.entry(g).or_default().insert(m); }
            }
        }
        for &u in dep.usado_por_id(f) {
            if !dueno.contains_key(&u) { juntan.entry(u).or_default().insert(m); }
        }
    }
    // Quien se toca con quien: directas y lo compartido (no "quien los junta": que alguien
    // use a los dos no los une entre si).
    let mut toca = alloc::vec![BTreeSet::<usize>::new(); n];
    let mut padre: Vec<usize> = (0..n).collect();
    fn raiz(p: &mut [usize], mut x: usize) -> usize { while p[x] != x { p[x] = p[p[x]]; x = p[x]; } x }
    let unir = |p: &mut Vec<usize>, a: usize, b: usize| { let (x, y) = (raiz(p, a), raiz(p, b)); if x != y { p[x.max(y)] = x.min(y); } };
    for &(a, b) in directas.keys() { toca[a].insert(b); toca[b].insert(a); unir(&mut padre, a, b); }
    for ms in comparten.values().filter(|s| s.len() >= 2) {
        let v: Vec<usize> = ms.iter().copied().collect();
        for &a in &v { for &b in &v { if a != b { toca[a].insert(b); } } }
        for w in v.windows(2) { unir(&mut padre, w[0], w[1]); }
    }
    for (m, t) in c.miembros.iter_mut().zip(&toca) { m.lazos = t.len() as u32; }
    c.sueltos = (0..n).filter(|i| toca[*i].is_empty()).collect();
    c.piezas = (0..n).filter(|i| raiz(&mut padre, *i) == *i).count() as u32;
    c.directas = directas.into_iter().map(|((a, b), k)| (a, b, k)).collect();
    let mejores = |mapa: BTreeMap<u32, BTreeSet<usize>>| -> (Vec<Puente>, u32) {
        let mut v: Vec<(u32, BTreeSet<usize>)> = mapa.into_iter().filter(|(_, s)| s.len() >= 2).collect();
        let total = v.len() as u32;
        v.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| dep.ruta(a.0).cmp(dep.ruta(b.0))));
        (v.into_iter().take(MAX_PUENTES).map(|(g, s)| Puente { ruta: dep.ruta(g).into(), miembros: s.into_iter().collect() }).collect(), total)
    };
    (c.comparten, c.total_comparten) = mejores(comparten);
    (c.juntan, c.total_juntan) = mejores(juntan);
    c
}
