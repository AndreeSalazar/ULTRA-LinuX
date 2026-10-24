// SPDX-License-Identifier: GPL-2.0-only
//! Editor de codigo (clic derecho en un nodo): estado puro. La plataforma lee y escribe
//! el archivo; aqui se decide que se abre, en que linea y si hay cambios sin guardar.
//!
//! Reglas: solo archivos que el disco listo (nunca rutas inventadas), hasta 2 MiB y solo
//! texto; guardar es siempre un clic o Ctrl+S del usuario, nunca automatico.

use alloc::string::String;

pub const MAX_EDITOR: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Editor {
    /// Ruta relativa del archivo abierto.
    pub id: String,
    pub texto: String,
    pub cargado: bool,
    pub sucio: bool,
    /// Linea a la que saltar; `saltos` sube con cada peticion (aunque sea la misma linea),
    /// asi la plataforma sabe cuando saltar sin tener que cambiar el estado.
    pub ir_a: Option<u32>,
    pub saltos: u64,
    pub aviso: Option<String>,
    /// Primer "cerrar" con cambios sin guardar: el segundo descarta.
    pub cerrar_pedido: bool,
    /// Sube cuando el texto cambia desde fuera (al leerlo): la plataforma rehace su copia.
    pub carga: u64,
}

/// Clases de texto para colorear codigo (el tema decide el color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token { Normal, Clave, Texto, Comentario, Numero, Simbolo }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lenguaje { Rust, C, Python, Shell, Toml, Otro }

pub fn lenguaje(nombre: &str) -> Lenguaje {
    match nombre.rsplit_once('.').map(|(_, x)| x).unwrap_or("") {
        "rs" => Lenguaje::Rust,
        "c" | "h" | "cpp" | "hpp" | "cc" | "js" | "ts" | "wgsl" | "glsl" | "java" | "go" => Lenguaje::C,
        "py" => Lenguaje::Python,
        "sh" | "bash" => Lenguaje::Shell,
        "toml" | "yaml" | "yml" | "ini" | "conf" => Lenguaje::Toml,
        _ if nombre == "Makefile" || nombre == "makefile" => Lenguaje::Shell,
        _ => Lenguaje::Otro,
    }
}

const CLAVES_RUST: &[&str] = &["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "Some", "None", "Ok", "Err"];
const CLAVES_C: &[&str] = &["auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float", "for",
    "goto", "if", "int", "long", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void",
    "volatile", "while", "class", "public", "private", "namespace", "include", "define", "function", "let", "var", "true", "false", "null", "fn"];
const CLAVES_PY: &[&str] = &["and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False",
    "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True",
    "try", "while", "with", "yield", "self"];
const CLAVES_SH: &[&str] = &["if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "function", "in", "local", "export",
    "return", "exit", "set", "readonly"];

/// Colorea una linea: (inicio, fin, token) en bytes, sin huecos ni solapes. Sencillo y
/// rapido a proposito (sin estado entre lineas: un comentario /* */ de varias lineas solo
/// se colorea en su primera linea).
pub fn resaltar(linea: &str, len: Lenguaje) -> alloc::vec::Vec<(usize, usize, Token)> {
    let mut v = alloc::vec::Vec::new();
    let b = linea.as_bytes();
    let (claves, coment): (&[&str], &[&str]) = match len {
        Lenguaje::Rust => (CLAVES_RUST, &["//"]),
        Lenguaje::C => (CLAVES_C, &["//", "/*"]),
        Lenguaje::Python => (CLAVES_PY, &["#"]),
        Lenguaje::Shell => (CLAVES_SH, &["#"]),
        Lenguaje::Toml => (&["true", "false"], &["#"]),
        Lenguaje::Otro => { if !linea.is_empty() { v.push((0, linea.len(), Token::Normal)); } return v; }
    };
    let mut i = 0;
    let empuja = |v: &mut alloc::vec::Vec<(usize, usize, Token)>, a: usize, z: usize, t: Token| {
        if a >= z { return; }
        match v.last_mut() { Some(u) if u.2 == t && u.1 == a => u.1 = z, _ => v.push((a, z, t)) }
    };
    while i < b.len() {
        let resto = &linea[i..];
        if coment.iter().any(|c| resto.starts_with(c)) { empuja(&mut v, i, b.len(), Token::Comentario); break; }
        let c = b[i];
        if c == b'"' || (c == b'\'' && len != Lenguaje::Rust) {
            let mut j = i + 1;
            while j < b.len() && b[j] != c { if b[j] == b'\\' { j += 1; } j += 1; }
            let fin = (j + 1).min(b.len());
            empuja(&mut v, i, fin, Token::Texto);
            i = fin;
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') { j += 1; }
            empuja(&mut v, i, j, Token::Numero);
            i = j;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') { j += 1; }
            let palabra = &linea[i..j];
            let es_macro = len == Lenguaje::Rust && b.get(j) == Some(&b'!');
            let t = if claves.contains(&palabra) || es_macro { Token::Clave } else { Token::Normal };
            empuja(&mut v, i, j, t);
            i = j;
        } else if c.is_ascii_punctuation() {
            empuja(&mut v, i, i + 1, Token::Simbolo);
            i += 1;
        } else {
            // Espacios y UTF-8: hasta el siguiente limite de caracter.
            let ancho = resto.chars().next().map(|ch| ch.len_utf8()).unwrap_or(1);
            empuja(&mut v, i, i + ancho, Token::Normal);
            i += ancho;
        }
    }
    v
}
