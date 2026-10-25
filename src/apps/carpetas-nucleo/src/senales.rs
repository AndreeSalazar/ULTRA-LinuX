// SPDX-License-Identifier: GPL-2.0-only
//! Lo que senalan los archivos de un grupo (panel Combinar): las sospechas del museo y los
//! errores del compilador, en el orden en que conviene arreglarlos, cada una con lo que
//! ensena su ficha. Y, si un archivo falla por otro, por donde le llega: la cadena de
//! `#include` hasta la causa probable.
//!
//! Orden: errores del compilador, Seguridad exacta, Seguridad a revisar, Rendimiento,
//! Elegancia; dentro, por archivo y linea. Asi «Siguiente» va primero a lo que rompe.

use crate::dependencias::{Dependencias, Motivo, Rol};
use crate::estado::Estado;
use crate::museo::{eje, Eje, Fuerza};
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Como mucho tantas senales por grupo (el resto se cuenta).
pub const MAX_SENALES: usize = 400;

#[derive(Debug, Clone, PartialEq)]
pub struct Senal {
    pub archivo: String,
    pub linea: u32,
    /// La ficha del museo (None: error o aviso del compilador).
    pub ficha: Option<&'static str>,
    pub eje: Eje,
    /// Lo que dice: el motivo de la sospecha o el mensaje del compilador.
    pub texto: String,
    /// Lo que se vio en la linea (sospechas).
    pub visto: String,
    pub exacta: bool,
    /// Error (no aviso) del compilador.
    pub grave: bool,
}

impl Senal {
    /// La prioridad para arreglar: lo que rompe primero.
    fn orden(&self) -> u8 {
        match (self.ficha, self.grave, self.eje, self.exacta) {
            (None, true, ..) => 0,
            (Some(_), _, Eje::Seguridad, true) => 1,
            (Some(_), _, Eje::Seguridad, false) => 2,
            (None, false, ..) => 3,
            (Some(_), _, Eje::Rendimiento, _) => 4,
            (Some(_), _, Eje::Elegancia, _) => 5,
        }
    }
}

/// Un paso de la cadena hasta la causa: `desde` incluye `hasta` en su linea.
#[derive(Debug, Clone, PartialEq)]
pub struct Eslabon { pub desde: String, pub hasta: String, pub motivo: Option<Motivo> }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Senales {
    pub lista: Vec<Senal>,
    pub total: u32,
    /// Por archivo del grupo: su papel en el fallo conectado y, si falla o arrastra por
    /// otro, la cadena hasta la causa probable mas cercana.
    pub por_que: BTreeMap<String, (Rol, Vec<Eslabon>)>,
}

/// Los archivos de un miembro (archivo o carpeta) que tienen algo que decir.
fn bajo<'a, V>(mapa: &'a BTreeMap<String, V>, id: &'a str) -> impl Iterator<Item = (&'a String, &'a V)> + 'a {
    mapa.range::<str, _>((core::ops::Bound::Included(id), core::ops::Bound::Unbounded))
        .take_while(move |(k, _)| k.starts_with(id))
        .filter(move |(k, _)| k.len() == id.len() || k.as_bytes()[id.len()] == b'/')
}

/// La cadena mas corta de `#include` de un archivo a alguna de las causas.
pub fn cadena(dep: &Dependencias, desde: &str, causas: &BTreeSet<String>) -> Vec<Eslabon> {
    let Some(o) = dep.id(desde) else { return Vec::new() };
    let metas: BTreeSet<u32> = causas.iter().filter_map(|c| dep.id(c)).collect();
    let mut padre: BTreeMap<u32, u32> = BTreeMap::new();
    let mut cola = VecDeque::from([o]);
    let mut vistos: BTreeSet<u32> = BTreeSet::from([o]);
    while let Some(x) = cola.pop_front() {
        if x != o && metas.contains(&x) {
            let mut v = Vec::new();
            let mut y = x;
            while let Some(&p) = padre.get(&y) {
                v.push(Eslabon { desde: dep.ruta(p).to_string(), hasta: dep.ruta(y).to_string(), motivo: dep.motivo(p, y).cloned() });
                y = p;
            }
            v.reverse();
            return v;
        }
        if vistos.len() > crate::dependencias::MAX_ALCANCE { break; }
        for &y in dep.usa_id(x) { if vistos.insert(y) { padre.insert(y, x); cola.push_back(y); } }
    }
    Vec::new()
}

/// Lo que senalan los del grupo, ordenado para arreglar.
pub fn senales(e: &Estado, grupo: &[String]) -> Senales {
    let mut s = Senales::default();
    let an = &e.analisis;
    let mut vistos: BTreeSet<(&str, u32, String)> = BTreeSet::new();
    for id in grupo {
        for (archivo, v) in bajo(&an.sospechas, id) {
            for x in v {
                if !vistos.insert((archivo.as_str(), x.linea, String::from(x.ficha))) { continue; }
                s.lista.push(Senal { archivo: archivo.clone(), linea: x.linea, ficha: Some(x.ficha), eje: eje(x.ficha), texto: x.motivo.clone(),
                                     visto: x.visto.clone(), exacta: x.fuerza == Fuerza::Exacto, grave: false });
            }
        }
        for (archivo, v) in bajo(&e.errores.por_nodo, id) {
            for d in v {
                if !vistos.insert((archivo.as_str(), d.linea, d.mensaje.clone())) { continue; }
                s.lista.push(Senal { archivo: archivo.clone(), linea: d.linea, ficha: None, eje: Eje::Seguridad, texto: d.mensaje.clone(),
                                     visto: String::new(), exacta: true, grave: d.grave });
            }
        }
    }
    s.total = s.lista.len() as u32;
    s.lista.sort_by(|a, b| (a.orden(), &a.archivo, a.linea).cmp(&(b.orden(), &b.archivo, b.linea)));
    s.lista.truncate(MAX_SENALES);
    // Por que falla: los del grupo con papel en el fallo conectado que no son causa.
    let roles = &e.conectado.conexion.roles;
    let causas: BTreeSet<String> = e.conectado.conexion.causas.iter().map(|c| c.ruta.clone()).collect();
    for id in grupo {
        if let Some(&rol) = roles.get(id) {
            let v = if matches!(rol, Rol::Falla | Rol::Arrastra) { cadena(&an.dep, id, &causas) } else { Vec::new() };
            s.por_que.insert(id.clone(), (rol, v));
        }
    }
    s
}
