// SPDX-License-Identifier: GPL-2.0-only
//! SARIF que deja el propio proyecto en su raiz (por ejemplo `guardianes.sarif`, que escriben
//! los guardianes de QUIMERA con `--sarif`). Cada analisis lo vuelve a leer y sus hallazgos se
//! marcan en sus archivos, como los diagnosticos de GCC: un `error` pinta de rojo, el resto de
//! amarillo. Lo revisado (con `suppressions`) se ve, pero dice su motivo.
//!
//! Solo se aceptan rutas relativas dentro de la carpeta; como mucho 2 MiB por archivo.

use carpetas_nucleo::diagnosticos::Diagnostico;
use serde_json::Value;
use std::path::{Component, Path};

/// Los archivos SARIF que se buscan en la raiz del proyecto.
pub const ARCHIVOS: &[&str] = &["guardianes.sarif"];

const MAX_BYTES: u64 = 2 * 1024 * 1024;

fn ruta_dentro(uri: &str) -> Option<String> {
    let p = Path::new(uri);
    if uri.is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) { return None; }
    Some(uri.trim_start_matches("./").replace('\\', "/"))
}

/// Los hallazgos de un documento SARIF: (archivo relativo, diagnostico).
pub fn leer(texto: &str) -> Vec<(String, Diagnostico)> {
    let Ok(doc) = serde_json::from_str::<Value>(texto) else { return Vec::new() };
    let mut v = Vec::new();
    for run in doc["runs"].as_array().into_iter().flatten() {
        let herramienta = run["tool"]["driver"]["name"].as_str().unwrap_or("SARIF");
        for r in run["results"].as_array().into_iter().flatten() {
            let loc = &r["locations"][0]["physicalLocation"];
            let Some(archivo) = loc["artifactLocation"]["uri"].as_str().and_then(ruta_dentro) else { continue };
            let regla = r["ruleId"].as_str().unwrap_or("?");
            let mut mensaje = format!("{herramienta} [{regla}]: {}", r["message"]["text"].as_str().unwrap_or(""));
            if let Some(motivo) = r["suppressions"][0]["justification"].as_str() {
                mensaje.push_str(&format!(" (revisado: {motivo})"));
            }
            let linea = loc["region"]["startLine"].as_u64().unwrap_or(1).max(1) as u32;
            let columna = loc["region"]["startColumn"].as_u64().unwrap_or(1).max(1) as u32;
            v.push((archivo, Diagnostico { linea, columna, mensaje, grave: r["level"].as_str() == Some("error") }));
        }
    }
    v
}

/// Todo el SARIF que el proyecto dejo en su raiz.
pub fn del_proyecto(raiz: &Path) -> Vec<(String, Diagnostico)> {
    let mut v = Vec::new();
    for nombre in ARCHIVOS {
        let ruta = raiz.join(nombre);
        if std::fs::metadata(&ruta).is_ok_and(|m| m.is_file() && m.len() <= MAX_BYTES) {
            if let Ok(t) = std::fs::read_to_string(&ruta) { v.extend(leer(&t)); }
        }
    }
    v
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_guardianes_con_revisiones_y_rechaza_rutas_de_fuera() {
        let t = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"Guardianes de QUIMERA"}},"results":[
            {"ruleId":"R-TAMANO","level":"error","message":{"text":"1001 lineas"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"web/dom/src/lib.rs"},"region":{"startLine":1001}}}]},
            {"ruleId":"A-CRATE","level":"note","message":{"text":"grande"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"web/dom/Cargo.toml"}}}],"suppressions":[{"kind":"external","justification":"se parte luego"}]},
            {"ruleId":"X","level":"error","message":{"text":"fuera"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"../otro/a.rs"}}}]},
            {"ruleId":"Y","level":"error","message":{"text":"sin lugar"}}]}]}"#;
        let v = leer(t);
        assert_eq!(v.len(), 2, "lo de fuera y lo que no tiene lugar no entra");
        assert_eq!(v[0].0, "web/dom/src/lib.rs");
        assert!(v[0].1.grave && v[0].1.linea == 1001);
        assert!(!v[1].1.grave);
        assert!(v[1].1.mensaje.contains("(revisado: se parte luego)"), "{}", v[1].1.mensaje);
        assert!(leer("no es json").is_empty());
    }
}
