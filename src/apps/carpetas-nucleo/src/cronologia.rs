// SPDX-License-Identifier: GPL-2.0-only
//! La cronologia: como se construyo el proyecto, commit a commit, desde el primero.
//!
//! Sin sacar ninguna version vieja del proyecto (con Chromium serian anos de trabajo). Dos
//! lecturas de git, que la plataforma pasa linea a linea (en flujo: nunca entera):
//!
//! 1. **Archivos** (`git log --reverse --name-status -M`): cuando nace, se mueve o muere
//!    cada archivo, y quien y por que (el asunto del commit).
//! 2. **Referencias** (`git log --reverse -p -U0 -G'include|import|mod|use'`): solo los
//!    commits que tocan una linea de `#include` (o `mod`, `use`, `import`); de cada linea
//!    que entra o sale se sacan sus referencias con el mismo lector que el mapa de hoy.
//!
//! Con eso se **reproduce** la historia: en cada corte se resuelve el mapa de ese momento
//! (con las reglas de hoy) y se cuentan archivos, dependencias y ciclos. Al final, lo
//! reconstruido se compara con lo que hay hoy: si no coincide, se dice cuanto.
//!
//! Limites (se dicen en cada respuesta): lo que un `#include` no escribe (macros, rutas
//! generadas) no se ve, igual que hoy; los `#if` de una linea suelta no se conocen; un
//! `use` de Rust partido en varias lineas se ve a medias; con ramas que se juntan, el
//! orden es el de las fechas de git.
//!
//! Puro (no_std). Rutas y textos se guardan una vez y despues van por numero.

use crate::ciclos::ciclos;
use crate::dependencias::{referencias, resolver, ClaseRef, Dependencias, Referencia, Rutas};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Un commit (sin los merges).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit { pub id: String, pub fecha: u64, pub autor: u32, pub asunto: String }

/// Lo que le pasa a un archivo en un commit (rutas por numero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evento { Nace(u32), Muere(u32), Mueve(u32, u32), Cambia(u32) }

/// Una referencia que entra (`suma`) o sale de un archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CambioRef { pub ruta: u32, pub suma: bool, pub clase: u8, pub texto: u32 }

/// Las dos lecturas juntas.
#[derive(Debug, Clone, Default)]
pub struct Cronologia {
    pub commits: Vec<Commit>,
    pub autores: Vec<String>,
    pub rutas: Vec<String>,
    pub textos: Vec<String>,
    /// Por commit, en el mismo orden.
    pub eventos: Vec<Vec<Evento>>,
    pub refs: Vec<Vec<CambioRef>>,
    /// Commits de la segunda lectura que no estaban en la primera (no deberia haber).
    pub sueltos: u32,
    ids_rutas: BTreeMap<String, u32>,
    ids_textos: BTreeMap<String, u32>,
    ids_autores: BTreeMap<String, u32>,
    ids_commits: BTreeMap<String, u32>,
    /// Lectura 2 en curso: commit y archivo actuales.
    actual: Option<u32>,
    archivo: Option<u32>,
}

fn clase_a(c: ClaseRef) -> u8 { match c { ClaseRef::Comillas => 0, ClaseRef::Angulos => 1, ClaseRef::ModRust => 2, ClaseRef::UsoRust => 3, ClaseRef::Python(n) => 4u8.saturating_add(n) } }
fn clase_de(b: u8) -> ClaseRef { match b { 0 => ClaseRef::Comillas, 1 => ClaseRef::Angulos, 2 => ClaseRef::ModRust, 3 => ClaseRef::UsoRust, n => ClaseRef::Python(n - 4) } }

fn numero(tabla: &mut Vec<String>, ids: &mut BTreeMap<String, u32>, s: &str) -> u32 {
    if let Some(&k) = ids.get(s) { return k; }
    let k = tabla.len() as u32;
    tabla.push(s.to_string());
    ids.insert(s.to_string(), k);
    k
}

/// El separador de la cabecera de cada commit (`--format=%x01%h%x02%ct%x02%an%x02%s`).
pub const INICIO: char = '\u{1}';
pub const CAMPO: char = '\u{2}';

impl Cronologia {
    fn ruta(&mut self, s: &str) -> u32 { numero(&mut self.rutas, &mut self.ids_rutas, s) }

    /// Una linea de la lectura 1 (`--format=%x01%h%x02%ct%x02%an%x02%s --name-status -M`).
    pub fn linea_archivos(&mut self, l: &str) {
        if let Some(c) = l.strip_prefix(INICIO) {
            let mut p = c.splitn(4, CAMPO);
            let (Some(id), Some(fecha), Some(autor)) = (p.next(), p.next(), p.next()) else { return };
            let asunto = p.next().unwrap_or("").to_string();
            let autor = numero(&mut self.autores, &mut self.ids_autores, autor);
            self.ids_commits.insert(id.to_string(), self.commits.len() as u32);
            self.commits.push(Commit { id: id.to_string(), fecha: fecha.trim().parse().unwrap_or(0), autor, asunto });
            self.eventos.push(Vec::new());
            self.refs.push(Vec::new());
            return;
        }
        if self.eventos.is_empty() { return; }
        let mut p = l.split('\t');
        let (Some(estado), Some(a)) = (p.next(), p.next()) else { return };
        let ev = match estado.as_bytes().first() {
            Some(b'A') | Some(b'C') => { let b = p.next().unwrap_or(a); Evento::Nace(self.ruta(b)) }
            Some(b'D') => Evento::Muere(self.ruta(a)),
            Some(b'R') => { let Some(b) = p.next() else { return }; let (x, y) = (self.ruta(a), self.ruta(b)); Evento::Mueve(x, y) }
            Some(b'M') | Some(b'T') => Evento::Cambia(self.ruta(a)),
            _ => return,
        };
        if let Some(v) = self.eventos.last_mut() { v.push(ev); }
    }

    /// Una linea de la lectura 2 (`--format=%x01%h%x02%ct -p -U0`): la cabecera del
    /// commit, `+++ b/ruta` (el archivo) y las lineas `+`/`-` (de ellas, sus referencias).
    pub fn linea_refs(&mut self, l: &str) {
        if let Some(c) = l.strip_prefix(INICIO) {
            let id = c.split(CAMPO).next().unwrap_or("");
            self.actual = self.ids_commits.get(id).copied();
            if self.actual.is_none() { self.sueltos += 1; }
            self.archivo = None;
            return;
        }
        let Some(k) = self.actual else { return };
        if let Some(r) = l.strip_prefix("+++ ") {
            self.archivo = r.strip_prefix("b/").map(|r| self.ruta(r));
            return;
        }
        if l.starts_with("--- ") || l.starts_with("diff --git") || l.starts_with("@@") { return; }
        let (suma, texto) = match l.as_bytes().first() { Some(b'+') => (true, &l[1..]), Some(b'-') => (false, &l[1..]), _ => return };
        let Some(ruta) = self.archivo else { return };
        let nombre = self.rutas[ruta as usize].clone();
        for r in referencias(&nombre, texto) {
            let t = numero(&mut self.textos, &mut self.ids_textos, &r.texto);
            self.refs[k as usize].push(CambioRef { ruta, suma, clase: clase_a(r.clase), texto: t });
        }
    }
}

/// Las rutas vivas en un momento, para resolver como hoy.
struct Vivas<'a> { todas: BTreeSet<&'a str>, nombres: BTreeMap<&'a str, Vec<&'a str>> }

impl<'a> Vivas<'a> {
    fn de(c: &'a Cronologia, vivos: &BTreeSet<u32>) -> Vivas<'a> {
        let mut v = Vivas { todas: BTreeSet::new(), nombres: BTreeMap::new() };
        for &k in vivos {
            let r = c.rutas[k as usize].as_str();
            v.todas.insert(r);
            v.nombres.entry(r.rsplit('/').next().unwrap_or(r)).or_default().push(r);
        }
        v
    }
}

impl Rutas for Vivas<'_> {
    fn existe(&self, ruta: &str) -> bool { self.todas.contains(ruta) }
    fn por_nombre(&self, nombre: &str) -> Vec<&str> { self.nombres.get(nombre).cloned().unwrap_or_default() }
}

/// Como estaba el proyecto en un corte.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Foto {
    pub commit: u32,
    pub archivos: u32,
    /// Archivos con referencias que se siguen (C, C++, ASM, Rust, Python).
    pub codigo: u32,
    pub dependencias: u32,
    pub ciclos: u32,
    pub archivos_en_ciclos: u32,
}

/// Lo que se sabe de un archivo al final de la reproduccion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vida {
    /// El commit en que nacio (siguiendo sus cambios de nombre).
    pub nace: u32,
    pub cambios: u32,
    /// Sus nombres anteriores, con el commit del cambio.
    pub antes: Vec<(u32, u32)>,
    pub muere: Option<u32>,
}

/// El estado al reproducir.
#[derive(Debug, Clone, Default)]
pub struct Reproduccion {
    pub vivos: BTreeSet<u32>,
    /// Por archivo: sus referencias (clase, texto) y cuantas veces estan escritas.
    refs: BTreeMap<u32, BTreeMap<(u8, u32), i32>>,
    pub vidas: BTreeMap<u32, Vida>,
    /// Cuando entro por ultima vez cada referencia de cada archivo (archivo, texto).
    pub entra: BTreeMap<(u32, u32), u32>,
    /// Cuando nacio cada carpeta (la primera vez que tuvo un archivo).
    pub carpetas: BTreeMap<String, u32>,
    pub fotos: Vec<Foto>,
}

impl Reproduccion {
    fn nace(&mut self, c: &Cronologia, k: u32, commit: u32) {
        self.vivos.insert(k);
        self.vidas.insert(k, Vida { nace: commit, ..Vida::default() });
        let r = c.rutas[k as usize].as_str();
        let mut fin = 0;
        while let Some(p) = r[fin..].find('/') {
            fin += p;
            self.carpetas.entry(r[..fin].to_string()).or_insert(commit);
            fin += 1;
        }
    }

    fn aplicar(&mut self, c: &Cronologia, i: u32) {
        for e in &c.eventos[i as usize] {
            match *e {
                Evento::Nace(k) => self.nace(c, k, i),
                Evento::Cambia(k) => {
                    if !self.vivos.contains(&k) { self.nace(c, k, i); }
                    if let Some(v) = self.vidas.get_mut(&k) { v.cambios += 1; }
                }
                Evento::Muere(k) => {
                    self.vivos.remove(&k);
                    self.refs.remove(&k);
                    if let Some(v) = self.vidas.get_mut(&k) { v.muere = Some(i); }
                }
                Evento::Mueve(a, b) => {
                    self.vivos.remove(&a);
                    self.vivos.insert(b);
                    if let Some(r) = self.refs.remove(&a) { self.refs.insert(b, r); }
                    let mut vida = self.vidas.remove(&a).unwrap_or(Vida { nace: i, ..Vida::default() });
                    vida.antes.push((a, i));
                    vida.cambios += 1;
                    self.vidas.insert(b, vida);
                    let movidas: Vec<(u32, u32)> = self.entra.range((a, 0)..=(a, u32::MAX)).map(|(&(_, t), &cm)| (t, cm)).collect();
                    for (t, cm) in movidas { self.entra.remove(&(a, t)); self.entra.insert((b, t), cm); }
                    let r = c.rutas[b as usize].as_str();
                    let mut fin = 0;
                    while let Some(p) = r[fin..].find('/') { fin += p; self.carpetas.entry(r[..fin].to_string()).or_insert(i); fin += 1; }
                }
            }
        }
        for x in &c.refs[i as usize] {
            if !self.vivos.contains(&x.ruta) { continue; }
            let n = self.refs.entry(x.ruta).or_default().entry((x.clase, x.texto)).or_insert(0);
            let antes = *n;
            *n += if x.suma { 1 } else { -1 };
            if *n < 0 { *n = 0; }
            if x.suma && antes == 0 { self.entra.insert((x.ruta, x.texto), i); }
        }
    }

    /// El mapa de este momento, resuelto con las reglas de hoy: (dependencias, rutas de
    /// cada arista con el texto que la escribe).
    pub fn mapa(&self, c: &Cronologia) -> (Dependencias, Vec<(u32, u32, u32)>) {
        let vivas = Vivas::de(c, &self.vivos);
        let mut nombres: Vec<String> = Vec::new();
        let mut ids: BTreeMap<u32, u32> = BTreeMap::new();
        let mut id = |k: u32, nombres: &mut Vec<String>| *ids.entry(k).or_insert_with(|| { nombres.push(c.rutas[k as usize].clone()); nombres.len() as u32 - 1 });
        let mut aristas = Vec::new();
        let mut con_texto = Vec::new();
        for (&k, refs) in &self.refs {
            let desde = c.rutas[k as usize].as_str();
            for (&(clase, t), &n) in refs {
                if n <= 0 { continue; }
                let r = Referencia { texto: c.textos[t as usize].clone(), linea: 0, clase: clase_de(clase), condicion: None };
                let Some(destino) = resolver(desde, &r, &vivas) else { continue };
                let Some(&kb) = c.ids_rutas.get(&destino) else { continue };
                let (a, b) = (id(k, &mut nombres), id(kb, &mut nombres));
                aristas.push((a, b));
                con_texto.push((k, kb, t));
            }
        }
        aristas.sort_unstable();
        aristas.dedup();
        (Dependencias::nuevo(nombres, &aristas), con_texto)
    }

    fn foto(&mut self, c: &Cronologia, i: u32) {
        let (dep, _) = self.mapa(c);
        let ci = ciclos(&dep);
        let codigo = self.vivos.iter().filter(|k| crate::dependencias::sigue(&c.rutas[**k as usize])).count() as u32;
        self.fotos.push(Foto { commit: i, archivos: self.vivos.len() as u32, codigo, dependencias: dep.aristas as u32, ciclos: ci.total, archivos_en_ciclos: ci.archivos });
    }
}

/// Los commits donde se corta: `cortes` repartidos por numero de commits (el ultimo siempre).
pub fn marcas(n: usize, cortes: usize) -> BTreeSet<usize> {
    if n == 0 { return BTreeSet::new(); }
    let cortes = cortes.clamp(1, n);
    (1..=cortes).map(|k| (k * n).div_ceil(cortes) - 1).collect()
}

/// Reproduce toda la historia y saca `cortes` fotos repartidas (la ultima, al final).
pub fn reproducir(c: &Cronologia, cortes: usize) -> Reproduccion {
    let mut r = Reproduccion::default();
    let m = marcas(c.commits.len(), cortes);
    for i in 0..c.commits.len() {
        r.aplicar(c, i as u32);
        if m.contains(&i) { r.foto(c, i as u32); }
    }
    r
}

/// Reproduce y llama a `al_corte` en cada corte (para medir algo propio, como lo que vive
/// dentro de una carpeta). Sin fotos del mapa: mas barato.
pub fn recorrer(c: &Cronologia, cortes: usize, mut al_corte: impl FnMut(&Reproduccion, u32)) {
    let mut r = Reproduccion::default();
    let m = marcas(c.commits.len(), cortes);
    for i in 0..c.commits.len() {
        r.aplicar(c, i as u32);
        if m.contains(&i) { al_corte(&r, i as u32); }
    }
}

impl Cronologia {
    pub fn ruta_de(&self, k: u32) -> &str { &self.rutas[k as usize] }
    pub fn texto_de(&self, k: u32) -> &str { &self.textos[k as usize] }
    pub fn id_ruta(&self, r: &str) -> Option<u32> { self.ids_rutas.get(r).copied() }
    pub fn id_texto(&self, t: &str) -> Option<u32> { self.ids_textos.get(t).copied() }
}
