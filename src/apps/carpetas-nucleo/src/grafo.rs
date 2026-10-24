// SPDX-License-Identifier: GPL-2.0-only
//! El modo grafo (como los editores de nodos de Blender y Houdini) y el arbol del
//! explorador. Comparten las carpetas desplegadas: abrir una en un lado la abre en el otro.

use crate::estado::{Entrada, Estado};
use crate::iconos::icono;
use crate::tipos::{legible, Rgb, Tipo};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub const NODO: [f32; 2] = [210.0, 74.0];
pub const MAX_NODOS: usize = 600;

/// Blender: el flujo va de izquierda a derecha. Houdini: de arriba abajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direccion { #[default] Blender, Houdini }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modo { #[default] Iconos, Grafo }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camara { pub x: f32, pub y: f32, pub zoom: f32 }

impl Default for Camara { fn default() -> Self { Camara { x: -40.0, y: -40.0, zoom: 1.0 } } }

#[derive(Debug, Clone, PartialEq)]
pub struct NodoGrafo {
    /// Ruta relativa a la carpeta abierta ("" es la raiz).
    pub id: String,
    pub nombre: String,
    pub tipo: Tipo,
    pub pos: [f32; 2],
    pub tam: [f32; 2],
    pub color: Rgb,
    pub icono: &'static str,
    pub detalle: String,
    /// 0..1: que parte de su carpeta ocupa (lo que leyo el disco); para optimizar espacio.
    pub peso: f32,
    /// None en archivos; Some(abierta) en carpetas.
    pub expandida: Option<bool>,
    pub seleccionado: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cable { pub desde: [f32; 2], pub hasta: [f32; 2], pub color: Rgb }

#[derive(Debug, Clone, PartialEq)]
pub struct FilaArbol {
    pub id: String,
    pub nombre: String,
    pub tipo: Tipo,
    pub nivel: usize,
    pub expandida: Option<bool>,
    pub seleccionado: bool,
    pub icono: &'static str,
    pub color: Rgb,
}

pub fn hijo(padre: &str, nombre: &str) -> String {
    if padre.is_empty() { nombre.to_string() } else { format!("{padre}/{nombre}") }
}

/// Recorre en profundidad lo que se ve: la raiz y lo que cuelga de carpetas desplegadas.
pub fn visibles(e: &Estado) -> Vec<(String, usize, Option<&Entrada>)> {
    let mut salida = Vec::new();
    salida.push((String::new(), 0, None));
    fn bajar<'a>(e: &'a Estado, carpeta: &str, nivel: usize, salida: &mut Vec<(String, usize, Option<&'a Entrada>)>) {
        let Some(hijos) = e.arbol.get(carpeta) else { return };
        for h in hijos {
            if salida.len() >= MAX_NODOS { return; }
            let id = hijo(carpeta, &h.nombre);
            let abre = h.tipo == Tipo::Carpeta && e.expandidas.contains(&id);
            salida.push((id.clone(), nivel, Some(h)));
            if abre { bajar(e, &id, nivel + 1, salida); }
        }
    }
    bajar(e, "", 1, &mut salida);
    salida
}

/// Acomodo de arbol: cada hoja ocupa un hueco; un padre se centra sobre sus hijos.
pub fn acomodar(e: &Estado) -> BTreeMap<String, [f32; 2]> {
    let lista = visibles(e);
    let mut hueco: BTreeMap<String, f32> = BTreeMap::new();
    let mut siguiente = 0.0f32;
    // De abajo arriba: los hijos ya tienen hueco cuando llega el padre.
    let mut hijos_de: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, _, _) in &lista {
        if id.is_empty() { continue; }
        let padre = id.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default();
        hijos_de.entry(padre).or_default().push(id.clone());
    }
    fn colocar(id: &str, hijos_de: &BTreeMap<String, Vec<String>>, hueco: &mut BTreeMap<String, f32>, siguiente: &mut f32) -> f32 {
        let pos = match hijos_de.get(id) {
            Some(hijos) if !hijos.is_empty() => {
                let slots: Vec<f32> = hijos.iter().map(|h| colocar(h, hijos_de, hueco, siguiente)).collect();
                (slots[0] + slots[slots.len() - 1]) / 2.0
            }
            _ => { let s = *siguiente; *siguiente += 1.0; s }
        };
        hueco.insert(id.to_string(), pos);
        pos
    }
    colocar("", &hijos_de, &mut hueco, &mut siguiente);
    let mut pos = BTreeMap::new();
    for (id, nivel, _) in &lista {
        let s = hueco.get(id).copied().unwrap_or(0.0);
        let automatica = match e.ajustes.direccion {
            Direccion::Blender => [*nivel as f32 * (NODO[0] + 90.0), s * (NODO[1] + 22.0)],
            Direccion::Houdini => [s * (NODO[0] + 30.0), *nivel as f32 * (NODO[1] + 80.0)],
        };
        pos.insert(id.clone(), e.disp.grafo.get(id).copied().unwrap_or(automatica));
    }
    pos
}

pub fn nodos_y_cables(e: &Estado, tema: &crate::tema::Tema) -> (Vec<NodoGrafo>, Vec<Cable>) {
    let lista = visibles(e);
    let pos = acomodar(e);
    let mut nodos = Vec::with_capacity(lista.len());
    for (id, _, entrada) in &lista {
        let (nombre, tipo, tam) = match entrada {
            Some(en) => (en.nombre.clone(), en.tipo, en.tam),
            None => (String::from("/"), Tipo::Carpeta, 0),
        };
        let hijos = e.arbol.get(id);
        // Lo que ocupa (con todo lo ya leido debajo) y que parte es de su carpeta padre.
        let propio = match (tipo, hijos) { (Tipo::Carpeta, Some(_)) => peso_leido(e, id), (Tipo::Carpeta, None) => 0, _ => tam };
        let detalle = match (tipo, hijos) {
            (Tipo::Carpeta, Some(h)) => format!("{} elementos · {}", h.len(), legible(propio)),
            (Tipo::Carpeta, None) => String::from("doble clic: desplegar"),
            _ => legible(tam),
        };
        let padre = id.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let total = if id.is_empty() { 0 } else { peso_leido(e, padre) };
        let peso = if total == 0 { 0.0 } else { (propio as f32 / total as f32).min(1.0) };
        nodos.push(NodoGrafo {
            id: id.clone(), nombre: if id.is_empty() { if e.nombre_raiz.is_empty() { String::from("/") } else { e.nombre_raiz.clone() } } else { nombre.clone() }, tipo,
            pos: pos.get(id).copied().unwrap_or([0.0, 0.0]), tam: NODO, color: tema.color_tipo(tipo),
            icono: icono(&nombre, tipo, e.ajustes.iconos), detalle, peso,
            expandida: (tipo == Tipo::Carpeta).then(|| id.is_empty() || e.expandidas.contains(id)),
            seleccionado: e.seleccion.contains(id),
        });
    }
    // Cables del padre al hijo: salida a la derecha (Blender) o abajo (Houdini).
    let mut cables = Vec::new();
    for n in &nodos {
        if n.id.is_empty() { continue; }
        let padre = n.id.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let Some(p) = pos.get(padre) else { continue };
        let (desde, hasta) = match e.ajustes.direccion {
            Direccion::Blender => ([p[0] + NODO[0], p[1] + NODO[1] / 2.0], [n.pos[0], n.pos[1] + NODO[1] / 2.0]),
            Direccion::Houdini => ([p[0] + NODO[0] / 2.0, p[1] + NODO[1]], [n.pos[0] + NODO[0] / 2.0, n.pos[1]]),
        };
        cables.push(Cable { desde, hasta, color: n.color });
    }
    (nodos, cables)
}

/// Bytes de una carpeta contando lo ya leido de sus subcarpetas (hasta 32 niveles).
pub fn peso_leido(e: &Estado, carpeta: &str) -> u64 {
    fn bajar(e: &Estado, carpeta: &str, nivel: usize) -> u64 {
        let Some(lista) = e.arbol.get(carpeta) else { return 0 };
        lista.iter().map(|x| if x.tipo == Tipo::Carpeta && nivel < 32 { bajar(e, &hijo(carpeta, &x.nombre), nivel + 1) } else { x.tam }).fold(0u64, |a, b| a.saturating_add(b))
    }
    bajar(e, carpeta, 0)
}

pub fn filas_arbol(e: &Estado, tema: &crate::tema::Tema) -> Vec<FilaArbol> {
    visibles(e).into_iter().filter(|(id, _, _)| !id.is_empty()).map(|(id, nivel, en)| {
        let en = en.expect("solo la raiz no tiene entrada");
        FilaArbol { nombre: en.nombre.clone(), tipo: en.tipo, nivel: nivel - 1,
                    expandida: (en.tipo == Tipo::Carpeta).then(|| e.expandidas.contains(&id)),
                    seleccionado: e.seleccion.contains(&id), icono: icono(&en.nombre, en.tipo, e.ajustes.iconos),
                    color: tema.color_tipo(en.tipo), id }
    }).collect()
}

/// Rectangulo que encierra todos los nodos (para encuadrar con Inicio, como en Blender).
pub fn limites(e: &Estado) -> ([f32; 2], [f32; 2]) {
    let pos = acomodar(e);
    let mut min = [f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN];
    for p in pos.values() {
        min = [min[0].min(p[0]), min[1].min(p[1])];
        max = [max[0].max(p[0] + NODO[0]), max[1].max(p[1] + NODO[1])];
    }
    if min[0] > max[0] { ([0.0, 0.0], NODO) } else { (min, max) }
}
