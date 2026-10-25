// SPDX-License-Identifier: GPL-2.0-only
//! Puente en vivo entre la ventana y el servidor MCP del mismo proyecto.
//!
//! Una carpeta privada por proyecto (`$XDG_RUNTIME_DIR/ultra-carpetas/vivo-<huella>`, o en la
//! cache si no hay runtime; permisos 700):
//!
//! - `ventana.json`: lo que se ve ahora (vista, grupo, seleccion, camino, archivo del
//!   editor...). La ventana lo reescribe al cambiar y cada pocos segundos (latido).
//! - `peticion-*.json`: lo que pide el MCP que se ensene (un grupo, una linea, un camino).
//!   La ventana las aplica y las borra.
//!
//! Solo mueve la vista: nunca escribe en el proyecto ni ejecuta nada.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Una peticion vale como mucho este tiempo (si la ventana no estaba, se descarta).
const PETICION_MAX_S: u64 = 120;

/// La huella de una carpeta (FNV-1a de su ruta canonica): la misma para la ventana y el MCP.
pub fn huella(raiz: &Path) -> Option<u64> {
    let r = raiz.canonicalize().ok()?;
    Some(r.to_string_lossy().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3)))
}

/// La carpeta del puente de un proyecto (se crea con permisos 700).
pub fn carpeta(raiz: &Path) -> Option<PathBuf> {
    let base = std::env::var_os("ULTRA_PUENTE_DIR").map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR").map(|x| PathBuf::from(x).join("ultra-carpetas")))
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(|x| PathBuf::from(x).join("ultra-carpetas")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/ultra-carpetas")))?;
    let d = base.join(format!("vivo-{:016x}", huella(raiz)?));
    #[cfg(unix)]
    { use std::os::unix::fs::DirBuilderExt; std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&d).ok()?; }
    #[cfg(not(unix))]
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn ahora() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

fn escribir(ruta: &Path, v: &Value) -> bool {
    let tmp = ruta.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(v).unwrap_or_default()).is_ok() && std::fs::rename(&tmp, ruta).is_ok()
}

/// La ventana publica lo que se ve.
pub fn publicar(dir: &Path, vista: &Value) -> bool {
    let mut v = vista.clone();
    v["escrito"] = json!(ahora());
    escribir(&dir.join("ventana.json"), &v)
}

/// Lo que se ve en la ventana y hace cuantos segundos se escribio (None: no hay ventana).
pub fn ventana(dir: &Path) -> Option<(Value, u64)> {
    let v: Value = serde_json::from_slice(&std::fs::read(dir.join("ventana.json")).ok()?).ok()?;
    let hace = ahora().saturating_sub(v["escrito"].as_u64().unwrap_or(0));
    Some((v, hace))
}

/// El MCP deja una peticion para la ventana.
pub fn pedir(dir: &Path, p: &Value) -> bool {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut v = p.clone();
    v["pedido"] = json!(ahora());
    escribir(&dir.join(format!("peticion-{n}.json")), &v)
}

/// La ventana recoge (y borra) las peticiones, en orden; las viejas se descartan.
pub fn recoger(dir: &Path) -> Vec<Value> {
    let Ok(lista) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut archivos: Vec<PathBuf> = lista.flatten().map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("peticion-") && n.ends_with(".json"))).collect();
    archivos.sort();
    let mut v = Vec::new();
    for a in archivos.into_iter().take(16) {
        let leida = std::fs::read(&a).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        let _ = std::fs::remove_file(&a);
        if let Some(p) = leida { if ahora().saturating_sub(p["pedido"].as_u64().unwrap_or(0)) <= PETICION_MAX_S { v.push(p); } }
    }
    v
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let dir = std::env::temp_dir().join(format!("ultra-puente-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(ventana(&dir).is_none(), "sin ventana abierta");
        assert!(publicar(&dir, &json!({ "modo": "ciudad", "grupo": ["a.c"] })));
        let (v, hace) = ventana(&dir).unwrap();
        assert_eq!(v["grupo"][0], "a.c");
        assert!(hace <= 1);
        assert!(pedir(&dir, &json!({ "accion": "revelar", "ruta": "a.c", "linea": 3 })));
        assert!(pedir(&dir, &json!({ "accion": "grupo", "rutas": ["b.c"] })));
        let p = recoger(&dir);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0]["accion"], "revelar", "en orden");
        assert!(recoger(&dir).is_empty(), "se borran al recogerlas");
        std::fs::remove_dir_all(&dir).ok();
    }
}
