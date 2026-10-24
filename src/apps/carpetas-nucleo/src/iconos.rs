// SPDX-License-Identifier: GPL-2.0-only
//! Que icono lleva cada carpeta o archivo segun el estilo elegido. Devuelve una clave
//! ("material/rust", "lucide/file-code"); la plataforma tiene los SVG. Neon: dibujado.

use crate::tipos::Tipo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstiloIconos { #[default] Neon, Material, Lucide }

impl EstiloIconos {
    pub fn siguiente(self) -> Self {
        match self { EstiloIconos::Neon => EstiloIconos::Material, EstiloIconos::Material => EstiloIconos::Lucide, EstiloIconos::Lucide => EstiloIconos::Neon }
    }
    pub fn nombre(self) -> &'static str {
        match self { EstiloIconos::Neon => "Neon (dibujados)", EstiloIconos::Material => "Material (VS Code)", EstiloIconos::Lucide => "Lucide (lineas)" }
    }
}

fn extension(nombre: &str) -> &str {
    nombre.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
}

fn igual(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }

fn carpeta_material(nombre: &str) -> &'static str {
    let n = nombre.trim_start_matches('.');
    let es = |opciones: &[&str]| opciones.iter().any(|o| igual(n, o));
    if es(&["src", "source", "fuente", "fuentes-src"]) { "material/folder-src" }
    else if es(&["Imágenes", "Imagenes", "Pictures", "images", "img", "capturas", "wallpapers", "fondos"]) { "material/folder-images" }
    else if es(&["Música", "Musica", "Music", "audio", "sonidos", "sounds"]) { "material/folder-audio" }
    else if es(&["Vídeos", "Videos", "video", "videos"]) { "material/folder-video" }
    else if es(&["Descargas", "Downloads"]) { "material/folder-download" }
    else if es(&["docs", "doc", "Documentos", "Documents", "documentacion"]) { "material/folder-docs" }
    else if es(&["config", "configuracion", "settings", "etc"]) { "material/folder-config" }
    else if es(&["scripts", "bin"]) { "material/folder-scripts" }
    else if es(&["tools", "herramientas", "utils"]) { "material/folder-tools" }
    else if es(&["tmp", "temp", "cache"]) { "material/folder-temp" }
    else if es(&["git", "github"]) { "material/folder-git" }
    else if es(&["public", "www"]) { "material/folder-public" }
    else if es(&["assets", "recursos", "resources", "res"]) { "material/folder-resource" }
    else if es(&["Escritorio", "Desktop"]) { "material/folder-desktop" }
    else if es(&["Proyectos", "Projects", "proyectos", "apps"]) { "material/folder-project" }
    else if es(&["lib", "libs", "vendor"]) { "material/folder-lib" }
    else if es(&["dist", "build", "out"]) { "material/folder-dist" }
    else if es(&["target"]) { "material/folder-target" }
    else if es(&["include", "includes"]) { "material/folder-include" }
    else if es(&["test", "tests", "pruebas", "spec"]) { "material/folder-test" }
    else if es(&["fonts", "fuentes"]) { "material/folder-font" }
    else if es(&["home", "ultra"]) { "material/folder-home" }
    else { "material/folder" }
}

fn archivo_material(nombre: &str, tipo: Tipo) -> &'static str {
    let especiales: &[(&str, &str)] = &[("README", "readme"), ("README.md", "readme"), ("LEEME.md", "readme"), ("LICENSE", "license"),
        ("COPYING", "license"), ("Makefile", "makefile"), ("Dockerfile", "docker"), (".gitignore", "git"), (".gitattributes", "git"),
        ("Cargo.lock", "lock"), ("flake.nix", "nix")];
    for (n, icono) in especiales {
        if igual(nombre, n) { return clave_material(icono); }
    }
    let ext = extension(nombre).to_ascii_lowercase();
    let porext: &[(&str, &str)] = &[("rs", "rust"), ("c", "c"), ("h", "h"), ("cpp", "cpp"), ("cc", "cpp"), ("hpp", "cpp"), ("py", "python"),
        ("js", "javascript"), ("ts", "typescript"), ("json", "json"), ("jsonc", "json"), ("toml", "toml"), ("lua", "lua"), ("sh", "console"),
        ("bash", "console"), ("css", "css"), ("scss", "sass"), ("md", "markdown"), ("pdf", "pdf"), ("txt", "document"), ("svg", "svg"),
        ("html", "html"), ("yml", "yaml"), ("yaml", "yaml"), ("xml", "xml"), ("csv", "table"), ("ttf", "font"), ("otf", "font"),
        ("pem", "key"), ("key", "key"), ("lock", "lock"), ("db", "database"), ("sqlite", "database"), ("s", "assembly"), ("asm", "assembly"),
        ("nix", "nix"), ("log", "log"), ("exe", "exe"), ("dll", "dll"), ("so", "dll"), ("ini", "settings"), ("conf", "settings"), ("desktop", "settings")];
    for (e, icono) in porext {
        if ext == *e { return clave_material(icono); }
    }
    match tipo {
        Tipo::Imagen => "material/image", Tipo::Video => "material/video", Tipo::Audio => "material/audio",
        Tipo::Comprimido => "material/zip", Tipo::Programa => "material/exe", Tipo::Documento => "material/document",
        Tipo::Codigo => "material/console", _ => "material/file",
    }
}

/// Las claves salen siempre de esta tabla: nunca de texto del usuario.
fn clave_material(icono: &str) -> &'static str {
    const TODAS: &[&str] = &["material/readme", "material/license", "material/makefile", "material/docker", "material/git", "material/lock",
        "material/nix", "material/rust", "material/c", "material/h", "material/cpp", "material/python", "material/javascript", "material/typescript",
        "material/json", "material/toml", "material/lua", "material/console", "material/css", "material/sass", "material/markdown", "material/pdf",
        "material/document", "material/svg", "material/html", "material/yaml", "material/xml", "material/table", "material/font", "material/key",
        "material/database", "material/assembly", "material/log", "material/exe", "material/dll", "material/settings"];
    TODAS.iter().copied().find(|c| &c[9..] == icono).unwrap_or("material/file")
}

fn lucide(nombre: &str, tipo: Tipo) -> &'static str {
    let ext = extension(nombre).to_ascii_lowercase();
    match tipo {
        Tipo::Carpeta => "lucide/folder",
        Tipo::Imagen => "lucide/file-image",
        Tipo::Video => "lucide/file-video-camera",
        Tipo::Audio => "lucide/file-music",
        Tipo::Comprimido => "lucide/file-archive",
        Tipo::Programa => "lucide/file-cog",
        Tipo::Enlace => "lucide/link",
        Tipo::Codigo if ext == "sh" || ext == "bash" => "lucide/file-terminal",
        Tipo::Codigo => "lucide/file-code",
        Tipo::Documento if ext == "csv" => "lucide/file-spreadsheet",
        Tipo::Documento => "lucide/file-text",
        Tipo::Otro if ext == "pem" || ext == "key" => "lucide/file-key",
        Tipo::Otro => "lucide/file",
    }
}

/// Clave del icono; vacia = dibujarlo con formas (estilo Neon).
pub fn icono(nombre: &str, tipo: Tipo, estilo: EstiloIconos) -> &'static str {
    match estilo {
        EstiloIconos::Neon => "",
        EstiloIconos::Material if tipo == Tipo::Carpeta => carpeta_material(nombre),
        EstiloIconos::Material if tipo == Tipo::Enlace => "material/file",
        EstiloIconos::Material => archivo_material(nombre, tipo),
        EstiloIconos::Lucide => lucide(nombre, tipo),
    }
}
