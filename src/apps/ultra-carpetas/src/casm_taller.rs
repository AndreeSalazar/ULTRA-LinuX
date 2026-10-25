// SPDX-License-Identifier: GPL-2.0-only
//! El taller CASM en el disco: encuentra la carpeta de CASM de la carpeta abierta, lee
//! sus fuentes (para las katanas) y sus compilaciones (`build/CASM/run-*`), y busca la
//! compilacion verificada de un `.casm` para el mapa linea -> bytes.
//!
//! Solo lee: archivos regulares (nunca enlaces), con tope de tamano. No compila nada: eso
//! lo hace el boton de play con `compilar.sh`, cuando el usuario lo pulsa.

use carpetas_nucleo::casm::{compilacion, katanas, mapa, MapaCasm, Taller};
use std::path::{Path, PathBuf};

const MAX_ARCHIVO: u64 = 1 << 20;
const MAX_COMPILACIONES: usize = 30;
const MAX_BUSCAR_MAPA: usize = 100;

/// Lee un archivo regular (no enlace) de hasta MAX_ARCHIVO bytes.
fn leer(p: &Path) -> Option<String> {
    let m = std::fs::symlink_metadata(p).ok()?;
    if !m.is_file() || m.len() > MAX_ARCHIVO { return None; }
    std::fs::read_to_string(p).ok()
}

fn es_archivo(p: &Path) -> bool { std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file()) }

/// La carpeta de CASM: la abierta o su `CASM/`. (ruta absoluta, ruta relativa)
pub fn carpeta_casm(raiz: &Path) -> Option<(PathBuf, String)> {
    for rel in ["", "CASM"] {
        let d = if rel.is_empty() { raiz.to_path_buf() } else { raiz.join(rel) };
        if !std::fs::symlink_metadata(&d).is_ok_and(|m| m.is_dir()) { continue; }
        if ["compilar.sh", "verify.py", "src/casm.h", "src/frontend.c"].iter().all(|f| es_archivo(&d.join(f))) { return Some((d, rel.to_string())); }
    }
    None
}

/// Las compilaciones, la mas nueva primero (los nombres llevan fecha y hora).
fn runs(casm: &Path, max: usize) -> Vec<PathBuf> {
    let build = casm.parent().unwrap_or(casm).join("build/CASM");
    let Ok(l) = std::fs::read_dir(&build) else { return Vec::new() };
    let mut v: Vec<PathBuf> = l.flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("run-")) && e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path()).collect();
    v.sort();
    v.reverse();
    v.truncate(max);
    v
}

pub fn explorar(raiz: &Path) -> Option<Taller> {
    let (casm, rel) = carpeta_casm(raiz)?;
    let nombres = ["src/main.c", "src/frontend.c", "src/emitter.c", "verify.py", "test.py"];
    let archivos: Vec<(String, String)> = nombres.iter()
        .filter_map(|n| leer(&casm.join(n)).map(|t| (if rel.is_empty() { n.to_string() } else { format!("{rel}/{n}") }, t)))
        .collect();
    let refs: Vec<(&str, &str)> = archivos.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let compilaciones = runs(&casm, MAX_COMPILACIONES).iter().map(|r| {
        let nombre = r.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        compilacion(&nombre, &leer(&r.join("origen.txt")).unwrap_or_default(), leer(&r.join("verified.json")).as_deref(),
                    &leer(&r.join("registro.txt")).unwrap_or_default(), es_archivo(&r.join("INICIO.X86")))
    }).collect();
    let toolchain = leer(&casm.parent().unwrap_or(&casm).join("build/CASM/host/toolchain.txt")).unwrap_or_default();
    Some(Taller { carpeta: rel, katanas: katanas(&refs), compilaciones, toolchain })
}

/// El mapa de un `.casm`: la compilacion verificada mas nueva cuya fuente congelada es
/// identica al texto. None si no hay ninguna (no se ensenan bytes de otra version).
pub fn mapa_para(raiz: &Path, texto: &str) -> Option<Result<MapaCasm, String>> {
    let (casm, _) = carpeta_casm(raiz)?;
    for r in runs(&casm, MAX_BUSCAR_MAPA) {
        if !es_archivo(&r.join("INICIO.X86")) { continue; }
        if leer(&r.join("source.casm")).as_deref() != Some(texto) { continue; }
        let (Some(ir), Some(hex)) = (leer(&r.join("program.ir.json")), leer(&r.join("program.hex"))) else { continue };
        let nombre = r.file_name().and_then(|n| n.to_str()).unwrap_or("");
        return Some(mapa(texto, &ir, &hex, nombre));
    }
    None
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn taller_de_este_repositorio() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let t = explorar(&raiz).expect("hay CASM/ en el repositorio");
        assert_eq!(t.carpeta, "CASM");
        assert!(t.katanas.iter().any(|k| k.regla == "NAME-UNDEFINED" && k.sitios.iter().any(|(a, _)| a == "CASM/src/frontend.c")));
        // El mapa solo aparece con una compilacion de ESTE texto exacto.
        let hola = std::fs::read_to_string(raiz.join("CASM/examples/hello.casm")).unwrap();
        if t.compilaciones.iter().any(|c| c.verde) {
            let m = mapa_para(&raiz, &hola).expect("compilacion del ejemplo").unwrap();
            assert_eq!(m.cuerpo, 40);
        }
        assert!(mapa_para(&raiz, &format!("{hola}\n// otro")).is_none(), "otro texto: sin bytes viejos");
    }
}
