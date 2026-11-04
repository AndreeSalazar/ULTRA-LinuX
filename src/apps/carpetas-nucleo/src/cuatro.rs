// SPDX-License-Identifier: GPL-2.0-only
//! 4D: la ciudad en el tiempo. Cada archivo que existio, con su vida entera: cuando nacio,
//! cuando murio (si murio), como crecio y encogio en lineas y que commits lo tocaron.
//!
//! Sale de la cronologia (`cronologia`): los eventos de la lectura 1 (nace, muere, se mueve,
//! cambia) siguen a cada archivo aunque cambie de nombre, y la lectura 3 (`--numstat`) da
//! las lineas que gana o pierde en cada commit. Sin sacar ninguna version vieja.
//!
//! Con eso la Ciudad puede mirar cualquier commit: que edificios existian, cuanto median,
//! cuales cambiaban (calor del momento) y cuales ya eran fantasmas. Y una IA puede preguntar
//! como estaba el proyecto en un momento o que cambio entre dos (`comparar`).
//!
//! Limites: las lineas son las de git (un binario no tiene); si la historia no empieza en
//! el primer commit (repositorio superficial), lo anterior no se conoce.
//!
//! Puro (no_std).

use crate::cronologia::{Cronologia, Evento};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// La vida de un archivo (siguiendo sus cambios de nombre).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vida4 {
    /// Su ultimo nombre (el de hoy, o el que tenia al morir).
    pub ruta: String,
    pub nace: u32,
    pub muere: Option<u32>,
    /// (commit, lineas desde ese commit), solo cuando cambia.
    pub lineas: Vec<(u32, u32)>,
    /// Los commits que lo tocaron (nacer, cambiar, moverse), en orden.
    pub cambios: Vec<u32>,
    /// Lo mas que llego a medir.
    pub maximo: u32,
}

impl Vida4 {
    /// Existia en el commit `t`.
    pub fn vivo_en(&self, t: u32) -> bool { self.nace <= t && self.muere.is_none_or(|m| t < m) }

    /// Ya habia muerto en `t`: un fantasma.
    pub fn fantasma_en(&self, t: u32) -> bool { self.muere.is_some_and(|m| m <= t) }

    /// Sus lineas en el commit `t` (0 si aun no tenia).
    pub fn lineas_en(&self, t: u32) -> u32 {
        match self.lineas.partition_point(|(c, _)| *c <= t) { 0 => 0, i => self.lineas[i - 1].1 }
    }

    /// Cuantas veces lo tocaron en los commits `(desde, hasta]`.
    pub fn cambios_entre(&self, desde: Option<u32>, hasta: u32) -> u32 {
        let fin = self.cambios.partition_point(|c| *c <= hasta);
        let inicio = desde.map_or(0, |d| self.cambios.partition_point(|c| *c <= d));
        fin.saturating_sub(inicio) as u32
    }
}

/// Todas las vidas del proyecto.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tiempo4 {
    pub vidas: Vec<Vida4>,
    /// Las vivas al final (hoy), por su ruta.
    pub hoy: BTreeMap<String, u32>,
    pub commits: u32,
    /// Se leyo la lectura 3: hay lineas.
    pub con_lineas: bool,
}

/// Como estaba el proyecto en un commit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Momento {
    pub commit: u32,
    pub vivos: u32,
    pub lineas: u64,
    pub fantasmas: u32,
    /// Vidas que nacieron o murieron en la ventana `(commit - ventana, commit]`.
    pub nacieron: Vec<u32>,
    pub murieron: Vec<u32>,
    /// Las mas tocadas en la ventana (vida, cambios), de mas a menos.
    pub calientes: Vec<(u32, u32)>,
}

/// Lo que cambio entre dos commits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comparacion {
    /// (vida, lineas en `a`, lineas en `b`), de mayor a menor cambio.
    pub crecieron: Vec<(u32, u32, u32)>,
    pub encogieron: Vec<(u32, u32, u32)>,
    pub nacieron: Vec<u32>,
    pub murieron: Vec<u32>,
    pub lineas: (u64, u64),
    pub vivos: (u32, u32),
}

/// Reproduce la cronologia y saca la vida de cada archivo.
pub fn construir(c: &Cronologia) -> Tiempo4 {
    let mut vidas: Vec<Vida4> = Vec::new();
    // Ruta (por numero) -> vida que hoy lleva ese nombre.
    let mut quien: BTreeMap<u32, u32> = BTreeMap::new();
    let nueva = |vidas: &mut Vec<Vida4>, k: u32, i: u32| -> u32 {
        vidas.push(Vida4 { ruta: c.rutas[k as usize].clone(), nace: i, ..Vida4::default() });
        vidas.len() as u32 - 1
    };
    for i in 0..c.commits.len() as u32 {
        // Lo que muere en este commit sale tambien en su --numstat: no se revive.
        let mut murio: alloc::collections::BTreeSet<u32> = alloc::collections::BTreeSet::new();
        for e in &c.eventos[i as usize] {
            match *e {
                Evento::Nace(k) | Evento::Cambia(k) => {
                    let v = match quien.get(&k) { Some(&v) => v, None => { let v = nueva(&mut vidas, k, i); quien.insert(k, v); v } };
                    tocar(&mut vidas[v as usize], i);
                }
                Evento::Muere(k) => {
                    if let Some(v) = quien.remove(&k) { vidas[v as usize].muere = Some(i); }
                    murio.insert(k);
                }
                Evento::Mueve(a, b) => {
                    let v = quien.remove(&a).unwrap_or_else(|| nueva(&mut vidas, a, i));
                    vidas[v as usize].ruta = c.rutas[b as usize].clone();
                    quien.insert(b, v);
                    tocar(&mut vidas[v as usize], i);
                }
            }
        }
        for &(k, d) in c.lineas.get(i as usize).map(Vec::as_slice).unwrap_or(&[]) {
            if murio.contains(&k) && !quien.contains_key(&k) { continue; }
            let v = match quien.get(&k) { Some(&v) => v, None => { let v = nueva(&mut vidas, k, i); quien.insert(k, v); v } };
            let vida = &mut vidas[v as usize];
            let antes = vida.lineas.last().map_or(0, |x| x.1) as i64;
            let ahora = (antes + d as i64).clamp(0, u32::MAX as i64) as u32;
            match vida.lineas.last_mut() { Some(x) if x.0 == i => x.1 = ahora, _ => vida.lineas.push((i, ahora)) }
            vida.maximo = vida.maximo.max(ahora);
        }
    }
    let hoy = quien.values().map(|&v| (vidas[v as usize].ruta.clone(), v)).collect();
    Tiempo4 { vidas, hoy, commits: c.commits.len() as u32, con_lineas: c.lineas.iter().any(|v| !v.is_empty()) }
}

fn tocar(v: &mut Vida4, i: u32) {
    if v.cambios.last() != Some(&i) { v.cambios.push(i); }
}

impl Tiempo4 {
    /// La vida que hoy se llama `ruta`.
    pub fn de(&self, ruta: &str) -> Option<&Vida4> { self.hoy.get(ruta).map(|&v| &self.vidas[v as usize]) }

    /// Las vidas que murieron y cuyo ultimo nombre no lo lleva nadie hoy: los fantasmas que
    /// la Ciudad puede poner en su sitio.
    pub fn muertas(&self) -> impl Iterator<Item = (u32, &Vida4)> {
        self.vidas.iter().enumerate().filter(|(_, v)| v.muere.is_some() && !self.hoy.contains_key(&v.ruta)).map(|(i, v)| (i as u32, v))
    }

    /// Como estaba el proyecto en `t`, con la ventana `(t - ventana, t]` para lo reciente.
    pub fn momento(&self, t: u32, ventana: u32, max: usize) -> Momento {
        let desde = t.checked_sub(ventana.max(1));
        let en = |x: u32| desde.is_none_or(|d| x > d) && x <= t;
        let mut m = Momento { commit: t, ..Momento::default() };
        let mut calientes: Vec<(u32, u32)> = Vec::new();
        for (i, v) in self.vidas.iter().enumerate() {
            let i = i as u32;
            if v.vivo_en(t) {
                m.vivos += 1;
                m.lineas += v.lineas_en(t) as u64;
                let n = v.cambios_entre(desde, t);
                if n > 0 { calientes.push((i, n)); }
            } else if v.fantasma_en(t) {
                m.fantasmas += 1;
            }
            if en(v.nace) { m.nacieron.push(i); }
            if v.muere.is_some_and(en) { m.murieron.push(i); }
        }
        calientes.sort_by_key(|&(i, n)| (core::cmp::Reverse(n), i));
        calientes.truncate(max);
        m.calientes = calientes;
        m
    }

    /// Lo que cambio de `a` a `b` (a <= b).
    pub fn comparar(&self, a: u32, b: u32) -> Comparacion {
        let (a, b) = (a.min(b), a.max(b));
        let mut r = Comparacion::default();
        for (i, v) in self.vidas.iter().enumerate() {
            let i = i as u32;
            let (va, vb) = (v.vivo_en(a), v.vivo_en(b));
            let (la, lb) = (if va { v.lineas_en(a) } else { 0 }, if vb { v.lineas_en(b) } else { 0 });
            r.lineas.0 += la as u64;
            r.lineas.1 += lb as u64;
            r.vivos.0 += va as u32;
            r.vivos.1 += vb as u32;
            match (va, vb) {
                (false, true) => r.nacieron.push(i),
                (true, false) => r.murieron.push(i),
                (true, true) if lb > la => r.crecieron.push((i, la, lb)),
                (true, true) if lb < la => r.encogieron.push((i, la, lb)),
                _ => {}
            }
        }
        r.crecieron.sort_by_key(|&(i, x, y)| (core::cmp::Reverse(y - x), i));
        r.encogieron.sort_by_key(|&(i, x, y)| (core::cmp::Reverse(x - y), i));
        // Lo que nace o muere en medio y no esta en ninguno de los dos extremos no cuenta.
        r
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::cronologia::{ruta_nueva, CAMPO, INICIO};
    use alloc::format;

    fn cabecera(id: &str, n: u64) -> String { format!("{INICIO}{id}{CAMPO}{n}{CAMPO}Ana{CAMPO}paso {id}") }

    /// c0: nace a.rs (10) y b.rs (5). c1: a.rs +20 -5. c2: b.rs se llama c.rs (+1).
    /// c3: muere viejo.txt, que nacio en c1. c4: a.rs -10.
    fn historia() -> Cronologia {
        let mut c = Cronologia::default();
        for l in [cabecera("c0", 1).as_str(), "A\tsrc/a.rs", "A\tsrc/b.rs", cabecera("c1", 2).as_str(), "M\tsrc/a.rs", "A\tviejo.txt",
                  cabecera("c2", 3).as_str(), "R100\tsrc/b.rs\tsrc/c.rs", cabecera("c3", 4).as_str(), "D\tviejo.txt",
                  cabecera("c4", 5).as_str(), "M\tsrc/a.rs"] {
            c.linea_archivos(l);
        }
        for l in [format!("{INICIO}c0"), "10\t0\tsrc/a.rs".into(), "5\t0\tsrc/b.rs".into(), format!("{INICIO}c1"), "20\t5\tsrc/a.rs".into(),
                  "3\t0\tviejo.txt".into(), format!("{INICIO}c2"), "1\t0\tsrc/{b.rs => c.rs}".into(), format!("{INICIO}c3"), "0\t3\tviejo.txt".into(),
                  format!("{INICIO}c4"), "0\t10\tsrc/a.rs".into(), "-\t-\timagen.png".into()] {
            c.linea_numstat(&l);
        }
        c
    }

    #[test]
    fn nombres_nuevos_de_numstat() {
        assert_eq!(ruta_nueva("src/{b.rs => c.rs}"), "src/c.rs");
        assert_eq!(ruta_nueva("viejo.rs => nuevo.rs"), "nuevo.rs");
        assert_eq!(ruta_nueva("src/{ => sub}/x.rs"), "src/sub/x.rs");
        assert_eq!(ruta_nueva("{a => b}/x.rs"), "b/x.rs");
        assert_eq!(ruta_nueva("nada/raro.rs"), "nada/raro.rs");
    }

    #[test]
    fn cada_vida_crece_cambia_de_nombre_y_muere() {
        let t = construir(&historia());
        assert!(t.con_lineas);
        assert_eq!(t.commits, 5);
        let a = t.de("src/a.rs").unwrap();
        assert_eq!((a.lineas_en(0), a.lineas_en(1), a.lineas_en(3), a.lineas_en(4)), (10, 25, 25, 15));
        assert_eq!(a.maximo, 25);
        assert_eq!(a.cambios, [0, 1, 4]);
        let c = t.de("src/c.rs").expect("b.rs sigue viva con su nombre nuevo");
        assert_eq!((c.nace, c.lineas_en(2)), (0, 6), "la vida sigue al cambiar de nombre");
        assert!(t.de("src/b.rs").is_none(), "el nombre viejo ya no es de nadie");
        let muertas: Vec<&str> = t.muertas().map(|(_, v)| v.ruta.as_str()).collect();
        assert_eq!(muertas, ["viejo.txt"]);
        let v = t.muertas().next().unwrap().1;
        assert!(v.vivo_en(2) && !v.vivo_en(3) && v.fantasma_en(3) && !v.fantasma_en(2));
        assert!(!t.vidas.iter().any(|v| v.ruta == "imagen.png" && v.maximo > 0), "un binario no tiene lineas");
    }

    #[test]
    fn el_momento_y_la_comparacion() {
        let t = construir(&historia());
        let m = t.momento(1, 1, 10);
        assert_eq!((m.vivos, m.lineas, m.fantasmas), (3, 25 + 5 + 3, 0));
        assert_eq!(m.nacieron.len(), 1, "viejo.txt nacio en c1");
        assert_eq!(m.calientes.len(), 2, "a.rs y viejo.txt se tocaron en c1");
        let m = t.momento(3, 1, 10);
        assert_eq!((m.vivos, m.fantasmas, m.murieron.len()), (2, 1, 1));
        let k = t.comparar(0, 4);
        assert_eq!(k.vivos, (2, 2));
        assert_eq!(k.lineas, (15, 21));
        assert_eq!(k.crecieron.iter().map(|x| (x.1, x.2)).collect::<Vec<_>>(), [(10, 15), (5, 6)], "de mas a menos");
        assert!(k.nacieron.is_empty() && k.murieron.is_empty(), "viejo.txt nacio y murio en medio");
        let k = t.comparar(1, 3);
        assert_eq!(k.murieron.len(), 1);
    }
}
