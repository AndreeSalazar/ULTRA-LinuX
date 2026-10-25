// SPDX-License-Identifier: GPL-2.0-only
//! Ciclos de cabeceras (SUPER GRAFO, eje Elegancia): a.h incluye b.h y b.h, por el camino
//! que sea, vuelve a incluir a.h. Compila gracias a las guardas (`#ifndef` o
//! `#pragma once`), pero ninguno se entiende ni se prueba sin el otro, y el orden de los
//! `#include` empieza a importar (C++ Core Guidelines SF.9).
//!
//! Solo C, C++ y ASM: en Rust los modulos de un crate se usan entre si sin problema, y eso
//! seria ruido.
//!
//! - Las componentes fuertes (Tarjan, sin recursion): cada grupo de archivos en el que
//!   todos llegan a todos.
//! - De cada una, la **vuelta** mas corta, con el motivo de cada paso (la linea del
//!   `#include`).
//! - **Donde cortar:** el paso de la vuelta cuyo incluido usan menos archivos. Es la
//!   referencia mas barata de quitar o mover. Es una pista: se comprueba compilando.

use crate::dependencias::{Dependencias, Motivo};
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// Como mucho tantos ciclos con su vuelta calculada (el resto solo se cuenta).
pub const MAX_CICLOS: usize = 100;
/// Desde cuantos nodos de cada componente se busca la vuelta mas corta.
pub const MAX_INICIOS: usize = 24;
/// Desde este tamano, la vuelta se busca desde un solo inicio.
pub const GIGANTE: usize = 2_000;

/// C, C++ y ASM: donde un ciclo de `#include` es un olor de diseno.
pub fn cuenta(ruta: &str) -> bool {
    let e = ruta.rsplit('/').next().unwrap_or(ruta).rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    matches!(e, "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "ipp" | "inl" | "tpp" | "cu" | "cuh" | "m" | "mm" | "S" | "s" | "asm" | "nasm" | "inc")
}

/// Un paso de la vuelta: `desde` incluye `hasta`.
#[derive(Debug, Clone, PartialEq)]
pub struct Eslabon {
    pub desde: String,
    pub hasta: String,
    pub motivo: Option<Motivo>,
    /// Cuantos archivos del proyecto usan a `hasta`.
    pub lo_usan: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ciclo {
    /// Todos los de la componente (ordenados).
    pub archivos: Vec<String>,
    /// Aristas dentro de la componente.
    pub aristas: u32,
    /// La vuelta mas corta encontrada (vuelve al primero).
    pub vuelta: Vec<Eslabon>,
    /// El paso de la vuelta donde cortar (indice en `vuelta`).
    pub corte: Option<usize>,
    /// Algun paso de la vuelta esta dentro de un `#if`: el ciclo solo existe si se cumple
    /// (por ejemplo, solo al compilar para Windows).
    pub condicional: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ciclos {
    /// Los mas grandes primero (como mucho MAX_CICLOS).
    pub lista: Vec<Ciclo>,
    /// Cuantos ciclos hay en total.
    pub total: u32,
    /// Cuantos archivos estan en algun ciclo.
    pub archivos: u32,
    /// Cuantos de los ciclos calculados dependen de un `#if`.
    pub condicionales: u32,
}

/// Componentes fuertes de 2 o mas archivos (Tarjan iterativo).
fn componentes(dep: &Dependencias, vale: &[bool]) -> Vec<Vec<u32>> {
    const NO: u32 = u32::MAX;
    let n = dep.len();
    let (mut indice, mut bajo, mut en_pila) = (vec![NO; n], vec![0u32; n], vec![false; n]);
    let (mut pila, mut llamadas, mut salida) = (Vec::new(), Vec::<(u32, usize)>::new(), Vec::new());
    let mut siguiente = 0u32;
    for s in 0..n as u32 {
        if !vale[s as usize] || indice[s as usize] != NO { continue; }
        indice[s as usize] = siguiente;
        bajo[s as usize] = siguiente;
        siguiente += 1;
        pila.push(s);
        en_pila[s as usize] = true;
        llamadas.push((s, 0));
        while let Some(&(v, k)) = llamadas.last() {
            let vecinos = dep.usa_id(v);
            if k < vecinos.len() {
                if let Some(arriba) = llamadas.last_mut() { arriba.1 += 1; }
                let w = vecinos[k];
                if !vale[w as usize] { continue; }
                if indice[w as usize] == NO {
                    indice[w as usize] = siguiente;
                    bajo[w as usize] = siguiente;
                    siguiente += 1;
                    pila.push(w);
                    en_pila[w as usize] = true;
                    llamadas.push((w, 0));
                } else if en_pila[w as usize] {
                    bajo[v as usize] = bajo[v as usize].min(indice[w as usize]);
                }
                continue;
            }
            llamadas.pop();
            if let Some(&(u, _)) = llamadas.last() { bajo[u as usize] = bajo[u as usize].min(bajo[v as usize]); }
            if bajo[v as usize] == indice[v as usize] {
                let mut comp = Vec::new();
                while let Some(x) = pila.pop() {
                    en_pila[x as usize] = false;
                    comp.push(x);
                    if x == v { break; }
                }
                if comp.len() >= 2 { salida.push(comp); }
            }
        }
    }
    salida
}

/// La vuelta mas corta dentro de una componente: de un inicio, por dentro, hasta el.
fn vuelta(dep: &Dependencias, comp: &[u32], dentro: &[bool], padre: &mut [u32]) -> Vec<u32> {
    const NO: u32 = u32::MAX;
    // Los inicios: los mas conectados dentro (pasan por mas vueltas).
    let grado = |x: u32| dep.usa_id(x).iter().filter(|y| dentro[**y as usize]).count() + dep.usado_por_id(x).iter().filter(|y| dentro[**y as usize]).count();
    let mut inicios: Vec<u32> = comp.to_vec();
    inicios.sort_by_key(|x| (core::cmp::Reverse(grado(*x)), *x));
    // En una componente gigante, un solo inicio: cada busqueda la recorre entera.
    inicios.truncate(if comp.len() > GIGANTE { 1 } else { MAX_INICIOS });
    let mut mejor: Vec<u32> = Vec::new();
    for &s in &inicios {
        let mut tocados = Vec::new();
        let mut cola = VecDeque::from([s]);
        let mut cierre = None;
        'busca: while let Some(x) = cola.pop_front() {
            for &y in dep.usa_id(x) {
                if !dentro[y as usize] { continue; }
                if y == s { cierre = Some(x); break 'busca; }
                if padre[y as usize] == NO { padre[y as usize] = x; tocados.push(y); cola.push_back(y); }
            }
        }
        let camino = cierre.map(|mut x| {
            let mut camino = vec![x];
            while x != s { x = padre[x as usize]; camino.push(x); }
            camino
        });
        for y in tocados { padre[y as usize] = NO; }
        let Some(mut camino) = camino else { continue };
        camino.reverse(); // s ... ultimo (que incluye a s)
        if mejor.is_empty() || camino.len() < mejor.len() { mejor = camino; }
        if mejor.len() == 2 { break; }
    }
    mejor
}

/// Los ciclos de cabeceras del proyecto (C, C++ y ASM).
pub fn ciclos(dep: &Dependencias) -> Ciclos {
    let vale: Vec<bool> = dep.rutas.iter().map(|r| cuenta(r)).collect();
    let mut comps = componentes(dep, &vale);
    comps.sort_by_key(|c| core::cmp::Reverse(c.len()));
    let mut r = Ciclos { total: comps.len() as u32, archivos: comps.iter().map(|c| c.len() as u32).sum(), lista: Vec::new(), condicionales: 0 };
    let mut dentro = vec![false; dep.len()];
    let mut padre = vec![u32::MAX; dep.len()];
    for comp in comps.iter().take(MAX_CICLOS) {
        for x in comp { dentro[*x as usize] = true; }
        let aristas = comp.iter().map(|x| dep.usa_id(*x).iter().filter(|y| dentro[**y as usize]).count() as u32).sum();
        let v = vuelta(dep, comp, &dentro, &mut padre);
        let vuelta: Vec<Eslabon> = (0..v.len()).map(|i| {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            Eslabon { desde: dep.ruta(a).to_string(), hasta: dep.ruta(b).to_string(), motivo: dep.motivo(a, b).cloned(), lo_usan: dep.usado_por_id(b).len() as u32 }
        }).collect();
        let corte = (0..vuelta.len()).min_by_key(|i| (vuelta[*i].lo_usan, *i));
        let mut archivos: Vec<String> = comp.iter().map(|x| dep.ruta(*x).to_string()).collect();
        archivos.sort();
        for x in comp { dentro[*x as usize] = false; }
        let condicional = vuelta.iter().any(|e| e.motivo.as_ref().is_some_and(|m| m.condicion.is_some()));
        r.condicionales += condicional as u32;
        r.lista.push(Ciclo { archivos, aristas, vuelta, corte, condicional });
    }
    // Primero los que existen siempre; dentro, los mas grandes.
    r.lista.sort_by_key(|c| (c.condicional, core::cmp::Reverse(c.archivos.len())));
    r
}
