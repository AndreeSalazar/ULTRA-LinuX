// SPDX-License-Identifier: GPL-2.0-only
//! Tipos de archivo y colores del tema Neon.

use alloc::format;
use alloc::string::String;

pub type Rgb = [u8; 3];

pub const NEGRO: Rgb = [6, 5, 10];
pub const TEXTO: Rgb = [236, 233, 220];
pub const TENUE: Rgb = [161, 158, 174];
pub const CYAN: Rgb = [0, 240, 255];
pub const ROJO: Rgb = [255, 42, 79];
pub const VERDE: Rgb = [61, 255, 154];
pub const VIOLETA: Rgb = [192, 59, 255];
pub const AMARILLO: Rgb = [252, 238, 10];
pub const CORAL: Rgb = [255, 122, 144];
pub const HIELO: Rgb = [90, 216, 255];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tipo { Carpeta, Imagen, Video, Audio, Codigo, Comprimido, Programa, Documento, Enlace, Otro }

impl Tipo {
    pub fn color(self, acento: Rgb) -> Rgb {
        match self {
            Tipo::Carpeta => acento, Tipo::Imagen => VIOLETA, Tipo::Video => ROJO, Tipo::Audio => AMARILLO,
            Tipo::Codigo => CYAN, Tipo::Comprimido => CORAL, Tipo::Programa => VERDE, Tipo::Documento => TEXTO,
            Tipo::Enlace | Tipo::Otro => TENUE,
        }
    }
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

pub fn legible(bytes: u64) -> String {
    for (u, t) in [("GiB", 1u64 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)] {
        if bytes >= t {
            return format!("{:.1} {u}", bytes as f64 / t as f64);
        }
    }
    format!("{bytes} B")
}
