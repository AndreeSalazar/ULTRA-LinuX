// SPDX-License-Identifier: GPL-2.0-only
//! Donde va cada icono. Logica pura: guardar y cargar el texto JSON lo hace la plataforma.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub const CELDA: [f32; 2] = [112.0, 104.0];
pub const ICONO: [f32; 2] = [96.0, 92.0];
pub const MAX_JSON: usize = 1 << 20;
const MAX_HISTORIAL: usize = 64;
const LIMITE: f32 = 1.0e6;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Disposicion {
    pub posiciones: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    pub rejilla: bool,
    #[serde(default)]
    pub bloqueada: bool,
    /// Posiciones que el usuario fijo a mano en el grafo (ruta relativa -> punto).
    #[serde(default)]
    pub grafo: BTreeMap<String, [f32; 2]>,
    #[serde(skip)]
    historial: Vec<(BTreeMap<String, [f32; 2]>, BTreeMap<String, [f32; 2]>)>,
}

/// Huella FNV-1a de 64 bits: nombre de archivo estable para guardar cada carpeta.
pub fn huella(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Redondeo sin libm (no_std): valido para 0..=LIMITE, que es lo unico que se guarda.
fn redondear(x: f32) -> f32 {
    if x <= 0.0 { 0.0 } else { (x + 0.5) as u32 as f32 }
}

pub fn ajustar(p: [f32; 2]) -> [f32; 2] {
    [redondear(p[0] / CELDA[0]) * CELDA[0], redondear(p[1] / CELDA[1]) * CELDA[1]]
}

fn columnas(ancho: f32) -> usize {
    if ancho >= CELDA[0] { (ancho / CELDA[0]) as usize } else { 1 }
}

fn celda_de(p: [f32; 2]) -> (usize, usize) {
    let a = ajustar(p);
    ((a[0] / CELDA[0]) as usize, (a[1] / CELDA[1]) as usize)
}

fn valido(v: f32) -> bool {
    v.is_finite() && (0.0..=LIMITE).contains(&v)
}

impl Disposicion {
    /// Texto danado, enorme o con numeros absurdos: disposicion vacia o filtrada.
    pub fn de_json(bytes: &[u8]) -> Disposicion {
        if bytes.len() > MAX_JSON {
            return Disposicion::default();
        }
        let mut d: Disposicion = serde_json::from_slice(bytes).unwrap_or_default();
        d.posiciones.retain(|_, p| valido(p[0]) && valido(p[1]));
        d.grafo.retain(|_, p| p.iter().all(|v| v.is_finite() && v.abs() <= LIMITE));
        d
    }

    pub fn a_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn sincronizar(&mut self, nombres: &[String], ancho: f32) -> bool {
        let presentes: BTreeSet<&String> = nombres.iter().collect();
        let antes = self.posiciones.len();
        self.posiciones.retain(|n, _| presentes.contains(n));
        let mut cambio = self.posiciones.len() != antes;
        let mut ocupadas: BTreeSet<(usize, usize)> = self.posiciones.values().map(|p| celda_de(*p)).collect();
        let cols = columnas(ancho);
        let mut siguiente = 0usize;
        for nombre in nombres {
            if self.posiciones.contains_key(nombre) {
                continue;
            }
            while ocupadas.contains(&(siguiente % cols, siguiente / cols)) {
                siguiente += 1;
            }
            let (c, f) = (siguiente % cols, siguiente / cols);
            ocupadas.insert((c, f));
            self.posiciones.insert(nombre.clone(), [c as f32 * CELDA[0], f as f32 * CELDA[1]]);
            cambio = true;
        }
        cambio
    }

    pub fn recordar(&mut self) {
        self.historial.push((self.posiciones.clone(), self.grafo.clone()));
        if self.historial.len() > MAX_HISTORIAL {
            self.historial.remove(0);
        }
    }

    pub fn deshacer(&mut self) -> bool {
        match self.historial.pop() {
            Some((previas, grafo)) => { self.posiciones = previas; self.grafo = grafo; true }
            None => false,
        }
    }

    pub fn puede_deshacer(&self) -> bool { !self.historial.is_empty() }

    pub fn mover(&mut self, nombres: &BTreeSet<String>, delta: [f32; 2]) {
        if self.bloqueada || !delta[0].is_finite() || !delta[1].is_finite() {
            return;
        }
        for n in nombres {
            if let Some(p) = self.posiciones.get_mut(n) {
                p[0] = (p[0] + delta[0]).clamp(0.0, LIMITE);
                p[1] = (p[1] + delta[1]).clamp(0.0, LIMITE);
            }
        }
    }

    /// Mueve nodos del grafo (coordenadas del mundo; pueden ser negativas).
    pub fn mover_grafo(&mut self, ids: &BTreeSet<String>, delta: [f32; 2]) {
        if self.bloqueada || !delta[0].is_finite() || !delta[1].is_finite() { return; }
        for id in ids {
            if let Some(p) = self.grafo.get_mut(id) {
                p[0] = (p[0] + delta[0]).clamp(-LIMITE, LIMITE);
                p[1] = (p[1] + delta[1]).clamp(-LIMITE, LIMITE);
            }
        }
    }

    pub fn soltar(&mut self, nombres: &BTreeSet<String>) {
        if self.rejilla {
            for n in nombres {
                if let Some(p) = self.posiciones.get_mut(n) {
                    *p = ajustar(*p);
                }
            }
        }
    }

    pub fn ordenar(&mut self, orden: &[String], ancho: f32) {
        if self.bloqueada {
            return;
        }
        self.recordar();
        let cols = columnas(ancho);
        for (i, n) in orden.iter().enumerate() {
            self.posiciones.insert(n.clone(), [(i % cols) as f32 * CELDA[0], (i / cols) as f32 * CELDA[1]]);
        }
    }
}
