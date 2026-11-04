// SPDX-License-Identifier: GPL-2.0-only
//! «Save para SUPER GRAFO»: el proyecto guarda su propia organizacion en `super-grafo/`.
//!
//! - **`orden.json`**: la disposicion (posiciones del grafo, iconos, carpetas desplegadas). Si
//!   la carpeta existe, SUPER GRAFO guarda ahi en vez de en la configuracion del usuario, asi
//!   que el orden viaja con el proyecto (y git lo recuerda).
//! - **Organizar por capas**: cada crate de Rust (una carpeta con `Cargo.toml`) es un nodo del
//!   grafo, colocado por sus dependencias entre crates (de `Cargo.toml`): la base a la
//!   izquierda y lo que lo usa todo a la derecha. Sus carpetas padre quedan desplegadas.
//! - **`MAPA.md`** y **`mapa.json`**: el proyecto en una pagina, para una persona o una IA que
//!   retoma el trabajo (capas, dependencias, guardianes, lo mas grande y lo mas tocado, su
//!   historia y lo ultimo visto en vivo). Siempre con tope.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Un crate del proyecto.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Crate {
    /// Su carpeta, relativa a la raiz.
    pub ruta: String,
    pub nombre: String,
    /// Los crates del proyecto que usa (por nombre).
    pub usa: Vec<String>,
    pub archivos: u32,
    pub lineas: u64,
}

/// El nombre del `[package]` y las dependencias (nombres) de un `Cargo.toml`. Lectura por
/// lineas: `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` y los `[target...]`.
pub fn leer_cargo(texto: &str) -> (Option<String>, Vec<String>) {
    let (mut seccion, mut nombre, mut deps) = (String::new(), None, Vec::new());
    for l in texto.lines().map(str::trim) {
        if l.starts_with('[') { seccion = l.trim_matches(['[', ']']).to_string(); continue; }
        let Some((clave, valor)) = l.split_once('=') else { continue };
        let clave = clave.trim();
        if seccion == "package" && clave == "name" { nombre = Some(valor.trim().trim_matches('"').to_string()); }
        if seccion.ends_with("dependencies") && !clave.starts_with('#') {
            let d = clave.split('.').next().unwrap_or(clave).trim_matches('"');
            if !d.is_empty() { deps.push(d.to_string()); }
        }
    }
    deps.sort();
    deps.dedup();
    (nombre, deps)
}

/// Los crates del proyecto (sin el `Cargo.toml` de la raiz, que suele ser el workspace), con
/// sus archivos `.rs` y sus lineas.
pub fn crates(raiz: &Path, rutas: &[String]) -> Vec<Crate> {
    let carpetas: Vec<&str> = rutas.iter().filter_map(|r| r.strip_suffix("/Cargo.toml")).collect();
    let mut v: Vec<Crate> = Vec::new();
    for c in carpetas {
        let Ok(texto) = std::fs::read_to_string(raiz.join(c).join("Cargo.toml")) else { continue };
        let (Some(nombre), usa) = leer_cargo(&texto) else { continue };
        v.push(Crate { ruta: c.to_string(), nombre, usa, ..Crate::default() });
    }
    let nombres: BTreeSet<String> = v.iter().map(|c| c.nombre.clone()).collect();
    for c in &mut v { c.usa.retain(|d| nombres.contains(d) && *d != c.nombre); }
    // Archivos y lineas: el crate mas hondo que contiene cada .rs.
    for r in rutas.iter().filter(|r| r.ends_with(".rs")) {
        let Some(c) = v.iter_mut().filter(|c| r.starts_with(&format!("{}/", c.ruta))).max_by_key(|c| c.ruta.len()) else { continue };
        c.archivos += 1;
        c.lineas += std::fs::read(raiz.join(r)).map(|b| b.iter().filter(|x| **x == b'\n').count() as u64).unwrap_or(0);
    }
    v.sort_by(|a, b| a.ruta.cmp(&b.ruta));
    v
}

/// La capa de cada crate: 0 si no usa ningun crate del proyecto; si no, uno mas que el mas
/// alto de los que usa. Un ciclo no hace bucle (se corta).
pub fn capas(cs: &[Crate]) -> BTreeMap<String, u32> {
    let por_nombre: BTreeMap<&str, &Crate> = cs.iter().map(|c| (c.nombre.as_str(), c)).collect();
    fn capa<'a>(c: &'a Crate, por: &BTreeMap<&str, &'a Crate>, memo: &mut BTreeMap<&'a str, u32>, camino: &mut BTreeSet<&'a str>) -> u32 {
        if let Some(&n) = memo.get(c.nombre.as_str()) { return n; }
        if !camino.insert(&c.nombre) { return 0; }
        let n = c.usa.iter().filter_map(|d| por.get(d.as_str())).map(|d| capa(d, por, memo, camino) + 1).max().unwrap_or(0);
        camino.remove(c.nombre.as_str());
        memo.insert(&c.nombre, n);
        n
    }
    let mut memo = BTreeMap::new();
    cs.iter().map(|c| (c.ruta.clone(), capa(c, &por_nombre, &mut memo, &mut BTreeSet::new()))).collect()
}

/// El grafo organizado: (posiciones de los nodos, carpetas que hay que desplegar). Cada capa es
/// una columna (de izquierda a derecha); las carpetas padre, en una columna a la izquierda, y
/// debajo de ellas lo demas de arriba del proyecto (`arriba`: documentos, datos, archivos sueltos).
pub fn organizar(cs: &[Crate], capa: &BTreeMap<String, u32>, arriba: &[String]) -> (BTreeMap<String, [f32; 2]>, BTreeSet<String>) {
    const ANCHO: f32 = 340.0;
    const ALTO: f32 = 110.0;
    let mut pos = BTreeMap::new();
    let mut por_capa: BTreeMap<u32, Vec<&Crate>> = BTreeMap::new();
    for c in cs { por_capa.entry(capa.get(&c.ruta).copied().unwrap_or(0)).or_default().push(c); }
    for (n, lista) in &por_capa {
        for (i, c) in lista.iter().enumerate() { pos.insert(c.ruta.clone(), [*n as f32 * ANCHO, i as f32 * ALTO]); }
    }
    let mut padres: BTreeSet<String> = BTreeSet::new();
    for c in cs {
        let mut r = c.ruta.as_str();
        while let Some((p, _)) = r.rsplit_once('/') { padres.insert(p.to_string()); r = p; }
    }
    // Las carpetas padre de arriba primero; despues, lo demas de arriba (sin repetir).
    let columna: Vec<&String> = padres.iter().filter(|p| !p.contains('/')).chain(arriba.iter().filter(|a| !padres.contains(*a))).collect();
    for (i, p) in columna.iter().enumerate() { pos.insert((*p).clone(), [-ANCHO * 1.4, i as f32 * ALTO]); }
    // Las padre mas hondas (si las hay), junto a su crate mas cercano.
    for (i, p) in padres.iter().filter(|p| p.contains('/')).enumerate() { pos.insert(p.clone(), [-ANCHO * 0.7, i as f32 * ALTO]); }
    pos.insert(String::new(), [-ANCHO * 2.6, 0.0]);
    (pos, padres)
}

/// Escribe `super-grafo/orden.json`, conservando lo que ya tuviera (iconos, foco).
pub fn escribir_orden(raiz: &Path, pos: BTreeMap<String, [f32; 2]>, desplegar: BTreeSet<String>) -> Result<(), String> {
    let carpeta = raiz.join(crate::almacen::SAVE);
    std::fs::create_dir_all(&carpeta).map_err(|e| format!("no se pudo crear {}: {e}", carpeta.display()))?;
    let ruta = carpeta.join("orden.json");
    let mut d = crate::almacen::cargar(&ruta);
    d.grafo.extend(pos);
    d.desplegadas.extend(desplegar);
    crate::almacen::guardar(&ruta, &d.a_json()).map_err(|e| format!("no se pudo guardar {}: {e}", ruta.display()))
}

/// Una tabla de Markdown.
pub fn tabla(cabecera: &[&str], filas: &[Vec<String>]) -> String {
    let mut t = format!("| {} |\n|{}|\n", cabecera.join(" | "), cabecera.iter().map(|_| "---").collect::<Vec<_>>().join("|"));
    for f in filas { t += &format!("| {} |\n", f.join(" | ")); }
    t
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn c(ruta: &str, nombre: &str, usa: &[&str]) -> Crate { Crate { ruta: ruta.into(), nombre: nombre.into(), usa: usa.iter().map(|x| x.to_string()).collect(), ..Crate::default() } }

    #[test]
    fn lee_nombre_y_dependencias_de_cargo() {
        let t = "[package]\nname = \"quimera-dom\"\n\n[dependencies]\nquimera-html.workspace = true\nquimera-css = { path = \"../css\" }\nserde = \"1\"\n\n[dev-dependencies]\nquimera-banco = { path = \"x\" }\n";
        let (n, d) = leer_cargo(t);
        assert_eq!(n.as_deref(), Some("quimera-dom"));
        assert_eq!(d, ["quimera-banco", "quimera-css", "quimera-html", "serde"]);
    }

    #[test]
    fn capas_por_dependencias_y_columnas_en_el_grafo() {
        let cs = [c("cimientos/valor", "valor", &[]), c("web/html", "html", &["valor"]), c("web/dom", "dom", &["html", "valor"]), c("navegador/ventana", "ventana", &["dom"])];
        let k = capas(&cs);
        assert_eq!((k["cimientos/valor"], k["web/html"], k["web/dom"], k["navegador/ventana"]), (0, 1, 2, 3));
        let (pos, padres) = organizar(&cs, &k, &["docs".to_string(), "web".to_string(), "Cargo.toml".to_string()]);
        assert_eq!(pos["web/dom"][0], 2.0 * 340.0, "la capa es la columna");
        assert!(pos["cimientos/valor"][0] < pos["navegador/ventana"][0], "la base a la izquierda");
        assert_eq!(padres.into_iter().collect::<Vec<_>>(), ["cimientos", "navegador", "web"]);
        assert!(pos.contains_key(""), "la raiz tambien");
        assert!(pos["docs"][1] > pos["web"][1] && pos["Cargo.toml"][0] == pos["web"][0], "lo demas de arriba, debajo de las padre");
    }

    #[test]
    fn un_ciclo_no_hace_bucle() {
        let k = capas(&[c("a", "a", &["b"]), c("b", "b", &["a"])]);
        assert!(k["a"] <= 2 && k["b"] <= 2);
    }
}
