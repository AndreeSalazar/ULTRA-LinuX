// SPDX-License-Identifier: GPL-2.0-only
//! Iconos SVG incrustados en el programa (Material y Lucide, ver iconos/AVISO.md).
//! Solo se cargan estos: el programa nunca abre un SVG de fuera, asi que resvg solo
//! procesa archivos conocidos. Se pasan a textura una vez por tamano (egui los guarda).

use eframe::egui::{self, load::Bytes, Color32, Rect};
use std::collections::HashMap;
use std::sync::Arc;

macro_rules! iconos {
    ($($clave:literal),* $(,)?) => {
        &[$(($clave, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/iconos/", $clave, ".svg")) as &[u8])),*]
    };
}

const TODOS: &[(&str, &[u8])] = iconos![
    "material/folder", "material/folder-src", "material/folder-images", "material/folder-audio", "material/folder-video",
    "material/folder-download", "material/folder-docs", "material/folder-config", "material/folder-scripts", "material/folder-tools",
    "material/folder-temp", "material/folder-git", "material/folder-public", "material/folder-resource", "material/folder-desktop",
    "material/folder-project", "material/folder-lib", "material/folder-dist", "material/folder-target", "material/folder-include",
    "material/folder-test", "material/folder-font", "material/folder-home", "material/file", "material/rust", "material/c", "material/h",
    "material/cpp", "material/python", "material/javascript", "material/typescript", "material/json", "material/toml", "material/lua",
    "material/console", "material/css", "material/sass", "material/markdown", "material/pdf", "material/document", "material/image",
    "material/video", "material/audio", "material/zip", "material/exe", "material/dll", "material/settings", "material/log", "material/git",
    "material/svg", "material/html", "material/yaml", "material/xml", "material/table", "material/font", "material/key", "material/lock",
    "material/database", "material/assembly", "material/nix", "material/makefile", "material/docker", "material/readme", "material/license",
    "material/hpp", "material/cmake", "material/go", "material/java", "material/kotlin", "material/zig", "material/csharp", "material/swift",
    "material/ruby", "material/php", "material/haskell", "material/dart", "material/react", "material/react_ts", "material/powershell",
    "material/cuda", "material/verilog", "material/meson", "material/editorconfig", "material/shader", "material/objective-c",
    "material/objective-cpp", "material/clangd", "material/gradle", "material/ocaml", "material/elixir", "material/erlang", "material/scala",
    "material/julia", "material/r", "material/vue", "material/svelte", "material/webassembly", "material/hcl", "material/terraform",
    "material/proto", "material/graphql", "material/diff", "material/certificate",
    "lucide/folder", "lucide/folder-open", "lucide/file", "lucide/file-image", "lucide/file-video-camera", "lucide/file-music", "lucide/file-code",
    "lucide/file-archive", "lucide/file-text", "lucide/file-cog", "lucide/file-terminal", "lucide/file-spreadsheet", "lucide/file-key", "lucide/link",
];

pub struct Iconos { mapa: HashMap<&'static str, Bytes> }

impl Iconos {
    pub fn nuevo() -> Iconos {
        let mapa = TODOS.iter().map(|(clave, bytes)| {
            let bytes = if clave.starts_with("lucide/") {
                // Lucide pinta con currentColor: en blanco, y se tine con el color del tipo.
                Bytes::Shared(Arc::from(String::from_utf8_lossy(bytes).replace("currentColor", "#ffffff").into_bytes().into_boxed_slice()))
            } else {
                Bytes::Static(bytes)
            };
            (*clave, bytes)
        }).collect();
        Iconos { mapa }
    }

    /// Pinta el icono si existe; devuelve false para dibujarlo a mano.
    pub fn pintar(&self, ui: &egui::Ui, clave: &str, rect: Rect, color: Color32) -> bool {
        let Some(bytes) = self.mapa.get(clave) else { return false };
        let mut imagen = egui::Image::from_bytes(format!("bytes://{clave}.svg"), bytes.clone()).fit_to_exact_size(rect.size());
        if clave.starts_with("lucide/") { imagen = imagen.tint(color); }
        imagen.paint_at(ui, rect);
        true
    }

    pub fn cantidad(&self) -> usize { self.mapa.len() }
}

#[cfg(test)]
mod pruebas {
    #[test]
    fn todos_los_iconos_son_svg_y_pequenos() {
        for (clave, bytes) in super::TODOS {
            let texto = std::str::from_utf8(bytes).unwrap();
            assert!(texto.trim_start().starts_with("<svg"), "{clave}");
            assert!(bytes.len() < 16 * 1024, "{clave} pesa demasiado");
            // Sin scripts, enlaces externos ni imagenes incrustadas.
            for prohibido in ["<script", "href=\"http", "xlink:href", "<image", "<foreignObject"] {
                assert!(!texto.contains(prohibido), "{clave} contiene {prohibido}");
            }
        }
    }

    #[test]
    fn el_nucleo_solo_pide_claves_que_existen() {
        use carpetas_nucleo::{iconos::icono, EstiloIconos, Tipo};
        let claves: std::collections::HashSet<&str> = super::TODOS.iter().map(|(c, _)| *c).collect();
        let nombres = ["main.rs", "a.c", "b.h", "x.cpp", "s.py", "j.js", "t.ts", "c.json", "Cargo.toml", "x.lua", "r.sh", "e.css", "e.scss",
            "README", "LEEME.md", "n.md", "d.pdf", "n.txt", "i.svg", "w.html", "y.yml", "x.xml", "t.csv", "f.ttf", "k.pem", "Cargo.lock", "d.db",
            "boot.s", "flake.nix", "a.log", "w.exe", "l.so", "c.ini", "LICENSE", "Makefile", "Dockerfile", ".gitignore", "f.png", "v.mp4", "a.ogg",
            "z.zst", "p.AppImage", "raro.xyz"];
        let tipos = [Tipo::Codigo, Tipo::Imagen, Tipo::Video, Tipo::Audio, Tipo::Comprimido, Tipo::Programa, Tipo::Documento, Tipo::Otro, Tipo::Enlace];
        let carpetas = ["src", "Imágenes", "Música", "Vídeos", "Descargas", "docs", "config", "scripts", "tools", "tmp", ".git", "public", "assets",
            "Escritorio", "Proyectos", "lib", "dist", "target", "include", "tests", "fuentes", "home", "cualquiera"];
        for c in carpetas_nucleo::iconos::claves() { assert!(claves.contains(c), "falta el SVG de {c}"); }
        for estilo in [EstiloIconos::Material, EstiloIconos::Lucide, EstiloIconos::Neon] {
            for n in nombres { for t in tipos { let c = icono(n, t, estilo); assert!(c.is_empty() && estilo == EstiloIconos::Neon || claves.contains(c), "{c} ({n}, {t:?})"); } }
            for n in carpetas { let c = icono(n, Tipo::Carpeta, estilo); assert!(c.is_empty() && estilo == EstiloIconos::Neon || claves.contains(c), "{c}"); }
        }
    }
}
