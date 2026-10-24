// SPDX-License-Identifier: GPL-2.0-only
//! Lee una carpeta sin seguir enlaces y sin tocar nada (parte de Linux; el nucleo no sabe de discos).

use std::fs;
use std::path::Path;

pub const MAX_ENTRADAS: usize = 5000;

pub use carpetas_nucleo::{legible, tipo_por_extension, Entrada, Tipo};

/// Entradas de la carpeta, ordenadas: carpetas primero y luego por nombre.
pub fn listar(carpeta: &Path, ocultos: bool) -> std::io::Result<Vec<Entrada>> {
    let mut salida = Vec::new();
    for e in fs::read_dir(carpeta)?.take(MAX_ENTRADAS) {
        let Ok(e) = e else { continue };
        let nombre = e.file_name().to_string_lossy().into_owned();
        if !ocultos && nombre.starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(e.path()) else { continue };
        let tipo = if meta.file_type().is_symlink() {
            // Un enlace a carpeta se abre como carpeta; el destino se resuelve al abrir.
            if fs::metadata(e.path()).map(|m| m.is_dir()).unwrap_or(false) { Tipo::Carpeta } else { Tipo::Enlace }
        } else if meta.is_dir() {
            Tipo::Carpeta
        } else {
            tipo_por_extension(&nombre)
        };
        salida.push(Entrada { nombre, tipo, tam: if meta.is_file() { meta.len() } else { 0 } });
    }
    salida.sort_by(|a, b| (a.tipo != Tipo::Carpeta).cmp(&(b.tipo != Tipo::Carpeta)).then_with(|| a.nombre.to_lowercase().cmp(&b.nombre.to_lowercase())));
    Ok(salida)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lista_ordena_y_oculta() {
        let base = std::env::temp_dir().join(format!("ultra-carpetas-lista-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("zeta")).unwrap();
        fs::write(base.join("Alfa.png"), b"x").unwrap();
        fs::write(base.join("beta.rs"), b"fn main(){}").unwrap();
        fs::write(base.join(".oculto"), b"").unwrap();
        std::os::unix::fs::symlink(base.join("zeta"), base.join("atajo")).unwrap();
        std::os::unix::fs::symlink("/no/existe", base.join("roto")).unwrap();
        let l = listar(&base, false).unwrap();
        let nombres: Vec<_> = l.iter().map(|e| e.nombre.as_str()).collect();
        assert_eq!(nombres, ["atajo", "zeta", "Alfa.png", "beta.rs", "roto"]);
        assert_eq!(l[2].tipo, Tipo::Imagen);
        assert_eq!(l[3].tipo, Tipo::Codigo);
        assert_eq!(l[4].tipo, Tipo::Enlace);
        assert_eq!(listar(&base, true).unwrap().len(), 6);
        assert_eq!(legible(1536), "1.5 KiB");
        let _ = fs::remove_dir_all(&base);
    }
}
