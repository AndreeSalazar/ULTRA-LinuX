// SPDX-License-Identifier: GPL-2.0-only
//! Historia de git: cuantas veces cambio cada archivo, cuando y cuantas personas lo
//! tocaron. Un archivo GRANDE que CAMBIA MUCHO es un "punto caliente": ahi se concentran
//! el trabajo y los fallos, y es lo primero que conviene mirar para optimizar o partir.
//! (La idea es de Adam Tornhill, "Your Code as a Crime Scene".)
//!
//! Puro (no_std): la plataforma ejecuta `git log` y pasa su salida en texto.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Commits como mucho que se leen (Chromium tiene ~1,5 millones: se miran los ultimos).
pub const MAX_COMMITS: u32 = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cambios {
    pub veces: u32,
    /// Fechas (segundos Unix) del ultimo y del primer cambio visto.
    pub ultimo: u64,
    pub primero: u64,
    pub autores: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Historia {
    pub por_ruta: BTreeMap<String, Cambios>,
    pub commits: u32,
    pub desde: u64,
    pub hasta: u64,
    /// Se corto en MAX_COMMITS (o por tiempo): solo la historia reciente.
    pub truncada: bool,
    pub autores: u32,
}

/// Lee la salida de
/// `git log --no-merges --no-renames --name-only --relative --format=%x01%ct%x02%an`.
/// Cada commit empieza con \x01fecha\x02autor y sigue con sus archivos, uno por linea.
pub fn leer_log(texto: &str, max_commits: u32) -> Historia {
    let mut h = Historia::default();
    let mut autores: BTreeMap<&str, u32> = BTreeMap::new();
    let mut quien: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    let (mut fecha, mut autor) = (0u64, 0u32);
    for linea in texto.lines() {
        if let Some(cab) = linea.strip_prefix('\u{1}') {
            if h.commits >= max_commits { h.truncada = true; break; }
            let (f, a) = cab.split_once('\u{2}').unwrap_or((cab, ""));
            fecha = f.trim().parse().unwrap_or(0);
            let n = autores.len() as u32;
            autor = *autores.entry(a).or_insert(n);
            h.commits += 1;
            if h.desde == 0 || fecha < h.desde { h.desde = fecha; }
            h.hasta = h.hasta.max(fecha);
            continue;
        }
        let ruta = linea.trim_end();
        // Rutas raras (absolutas, con ..) no se aceptan: solo relativas limpias.
        if ruta.is_empty() || h.commits == 0 || ruta.starts_with('/') || ruta.split('/').any(|t| t.is_empty() || t == "..") { continue; }
        let c = h.por_ruta.entry(ruta.to_string()).or_default();
        c.veces += 1;
        c.ultimo = c.ultimo.max(fecha);
        c.primero = if c.primero == 0 { fecha } else { c.primero.min(fecha) };
        quien.entry(ruta).or_default().insert(autor);
    }
    for (ruta, set) in quien { if let Some(c) = h.por_ruta.get_mut(ruta) { c.autores = set.len().min(u16::MAX as usize) as u16; } }
    h.autores = autores.len() as u32;
    h
}

impl Historia {
    /// Cambios de un archivo o de todo lo que hay debajo de una carpeta (bloques, distritos).
    pub fn de(&self, id: &str) -> Cambios {
        if let Some(c) = self.por_ruta.get(id) { return *c; }
        let mut t = Cambios::default();
        for (k, c) in self.por_ruta.range::<str, _>((core::ops::Bound::Excluded(id), core::ops::Bound::Unbounded)) {
            if !k.starts_with(id) { break; }
            if !id.is_empty() && k.as_bytes().get(id.len()) != Some(&b'/') { continue; }
            t.veces += c.veces;
            t.ultimo = t.ultimo.max(c.ultimo);
            t.primero = if t.primero == 0 { c.primero } else { t.primero.min(c.primero) };
            t.autores = t.autores.max(c.autores);
        }
        t
    }
    pub fn max_veces(&self) -> u32 { self.por_ruta.values().map(|c| c.veces).max().unwrap_or(0) }
}

/// Puntos calientes: cambios x tamano. Mandan los cambios; el tamano pesa con su raiz
/// cuarta, asi un archivo enorme que cambio una vez no tapa a uno mediano que cambia todo
/// el rato (5 000 lineas y 1 cambio < 400 lineas y 3 cambios). De mayor a menor.
pub fn puntos_calientes<'a>(h: &Historia, lineas: impl Iterator<Item = (&'a str, u64)>, max: usize) -> Vec<(String, u32, u64, f32)> {
    let mut v: Vec<(String, u32, u64, f32)> = lineas.filter_map(|(r, l)| {
        let c = h.por_ruta.get(r)?;
        Some((r.to_string(), c.veces, l, c.veces as f32 * crate::ciudad::raiz2(crate::ciudad::raiz2(l.max(1) as f32))))
    }).collect();
    v.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(core::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    v.truncate(max);
    v
}

/// "hace 3 dias", "hace 2 meses"... desde `ahora` (segundos Unix).
pub fn hace(fecha: u64, ahora: u64) -> String {
    if fecha == 0 { return String::from("nunca (en lo leido)"); }
    let s = ahora.saturating_sub(fecha);
    let (n, uno, varios) = match s {
        0..=3599 => (s / 60, "minuto", "minutos"),
        3600..=86_399 => (s / 3600, "hora", "horas"),
        86_400..=2_591_999 => (s / 86_400, "dia", "dias"),
        2_592_000..=31_535_999 => (s / 2_592_000, "mes", "meses"),
        _ => (s / 31_536_000, "ano", "anos"),
    };
    alloc::format!("hace {} {}", n.max(1), if n <= 1 { uno } else { varios })
}
