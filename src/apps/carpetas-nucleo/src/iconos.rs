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

/// Una fila por lenguaje o formato: extensiones, icono de Material y si es un **logo
/// reconocible** (Rust, C, C++...). Los logos se ven tambien en el estilo Neon, porque
/// un engranaje de Rust o una C azul se reconocen de un vistazo en el explorador.
const POR_EXTENSION: &[(&[&str], &str, bool)] = &[
    (&["rs"], "material/rust", true),
    (&["c"], "material/c", true),
    (&["h"], "material/h", true),
    (&["cpp", "cc", "cxx", "c++", "cppm", "ixx"], "material/cpp", true),
    (&["hpp", "hh", "hxx", "h++", "inl", "ipp", "tpp"], "material/hpp", true),
    (&["cu", "cuh"], "material/cuda", true),
    (&["m"], "material/objective-c", true),
    (&["mm"], "material/objective-cpp", true),
    (&["s", "asm", "nasm"], "material/assembly", true),
    // CASM (lenguaje propio que emite x86-64 exacto) y el .hex escrito a mano.
    (&["casm", "hex"], "material/assembly", true),
    (&["zig"], "material/zig", true),
    (&["go"], "material/go", true),
    (&["py", "pyi", "pyw"], "material/python", true),
    (&["java"], "material/java", true),
    (&["kt", "kts"], "material/kotlin", true),
    (&["cs"], "material/csharp", true),
    (&["swift"], "material/swift", true),
    (&["rb"], "material/ruby", true),
    (&["php"], "material/php", true),
    (&["hs"], "material/haskell", true),
    (&["ml", "mli"], "material/ocaml", true),
    (&["ex", "exs"], "material/elixir", true),
    (&["erl", "hrl"], "material/erlang", true),
    (&["scala", "sc"], "material/scala", true),
    (&["jl"], "material/julia", true),
    (&["r"], "material/r", true),
    (&["dart"], "material/dart", true),
    (&["js", "mjs", "cjs"], "material/javascript", true),
    (&["ts", "mts", "cts"], "material/typescript", true),
    (&["jsx"], "material/react", true),
    (&["tsx"], "material/react_ts", true),
    (&["vue"], "material/vue", true),
    (&["svelte"], "material/svelte", true),
    (&["lua"], "material/lua", true),
    (&["sh", "bash", "zsh", "fish"], "material/console", true),
    (&["ps1"], "material/powershell", true),
    (&["v", "sv", "svh", "vh"], "material/verilog", true),
    (&["wasm", "wat"], "material/webassembly", true),
    (&["glsl", "vert", "frag", "comp", "geom", "tesc", "tese", "wgsl", "hlsl", "spv"], "material/shader", true),
    (&["cmake"], "material/cmake", true),
    (&["gradle"], "material/gradle", true),
    (&["tf", "tfvars"], "material/terraform", true),
    (&["hcl"], "material/hcl", true),
    (&["proto"], "material/proto", true),
    (&["graphql", "gql"], "material/graphql", true),
    (&["nix"], "material/nix", true),
    (&["sql"], "material/database", true),
    (&["html", "htm"], "material/html", true),
    (&["css"], "material/css", true),
    (&["scss", "sass"], "material/sass", true),
    (&["json", "jsonc"], "material/json", true),
    (&["toml"], "material/toml", true),
    (&["yml", "yaml"], "material/yaml", true),
    (&["xml"], "material/xml", true),
    (&["md", "markdown"], "material/markdown", true),
    (&["diff", "patch"], "material/diff", true),
    (&["pdf"], "material/pdf", false),
    (&["txt"], "material/document", false),
    (&["svg"], "material/svg", false),
    (&["csv", "tsv"], "material/table", false),
    (&["ttf", "otf", "woff", "woff2"], "material/font", false),
    (&["pem", "key"], "material/key", false),
    (&["crt", "cer"], "material/certificate", false),
    (&["lock"], "material/lock", false),
    (&["db", "sqlite"], "material/database", false),
    (&["log"], "material/log", false),
    (&["exe"], "material/exe", false),
    (&["dll", "so"], "material/dll", false),
    (&["ini", "conf", "cfg", "desktop"], "material/settings", false),
];

/// Nombres exactos que mandan sobre la extension (CMakeLists.txt no es un .txt cualquiera).
const POR_NOMBRE: &[(&str, &str, bool)] = &[
    ("Makefile", "material/makefile", true), ("GNUmakefile", "material/makefile", true),
    ("CMakeLists.txt", "material/cmake", true), ("meson.build", "material/meson", true), ("meson_options.txt", "material/meson", true),
    ("Dockerfile", "material/docker", true), (".gitignore", "material/git", true), (".gitattributes", "material/git", true),
    (".gitmodules", "material/git", true), ("flake.nix", "material/nix", true), (".editorconfig", "material/editorconfig", true),
    (".clangd", "material/clangd", true), (".clang-format", "material/clangd", true), (".clang-tidy", "material/clangd", true),
    ("compile_commands.json", "material/clangd", true),
    ("README", "material/readme", false), ("README.md", "material/readme", false), ("LEEME.md", "material/readme", false),
    ("LICENSE", "material/license", false), ("LICENSE.md", "material/license", false), ("COPYING", "material/license", false),
    ("Cargo.lock", "material/lock", false),
];

/// (clave de Material, es logo) por nombre o extension; None si no se conoce.
fn conocido(nombre: &str) -> Option<(&'static str, bool)> {
    if let Some((_, c, l)) = POR_NOMBRE.iter().find(|(n, _, _)| igual(nombre, n)) { return Some((c, *l)); }
    // ".rs" o ".bashrc" son archivos ocultos sin extension, no codigo.
    if !nombre.trim_start_matches('.').contains('.') { return None; }
    let ext = extension(nombre);
    POR_EXTENSION.iter().find(|(exts, _, _)| exts.iter().any(|e| igual(ext, e))).map(|(_, c, l)| (*c, *l))
}

/// Nombres exactos que son codigo o build (Makefile, CMakeLists.txt...).
pub fn nombre_de_codigo(nombre: &str) -> bool { POR_NOMBRE.iter().any(|(n, _, logo)| *logo && igual(nombre, n)) }

/// Es un archivo de codigo segun la tabla (para colorear y para el tipo).
pub fn es_codigo(nombre: &str) -> bool { conocido(nombre).is_some_and(|(_, logo)| logo) }

fn archivo_material(nombre: &str, tipo: Tipo) -> &'static str {
    if let Some((c, _)) = conocido(nombre) { return c; }
    match tipo {
        Tipo::Imagen => "material/image", Tipo::Video => "material/video", Tipo::Audio => "material/audio",
        Tipo::Comprimido => "material/zip", Tipo::Programa => "material/exe", Tipo::Documento => "material/document",
        Tipo::Codigo => "material/console", _ => "material/file",
    }
}

/// Todas las claves que puede pedir el nucleo (la plataforma comprueba que las tiene).
pub fn claves() -> impl Iterator<Item = &'static str> {
    POR_EXTENSION.iter().map(|x| x.1).chain(POR_NOMBRE.iter().map(|x| x.1))
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

/// Clave del icono; vacia = dibujarlo con formas (estilo Neon para lo que no es codigo).
pub fn icono(nombre: &str, tipo: Tipo, estilo: EstiloIconos) -> &'static str {
    match estilo {
        // Neon: formas dibujadas, salvo los logos de lenguaje, que se reconocen solos.
        EstiloIconos::Neon if tipo == Tipo::Carpeta || tipo == Tipo::Enlace => "",
        EstiloIconos::Neon => match conocido(nombre) { Some((c, true)) => c, _ => "" },
        EstiloIconos::Material if tipo == Tipo::Carpeta => carpeta_material(nombre),
        EstiloIconos::Material if tipo == Tipo::Enlace => "material/file",
        EstiloIconos::Material => archivo_material(nombre, tipo),
        EstiloIconos::Lucide => lucide(nombre, tipo),
    }
}
