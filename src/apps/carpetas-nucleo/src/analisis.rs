// SPDX-License-Identifier: GPL-2.0-only
//! Lo que manda el hilo de analisis (museo + dependencias) y la conexion de fallos que
//! dibuja el SUPER GRAFO: rojo neon con nucleo blanco en la causa probable, cables rojos
//! hasta todo lo que arrastra, y el resto apagado cuando se aisla.

use crate::dependencias::{aislar, conectar, Conexion, Dependencias, Rol};
use crate::diagnosticos::Errores;
use crate::museo::{eje, Eje, Fuerza, Sospecha};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

/// El resultado del hilo de analisis para todo el proyecto.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResultadoAnalisis {
    /// Rutas con alguna arista, y las aristas (a usa b) como indices en `rutas`.
    pub rutas: Vec<String>,
    pub aristas: Vec<(u32, u32)>,
    /// El motivo de cada arista (la linea del `#include`...), en el mismo orden.
    pub motivos: Vec<crate::dependencias::Motivo>,
    pub sospechas: Vec<(String, Vec<Sospecha>)>,
    /// Archivos leidos (C, C++, ASM, Rust, Python) y bytes.
    pub archivos: u64,
    pub bytes: u64,
    /// Se corto por el presupuesto (bytes o tiempo): no se leyo todo.
    pub truncado: bool,
    pub micros: u64,
    /// Archivos con sospechas que estan en una carpeta marcada con CASOS-MUSEO.txt (codigo
    /// malo a proposito: minas, casos de estudio): se ensenan, pero no son fallos.
    pub estudio: Vec<String>,
}

/// El archivo que marca una carpeta de casos de estudio (codigo malo a proposito).
pub const MARCA_ESTUDIO: &str = "CASOS-MUSEO.txt";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EstadoAnalisis {
    pub dep: Dependencias,
    pub sospechas: BTreeMap<String, Vec<Sospecha>>,
    /// Las de los casos de estudio (a proposito): solo para leer en el panel.
    pub estudio: BTreeMap<String, Vec<Sospecha>>,
    pub total_estudio: u32,
    pub listo: bool,
    pub archivos: u64,
    pub bytes: u64,
    pub truncado: bool,
    pub micros: u64,
    pub total: u32,
    pub version: u64,
}

impl EstadoAnalisis {
    pub fn llegar(&mut self, r: ResultadoAnalisis) {
        self.dep = Dependencias::nuevo(r.rutas, &r.aristas);
        self.dep.poner_motivos(&r.aristas, r.motivos);
        let estudio: BTreeSet<String> = r.estudio.into_iter().collect();
        let (casos, reales): (Vec<_>, Vec<_>) = r.sospechas.into_iter().filter(|(_, v)| !v.is_empty()).partition(|(k, _)| estudio.contains(k));
        self.total = reales.iter().map(|(_, v)| v.len() as u32).sum();
        self.total_estudio = casos.iter().map(|(_, v)| v.len() as u32).sum();
        self.sospechas = reales.into_iter().collect();
        self.estudio = casos.into_iter().collect();
        self.archivos = r.archivos;
        self.bytes = r.bytes;
        self.truncado = r.truncado;
        self.micros = r.micros;
        self.listo = true;
        self.version += 1;
    }

    pub fn de(&self, id: &str) -> &[Sospecha] { self.sospechas.get(id).map(|v| v.as_slice()).unwrap_or(&[]) }

    fn debajo<'a>(&'a self, id: &'a str) -> impl Iterator<Item = (&'a String, &'a Vec<Sospecha>)> + 'a {
        self.sospechas.range::<str, _>((core::ops::Bound::Included(id), core::ops::Bound::Unbounded))
            .take_while(move |(k, _)| k.starts_with(id))
            .filter(move |(k, _)| id.is_empty() || k.len() == id.len() || k.as_bytes()[id.len()] == b'/')
    }

    /// (exactas, revisar) de SEGURIDAD del nodo; en una carpeta, todo lo de dentro.
    pub fn conteo(&self, id: &str) -> (u32, u32) {
        self.debajo(id).flat_map(|(_, v)| v.iter()).filter(|s| eje(s.ficha) == Eje::Seguridad)
            .fold((0, 0), |(e, r), s| if s.fuerza == Fuerza::Exacto { (e + 1, r) } else { (e, r + 1) })
    }

    /// Sospechas del nodo por eje del triangulo: [seguridad, rendimiento, elegancia].
    pub fn conteo_ejes(&self, id: &str) -> [u32; 3] {
        let mut c = [0u32; 3];
        for s in self.debajo(id).flat_map(|(_, v)| v.iter()) { c[eje(s.ficha).indice()] += 1; }
        c
    }

    /// La que mas importa: seguridad exacta, luego seguridad, luego cualquiera.
    pub fn primera<'a>(&'a self, id: &'a str) -> Option<(&'a String, &'a Sospecha)> {
        let todas = || self.debajo(id).flat_map(|(k, v)| v.iter().map(move |s| (k, s)));
        todas().find(|(_, s)| eje(s.ficha) == Eje::Seguridad && s.fuerza == Fuerza::Exacto)
            .or_else(|| todas().find(|(_, s)| eje(s.ficha) == Eje::Seguridad))
            .or_else(|| todas().next())
    }

    /// Archivos con alguna sospecha de seguridad (las unicas que son fallos).
    pub fn con_seguridad(&self) -> impl Iterator<Item = &String> {
        self.sospechas.iter().filter(|(_, v)| v.iter().any(|s| eje(s.ficha) == Eje::Seguridad)).map(|(k, _)| k)
    }

    /// Las sospechas agrupadas por ficha (para el panel del museo), por id de ficha.
    pub fn por_ficha(&self) -> BTreeMap<&'static str, Vec<(&String, &Sospecha)>> { agrupar(&self.sospechas) }
    pub fn estudio_por_ficha(&self) -> BTreeMap<&'static str, Vec<(&String, &Sospecha)>> { agrupar(&self.estudio) }
}

fn agrupar(mapa: &BTreeMap<String, Vec<Sospecha>>) -> BTreeMap<&'static str, Vec<(&String, &Sospecha)>> {
    let mut m: BTreeMap<&'static str, Vec<(&String, &Sospecha)>> = BTreeMap::new();
    for (r, v) in mapa { for s in v { m.entry(s.ficha).or_default().push((r, s)); } }
    m
}

impl EstadoAnalisis {
}

/// Que se esta conectando.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Modo {
    #[default]
    Ninguno,
    /// Los fallos: errores del compilador y sospechas del museo.
    Fallos,
    /// Un archivo, grupo o carpeta aislado: lo que usa y quien lo usa.
    Alrededor(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Conectado {
    pub modo: Modo,
    pub conexion: Conexion,
    /// Cuantos archivos fallan (errores graves + museo).
    pub fallos: u32,
    pub clave: (u64, u64, Option<String>),
}

/// Archivos que fallan: un error grave del compilador o una sospecha de SEGURIDAD. Rendimiento
/// y Elegancia se ensenan, pero no son fallos: una funcion larga no rompe nada.
pub fn fallos(errores: &Errores, analisis: &EstadoAnalisis) -> BTreeSet<String> {
    let mut f: BTreeSet<String> = errores.por_nodo.iter().filter(|(_, v)| v.iter().any(|d| d.grave)).map(|(k, _)| k.clone()).collect();
    f.extend(analisis.con_seguridad().cloned());
    f
}

/// Recalcula la conexion. `centros`: los archivos del nodo aislado (si se aislo uno).
pub fn conectar_estado(errores: &Errores, analisis: &EstadoAnalisis, aislado: Option<(&str, Vec<String>)>) -> (Modo, Conexion, u32) {
    let f = fallos(errores, analisis);
    let n = f.len() as u32;
    match aislado {
        Some((id, centros)) if !centros.is_empty() => {
            let mut c = Conexion::default();
            for centro in &centros {
                let a = aislar(&analisis.dep, centro);
                c.truncada |= a.truncada;
                for (r, rol) in a.roles {
                    let e = c.roles.entry(r).or_insert(rol);
                    if rol > *e { *e = rol; }
                }
                for x in a.aristas { if c.aristas.len() < crate::dependencias::MAX_ARISTAS { c.aristas.push(x); } }
                c.causas.extend(a.causas);
                if c.roles.len() > crate::dependencias::MAX_ALCANCE { c.truncada = true; break; }
            }
            c.aristas.sort();
            c.aristas.dedup();
            (Modo::Alrededor(String::from(id)), c, n)
        }
        _ if f.is_empty() => (Modo::Ninguno, Conexion::default(), 0),
        _ => {
            let propios: BTreeSet<String> = analisis.con_seguridad().cloned().collect();
            (Modo::Fallos, conectar(&analisis.dep, &f, &propios), n)
        }
    }
}

/// El papel mas fuerte de un conjunto (para un nodo que agrupa varios archivos).
pub fn rol_de<'a>(roles: &BTreeMap<String, Rol>, archivos: impl Iterator<Item = &'a str>) -> Option<Rol> {
    archivos.filter_map(|a| roles.get(a).copied()).max()
}
