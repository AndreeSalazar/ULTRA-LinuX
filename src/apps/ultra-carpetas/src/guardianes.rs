// SPDX-License-Identifier: GPL-2.0-only
//! El perfil de los guardianes (src/guardian/produccion/perfil-gcc.sh): la fuente unica de
//! banderas de GCC para el codigo C de usuario. El SUPER GRAFO no copia las banderas: las
//! lee de ahi, y revisa como `ultra_revisar`:
//!
//! 1. `ULTRA_WARN` + `-O0 -fanalyzer` (el analizador de GCC 16 solo ve fugas, NULL y
//!    lecturas sin iniciar en -O0: optimizar los esconde);
//! 2. `ULTRA_WARN` + `-O2` (los avisos que necesitan optimizar).
//!
//! `ULTRA_WARN` lleva `-Werror`: lo que el guardian avisa es un error, y entra como fallo.

use std::path::Path;

/// Donde suele estar el perfil, relativo a la raiz del proyecto.
pub const LUGAR: &str = "src/guardian/produccion/perfil-gcc.sh";

#[derive(Debug, Clone, PartialEq)]
pub struct Perfil {
    /// De donde se leyo (relativo a la raiz).
    pub origen: String,
    /// Las banderas de ULTRA_WARN.
    pub warn: Vec<String>,
}

/// Una bandera que se puede pasar sin riesgo (no ejecuta ni escribe nada).
fn segura(f: &str) -> bool {
    (f.starts_with("-W") && !f.starts_with("-Wl,") && !f.starts_with("-Wa,") && !f.starts_with("-Wp,"))
        || f.starts_with("-std=") || f.starts_with("-D") || f.starts_with("-U")
        || crate::compilacion::semantica_segura(f)
}

/// Saca un array de bash (`NOMBRE=(a b \n c)`) de un texto; sin comentarios.
pub fn array_bash(texto: &str, nombre: &str) -> Option<Vec<String>> {
    let inicio = texto.find(&format!("{nombre}=("))? + nombre.len() + 2;
    let resto = &texto[inicio..];
    let mut v = Vec::new();
    for linea in resto.lines() {
        let l = linea.split('#').next().unwrap_or("");
        let (dentro, fin) = match l.find(')') { Some(p) => (&l[..p], true), None => (l, false) };
        v.extend(dentro.split_whitespace().map(|x| x.trim_matches(|c| c == '"' || c == '\'').to_string()));
        if fin { return Some(v); }
    }
    None
}

/// El perfil de los guardianes de un proyecto (None: no tiene).
pub fn leer(raiz: &Path) -> Option<Perfil> {
    let texto = std::fs::read_to_string(raiz.join(LUGAR)).ok()?;
    let warn: Vec<String> = array_bash(&texto, "ULTRA_WARN")?.into_iter().filter(|f| segura(f)).collect();
    if warn.is_empty() { return None; }
    Some(Perfil { origen: LUGAR.to_string(), warn })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_ultra_warn_del_perfil_real() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let p = leer(&raiz).expect("el perfil de produccion del repositorio");
        assert_eq!(p.origen, LUGAR);
        for f in ["-std=c17", "-Werror", "-Wconversion", "-Wnull-dereference", "-Wtrampolines"] { assert!(p.warn.iter().any(|x| x == f), "{f} en {:?}", p.warn); }
    }

    #[test]
    fn arrays_de_bash_y_banderas_seguras() {
        let t = "X=1\nULTRA_WARN=(-std=c17 -Wall # comentario\n   -Wextra \"-Wformat=2\" -fplugin=mal.so -Wl,--x)\nOTRO=(a)\n";
        assert_eq!(array_bash(t, "ULTRA_WARN").unwrap(), ["-std=c17", "-Wall", "-Wextra", "-Wformat=2", "-fplugin=mal.so", "-Wl,--x"]);
        assert!(!segura("-fplugin=mal.so") && !segura("-Wl,--x") && segura("-Wformat=2"));
        assert!(array_bash(t, "NADA").is_none());
    }
}
