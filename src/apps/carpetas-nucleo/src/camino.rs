// SPDX-License-Identifier: GPL-2.0-only
//! El camino entre dos elegidos (SUPER GRAFO): por donde se unen, con el motivo de cada
//! paso (la linea del `#include`, `use` o `import`) y por que se ve asi.
//!
//! Se busca, en este orden, la union mas fuerte:
//! 1. **A usa B** (directa o a traves de otros): si cambia B, A se recompila.
//! 2. **B usa A**: lo mismo al reves.
//! 3. **Comparten X:** ninguno usa al otro, pero los dos usan X (el X mas cercano a los dos).
//! 4. **Los junta X:** X usa a los dos.
//! 5. **Nada:** no se tocan por ningun camino conocido.
//!
//! Para optimizar, cada paso dice que pasaria sin el: si los dos siguen unidos por otro
//! camino (y de cuantos saltos) o si se separan. Un paso **imprescindible** es el sitio
//! donde cortar para desacoplarlos.

use crate::combinar::archivos_de;
use crate::dependencias::{Dependencias, Motivo, MAX_ALCANCE};
use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};

/// Como mucho tantos pasos se analizan (quitar cada uno y volver a buscar).
pub const MAX_PASOS_ANALIZADOS: usize = 16;
/// Tope del conteo de caminos igual de cortos.
pub const MAX_CUENTA: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Union {
    /// A usa B (directa o indirectamente).
    Usa,
    /// B usa A.
    EsUsado,
    /// Los dos usan el punto.
    Comparten,
    /// El punto usa a los dos.
    Juntan,
}

impl Union {
    pub fn nombre(self) -> &'static str {
        match self { Union::Usa => "A usa B", Union::EsUsado => "B usa A", Union::Comparten => "comparten", Union::Juntan => "los junta" }
    }
}

/// Que pasa si se quita un paso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinPaso {
    /// Siguen unidos igual de fuerte, por otro camino de tantos saltos.
    Desvio(Union, u32),
    /// Se separan del todo: es el sitio donde cortar.
    Imprescindible,
    /// No se analizo (camino muy largo).
    SinAnalizar,
}

/// Un paso: `desde` usa `hasta` (la direccion real de la arista).
#[derive(Debug, Clone, PartialEq)]
pub struct Paso {
    pub desde: String,
    pub hasta: String,
    pub motivo: Option<Motivo>,
    pub sin: SinPaso,
    /// Cuantos archivos del proyecto usan a `hasta` (lo cargado que esta ese eslabon).
    pub lo_usan: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Camino {
    pub a: String,
    pub b: String,
    /// None: no se tocan.
    pub union: Option<Union>,
    /// De A a B, para dibujar en orden.
    pub nodos: Vec<String>,
    /// Los pasos entre nodos seguidos (`pasos[i]` une `nodos[i]` y `nodos[i + 1]`).
    pub pasos: Vec<Paso>,
    /// El punto de encuentro (Comparten / Juntan): indice en `nodos`.
    pub punto: Option<usize>,
    /// Cuantos caminos igual de cortos hay (este es uno).
    pub igual_de_cortos: u64,
    /// Archivos recorridos al buscar.
    pub explorados: u32,
    /// Por que se ve asi, en una frase.
    pub por_que: String,
    pub recortado: bool,
}

/// Un recorrido en anchura de varios origenes. `arriba`: por quien usa (si no, por lo que se usa).
struct Anchura { dist: Vec<u32>, padre: Vec<u32>, cuenta: Vec<u64>, vistos: u32, recortado: bool }

const NADA: u32 = u32::MAX;

fn anchura(dep: &Dependencias, origen: &[u32], arriba: bool, sin: Option<(u32, u32)>, parar_en: Option<&BTreeSet<u32>>) -> Anchura {
    let n = dep.len();
    let mut r = Anchura { dist: vec![NADA; n], padre: vec![NADA; n], cuenta: vec![0; n], vistos: 0, recortado: false };
    let mut nivel: Vec<u32> = Vec::new();
    for &o in origen { if (o as usize) < n && r.dist[o as usize] == NADA { r.dist[o as usize] = 0; r.cuenta[o as usize] = 1; nivel.push(o); } }
    r.vistos = nivel.len() as u32;
    let mut d = 0;
    while !nivel.is_empty() {
        // Se termina el nivel en el que aparece la meta (para contar bien los caminos).
        if let Some(m) = parar_en { if d > 0 && nivel.iter().any(|x| m.contains(x)) { break; } }
        let mut siguiente = Vec::new();
        for &x in &nivel {
            let vecinos = if arriba { dep.usado_por_id(x) } else { dep.usa_id(x) };
            for &y in vecinos {
                let arista = if arriba { (y, x) } else { (x, y) };
                if Some(arista) == sin { continue; }
                let yi = y as usize;
                if r.dist[yi] == NADA {
                    if r.vistos as usize >= MAX_ALCANCE { r.recortado = true; continue; }
                    r.dist[yi] = d + 1;
                    r.padre[yi] = x;
                    r.vistos += 1;
                    siguiente.push(y);
                }
                if r.dist[yi] == d + 1 { r.cuenta[yi] = (r.cuenta[yi] + r.cuenta[x as usize]).min(MAX_CUENTA); }
            }
        }
        nivel = siguiente;
        d += 1;
    }
    r
}

impl Anchura {
    /// De un origen hasta `x`, por los padres.
    fn hasta(&self, mut x: u32) -> Vec<u32> {
        let mut v = vec![x];
        while self.padre[x as usize] != NADA { x = self.padre[x as usize]; v.push(x); }
        v.reverse();
        v
    }
}

/// La mejor union encontrada: nodos de A a B y donde esta el punto de encuentro.
struct Hallazgo { union: Union, nodos: Vec<u32>, punto: Option<usize>, cuenta: u64, vistos: u32, recortado: bool }

impl Hallazgo { fn saltos(&self) -> u32 { self.nodos.len().saturating_sub(1) as u32 } }

/// La meta mas cercana alcanzada (la de id menor si empatan: siempre la misma).
fn meta_alcanzada(r: &Anchura, meta: &BTreeSet<u32>) -> Option<u32> {
    meta.iter().copied().filter(|m| r.dist[*m as usize] != NADA).min_by_key(|m| (r.dist[*m as usize], *m))
}

fn mejor(dep: &Dependencias, fa: &[u32], fb: &[u32], sin: Option<(u32, u32)>) -> Option<Hallazgo> {
    let (sa, sb): (BTreeSet<u32>, BTreeSet<u32>) = (fa.iter().copied().collect(), fb.iter().copied().collect());
    // 1 y 2: uno usa al otro.
    for (desde, meta, union) in [(fa, &sb, Union::Usa), (fb, &sa, Union::EsUsado)] {
        let r = anchura(dep, desde, false, sin, Some(meta));
        if let Some(m) = meta_alcanzada(&r, meta) {
            let mut nodos = r.hasta(m);
            if union == Union::EsUsado { nodos.reverse(); }
            return Some(Hallazgo { union, nodos, punto: None, cuenta: r.cuenta[m as usize], vistos: r.vistos, recortado: r.recortado });
        }
    }
    // 3 y 4: algo de fuera que los dos usan, o que usa a los dos (el mas cercano a ambos).
    for (arriba, union) in [(false, Union::Comparten), (true, Union::Juntan)] {
        let (ra, rb) = (anchura(dep, fa, arriba, sin, None), anchura(dep, fb, arriba, sin, None));
        let punto = (0..dep.len() as u32)
            .filter(|x| ra.dist[*x as usize] != NADA && rb.dist[*x as usize] != NADA && !sa.contains(x) && !sb.contains(x))
            .min_by_key(|x| (ra.dist[*x as usize] + rb.dist[*x as usize], ra.dist[*x as usize].abs_diff(rb.dist[*x as usize]), *x));
        if let Some(x) = punto {
            let mut nodos = ra.hasta(x);
            let i = nodos.len() - 1;
            let mut de_b = rb.hasta(x);
            de_b.pop();
            de_b.reverse();
            nodos.extend(de_b);
            let cuenta = ra.cuenta[x as usize].saturating_mul(rb.cuenta[x as usize]).min(MAX_CUENTA);
            return Some(Hallazgo { union, nodos, punto: Some(i), cuenta, vistos: ra.vistos + rb.vistos, recortado: ra.recortado || rb.recortado });
        }
    }
    None
}

/// La arista real (quien usa a quien) entre dos nodos seguidos del camino.
fn arista(h: &Hallazgo, i: usize) -> (u32, u32) {
    let (x, y) = (h.nodos[i], h.nodos[i + 1]);
    let hacia_b = match h.union {
        Union::Usa => true,
        Union::EsUsado => false,
        // Comparten: de A baja hasta el punto y de B tambien. Juntan: el punto usa a los dos.
        Union::Comparten => i < h.punto.unwrap_or(0),
        Union::Juntan => i >= h.punto.unwrap_or(0),
    };
    if hacia_b { (x, y) } else { (y, x) }
}

fn nombre(r: &str) -> &str { r.rsplit('/').next().unwrap_or(r) }

/// El camino entre `a` y `b` (archivos o carpetas).
pub fn camino(dep: &Dependencias, a: &str, b: &str) -> Camino {
    let (fa, fb) = (archivos_de(dep, a), archivos_de(dep, b));
    let mut c = Camino { a: a.to_string(), b: b.to_string(), union: None, nodos: Vec::new(), pasos: Vec::new(), punto: None,
                         igual_de_cortos: 0, explorados: 0, por_que: String::new(), recortado: false };
    if fa.is_empty() || fb.is_empty() {
        let falta = if fa.is_empty() { a } else { b };
        c.por_que = format!("{} no tiene dependencias conocidas (no es C, C++, ASM, Rust ni Python, o nada lo usa ni usa nada).", nombre(falta));
        return c;
    }
    let sa: BTreeSet<u32> = fa.iter().copied().collect();
    if fb.iter().any(|x| sa.contains(x)) {
        c.por_que = "Uno esta dentro del otro: no hay camino que buscar.".to_string();
        return c;
    }
    let Some(h) = mejor(dep, &fa, &fb, None) else {
        c.por_que = format!("{} y {} no se tocan: ninguno usa al otro, no comparten nada y nada usa a los dos.", nombre(a), nombre(b));
        return c;
    };
    c.union = Some(h.union);
    c.punto = h.punto;
    c.igual_de_cortos = h.cuenta;
    c.explorados = h.vistos;
    c.recortado = h.recortado;
    c.nodos = h.nodos.iter().map(|x| dep.ruta(*x).to_string()).collect();
    for i in 0..h.nodos.len() - 1 {
        let (x, y) = arista(&h, i);
        let sin = if i < MAX_PASOS_ANALIZADOS {
            match mejor(dep, &fa, &fb, Some((x, y))) {
                // Solo cuenta como desvio si la union sigue siendo de la misma clase.
                Some(o) if o.union == h.union => SinPaso::Desvio(o.union, o.saltos()),
                _ => SinPaso::Imprescindible,
            }
        } else { SinPaso::SinAnalizar };
        c.pasos.push(Paso { desde: dep.ruta(x).to_string(), hasta: dep.ruta(y).to_string(), motivo: dep.motivo(x, y).cloned(), sin,
                            lo_usan: dep.usado_por_id(y).len() as u32 });
    }
    let saltos = h.saltos();
    let medio = saltos.saturating_sub(1);
    let otros = if h.cuenta > 1 { format!(" Hay {} caminos igual de cortos; este es uno.", if h.cuenta >= MAX_CUENTA { "mas de un millon de".to_string() } else { h.cuenta.to_string() }) }
                else { " Es el unico tan corto.".to_string() };
    let cortes = c.pasos.iter().filter(|p| p.sin == SinPaso::Imprescindible).count();
    let corte = match cortes {
        0 => " Ningun paso es imprescindible: quitar uno solo no los separa.".to_string(),
        1 => " Un paso es imprescindible: ahi se cortaria la union.".to_string(),
        n => format!(" {n} pasos son imprescindibles: cualquiera de ellos corta la union."),
    };
    let (na, nb) = (nombre(a), nombre(b));
    c.por_que = match h.union {
        Union::Usa if saltos == 1 => format!("{na} usa a {nb} directamente: si cambia {nb}, {na} se recompila."),
        Union::Usa => format!("{na} usa a {nb} a traves de {medio} {}: si cambia {nb}, {na} se recompila.", if medio == 1 { "archivo" } else { "archivos" }),
        Union::EsUsado if saltos == 1 => format!("{nb} usa a {na} directamente: si cambia {na}, {nb} se recompila."),
        Union::EsUsado => format!("{nb} usa a {na} a traves de {medio} {}: si cambia {na}, {nb} se recompila.", if medio == 1 { "archivo" } else { "archivos" }),
        Union::Comparten => {
            let i = h.punto.unwrap_or(0);
            format!("Ninguno usa al otro. Los une {}, que usan los dos ({na} a {} {}, {nb} a {}).", nombre(&c.nodos[i]), i, if i == 1 { "salto" } else { "saltos" }, saltos as usize - i)
        }
        Union::Juntan => {
            let i = h.punto.unwrap_or(0);
            format!("Ninguno usa al otro ni comparten nada. Los junta {}, que usa a los dos.", nombre(&c.nodos[i]))
        }
    } + &otros + &corte;
    c
}
