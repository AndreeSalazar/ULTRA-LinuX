// SPDX-License-Identifier: GPL-2.0-only
//! Notas por nodo: que es cada carpeta o archivo y como se comporta, escritas por una persona o
//! una IA, en `super-grafo/NOTAS.md` del proyecto (junto al save).
//!
//! El formato es Markdown legible a mano: una seccion por nodo, con su ruta como titulo.
//!
//! ```text
//! ## web/estilo
//! La cascada: para cada elemento, el valor de cada propiedad...
//! ```
//!
//! Lo que va antes de la primera seccion es la cabecera y se conserva. SUPER GRAFO ensena la
//! nota al pasar el raton por el nodo (grafo y Ciudad) y la consulta `nota` la lee o la escribe.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::RwLock;

pub const ARCHIVO: &str = "NOTAS.md";
/// Una nota no pasa de esto (el resto se corta al leer).
const MAX_NOTA: usize = 4000;

/// Las notas del proyecto abierto (para los tooltips, sin pasarlas por toda la vista).
static NOTAS: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

/// (cabecera, ruta -> nota) de un NOTAS.md.
pub fn leer(texto: &str) -> (String, BTreeMap<String, String>) {
    let mut cabecera = String::new();
    let mut notas: BTreeMap<String, String> = BTreeMap::new();
    let mut actual: Option<String> = None;
    for l in texto.lines() {
        if let Some(t) = l.strip_prefix("## ") {
            let ruta = t.trim().trim_matches('`').trim_matches('/').to_string();
            notas.entry(ruta.clone()).or_default();
            actual = Some(ruta);
            continue;
        }
        match &actual {
            Some(r) => { let n = notas.entry(r.clone()).or_default(); if n.len() < MAX_NOTA { n.push_str(l); n.push('\n'); } }
            None => { cabecera.push_str(l); cabecera.push('\n'); }
        }
    }
    for n in notas.values_mut() { *n = n.trim().to_string(); }
    (cabecera, notas)
}

/// El NOTAS.md de una cabecera y sus notas (en orden de ruta).
pub fn escribir(cabecera: &str, notas: &BTreeMap<String, String>) -> String {
    let mut t = cabecera.trim_end().to_string();
    if !t.is_empty() { t.push_str("\n\n"); }
    for (r, n) in notas.iter().filter(|(_, n)| !n.trim().is_empty()) { t += &format!("## {r}\n\n{}\n\n", n.trim()); }
    t.trim_end().to_string() + "\n"
}

fn ruta_archivo(raiz: &Path) -> std::path::PathBuf { raiz.join(crate::almacen::SAVE).join(ARCHIVO) }

/// Las notas de un proyecto (vacio si no tiene).
pub fn del_proyecto(raiz: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(ruta_archivo(raiz)).map(|t| leer(&t).1).unwrap_or_default()
}

/// Carga las notas del proyecto abierto para los tooltips.
pub fn cargar(raiz: &Path) {
    if let Ok(mut n) = NOTAS.write() { *n = del_proyecto(raiz); }
}

/// La nota de un nodo del proyecto abierto.
pub fn de(ruta: &str) -> Option<String> { NOTAS.read().ok()?.get(ruta.trim_matches('/')).cloned() }

/// Escribe (o borra, con texto vacio) la nota de `ruta`, conservando las demas y la cabecera.
pub fn poner(raiz: &Path, ruta: &str, texto: &str) -> Result<(), String> {
    let archivo = ruta_archivo(raiz);
    let (mut cabecera, mut notas) = std::fs::read_to_string(&archivo).map(|t| leer(&t)).unwrap_or_default();
    if cabecera.trim().is_empty() {
        cabecera = "# Notas de SUPER GRAFO\n\nQue es cada parte del proyecto y como se comporta. Una seccion por nodo (su ruta como titulo); SUPER GRAFO la ensena al pasar el raton por el nodo.".into();
    }
    let ruta = ruta.trim_matches('/').to_string();
    if texto.trim().is_empty() { notas.remove(&ruta); } else { notas.insert(ruta, texto.chars().take(MAX_NOTA).collect()); }
    std::fs::create_dir_all(archivo.parent().ok_or("sin carpeta")?).map_err(|e| e.to_string())?;
    std::fs::write(&archivo, escribir(&cabecera, &notas)).map_err(|e| format!("{}: {e}", archivo.display()))?;
    cargar(raiz);
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_y_escribe_secciones_por_ruta() {
        let t = "# Notas\n\nCabecera.\n\n## `web/estilo`\n\nLa cascada.\nSegunda linea.\n\n## red/red/\nLa red.\n";
        let (cab, n) = leer(t);
        assert!(cab.contains("Cabecera."));
        assert_eq!(n["web/estilo"], "La cascada.\nSegunda linea.");
        assert_eq!(n["red/red"], "La red.");
        let otra = escribir(&cab, &n);
        assert_eq!(leer(&otra).1, n, "ida y vuelta");
    }

    #[test]
    fn poner_conserva_lo_demas() {
        let dir = std::env::temp_dir().join(format!("sg-notas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        poner(&dir, "web/html", "El analizador.").unwrap();
        poner(&dir, "red/red", "La red.").unwrap();
        poner(&dir, "web/html", "El analizador de HTML5.").unwrap();
        let n = del_proyecto(&dir);
        assert_eq!((n["web/html"].as_str(), n["red/red"].as_str()), ("El analizador de HTML5.", "La red."));
        poner(&dir, "red/red", "").unwrap();
        assert!(!del_proyecto(&dir).contains_key("red/red"), "texto vacio = borrar");
        assert_eq!(de("web/html").as_deref(), Some("El analizador de HTML5."), "los tooltips la ven");
        std::fs::remove_dir_all(&dir).ok();
    }
}
