// SPDX-License-Identifier: GPL-2.0-only
//! Editor de codigo (clic derecho en un nodo): estado puro. La plataforma lee y escribe
//! el archivo; aqui se decide que se abre, en que linea y si hay cambios sin guardar.
//!
//! Reglas: solo archivos que el disco listo (nunca rutas inventadas), hasta 2 MiB y solo
//! texto; guardar es siempre un clic o Ctrl+S del usuario, nunca automatico.

use alloc::string::String;
use alloc::vec::Vec;

pub const MAX_EDITOR: u64 = 2 * 1024 * 1024;
/// Pestanas abiertas a la vez. Al pasar, se recicla la mas vieja sin cambios.
pub const MAX_PESTANAS: usize = 12;
/// Coincidencias que se marcan como mucho (buscar "e" en 2 MiB no se come la memoria).
pub const MAX_HALLAZGOS: usize = 10_000;
pub const MAX_PATRON: usize = 256;

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
    /// O un tramo a seleccionar (bytes): una coincidencia de la busqueda. No roba el foco.
    pub rango: Option<(usize, usize)>,
    pub saltos: u64,
    pub aviso: Option<String>,
    /// Primer "cerrar" con cambios sin guardar: el segundo descarta.
    pub cerrar_pedido: bool,
    /// Sube cuando el texto cambia desde el nucleo (leer, reemplazar): la plataforma rehace su copia.
    pub carga: u64,
    /// Huella (largo + FNV) de lo que hay en disco: si el texto vuelve a ser igual
    /// (deshacer hasta el principio), deja de estar "sin guardar".
    pub en_disco: (usize, u64),
    /// Huella del texto que se mando a guardar (para marcarlo cuando llegue el Guardado).
    guardando: Option<(usize, u64)>,
}

/// Largo + FNV-1a del texto: barato y suficiente para saber si volvio a lo de disco.
pub fn huella(t: &str) -> (usize, u64) {
    (t.len(), t.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3)))
}

impl Editor {
    pub fn nombre(&self) -> &str { self.id.rsplit('/').next().unwrap_or(&self.id) }
    /// Texto nuevo (teclear, reemplazar): sucio solo si no es lo que hay en disco.
    fn poner_texto(&mut self, t: String) {
        // El largo primero: casi siempre distinto, y asi no se recorre el texto entero.
        self.sucio = t.len() != self.en_disco.0 || huella(&t) != self.en_disco;
        self.texto = t;
        self.cerrar_pedido = false;
    }

    fn saltar_a(&mut self, linea: Option<u32>, rango: Option<(usize, usize)>) { self.ir_a = linea; self.rango = rango; self.saltos += 1; }
}

/// Barra de buscar y reemplazar (Ctrl+F / Ctrl+H), sobre la pestana activa.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Busqueda {
    pub patron: String,
    pub reemplazo: String,
    /// Distinguir mayusculas y minusculas.
    pub mayusculas: bool,
    /// Solo palabras enteras.
    pub palabra: bool,
    /// Mostrar la fila de reemplazar.
    pub reemplazar: bool,
    /// Coincidencias en bytes, en orden, sin solaparse.
    pub hallazgos: Vec<(usize, usize)>,
    pub actual: usize,
    /// Hubo mas de MAX_HALLAZGOS.
    pub demasiados: bool,
    /// Sube cuando hay que poner el foco en el cuadro de buscar.
    pub foco: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccionBuscar {
    Abrir { reemplazar: bool },
    Cerrar,
    Patron(String),
    Reemplazo(String),
    Mayusculas,
    Palabra,
    Siguiente,
    Anterior,
    Reemplazar,
    ReemplazarTodo,
}

fn es_palabra(c: char) -> bool { c.is_alphanumeric() || c == '_' }

/// Coincidencias de `patron` en `texto` (bytes, sin solaparse), hasta `max`.
/// Sin mayusculas: ASCII rapido; si el patron tiene otras letras, se compara letra a letra.
pub fn buscar(texto: &str, patron: &str, mayusculas: bool, palabra: bool, max: usize) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    if patron.is_empty() { return v; }
    let entera = |a: usize, z: usize| {
        !palabra || (!texto[..a].chars().next_back().is_some_and(es_palabra) && !texto[z..].chars().next().is_some_and(es_palabra))
    };
    let empuja = |a: usize, z: usize, v: &mut Vec<(usize, usize)>| -> bool {
        if entera(a, z) { v.push((a, z)); }
        v.len() < max
    };
    if mayusculas {
        let mut desde = 0;
        while let Some(i) = texto[desde..].find(patron) {
            let (a, z) = (desde + i, desde + i + patron.len());
            if !empuja(a, z, &mut v) { break; }
            desde = z;
        }
    } else if patron.is_ascii() {
        let (t, p) = (texto.as_bytes(), patron.as_bytes());
        let mut i = 0;
        while i + p.len() <= t.len() {
            // Un byte ASCII nunca esta en medio de una letra UTF-8: el inicio es un limite valido.
            if t[i..i + p.len()].eq_ignore_ascii_case(p) {
                if !empuja(i, i + p.len(), &mut v) { break; }
                i += p.len();
            } else { i += 1; }
        }
    } else {
        let menor = |c: char| c.to_lowercase().next().unwrap_or(c);
        let p: Vec<char> = patron.chars().map(menor).collect();
        let mut indices = texto.char_indices().peekable();
        while let Some((a, _)) = indices.next() {
            let mut resto = texto[a..].char_indices();
            let mut z = a;
            let iguales = p.iter().all(|pc| match resto.next() { Some((k, c)) if menor(c) == *pc => { z = a + k + c.len_utf8(); true } _ => false });
            if iguales {
                if !empuja(a, z, &mut v) { break; }
                while indices.peek().is_some_and(|(k, _)| *k < z) { indices.next(); }
            }
        }
    }
    v
}

/// El texto con todas las coincidencias cambiadas (de una vez, sin volver a buscar dentro
/// de lo reemplazado).
pub fn reemplazar_todo(texto: &str, hallazgos: &[(usize, usize)], por: &str) -> String {
    let mut s = String::with_capacity(texto.len());
    let mut ultimo = 0;
    for (a, z) in hallazgos {
        s.push_str(&texto[ultimo..*a]);
        s.push_str(por);
        ultimo = *z;
    }
    s.push_str(&texto[ultimo..]);
    s
}

/// Las pestanas del editor y su busqueda. Sub-reducer: el Estado le pasa las acciones.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pestanas {
    pub lista: Vec<Editor>,
    pub activa: usize,
    pub busqueda: Option<Busqueda>,
    /// Lo ultimo que se busco (Ctrl+F lo vuelve a poner).
    pub ultimo_patron: String,
}

impl Pestanas {
    pub fn activa(&self) -> Option<&Editor> { self.lista.get(self.activa) }
    fn activa_mut(&mut self) -> Option<&mut Editor> { self.lista.get_mut(self.activa) }
    pub fn abierta(&self) -> bool { !self.lista.is_empty() }

    /// Abre (o trae al frente) un archivo. Devuelve true si hay que leerlo del disco.
    pub fn abrir(&mut self, id: &str, linea: Option<u32>) -> Result<bool, String> {
        if let Some(i) = self.lista.iter().position(|e| e.id == id) {
            self.activa = i;
            let ed = &mut self.lista[i];
            let l = linea.or(ed.ir_a);
            ed.saltar_a(l, None);
            self.rebuscar(false);
            return Ok(false);
        }
        if self.lista.len() >= MAX_PESTANAS {
            // Se recicla la pestana mas vieja sin cambios; si todas tienen cambios, no se abre.
            let Some(i) = self.lista.iter().position(|e| !e.sucio) else {
                return Err(alloc::format!("Hay {MAX_PESTANAS} pestanas con cambios sin guardar: guarda o cierra alguna"));
            };
            self.lista.remove(i);
        }
        let carga = self.lista.iter().map(|e| e.carga).max().unwrap_or(0) + 1;
        self.lista.push(Editor { id: id.into(), ir_a: linea.or(Some(1)), saltos: 1, carga, ..Editor::default() });
        self.activa = self.lista.len() - 1;
        Ok(true)
    }

    pub fn leido(&mut self, id: &str, resultado: Result<String, String>) {
        let Some(ed) = self.lista.iter_mut().find(|e| e.id == id && !e.cargado) else { return };
        match resultado {
            Ok(t) => { ed.en_disco = huella(&t); ed.texto = t; ed.cargado = true; ed.carga += 1; }
            Err(m) => ed.aviso = Some(m),
        }
        self.rebuscar(false);
    }

    pub fn editar(&mut self, t: String) {
        let Some(ed) = self.activa_mut() else { return };
        if !ed.cargado || t == ed.texto { return; }
        if t.len() as u64 > MAX_EDITOR { ed.aviso = Some(String::from("El texto pasa de 2 MiB: no se acepta")); return; }
        ed.poner_texto(t);
        // Los avisos (guardado, N reemplazos) son de un momento: al teclear se van.
        if ed.aviso.as_deref().is_some_and(|a| a.starts_with("Guardado") || a.ends_with("reemplazos")) { ed.aviso = None; }
        self.rebuscar(false);
    }

    /// (id, texto) a guardar, si la pestana activa tiene cambios. Recuerda que se mando.
    pub fn para_guardar(&mut self) -> Option<(String, String)> {
        let ed = self.activa_mut().filter(|e| e.cargado && e.sucio)?;
        ed.guardando = Some(huella(&ed.texto));
        Some((ed.id.clone(), ed.texto.clone()))
    }

    pub fn guardado(&mut self, id: &str, resultado: Result<(), String>) {
        let Some(ed) = self.lista.iter_mut().find(|e| e.id == id) else { return };
        match resultado {
            Ok(()) => {
                // Lo guardado es lo que se mando; si se tecleo entre medias, sigue sucio.
                if let Some(h) = ed.guardando.take() { ed.en_disco = h; }
                ed.sucio = huella(&ed.texto) != ed.en_disco;
                ed.cerrar_pedido = false;
                ed.aviso = Some(String::from("Guardado. F5 para volver a ejecutar"));
            }
            Err(m) => ed.aviso = Some(m),
        }
    }

    /// Selecciona un tramo (una coincidencia de la busqueda del proyecto) en la pestana del archivo.
    pub fn marcar(&mut self, id: &str, rango: (usize, usize)) {
        if let Some(ed) = self.lista.iter_mut().find(|e| e.id == id) { let l = ed.ir_a; ed.saltar_a(l, Some(rango)); }
    }

    pub fn elegir(&mut self, i: usize) {
        if i < self.lista.len() && i != self.activa { self.activa = i; self.rebuscar(false); }
    }

    pub fn mover(&mut self, paso: i32) {
        let n = self.lista.len() as i32;
        if n > 1 { self.elegir((self.activa as i32 + paso).rem_euclid(n) as usize); }
    }

    /// Cerrar una pestana. Con cambios sin guardar, la primera vez solo avisa (y la trae al frente).
    pub fn cerrar(&mut self, i: usize) {
        let Some(ed) = self.lista.get_mut(i) else { return };
        if ed.sucio && !ed.cerrar_pedido {
            ed.cerrar_pedido = true;
            ed.aviso = Some(String::from("Cambios sin guardar: Ctrl+S guarda; cerrar otra vez los descarta"));
            self.activa = i;
            return;
        }
        self.lista.remove(i);
        if self.activa > i || self.activa >= self.lista.len() { self.activa = self.activa.saturating_sub(1); }
        if self.lista.is_empty() { self.busqueda = None; } else { self.rebuscar(false); }
    }

    /// Vuelve a buscar en la pestana activa; si `saltar`, selecciona la coincidencia actual.
    fn rebuscar(&mut self, saltar: bool) {
        let Some(b) = &mut self.busqueda else { return };
        let Some(ed) = self.lista.get_mut(self.activa) else { return };
        b.hallazgos = buscar(&ed.texto, &b.patron, b.mayusculas, b.palabra, MAX_HALLAZGOS + 1);
        b.demasiados = b.hallazgos.len() > MAX_HALLAZGOS;
        b.hallazgos.truncate(MAX_HALLAZGOS);
        if b.actual >= b.hallazgos.len() { b.actual = 0; }
        if saltar { if let Some(r) = b.hallazgos.get(b.actual) { ed.saltar_a(None, Some(*r)); } }
    }

    pub fn buscar(&mut self, accion: AccionBuscar) {
        if self.activa().is_none_or(|e| !e.cargado) { return; }
        if let AccionBuscar::Abrir { reemplazar } = accion {
            let ultimo = self.ultimo_patron.clone();
            let b = self.busqueda.get_or_insert_with(|| Busqueda { patron: ultimo, ..Busqueda::default() });
            b.reemplazar |= reemplazar;
            b.foco += 1;
            return self.rebuscar(true);
        }
        let Some(b) = &mut self.busqueda else { return };
        let n = b.hallazgos.len();
        match accion {
            AccionBuscar::Abrir { .. } => {}
            AccionBuscar::Cerrar => { self.busqueda = None; }
            AccionBuscar::Patron(p) => {
                b.patron = p.chars().take(MAX_PATRON).collect();
                b.actual = 0;
                self.ultimo_patron = b.patron.clone();
                self.rebuscar(true);
            }
            AccionBuscar::Reemplazo(r) => b.reemplazo = r.chars().take(MAX_PATRON).collect(),
            AccionBuscar::Mayusculas => { b.mayusculas = !b.mayusculas; self.rebuscar(true); }
            AccionBuscar::Palabra => { b.palabra = !b.palabra; self.rebuscar(true); }
            AccionBuscar::Siguiente | AccionBuscar::Anterior if n > 0 => {
                b.actual = if accion == AccionBuscar::Siguiente { (b.actual + 1) % n } else { (b.actual + n - 1) % n };
                self.rebuscar(true);
            }
            AccionBuscar::Siguiente | AccionBuscar::Anterior => {}
            AccionBuscar::Reemplazar if n > 0 => {
                let (a, z) = b.hallazgos[b.actual];
                let por = b.reemplazo.clone();
                let ed = &mut self.lista[self.activa];
                let mut t = ed.texto.clone();
                t.replace_range(a..z, &por);
                if t.len() as u64 > MAX_EDITOR { ed.aviso = Some(String::from("El texto pasaria de 2 MiB: no se reemplaza")); return; }
                ed.poner_texto(t);
                ed.carga += 1;
                // La siguiente coincidencia queda en el mismo indice (lo reemplazado ya no casa).
                self.rebuscar(true);
            }
            AccionBuscar::ReemplazarTodo if n > 0 => {
                let t = reemplazar_todo(&self.lista[self.activa].texto, &b.hallazgos, &b.reemplazo);
                let ed = &mut self.lista[self.activa];
                if t.len() as u64 > MAX_EDITOR { ed.aviso = Some(String::from("El texto pasaria de 2 MiB: no se reemplaza")); return; }
                ed.poner_texto(t);
                ed.carga += 1;
                ed.aviso = Some(alloc::format!("{n} reemplazos{}", if b.demasiados { " (habia mas: repite para seguir)" } else { "" }));
                b.actual = 0;
                self.rebuscar(false);
            }
            AccionBuscar::Reemplazar | AccionBuscar::ReemplazarTodo => {}
        }
    }
}

/// Clases de texto para colorear codigo (el tema decide el color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token { Normal, Clave, Texto, Comentario, Numero, Simbolo }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lenguaje { Rust, C, Python, Shell, Toml, Otro }

pub fn lenguaje(nombre: &str) -> Lenguaje {
    let base = nombre.rsplit('/').next().unwrap_or(nombre);
    if ["Makefile", "makefile", "GNUmakefile", "CMakeLists.txt", "Dockerfile", "meson.build"].contains(&base) { return Lenguaje::Shell; }
    let ext = base.rsplit_once('.').map(|(_, x)| x).unwrap_or("");
    let es = |lista: &[&str]| lista.iter().any(|e| e.eq_ignore_ascii_case(ext));
    if es(&["rs"]) { Lenguaje::Rust }
    else if es(&["c", "h", "cpp", "cc", "cxx", "c++", "cppm", "ixx", "hpp", "hh", "hxx", "h++", "inl", "ipp", "tpp", "cu", "cuh", "m", "mm",
                 "js", "mjs", "cjs", "ts", "mts", "cts", "jsx", "tsx", "wgsl", "glsl", "hlsl", "vert", "frag", "comp", "geom", "java", "go",
                 "kt", "kts", "cs", "swift", "zig", "dart", "scala", "php", "proto"]) { Lenguaje::C }
    else if es(&["py", "pyi", "pyw"]) { Lenguaje::Python }
    else if es(&["sh", "bash", "zsh", "fish", "cmake", "nix", "mk"]) { Lenguaje::Shell }
    else if es(&["toml", "yaml", "yml", "ini", "conf", "cfg"]) { Lenguaje::Toml }
    else { Lenguaje::Otro }
}

const CLAVES_RUST: &[&str] = &["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "Some", "None", "Ok", "Err"];
/// C, C++ y los lenguajes con sintaxis de llaves (Java, Go, JS/TS, shaders...).
const CLAVES_C: &[&str] = &["auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float",
    "for", "goto", "if", "int", "long", "register", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
    "unsigned", "void", "volatile", "while", "inline", "restrict", "bool", "true", "false", "NULL",
    // C++
    "class", "namespace", "template", "typename", "using", "public", "private", "protected", "virtual", "override", "final", "constexpr",
    "consteval", "constinit", "nullptr", "new", "delete", "this", "operator", "friend", "explicit", "mutable", "noexcept", "try", "catch",
    "throw", "static_cast", "dynamic_cast", "const_cast", "reinterpret_cast", "decltype", "concept", "requires", "co_await", "co_return",
    "co_yield", "export", "import", "module", "std",
    // otros con llaves
    "function", "let", "var", "null", "undefined", "fn", "func", "package", "interface", "implements", "extends", "async", "await",
    "uniform", "in", "out", "vec2", "vec3", "vec4", "mat4", "f32", "u32", "i32"];
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
        // Directivas de C/C++ (#include, #define, #pragma...): la palabra entera es clave.
        if c == b'#' && len == Lenguaje::C && linea[..i].trim().is_empty() {
            let mut j = i + 1;
            while j < b.len() && (b[j] == b' ' || b[j] == b'\t') { j += 1; }
            while j < b.len() && b[j].is_ascii_alphabetic() { j += 1; }
            empuja(&mut v, i, j, Token::Clave);
            i = j;
            continue;
        }
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
