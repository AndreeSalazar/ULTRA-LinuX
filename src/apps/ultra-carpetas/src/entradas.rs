// SPDX-License-Identifier: GPL-2.0-only
//! Lee una carpeta sin seguir enlaces y sin tocar nada.

use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_ENTRADAS: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tipo { Carpeta, Imagen, Video, Audio, Codigo, Comprimido, Programa, Documento, Enlace, Otro }

#[derive(Debug, Clone)]
pub struct Entrada {
    pub nombre: String,
    pub ruta: PathBuf,
    pub tipo: Tipo,
    pub tam: u64,
}

pub fn tipo_por_extension(nombre: &str) -> Tipo {
    let ext = nombre.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" | "avif" => Tipo::Imagen,
        "mp4" | "mkv" | "webm" | "mov" | "avi" => Tipo::Video,
        "mp3" | "ogg" | "opus" | "flac" | "wav" | "m4a" => Tipo::Audio,
        "rs" | "c" | "h" | "cpp" | "hpp" | "py" | "sh" | "lua" | "js" | "ts" | "toml" | "json" | "jsonc" | "yuck" | "scss" | "css" | "s" | "asm" => Tipo::Codigo,
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "bz2" => Tipo::Comprimido,
        "exe" | "appimage" | "so" | "bin" => Tipo::Programa,
        "pdf" | "txt" | "md" | "odt" | "docx" | "epub" | "log" | "csv" => Tipo::Documento,
        _ => Tipo::Otro,
    }
}

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
        salida.push(Entrada { nombre, ruta: e.path(), tipo, tam: if meta.is_file() { meta.len() } else { 0 } });
    }
    salida.sort_by(|a, b| (a.tipo != Tipo::Carpeta).cmp(&(b.tipo != Tipo::Carpeta)).then_with(|| a.nombre.to_lowercase().cmp(&b.nombre.to_lowercase())));
    Ok(salida)
}

pub fn legible(bytes: u64) -> String {
    let unidades = [("GiB", 1u64 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)];
    for (u, t) in unidades {
        if bytes >= t {
            return format!("{:.1} {u}", bytes as f64 / t as f64);
        }
    }
    format!("{bytes} B")
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
