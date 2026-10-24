// SPDX-License-Identifier: GPL-2.0-only
//! Botones de play (como en VS Code) y la terminal de abajo. Logica pura (no_std):
//! que nodo se puede arrancar y con que programa; la plataforma lanza el proceso.
//!
//! Reglas de seguridad: solo se arranca con un clic del usuario, nunca solo; los botones
//! usan una lista cerrada de programas (cargo, make, python3, bash) y sin shell; la linea
//! de comandos tampoco pasa por una shell (sin tuberias, comodines ni sustituciones).

use crate::estado::Estado;
use crate::grafo::hijo;
use crate::tipos::Tipo;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub const MAX_LINEAS: usize = 2000;
pub const MAX_LARGO_LINEA: usize = 1000;
pub const MAX_ARGUMENTOS: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Lanzador {
    /// Lo que se ve en el boton y en la terminal ("cargo run").
    pub titulo: String,
    pub programa: &'static str,
    pub args: Vec<String>,
    /// Carpeta de trabajo, relativa a la abierta.
    pub carpeta: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Terminal {
    pub abierta: bool,
    pub lineas: VecDeque<(String, bool)>,
    pub en_curso: Option<String>,
}

impl Terminal {
    pub fn escribir(&mut self, linea: &str, error: bool) {
        let mut l: String = linea.chars().filter(|c| *c == '\t' || !c.is_control()).take(MAX_LARGO_LINEA).collect();
        if linea.chars().count() > MAX_LARGO_LINEA { l.push('…'); }
        self.lineas.push_back((l, error));
        while self.lineas.len() > MAX_LINEAS { self.lineas.pop_front(); }
    }
}

fn contiene(e: &Estado, carpeta: &str, nombre: &str) -> bool {
    e.arbol.get(carpeta).is_some_and(|l| l.iter().any(|x| x.nombre == nombre && x.tipo != Tipo::Carpeta))
}

fn cargo(carpeta: &str) -> Lanzador {
    Lanzador { titulo: String::from("cargo run"), programa: "cargo", args: vec![String::from("run")], carpeta: carpeta.to_string() }
}

/// Que se arranca desde este nodo, si algo. Solo mira lo leido del disco.
pub fn lanzador(e: &Estado, id: &str) -> Option<Lanzador> {
    let (padre, nombre) = id.rsplit_once('/').unwrap_or(("", id));
    let tipo = if id.is_empty() { Tipo::Carpeta } else { e.entrada(id)?.tipo };
    if tipo == Tipo::Carpeta {
        // Una carpeta con Cargo.toml es un proyecto Rust; con Makefile, un proyecto make.
        if contiene(e, id, "Cargo.toml") { return Some(cargo(id)); }
        if contiene(e, id, "Makefile") || contiene(e, id, "makefile") {
            return Some(Lanzador { titulo: String::from("make"), programa: "make", args: vec![], carpeta: id.to_string() });
        }
        return None;
    }
    let ext = nombre.rsplit_once('.').map(|(_, x)| x.to_ascii_lowercase()).unwrap_or_default();
    match (nombre, ext.as_str()) {
        ("Cargo.toml", _) => Some(cargo(padre)),
        ("main.rs", _) => {
            // src/main.rs de un proyecto: se arranca el proyecto (la carpeta de encima de src).
            let (abuelo, carpeta) = padre.rsplit_once('/').unwrap_or(("", padre));
            (carpeta == "src" && contiene(e, abuelo, "Cargo.toml")).then(|| cargo(abuelo))
        }
        ("Makefile" | "makefile", _) => Some(Lanzador { titulo: String::from("make"), programa: "make", args: vec![], carpeta: padre.to_string() }),
        (_, "py") => Some(Lanzador { titulo: format!("python3 {nombre}"), programa: "python3", args: vec![nombre.to_string()], carpeta: padre.to_string() }),
        (_, "sh") => Some(Lanzador { titulo: format!("bash {nombre}"), programa: "bash", args: vec![nombre.to_string()], carpeta: padre.to_string() }),
        _ => None,
    }
}

/// Parte una linea escrita en palabras, sin shell. None si esta vacia o es demasiado larga.
pub fn partir_linea(linea: &str) -> Option<(String, Vec<String>)> {
    let palabras: Vec<String> = linea.split_whitespace().map(|p| p.chars().take(512).collect()).collect();
    if palabras.is_empty() || palabras.len() > MAX_ARGUMENTOS { return None; }
    let mut it = palabras.into_iter();
    let programa = it.next()?;
    Some((programa, it.collect()))
}

/// Ruta de un hijo para la plataforma (misma regla que el grafo).
pub fn ruta(padre: &str, nombre: &str) -> String { hijo(padre, nombre) }
