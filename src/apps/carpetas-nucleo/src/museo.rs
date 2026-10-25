// SPDX-License-Identifier: GPL-2.0-only
//! El museo dentro del codigo (SUPER GRAFO). Busca en C, C++ y ASM los patrones de las
//! fichas del museo (docs/MUSEO-UB.md y Museo/README.md) y dice cual, donde y por que.
//!
//! Reglas de honestidad, las mismas del museo:
//!
//! - Es analisis **lexico**: no compila, no ejecuta y no ve tipos ni valores. Una sospecha
//!   no es una prueba de UB ni su ausencia una garantia.
//! - `Exacto`: lo escrito es el patron de la ficha tal cual (`free(p + 1)`, `1 << 32`,
//!   `va_arg(ap, char)`). `Revisar`: depende de tipos, valores o del dialecto.
//! - Cada detector esta aislado (una funcion por ficha) y tiene su caso malo y su
//!   control valido en `tests/museo.rs`.
//! - Lo que no se puede ver sin compilar (restrict, va_arg en tiempo de ejecucion,
//!   formatos de printf, que ya avisa -Wformat) no se inventa: esas fichas quedan solo
//!   para leer.

use crate::fichas::{Ficha, FICHAS};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub const MAX_SOSPECHAS_POR_ARCHIVO: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fuerza {
    /// Depende de tipos, valores o del dialecto: leer la ficha y mirar el codigo.
    Revisar,
    /// Lo escrito es el patron de la ficha tal cual.
    Exacto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sospecha {
    /// Id de la ficha del museo ("C-001", "UB-003", "ASM-002"...).
    pub ficha: &'static str,
    pub linea: u32,
    pub columna: u32,
    pub fuerza: Fuerza,
    /// Lo que se vio, recortado ("free(p + 1)").
    pub visto: String,
    /// Por que salta en este caso concreto.
    pub motivo: String,
}

pub fn ficha(id: &str) -> Option<&'static Ficha> { FICHAS.iter().find(|f| f.id == id) }

/// Que lenguaje del museo es una ruta (None: no se analiza).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idioma { C, Cpp, CoCpp, Gas { cpp: bool }, Nasm }

pub fn idioma(ruta: &str) -> Option<Idioma> {
    let nombre = ruta.rsplit('/').next().unwrap_or(ruta);
    let (_, ext) = nombre.rsplit_once('.')?;
    match ext {
        "c" => Some(Idioma::C),
        "h" => Some(Idioma::CoCpp),
        "S" | "sx" => Some(Idioma::Gas { cpp: true }),
        _ => {
            let e = ext.to_ascii_lowercase();
            match e.as_str() {
                "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" | "ipp" | "inl" | "tpp" | "cu" | "cuh" | "cppm" | "ixx" => Some(Idioma::Cpp),
                "s" => Some(Idioma::Gas { cpp: false }),
                "asm" | "nasm" => Some(Idioma::Nasm),
                _ => None,
            }
        }
    }
}

/// Analiza un archivo entero. Devuelve las sospechas ordenadas por linea (sin repetir
/// la misma ficha en la misma linea) y como mucho MAX_SOSPECHAS_POR_ARCHIVO.
pub fn analizar(ruta: &str, texto: &str) -> Vec<Sospecha> {
    let Some(id) = idioma(ruta) else { return Vec::new() };
    let mut out = match id {
        Idioma::C | Idioma::Cpp | Idioma::CoCpp => {
            let toks = tokens(texto);
            let cpp = match id { Idioma::Cpp => true, Idioma::C => false, _ => parece_cpp(&toks) };
            let mut c = Ctx::nuevo(&toks, cpp);
            c.todo();
            c.out
        }
        Idioma::Gas { cpp } => asm_archivo(texto, false, cpp),
        Idioma::Nasm => asm_archivo(texto, true, false),
    };
    out.sort_by(|a, b| (a.linea, a.columna, a.ficha).cmp(&(b.linea, b.columna, b.ficha)));
    let mut vistos = BTreeSet::new();
    out.retain(|s| vistos.insert((s.ficha, s.linea)));
    out.truncate(MAX_SOSPECHAS_POR_ARCHIVO);
    out
}

// ====================================================================================
// Lexer de C y C++: identificadores, numeros, cadenas, caracteres, signos y directivas.
// Quita comentarios. Los cuerpos de #define se analizan tambien (ahi viven muchos bugs),
// rodeados de ';' para no mezclarlos con el codigo de alrededor.
// ====================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase { Ident, Numero, Cadena, Caracter, Signo, Directiva }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tok<'a> { pub clase: Clase, pub t: &'a str, pub linea: u32, pub col: u32 }

const SIGNOS3: &[&str] = &["<<=", ">>=", "...", "->*", "<=>"];
const SIGNOS2: &[&str] = &["->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "::", "##"];

pub fn tokens(texto: &str) -> Vec<Tok<'_>> {
    let mut v = Vec::new();
    lexar(texto, 0, texto.len(), 1, 0, false, &mut v);
    v
}

fn es_ident(c: u8) -> bool { c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80 }

#[allow(clippy::too_many_arguments)]
fn lexar<'a>(texto: &'a str, desde: usize, hasta: usize, linea0: u32, ini0: usize, en_define: bool, v: &mut Vec<Tok<'a>>) {
    let b = &texto.as_bytes()[..hasta];
    let (mut i, mut linea, mut ini) = (desde, linea0, ini0);
    let mut al_inicio = !en_define;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' { linea += 1; i += 1; ini = i; al_inicio = !en_define; continue; }
        if c.is_ascii_whitespace() { i += 1; continue; }
        let col = (i - ini) as u32 + 1;
        let l0 = linea;
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') { i += 1; linea += 1; ini = i + 1; }
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                if b[i] == b'\n' { linea += 1; ini = i + 1; }
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        if c == b'#' && al_inicio {
            let a = i;
            while i < b.len() && b[i] != b'\n' {
                if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') { i += 2; linea += 1; ini = i; continue; }
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') { break; }
                if b[i] == b'/' && b.get(i + 1) == Some(&b'/') { break; }
                i += 1;
            }
            v.push(Tok { clase: Clase::Directiva, t: &texto[a..i], linea: l0, col });
            // #define NOMBRE(args) cuerpo: el cuerpo tambien se lee.
            if let Some(cuerpo) = cuerpo_define(texto, a, i) {
                v.push(Tok { clase: Clase::Signo, t: ";", linea: l0, col });
                lexar(texto, cuerpo, i, l0, ini0_de(texto, a), true, v);
                v.push(Tok { clase: Clase::Signo, t: ";", linea: l0, col });
            }
            continue;
        }
        al_inicio = false;
        if c == b'"' {
            let a = i;
            i += 1;
            while i < b.len() && b[i] != b'"' && b[i] != b'\n' { if b[i] == b'\\' { i += 1; } i += 1; }
            i = (i + 1).min(b.len());
            v.push(Tok { clase: Clase::Cadena, t: corte(texto, a, i), linea: l0, col });
            continue;
        }
        if c == b'\'' {
            let a = i;
            i += 1;
            while i < b.len() && b[i] != b'\'' && b[i] != b'\n' { if b[i] == b'\\' { i += 1; } i += 1; }
            i = (i + 1).min(b.len());
            v.push(Tok { clase: Clase::Caracter, t: corte(texto, a, i), linea: l0, col });
            continue;
        }
        if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let a = i;
            i += 1;
            while i < b.len() {
                let d = b[i];
                let previo = b[i - 1];
                if es_ident(d) || d == b'.' || ((d == b'+' || d == b'-') && matches!(previo, b'e' | b'E' | b'p' | b'P') && !texto[a..i].starts_with("0x"))
                   || (d == b'\'' && b.get(i + 1).is_some_and(|x| x.is_ascii_alphanumeric())) { i += 1; } else { break; }
            }
            v.push(Tok { clase: Clase::Numero, t: corte(texto, a, i), linea: l0, col });
            continue;
        }
        if es_ident(c) {
            let a = i;
            while i < b.len() && es_ident(b[i]) { i += 1; }
            let palabra = corte(texto, a, i);
            // Cadena cruda de C++: R"delim( ... )delim"
            if b.get(i) == Some(&b'"') && matches!(palabra, "R" | "u8R" | "LR" | "uR" | "UR") {
                let ab = i + 1;
                let mut j = ab;
                while j < b.len() && b[j] != b'(' && j - ab < 16 { j += 1; }
                let delim = &b[ab..j];
                let mut k = j + 1;
                while k < b.len() {
                    if b[k] == b'\n' { linea += 1; ini = k + 1; }
                    if b[k] == b')' && b[k + 1..].starts_with(delim) && b.get(k + 1 + delim.len()) == Some(&b'"') { k += 2 + delim.len(); break; }
                    k += 1;
                }
                i = k.min(b.len());
                v.push(Tok { clase: Clase::Cadena, t: corte(texto, a, i), linea: l0, col });
                continue;
            }
            v.push(Tok { clase: Clase::Ident, t: palabra, linea: l0, col });
            continue;
        }
        let resto = &b[i..];
        let largo = if SIGNOS3.iter().any(|s| resto.starts_with(s.as_bytes())) { 3 }
                    else if SIGNOS2.iter().any(|s| resto.starts_with(s.as_bytes())) { 2 } else { 1 };
        v.push(Tok { clase: Clase::Signo, t: corte(texto, i, i + largo), linea: l0, col });
        i += largo;
    }
}

/// Corte seguro (si un indice cae dentro de un caracter UTF-8, se avanza hasta el limite).
fn corte(t: &str, mut a: usize, mut b: usize) -> &str {
    b = b.min(t.len());
    while b < t.len() && !t.is_char_boundary(b) { b += 1; }
    a = a.min(b);
    while a > 0 && !t.is_char_boundary(a) { a -= 1; }
    &t[a..b]
}

fn ini0_de(texto: &str, a: usize) -> usize { texto[..a].rfind('\n').map(|p| p + 1).unwrap_or(0) }

fn cuerpo_define(texto: &str, a: usize, b: usize) -> Option<usize> {
    let s = &texto[a..b];
    let resto = s[1..].trim_start();
    let resto = resto.strip_prefix("define")?;
    if !resto.starts_with([' ', '\t']) { return None; }
    let base = b - resto.len();
    let bytes = texto.as_bytes();
    let mut i = base;
    while i < b && (bytes[i] == b' ' || bytes[i] == b'\t') { i += 1; }
    while i < b && es_ident(bytes[i]) { i += 1; }
    if bytes.get(i) == Some(&b'(') {
        while i < b && bytes[i] != b')' { i += 1; }
        i += 1;
    }
    (i < b).then_some(i)
}

/// Un .h es C++ si usa algo que solo existe en C++.
fn parece_cpp(t: &[Tok]) -> bool {
    t.iter().any(|x| x.clase == Clase::Ident && matches!(x.t, "class" | "namespace" | "template" | "typename" | "nullptr" | "constexpr" | "public" | "private"))
        || t.iter().any(|x| x.clase == Clase::Signo && x.t == "::")
}

// ====================================================================================
// C y C++: un detector por ficha.
// ====================================================================================

const TIPOS: &[&str] = &["int", "char", "short", "long", "float", "double", "unsigned", "signed", "size_t", "ssize_t", "bool", "_Bool",
    "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "intptr_t", "uintptr_t", "ptrdiff_t", "wchar_t"];
const HILOS: &[&str] = &["pthread_create", "thrd_create", "jthread", "CreateThread", "_beginthreadex"];
const CTYPE: &[&str] = &["isalnum", "isalpha", "isblank", "iscntrl", "isdigit", "isgraph", "islower", "isprint", "ispunct", "isspace",
    "isupper", "isxdigit", "tolower", "toupper"];

struct Ctx<'a> {
    k: &'a [Tok<'a>],
    cpp: bool,
    hilos: bool,
    /// Para cada token: esta dentro de un cuerpo de funcion o bloque (no en struct/enum/inicializador).
    en_codigo: Vec<bool>,
    /// Para cada token: indice de la funcion (cuerpo de nivel superior) en la que esta.
    funcion: Vec<u32>,
    out: Vec<Sospecha>,
}

impl<'a> Ctx<'a> {
    fn nuevo(k: &'a [Tok<'a>], cpp: bool) -> Ctx<'a> {
        let hilos = k.windows(3).any(|w| w[0].t == "std" && w[1].t == "::" && w[2].t == "thread")
            || k.iter().any(|x| x.clase == Clase::Ident && HILOS.contains(&x.t))
            || k.iter().any(|x| x.clase == Clase::Directiva && x.t.contains("include") && ["<pthread.h>", "<thread>", "<threads.h>"].iter().any(|h| x.t.contains(h)));
        let (mut en_codigo, mut funcion) = (Vec::with_capacity(k.len()), Vec::with_capacity(k.len()));
        let mut pila: Vec<bool> = Vec::new();
        let mut n_funcion = 0u32;
        for (i, x) in k.iter().enumerate() {
            let previo = if i > 0 { k[i - 1].t } else { "" };
            if x.clase == Clase::Signo && x.t == "{" {
                let arriba = pila.last().copied().unwrap_or(false);
                let codigo = matches!(previo, ")" | "else" | "do" | "try" | "const" | "noexcept" | "override" | "final" | "mutable")
                    || (arriba && matches!(previo, ";" | "{" | "}" | ":"));
                if codigo && !arriba { n_funcion += 1; }
                pila.push(codigo);
            }
            en_codigo.push(pila.last().copied().unwrap_or(false));
            funcion.push(if pila.iter().any(|c| *c) { n_funcion } else { 0 });
            if x.clase == Clase::Signo && x.t == "}" { pila.pop(); }
        }
        Ctx { k, cpp, hilos, en_codigo, funcion, out: Vec::new() }
    }

    fn todo(&mut self) {
        self.free_interior();
        self.new_y_delete();
        self.desplazamiento();
        self.sin_secuenciar();
        self.ctype();
        self.memcpy_solapado();
        self.volatile_hilos();
        self.string_view_temporal();
        self.vla();
        self.null();
        self.direccion_local();
        self.va_arg();
        self.vector_invalidado();
        self.firma_incompatible();
        self.division();
        self.desborde_literal();
        self.alias_y_alineacion();
        self.setjmp();
        self.strncpy_sin_cero();
        self.fuera_del_arreglo();
        self.asm_en_linea();
    }

    // ---------- utilidades ----------
    /// Texto del token (las cadenas cuentan como "" para no confundirlas con codigo).
    fn t(&self, i: usize) -> &'a str {
        match self.k.get(i) { Some(x) if x.clase != Clase::Cadena && x.clase != Clase::Directiva => x.t, _ => "" }
    }
    fn antes(&self, i: usize, n: usize) -> &'a str { i.checked_sub(n).map_or("", |j| self.t(j)) }
    fn ident(&self, i: usize) -> bool { self.k.get(i).is_some_and(|x| x.clase == Clase::Ident) }
    fn numero(&self, i: usize) -> bool { self.k.get(i).is_some_and(|x| x.clase == Clase::Numero) }
    /// Llamada a funcion libre (no metodo): nombre seguido de '(' y sin '.'/'->' delante.
    fn llamada(&self, i: usize, nombres: &[&str]) -> bool {
        self.ident(i) && nombres.contains(&self.t(i)) && self.t(i + 1) == "(" && !matches!(self.antes(i, 1), "." | "->")
            && !(self.antes(i, 1) == "::" && self.antes(i, 2) != "std")
    }
    fn cierre(&self, i: usize) -> Option<usize> {
        let mut nivel = 0i32;
        for j in i..self.k.len().min(i + 4000) {
            match self.t(j) { "(" | "[" | "{" => nivel += 1, ")" | "]" | "}" => { nivel -= 1; if nivel == 0 { return Some(j); } } _ => {} }
        }
        None
    }
    /// Argumentos de una llamada: tramos [a, b) entre comas de nivel superior.
    fn args(&self, abre: usize, cierra: usize) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        let (mut nivel, mut a) = (0i32, abre + 1);
        for j in abre + 1..cierra {
            match self.t(j) {
                "(" | "[" | "{" => nivel += 1,
                ")" | "]" | "}" => nivel -= 1,
                "," if nivel == 0 => { v.push((a, j)); a = j + 1; }
                _ => {}
            }
        }
        if a < cierra { v.push((a, cierra)); }
        v
    }
    fn texto(&self, a: usize, b: usize) -> String {
        let mut s = String::new();
        for j in a..b.min(self.k.len()) {
            let x = &self.k[j];
            let pegado = j > a && {
                let p = &self.k[j - 1];
                let palabra = |c: Clase| matches!(c, Clase::Ident | Clase::Numero);
                !(palabra(p.clase) && palabra(x.clase)) && !matches!(x.t, "=" | "+" | "-" | "<<" | ">>" | "==" | "<=" | "&&" | "||" | "?" | ":")
                    && !matches!(p.t, "=" | "+" | "-" | "<<" | ">>" | "," | "==" | "<=" | "&&" | "||" | "?" | ":")
            };
            if j > a && !pegado { s.push(' '); }
            s.push_str(x.t);
            if s.len() > 90 { s.truncate(s.char_indices().take_while(|(i, _)| *i < 87).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(0)); s.push('…'); break; }
        }
        s
    }
    fn igual(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        a.1 - a.0 == b.1 - b.0 && a.1 > a.0 && (0..a.1 - a.0).all(|d| self.k[a.0 + d].t == self.k[b.0 + d].t)
    }
    fn marcar(&mut self, ficha: &'static str, en: usize, fuerza: Fuerza, motivo: String, a: usize, b: usize) {
        let Some(x) = self.k.get(en) else { return };
        let visto = self.texto(a, b);
        self.out.push(Sospecha { ficha, linea: x.linea, columna: x.col, fuerza, visto, motivo });
    }

    // ---------- C-001: free de un puntero interior ----------
    fn free_interior(&mut self) {
        for i in 0..self.k.len() {
            if !self.llamada(i, &["free"]) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            let a = self.args(i + 1, c);
            if a.len() != 1 || !self.ident(a[0].0) || a[0].1 - a[0].0 < 3 { continue; }
            let signo = self.t(a[0].0 + 1);
            if signo == "+" || signo == "-" {
                let fuerza = if signo == "+" { Fuerza::Exacto } else { Fuerza::Revisar };
                let p = self.t(a[0].0);
                let m = alloc::format!("free recibe {p} {signo} …: free solo acepta el puntero que devolvio la reserva (o NULL)");
                self.marcar("C-001", i, fuerza, m, i, c + 1);
            }
        }
    }

    // ---------- CXX-003: new[] con delete (o new con delete[]) ----------
    fn new_y_delete(&mut self) {
        if !self.cpp { return; }
        // (funcion, nombre) -> linea: los nombres se repiten entre funciones (y entre TEST de gtest).
        let (mut arreglos, mut sueltos): (BTreeMap<(u32, &str), u32>, BTreeMap<(u32, &str), u32>) = (BTreeMap::new(), BTreeMap::new());
        let mut otros: BTreeSet<(u32, &str)> = BTreeSet::new();
        for i in 2..self.k.len() {
            if self.ident(i) && self.t(i + 1) == "=" && self.t(i + 2) != "new" { otros.insert((self.funcion[i], self.t(i))); }
            if self.t(i) != "new" || self.antes(i, 1) != "=" || !self.ident(i - 2) || self.t(i + 1) == "(" { continue; }
            let nombre = (self.funcion[i], self.t(i - 2));
            let mut arreglo = false;
            for j in i + 1..self.k.len().min(i + 40) {
                match self.t(j) { "[" => { arreglo = true; break; } ";" | "(" | "{" | ")" | "," => break, _ => {} }
            }
            let linea = self.k[i].linea;
            if arreglo { arreglos.insert(nombre, linea); } else { sueltos.insert(nombre, linea); }
        }
        for i in 0..self.k.len() {
            if self.t(i) != "delete" { continue; }
            let corchetes = self.t(i + 1) == "[" && self.t(i + 2) == "]";
            let n = if corchetes { i + 3 } else { i + 1 };
            if !self.ident(n) || self.t(n + 1) != ";" { continue; }
            let clave = (self.funcion[i], self.t(n));
            let nombre = clave.1;
            let (mal, otro, de) = if corchetes { (sueltos.get(&clave), arreglos.contains_key(&clave), "new sin []") } else { (arreglos.get(&clave), sueltos.contains_key(&clave), "new[]") };
            if let (Some(l), false, false) = (mal, otro, otros.contains(&clave)) {
                let pareja = if corchetes { "delete[]" } else { "delete" };
                let m = alloc::format!("{nombre} se reservo con {de} (linea {l}) y se libera con {pareja}");
                self.marcar("CXX-003", i, Fuerza::Revisar, m, i, n + 1);
            }
        }
    }

    // ---------- UB-003: desplazar fuera del ancho (o, en C, hasta el bit de signo) ----------
    fn desplazamiento(&mut self) {
        for i in 1..self.k.len() {
            if self.t(i) != "<<" || !self.numero(i - 1) || !self.numero(i + 1) { continue; }
            if matches!(self.antes(i, 2), "*" | "/" | "%" | "+" | "-" | "." | "->" | ")" | "]") || self.ident(i - 2) && !matches!(self.antes(i, 2), "return" | "case") { continue; }
            if matches!(self.t(i + 2), "+" | "-" | "*" | "/" | "%") { continue; }
            let (Some((v, ancho, con_signo)), Some((n, _, _))) = (literal(self.k[i - 1].t), literal(self.k[i + 1].t)) else { continue };
            let ancho = ancho as u128;
            let m = if n >= ancho {
                alloc::format!("se desplaza {n} bits un valor de {ancho} bits: el ancho es el limite")
            } else if con_signo && !self.cpp && (v << n) > ((1u128 << (ancho - 1)) - 1) {
                alloc::format!("{v} << {n} no cabe en un int con signo de {ancho} bits (en C17 es UB; usar 1u)")
            } else { continue };
            self.marcar("UB-003", i, Fuerza::Exacto, m, i - 1, i + 2);
        }
    }

    // ---------- UB-009 (C) / CXX-005 (C++): dos modificaciones sin secuenciar ----------
    fn sin_secuenciar(&mut self) {
        let mut a = 0;
        for i in 0..=self.k.len() {
            let fin = i == self.k.len() || matches!(self.t(i), ";" | "{" | "}");
            if !fin { continue; }
            self.sentencia(a, i);
            a = i + 1;
        }
    }
    fn sentencia(&mut self, a: usize, b: usize) {
        if b <= a + 2 { return; }
        // && || ?: y la coma de nivel superior si secuencian: no se juzga.
        let mut nivel = 0;
        for j in a..b {
            match self.t(j) { "(" | "[" => nivel += 1, ")" | "]" => nivel -= 1, "&&" | "||" | "?" => return, "," if nivel == 0 => return, _ => {} }
        }
        let mut cuentas: BTreeMap<&str, (u32, usize)> = BTreeMap::new();
        for j in a..b {
            if !self.ident(j) || matches!(self.antes(j, 1), "." | "->") || self.t(j + 1) == "[" || self.t(j + 1) == "(" { continue; }
            let inc = matches!(self.t(j + 1), "++" | "--") || (j > a && matches!(self.antes(j, 1), "++" | "--"));
            if inc { let e = cuentas.entry(self.t(j)).or_insert((0, j)); e.0 += 1; }
        }
        // x = x++ : la asignacion y el incremento de la misma variable.
        let asigna = self.ident(a) && self.t(a + 1) == "=" && cuentas.contains_key(self.t(a));
        for (nombre, (n, j)) in cuentas {
            if n >= 2 || (asigna && nombre == self.t(a)) {
                let (ficha, fuerza) = if self.cpp { ("CXX-005", Fuerza::Revisar) } else { ("UB-009", Fuerza::Exacto) };
                let m = if self.cpp { alloc::format!("{nombre} se modifica dos veces en una expresion: su clasificacion cambia con el dialecto de C++") }
                        else { alloc::format!("{nombre} se modifica dos veces sin un punto de secuencia entre medias") };
                self.marcar(ficha, j, fuerza, m, a, b);
                return;
            }
        }
    }

    // ---------- UB-011: char posiblemente negativo a <ctype.h> ----------
    fn ctype(&mut self) {
        for i in 0..self.k.len() {
            if !self.llamada(i, CTYPE) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            let a = self.args(i + 1, c);
            if a.len() != 1 { continue; }
            let (x, y) = a[0];
            if (x..y).any(|j| matches!(self.t(j), "unsigned" | "uint8_t" | "u8" | "UCHAR" | "guchar" | "EOF")) { continue; }
            if self.t(x) == "*" || self.t(y - 1) == "]" {
                let m = String::from("el argumento sale de un char: si es negativo (y no es EOF) el valor queda fuera del dominio; convertir con (unsigned char)");
                self.marcar("UB-011", i, Fuerza::Revisar, m, i, c + 1);
            }
        }
    }

    // ---------- UB-018: memcpy entre trozos del mismo objeto ----------
    fn memcpy_solapado(&mut self) {
        for i in 0..self.k.len() {
            if !self.llamada(i, &["memcpy", "__builtin_memcpy"]) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            let a = self.args(i + 1, c);
            if a.len() != 3 { continue; }
            let (b0, b1) = (self.base(a[0]), self.base(a[1]));
            if self.igual(b0, b1) {
                // Con desplazamientos y tamano literales se sabe si se tocan: p + 4, p, 4 no se solapan.
                let n = (a[2].1 == a[2].0 + 1 && self.numero(a[2].0)).then(|| literal(self.k[a[2].0].t).map(|x| x.0)).flatten();
                if let (Some(d0), Some(d1), Some(n)) = (self.desplazamiento_de(a[0], b0), self.desplazamiento_de(a[1], b1), n) {
                    if d0.abs_diff(d1) >= n { continue; }
                }
                let m = alloc::format!("origen y destino salen de {}: si se solapan es UB; memmove si permite solapamiento", self.texto(b0.0, b0.1));
                self.marcar("UB-018", i, Fuerza::Revisar, m, i, c + 1);
            }
        }
    }
    /// La base de un puntero: sin '&' delante, sin "+ n" ni "[i]" detras y sin conversiones.
    fn base(&self, (mut a, mut b): (usize, usize)) -> (usize, usize) {
        while a < b && self.t(a) == "(" {
            match self.cierre(a) {
                Some(c) if c + 1 < b && (a + 1..c).all(|j| self.ident(j) || self.t(j) == "*") => a = c + 1,
                _ => break,
            }
        }
        if self.t(a) == "&" { a += 1; }
        let mut nivel = 0;
        for j in a..b {
            match self.t(j) { "(" | "[" => { if nivel == 0 && self.t(j) == "[" { b = j; break; } nivel += 1; } ")" | "]" => nivel -= 1,
                              "+" | "-" if nivel == 0 => { b = j; break; } _ => {} }
        }
        (a, b)
    }

    // ---------- UB-021 (C) / CXX-004 (C++): volatile en un programa con hilos ----------
    fn volatile_hilos(&mut self) {
        if !self.hilos { return; }
        for i in 0..self.k.len() {
            if !matches!(self.t(i), "volatile" | "__volatile__") || self.t(i + 1) == "(" || matches!(self.antes(i, 1), "asm" | "__asm__" | "__asm") { continue; }
            if self.antes(i, 1) == "(" || self.t(i + 1) == "*" { continue; } // conversion o puntero a volatile (MMIO)
            if self.en_codigo[i] { continue; } // una local no se comparte entre hilos (no_optimize, contadores de prueba)
            let (ficha, m) = if self.cpp { ("CXX-004", "volatile no ordena ni hace atomico el acceso entre hilos: std::atomic o un mutex") }
                             else { ("UB-021", "volatile no sincroniza hilos: _Atomic o un mutex") };
            let fin = (i..self.k.len().min(i + 12)).find(|j| self.t(*j) == ";").unwrap_or(i + 1);
            self.marcar(ficha, i, Fuerza::Revisar, m.to_string(), i, fin);
        }
    }

    // ---------- CXX-001: string_view sobre un string temporal ----------
    fn string_view_temporal(&mut self) {
        if !self.cpp { return; }
        for i in 0..self.k.len() {
            if !matches!(self.t(i), "string_view" | "wstring_view" | "u8string_view" | "u16string_view" | "u32string_view") { continue; }
            if !self.ident(i + 1) || self.t(i + 2) != "=" { continue; }
            let fin = (i + 3..self.k.len().min(i + 60)).find(|j| self.t(*j) == ";").unwrap_or(i + 3);
            let tramo = i + 3..fin;
            let temporal = tramo.clone().any(|j| (self.t(j) == "to_string" || self.t(j) == "string") && self.t(j + 1) == "(" && self.antes(j, 1) != ".")
                || (tramo.clone().any(|j| self.t(j) == "+") && tramo.clone().any(|j| self.k[j].clase == Clase::Cadena));
            if temporal {
                let m = alloc::format!("{} mira un std::string temporal que se destruye al terminar esta linea", self.t(i + 1));
                self.marcar("CXX-001", i + 1, Fuerza::Revisar, m, i, fin);
            }
        }
    }

    // ---------- C-002: arreglo de longitud variable ----------
    fn vla(&mut self) {
        if self.cpp { return; }
        for i in 1..self.k.len() {
            if !self.en_codigo[i] || !matches!(self.antes(i, 1), ";" | "{" | "}") || !TIPOS.contains(&self.t(i)) { continue; }
            let mut j = i;
            while TIPOS.contains(&self.t(j)) || matches!(self.t(j), "const" | "*") { j += 1; }
            if !self.ident(j) || self.t(j + 1) != "[" { continue; }
            let Some(c) = self.cierre(j + 1) else { continue };
            if !matches!(self.t(c + 1), ";" | "=" | "[") || c == j + 2 { continue; }
            let variable = (j + 2..c).any(|x| self.ident(x) && self.t(x) != "sizeof" && !es_macro(self.t(x)) && self.antes(x, 1) != "sizeof");
            let tras_sizeof = (j + 2..c).any(|x| self.t(x) == "sizeof");
            if variable && !tras_sizeof {
                let m = alloc::format!("{} se declara con una longitud evaluada en ejecucion: comprobar que es mayor que cero y que cabe en la pila", self.t(j));
                self.marcar("C-002", j, Fuerza::Revisar, m, i, c + 1);
            }
        }
    }

    // ---------- UB-015: desreferenciar el puntero nulo ----------
    fn null(&mut self) {
        for i in 0..self.k.len() {
            if self.t(i) != "*" || self.t(i + 1) != "(" { continue; }
            let previo = self.antes(i, 1);
            if i > 0 && ((self.ident(i - 1) && previo != "return") || self.numero(i - 1) || matches!(previo, ")" | "]")) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            if self.t(c - 1) != "*" { continue; }
            if matches!(self.t(c + 1), "0" | "NULL" | "nullptr") && !matches!(self.t(c + 2), "+" | "-" | "[") {
                let m = String::from("se lee o escribe a traves de un puntero nulo convertido");
                self.marcar("UB-015", i, Fuerza::Exacto, m, i, c + 2);
            }
        }
    }

    // ---------- UB-005: devolver la direccion de una local ----------
    fn direccion_local(&mut self) {
        let mut locales: BTreeMap<&str, bool> = BTreeMap::new();
        let mut funcion = 0;
        for i in 1..self.k.len() {
            if self.funcion[i] != funcion { funcion = self.funcion[i]; locales.clear(); }
            if !self.en_codigo[i] { continue; }
            if matches!(self.antes(i, 1), ";" | "{" | "}") && (TIPOS.contains(&self.t(i)) || self.t(i) == "struct") && self.antes(i, 1) != "static" {
                let mut j = i;
                if self.t(j) == "struct" { j += 2; }
                while TIPOS.contains(&self.t(j)) || matches!(self.t(j), "const") { j += 1; }
                let puntero = self.t(j) == "*";
                while self.t(j) == "*" { j += 1; }
                if self.ident(j) && matches!(self.t(j + 1), ";" | "=" | "[" | ",") { locales.insert(self.t(j), self.t(j + 1) == "[" && !puntero); }
            }
            if self.t(i) == "static" { if let Some(n) = (i + 1..i + 6).find(|j| self.ident(*j) && matches!(self.t(j + 1), ";" | "=" | "[")) { locales.remove(self.t(n)); } }
            if self.t(i) != "return" { continue; }
            let (amp, n) = if self.t(i + 1) == "&" { (true, i + 2) } else { (false, i + 1) };
            if !self.ident(n) || self.t(n + 1) != ";" { continue; }
            if let Some(arreglo) = locales.get(self.t(n)) {
                if amp || *arreglo {
                    let m = alloc::format!("{} es local: su vida termina al salir de la funcion y la direccion queda colgando", self.t(n));
                    self.marcar("UB-005", i, Fuerza::Exacto, m, i, n + 2);
                }
            }
        }
    }

    // ---------- UB-012: va_arg con un tipo que se promueve ----------
    fn va_arg(&mut self) {
        for i in 0..self.k.len() {
            if !self.llamada(i, &["va_arg"]) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            let a = self.args(i + 1, c);
            if a.len() != 2 { continue; }
            let tipo: Vec<&str> = (a[1].0..a[1].1).map(|j| self.t(j)).collect();
            let promovido = matches!(tipo.as_slice(), ["char"] | ["signed", "char"] | ["unsigned", "char"] | ["short"] | ["short", "int"] | ["unsigned", "short"]
                | ["float"] | ["bool"] | ["_Bool"] | ["int8_t"] | ["uint8_t"] | ["int16_t"] | ["uint16_t"]);
            if promovido {
                let m = alloc::format!("los argumentos variables llegan promovidos: {} viaja como int o double", tipo.join(" "));
                self.marcar("UB-012", i, Fuerza::Exacto, m, i, c + 1);
            }
        }
    }

    // ---------- CXX-002: referencia o iterador invalidado al crecer el vector ----------
    fn vector_invalidado(&mut self) {
        if !self.cpp { return; }
        const CAMBIAN: &[&str] = &["push_back", "emplace_back", "insert", "emplace", "resize", "reserve", "clear", "erase", "assign", "shrink_to_fit"];
        struct Viva<'b> { nombre: &'b str, vector: &'b str, invalidada: Option<(u32, usize, i32)>, fin_bucle: Option<usize>, es_ref: bool }
        let mut vivas: Vec<Viva> = Vec::new();
        let (mut funcion, mut nivel) = (0, 0i32);
        let mut estables: BTreeMap<&str, bool> = BTreeMap::new();
        for i in 0..self.k.len() {
            if self.funcion[i] != funcion { funcion = self.funcion[i]; vivas.clear(); nivel = 0; }
            match self.t(i) {
                "{" => nivel += 1,
                "}" => {
                    nivel -= 1;
                    // Lo que cambio dentro del if no afecta a su else.
                    if self.t(i + 1) == "else" { for v in vivas.iter_mut() { if v.invalidada.is_some_and(|x| x.2 > nivel) { v.invalidada = None; } } }
                }
                _ => {}
            }
            if !self.en_codigo[i] { continue; }
            vivas.retain(|v| v.fin_bucle.is_none_or(|fin| i <= fin));
            // auto& r = v[...] | v.front() | v.back() | v.at(...);  auto it = v.begin();
            if i >= 2 && self.t(i) == "=" && self.ident(i - 1) && self.ident(i + 1) {
                let (r, v) = (self.t(i - 1), self.t(i + 1));
                // auto& r = v.front() es una referencia; auto r = v.front() es una copia (no se invalida).
                let referencia = self.antes(i, 2) == "&" && (self.t(i + 2) == "[" || (self.t(i + 2) == "." && matches!(self.t(i + 3), "front" | "back" | "at")));
                let iterador = matches!(self.antes(i, 2), "auto" | "*") && self.t(i + 2) == "." && matches!(self.t(i + 3), "begin" | "end" | "data");
                if referencia || iterador {
                    vivas.retain(|x| x.nombre != r);
                    vivas.push(Viva { nombre: r, vector: v, invalidada: None, fin_bucle: None, es_ref: self.antes(i, 2) == "&" });
                    continue;
                }
            }
            // for (auto& x : v) { ... v.push_back(...) ... }
            if self.t(i) == "for" && self.t(i + 1) == "(" {
                if let Some(c) = self.cierre(i + 1) {
                    if let Some(dos) = (i + 2..c).find(|j| self.t(*j) == ":") {
                        if dos + 2 == c && self.ident(dos + 1) && self.t(c + 1) == "{" {
                            if let Some(fin) = self.cierre(c + 1) { vivas.push(Viva { nombre: "", vector: self.t(dos + 1), invalidada: None, fin_bucle: Some(fin), es_ref: false }); }
                        }
                    }
                }
            }
            if self.ident(i) && self.t(i + 1) == "." && CAMBIAN.contains(&self.t(i + 2)) && self.t(i + 3) == "(" && !matches!(self.antes(i, 1), "." | "->") {
                let v = self.t(i);
                let estable = *estables.entry(v).or_insert_with(|| self.nodos_estables(v));
                if estable { continue; }
                let linea = self.k[i].linea;
                let cierra = self.cierre(i + 3).unwrap_or(i + 3);
                // it = v.erase(it): el iterador se vuelve a atar al resultado.
                let reata = (i >= 2 && self.antes(i, 1) == "=" && self.ident(i - 2)).then(|| self.antes(i, 2));
                let mut bucle = false;
                for x in vivas.iter_mut().filter(|x| x.vector == v) {
                    if x.nombre.is_empty() { bucle = true; } else if x.invalidada.is_none() && Some(x.nombre) != reata { x.invalidada = Some((linea, cierra, nivel)); }
                }
                if bucle {
                    let m = alloc::format!("{v}.{}() dentro de un for que recorre {v}: puede realocar y dejar el recorrido colgando", self.t(i + 2));
                    self.marcar("CXX-002", i, Fuerza::Revisar, m, i, i + 4);
                    vivas.retain(|x| !(x.nombre.is_empty() && x.vector == v));
                }
                continue;
            }
            if self.ident(i) && !matches!(self.antes(i, 1), "." | "->" | "&" | "auto") {
                let nombre = self.t(i);
                // Un iterador o puntero que se vuelve a asignar ya no apunta a lo viejo.
                if self.t(i + 1) == "=" { vivas.retain(|x| x.nombre != nombre || x.es_ref); }
                if let Some(p) = vivas.iter().position(|x| x.nombre == nombre && x.invalidada.is_some_and(|(_, desde, _)| i > desde)) {
                    let x = vivas.remove(p);
                    let m = alloc::format!("{nombre} apunta dentro de {}, que cambio en la linea {}: puede haber realocado", x.vector, x.invalidada.map(|y| y.0).unwrap_or(0));
                    self.marcar("CXX-002", i, Fuerza::Revisar, m, i, i + 1);
                }
            }
        }
    }

    /// El contenedor se declara en este archivo como set/map/list: sus referencias no se mueven al insertar.
    fn nodos_estables(&self, v: &str) -> bool {
        const NODOS: &[&str] = &["set", "map", "multiset", "multimap", "list", "forward_list", "unordered_set", "unordered_map", "unordered_multiset", "unordered_multimap"];
        for j in 1..self.k.len() {
            if self.k[j].t != v || !matches!(self.antes(j, 1), ">" | ">>") { continue; }
            let mut nivel = 0i32;
            for x in (0..j).rev() {
                match self.t(x) { ">" => nivel += 1, ">>" => nivel += 2, "<" => { nivel -= 1; if nivel == 0 { return NODOS.contains(&self.antes(x, 1)); } } ";" | "{" | "}" => break, _ => {} }
            }
        }
        false
    }

    // ---------- C-003: llamar mediante un tipo de funcion convertido ----------
    fn firma_incompatible(&mut self) {
        for i in 0..self.k.len().saturating_sub(4) {
            if self.t(i) != "(" || self.t(i + 1) != "(" { continue; }
            let (Some(fuera), Some(cast)) = (self.cierre(i), self.cierre(i + 1)) else { continue };
            let puntero_a_funcion = (i + 2..cast).any(|j| self.t(j) == "(" && self.t(j + 1) == "*" && self.t(j + 2) == ")");
            if puntero_a_funcion && self.ident(cast + 1) && fuera == cast + 2 && self.t(fuera + 1) == "(" {
                let m = alloc::format!("{} se llama a traves de otra firma: la conversion no hace compatible el tipo", self.t(cast + 1));
                self.marcar("C-003", i, Fuerza::Revisar, m, i, fuera + 2);
            }
        }
    }

    // ---------- UB-002: division entera por cero o MIN / -1 ----------
    fn division(&mut self) {
        for i in 1..self.k.len() {
            if !matches!(self.t(i), "/" | "%" | "/=" | "%=") { continue; }
            let simple = self.ident(i - 1) || self.numero(i - 1) || self.antes(i, 1) == "]";
            if simple && self.numero(i + 1) && literal(self.k[i + 1].t).is_some_and(|(v, _, _)| v == 0) && !matches!(self.t(i + 2), "." | "[" ) {
                // En C++ el operando puede ser una clase con su operador (CheckedNumeric % 0 no es UB).
                let (fuerza, m) = if self.cpp { (Fuerza::Revisar, "si el operando es un entero, el divisor cero es indefinido") } else { (Fuerza::Exacto, "el divisor es cero: division entera indefinida") };
                self.marcar("UB-002", i, fuerza, m.to_string(), i - 1, i + 2);
            } else if matches!(self.antes(i, 1), "INT_MIN" | "LONG_MIN" | "LLONG_MIN" | "INT32_MIN" | "INT64_MIN") && self.t(i + 1) == "-" && self.t(i + 2) == "1" {
                let m = alloc::format!("{} / -1 no es representable", self.antes(i, 1));
                self.marcar("UB-002", i, Fuerza::Exacto, m, i - 1, i + 3);
            }
        }
    }

    // ---------- UB-001: el entero con signo que se sale (literal) ----------
    fn desborde_literal(&mut self) {
        for i in 1..self.k.len() {
            let (max, op) = (self.antes(i, 1), self.t(i));
            let sale = (matches!(max, "INT_MAX" | "LONG_MAX" | "LLONG_MAX" | "INT32_MAX" | "INT64_MAX") && op == "+")
                || (matches!(max, "INT_MIN" | "LONG_MIN" | "LLONG_MIN" | "INT32_MIN" | "INT64_MIN") && op == "-");
            if !sale || !self.numero(i + 1) || !literal(self.k[i + 1].t).is_some_and(|(v, _, s)| v > 0 && s) { continue; }
            {
                if matches!(self.antes(i, 2), "-" | "+" | "*" | "/" | ")") { continue; }
                let m = alloc::format!("{max} {op} {}: la suma matematica no cabe en el tipo con signo", self.t(i + 1));
                self.marcar("UB-001", i, Fuerza::Exacto, m, i - 1, i + 2);
            }
        }
    }

    // ---------- UB-007 / UB-008: leer un objeto como otro tipo / sin alinear ----------
    fn alias_y_alineacion(&mut self) {
        const ANCHOS: &[&str] = &["int", "long", "short", "float", "double", "unsigned", "int16_t", "int32_t", "int64_t", "uint16_t", "uint32_t", "uint64_t", "size_t"];
        for i in 0..self.k.len() {
            if self.t(i) != "*" || self.t(i + 1) != "(" || matches!(self.antes(i, 1), ")" | "]") || (i > 0 && (self.ident(i - 1) && self.antes(i, 1) != "return" || self.numero(i - 1))) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            if self.t(c - 1) != "*" || c < i + 4 { continue; }
            let tipo: Vec<&str> = (i + 2..c - 1).map(|j| self.t(j)).filter(|t| *t != "const" && *t != "volatile").collect();
            if tipo.iter().any(|t| matches!(*t, "char" | "uint8_t" | "int8_t" | "void" | "u8" | "std" | "byte")) || !tipo.iter().all(|t| ANCHOS.contains(t)) { continue; }
            let es_flotante = tipo.iter().any(|t| matches!(*t, "float" | "double"));
            if self.t(c + 1) == "&" && self.ident(c + 2) && !matches!(self.t(c + 3), "[" | "." | "->") {
                let m = alloc::format!("{} se lee como {}: rompe la regla de alias; copiar con memcpy", self.t(c + 2), tipo.join(" "));
                self.marcar("UB-007", i, Fuerza::Revisar, m, i, c + 3);
            } else if self.t(c + 1) == "(" && !es_flotante {
                if let Some(e) = self.cierre(c + 1) {
                    if (c + 2..e).any(|j| self.t(j) == "+") {
                        let m = alloc::format!("se lee un {} en una direccion desplazada: puede no estar alineada; copiar con memcpy", tipo.join(" "));
                        self.marcar("UB-008", i, Fuerza::Revisar, m, i, e + 1);
                    }
                }
            }
        }
    }

    // ---------- UB-014: setjmp sin volatile ----------
    fn setjmp(&mut self) {
        if self.k.iter().any(|x| x.t == "volatile") { return; }
        for i in 0..self.k.len() {
            if self.llamada(i, &["setjmp", "sigsetjmp", "_setjmp"]) {
                let m = String::from("tras longjmp, las locales cambiadas despues de setjmp y no volatile quedan indeterminadas");
                let c = self.cierre(i + 1).unwrap_or(i + 1);
                self.marcar("UB-014", i, Fuerza::Revisar, m, i, c + 1);
                return;
            }
        }
    }

    // ---------- UB-017: strncpy con todo el tamano no deja el cero final ----------
    fn strncpy_sin_cero(&mut self) {
        for i in 0..self.k.len() {
            if !self.llamada(i, &["strncpy"]) { continue; }
            let Some(c) = self.cierre(i + 1) else { continue };
            let a = self.args(i + 1, c);
            if a.len() != 3 || self.t(a[2].0) != "sizeof" { continue; }
            let mut n = (a[2].0 + 1, a[2].1);
            if self.t(n.0) == "(" && self.t(n.1 - 1) == ")" { n = (n.0 + 1, n.1 - 1); }
            if self.igual(n, a[0]) {
                // Si la linea siguiente pone el cero a mano, esta bien.
                let linea = self.k[c].linea;
                let pone_cero = (c..self.k.len().min(c + 30)).take_while(|j| self.k[*j].linea <= linea + 1).any(|j| self.t(j) == "=" && matches!(self.t(j + 1), "0" | "'\\0'") || self.k[j].t == "'\\0'");
                if pone_cero { continue; }
                let m = alloc::format!("si el origen ocupa todo {}, no queda el cero final y un strlen posterior sigue leyendo", self.texto(a[0].0, a[0].1));
                self.marcar("UB-017", i, Fuerza::Revisar, m, i, c + 1);
            }
        }
    }

    // ---------- UB-004: fuera del arreglo (indice literal o bucle hasta <=) ----------
    fn fuera_del_arreglo(&mut self) {
        // nombre -> (tamano, token del tamano); los nombres repetidos no cuentan.
        let mut tam: BTreeMap<&str, Option<(&str, usize)>> = BTreeMap::new();
        for i in 1..self.k.len() {
            if self.ident(i) && self.t(i + 1) == "[" && (self.numero(i + 2) || (self.ident(i + 2) && es_macro(self.t(i + 2)))) && self.t(i + 3) == "]"
               && matches!(self.t(i + 4), ";" | "=" | "," | "[") && (TIPOS.contains(&self.antes(i, 1)) || self.ident(i - 1) && !matches!(self.antes(i, 1), "return" | "sizeof" | "new" | "delete" | "case")) {
                let e = tam.entry(self.t(i)).or_insert(Some((self.t(i + 2), i)));
                if e.is_some_and(|(_, d)| d != i) { *e = None; }
            }
        }
        for i in 1..self.k.len() {
            // a[K] con K literal >= tamano.
            if self.ident(i) && self.t(i + 1) == "[" && self.numero(i + 2) && self.t(i + 3) == "]" && self.es_acceso(i) {
                if let Some(Some((n, decl))) = tam.get(self.t(i)) {
                    if *decl != i {
                        if let (Some((k, _, _)), Some((n, _, _))) = (literal(self.k[i + 2].t), literal(n)) {
                            if k >= n && self.antes(i, 1) != "&" {
                                let m = alloc::format!("{}[{k}] con {n} elementos: el ultimo indice valido es {}", self.t(i), n - 1);
                                self.marcar("UB-004", i, Fuerza::Exacto, m, i, i + 4);
                            }
                        }
                    }
                }
            }
            // for (i = 0; i <= N; ...) a[i]
            if self.t(i) == "for" && self.t(i + 1) == "(" {
                let Some(c) = self.cierre(i + 1) else { continue };
                let Some(le) = (i + 2..c).find(|j| self.t(*j) == "<=") else { continue };
                if !self.ident(le - 1) || (self.t(le + 2) != ";") { continue; }
                let (var, limite) = (self.t(le - 1), self.t(le + 1));
                let fin = if self.t(c + 1) == "{" { self.cierre(c + 1).unwrap_or(c + 1) } else { (c..self.k.len().min(c + 40)).find(|j| self.t(*j) == ";").unwrap_or(c) };
                for j in c..fin {
                    if self.ident(j) && self.t(j + 1) == "[" && self.t(j + 2) == var && self.t(j + 3) == "]" {
                        if let Some(Some((n, _))) = tam.get(self.t(j)) {
                            if *n == limite {
                                let m = alloc::format!("el bucle llega a {var} == {limite} y {}[{limite}] ya esta fuera (el ultimo es {limite} - 1)", self.t(j));
                                self.marcar("UB-004", le, Fuerza::Revisar, m, i, c + 1);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// `a[4]` es un acceso (no una declaracion ni un parametro): lo que hay delante es un
    /// operador, o una coma dentro de los parentesis de una llamada.
    fn es_acceso(&self, i: usize) -> bool {
        let p = self.antes(i, 1);
        if p == "," {
            let mut nivel = 0i32;
            for j in (0..i).rev() {
                match self.t(j) { ")" => nivel += 1, "(" => { if nivel == 0 { return true; } nivel -= 1; }
                                  ";" | "{" | "}" => return false, _ => {} }
            }
            return false;
        }
        matches!(p, "=" | "(" | ";" | "{" | "}" | "return" | "+" | "-" | "*" | "/" | "%" | "&&" | "||" | "!" | "?" | ":" | "==" | "!=" | "<" | ">"
                 | "<=" | ">=" | "+=" | "-=" | "|" | "^" | "[" | "<<" | ">>")
    }

    /// Desplazamiento literal de un argumento respecto de su base ("p + 4" -> 4, "p" -> 0).
    fn desplazamiento_de(&self, arg: (usize, usize), base: (usize, usize)) -> Option<u128> {
        if base.1 == arg.1 { return Some(0); }
        (self.t(base.1) == "+" && self.numero(base.1 + 1) && base.1 + 2 == arg.1).then(|| literal(self.k[base.1 + 1].t).map(|x| x.0)).flatten()
    }

    // ---------- ASM-003 / ASM-004 / ASM-005: asm en linea de GCC ----------
    fn asm_en_linea(&mut self) {
        // register long r10 asm("r10"): ese registro queda atado a una variable.
        let mut atados: BTreeSet<&'static str> = BTreeSet::new();
        for i in 0..self.k.len() {
            if matches!(self.t(i), "asm" | "__asm__" | "__asm") && self.t(i + 1) == "(" && self.k.get(i + 2).is_some_and(|x| x.clase == Clase::Cadena) && self.t(i + 3) == ")" {
                if let Some(f) = familia(self.k[i + 2].t.trim_matches('"').trim_start_matches('%')) { atados.insert(f); }
            }
        }
        for i in 0..self.k.len() {
            if !matches!(self.t(i), "asm" | "__asm__" | "__asm") { continue; }
            let mut a = i + 1;
            while matches!(self.t(a), "volatile" | "__volatile__" | "goto" | "inline" | "__inline__") { a += 1; }
            if self.t(a) != "(" || !self.k.get(a + 1).is_some_and(|x| x.clase == Clase::Cadena) { continue; }
            let Some(c) = self.cierre(a) else { continue };
            // Secciones separadas por ':' de nivel superior ("::" cuenta como dos).
            let mut secciones: Vec<(usize, usize)> = Vec::new();
            let (mut nivel, mut desde) = (0, a + 1);
            for j in a + 1..c {
                match self.t(j) {
                    "(" | "[" => nivel += 1, ")" | "]" => nivel -= 1,
                    ":" if nivel == 0 => { secciones.push((desde, j)); desde = j + 1; }
                    "::" if nivel == 0 => { secciones.push((desde, j)); secciones.push((j, j)); desde = j + 1; }
                    _ => {}
                }
            }
            secciones.push((desde, c));
            let plantilla: String = (secciones[0].0..secciones[0].1).filter(|j| self.k[*j].clase == Clase::Cadena).map(|j| desescapar(self.k[j].t)).collect();
            let cadenas = |s: Option<&(usize, usize)>| -> Vec<String> {
                s.map(|(x, y)| (*x..*y).filter(|j| self.k[*j].clase == Clase::Cadena).map(|j| desescapar(self.k[j].t)).collect()).unwrap_or_default()
            };
            let salidas = cadenas(secciones.get(1));
            let entradas = cadenas(secciones.get(2));
            let clobbers = cadenas(secciones.get(3));
            let extendido = secciones.len() > 1;
            let memoria = clobbers.iter().any(|x| x == "memory") || salidas.iter().any(|x| x.contains('m'));
            let mut escritos: BTreeSet<&'static str> = clobbers.iter().filter_map(|x| familia(x.trim_start_matches('%'))).collect();
            for s in &salidas { escritos.extend(restricciones(s)); }
            let leidos: BTreeSet<&'static str> = entradas.iter().flat_map(|s| restricciones(s)).collect();
            let instrucciones = instrucciones_de(&plantilla);
            let fin = c + 1;
            if !extendido && !self.en_codigo[i] { continue; } // asm de archivo: define funciones enteras
            let guardados = guardados_en(&instrucciones);
            if instrucciones.is_empty() {
                if extendido && memoria && self.hilos {
                    let m = String::from("asm(\"\" ::: \"memory\") solo frena al compilador: la CPU puede reordenar; std::atomic o un fence de verdad");
                    self.marcar("ASM-004", i, Fuerza::Revisar, m, i, fin);
                }
                continue;
            }
            let mut faltan: Vec<&'static str> = Vec::new();
            let mut divide = false;
            for (mnemo, ops) in &instrucciones {
                for f in implicitos(mnemo) { if !escritos.contains(f) && !faltan.contains(f) && !atados.contains(f) && !guardados.contains(f) { faltan.push(f); } }
                for r in registros_en(ops, if extendido { "%%" } else { "%" }) {
                    if !escritos.contains(r) && !leidos.contains(r) && !faltan.contains(&r) && !atados.contains(r) && r != "rsp" && !guardados.contains(r) { faltan.push(r); }
                }
                if matches!(mnemo.as_str(), "div" | "divq" | "divl" | "idiv" | "idivq" | "idivl") { divide = true; }
            }
            if !faltan.is_empty() {
                let lista: Vec<&str> = faltan.clone();
                let m = if extendido { alloc::format!("la plantilla cambia {} y no esta en las salidas ni en los clobbers: GCC cree que sigue igual", lista.join(", ")) }
                        else { alloc::format!("asm basico que toca {}: no puede declarar clobbers; usar asm extendido", lista.join(", ")) };
                self.marcar("ASM-003", i, Fuerza::Revisar, m, i, fin);
            } else if extendido && !memoria && instrucciones.iter().any(|(m, _)| m.starts_with("rep") || m.starts_with("stos") || m.starts_with("movs") || m.starts_with("cmpxchg") || m == "lock") {
                let m = String::from("la plantilla escribe memoria y no declara \"memory\" ni un operando \"=m\"");
                self.marcar("ASM-003", i, Fuerza::Revisar, m, i, fin);
            }
            if divide && !leidos.contains("rdx") && !escritos.contains("rdx") && !instrucciones.iter().any(|(m, o)| matches!(m.as_str(), "cqo" | "cdq" | "cqto" | "cltd")
                   || (m.starts_with("xor") && o.contains("dx")) || (m.starts_with("mov") && o.ends_with("dx"))) {
                let m = String::from("div usa rdx:rax como dividendo y rdx no se prepara: puede saltar #DE aunque el divisor no sea cero");
                self.marcar("ASM-005", i, Fuerza::Revisar, m, i, fin);
            }
        }
    }
}

/// Un identificador en MAYUSCULAS es (casi siempre) una macro constante.
fn es_macro(t: &str) -> bool { t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') && t.bytes().any(|b| b.is_ascii_uppercase()) }

/// Valor, ancho en bits (LP64) y si tiene signo de un literal entero ("1", "0x1fu", "1ULL").
fn literal(t: &str) -> Option<(u128, u32, bool)> {
    let t = t.replace('\'', "");
    let sufijo_ini = t.trim_end_matches(['u', 'U', 'l', 'L']).len();
    let (cuerpo, sufijo) = t.split_at(sufijo_ini);
    let s = sufijo.to_ascii_lowercase();
    let (largo, sin) = (s.contains('l'), s.contains('u'));
    let valor = if let Some(h) = cuerpo.strip_prefix("0x").or_else(|| cuerpo.strip_prefix("0X")) { u128::from_str_radix(h, 16).ok()? }
                else if let Some(b) = cuerpo.strip_prefix("0b").or_else(|| cuerpo.strip_prefix("0B")) { u128::from_str_radix(b, 2).ok()? }
                else if cuerpo.len() > 1 && cuerpo.starts_with('0') { u128::from_str_radix(&cuerpo[1..], 8).ok()? }
                else { cuerpo.parse::<u128>().ok()? };
    Some((valor, if largo { 64 } else { 32 }, !sin))
}

fn desescapar(t: &str) -> String {
    let s = t.strip_prefix('"').unwrap_or(t);
    let s = s.strip_suffix('"').unwrap_or(s);
    s.replace("\\n", "\n").replace("\\t", " ").replace("\\\"", "\"")
}

// ====================================================================================
// x86-64: registros, instrucciones y archivos de ensamblador (GAS AT&T/Intel y NASM).
// ====================================================================================

/// La familia de 64 bits de un registro ("eax" -> "rax", "r12d" -> "r12").
pub fn familia(r: &str) -> Option<&'static str> {
    const TABLA: &[(&str, &[&str])] = &[
        ("rax", &["rax", "eax", "ax", "al", "ah"]), ("rbx", &["rbx", "ebx", "bx", "bl", "bh"]), ("rcx", &["rcx", "ecx", "cx", "cl", "ch"]),
        ("rdx", &["rdx", "edx", "dx", "dl", "dh"]), ("rsi", &["rsi", "esi", "si", "sil"]), ("rdi", &["rdi", "edi", "di", "dil"]),
        ("rbp", &["rbp", "ebp", "bp", "bpl"]), ("rsp", &["rsp", "esp", "sp", "spl"]),
    ];
    let r = r.trim().to_ascii_lowercase();
    for (f, nombres) in TABLA { if nombres.contains(&r.as_str()) { return Some(f); } }
    const ALTOS: [&str; 8] = ["r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];
    let base = r.trim_end_matches(['d', 'w', 'b']);
    ALTOS.iter().find(|x| **x == base).copied()
}

/// Registros que una restriccion de GCC ata ("=a" -> rax, "+d" -> rdx, "=A" -> rax y rdx).
fn restricciones(s: &str) -> Vec<&'static str> {
    let mut v = Vec::new();
    for c in s.chars() {
        match c { 'a' => v.push("rax"), 'b' => v.push("rbx"), 'c' => v.push("rcx"), 'd' => v.push("rdx"), 'S' => v.push("rsi"), 'D' => v.push("rdi"),
                  'A' => { v.push("rax"); v.push("rdx"); } _ => {} }
    }
    v
}

/// Lo que una instruccion escribe sin nombrarlo.
fn implicitos(m: &str) -> &'static [&'static str] {
    match m {
        "cpuid" => &["rax", "rbx", "rcx", "rdx"],
        "rdtsc" | "rdmsr" | "xgetbv" | "rdpmc" => &["rax", "rdx"],
        "rdtscp" => &["rax", "rdx", "rcx"],
        "syscall" => &["rax", "rcx", "r11"],
        _ => &[],
    }
}

/// (mnemonico, operandos) de una plantilla o de una linea ya sin comentarios.
fn instrucciones_de(texto: &str) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for parte in texto.split(['\n', ';']) {
        let mut p = parte.trim();
        while let Some((etiqueta, resto)) = p.split_once(':').filter(|(e, _)| !e.is_empty() && e.bytes().all(|b| es_ident(b) || b == b'.')) { let _ = etiqueta; p = resto.trim(); }
        if p.is_empty() || p.starts_with('.') || p.starts_with('#') { continue; }
        let (m, ops) = p.split_once([' ', '\t']).unwrap_or((p, ""));
        let m = m.to_ascii_lowercase();
        if m == "lock" || m.starts_with("rep") {
            v.push((m.clone(), String::new()));
            let resto = ops.trim();
            if !resto.is_empty() { let (m2, o2) = resto.split_once([' ', '\t']).unwrap_or((resto, "")); v.push((m2.to_ascii_lowercase(), o2.trim().to_string())); }
            continue;
        }
        v.push((m, ops.trim().to_string()));
    }
    v
}

/// Registros que la propia plantilla guarda y repone: push/pop, xchg, o mov a otro y vuelta
/// (el truco de cpuid con PIC: mov %%ebx, %%edi; cpuid; xchg %%edi, %%ebx).
fn guardados_en(ins: &[(String, String)]) -> BTreeSet<&'static str> {
    let mut g = BTreeSet::new();
    let regs = |o: &str| -> Vec<&'static str> { o.split(',').filter_map(|x| familia(x.trim().trim_start_matches('%'))).collect() };
    let empujados: Vec<&'static str> = ins.iter().filter(|(m, _)| m.starts_with("push")).flat_map(|(_, o)| regs(o)).collect();
    for (m, o) in ins {
        if m.starts_with("pop") { for r in regs(o) { if empujados.contains(&r) { g.insert(r); } } }
        if m.starts_with("xchg") { g.extend(regs(o)); }
    }
    // mov A, B ... mov B, A (en cualquier sintaxis, el par se repone).
    let movs: Vec<Vec<&'static str>> = ins.iter().filter(|(m, _)| m.starts_with("mov")).map(|(_, o)| regs(o)).filter(|v| v.len() == 2).collect();
    for (k, a) in movs.iter().enumerate() {
        if movs[k + 1..].iter().any(|b| b[0] == a[1] && b[1] == a[0]) { g.insert(a[0]); g.insert(a[1]); }
    }
    g
}

/// Registros nombrados con un prefijo ("%%" en asm extendido, "%" en AT&T, "" en Intel).
fn registros_en(ops: &str, prefijo: &str) -> Vec<&'static str> {
    let mut v = Vec::new();
    if prefijo.is_empty() {
        for p in ops.split(|c: char| !c.is_ascii_alphanumeric()) { if let Some(f) = familia(p) { if !v.contains(&f) { v.push(f); } } }
        return v;
    }
    let mut resto = ops;
    while let Some(i) = resto.find(prefijo) {
        resto = &resto[i + prefijo.len()..];
        if resto.starts_with('%') { continue; }
        let fin = resto.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(resto.len());
        if let Some(f) = familia(&resto[..fin]) { if !v.contains(&f) { v.push(f); } }
    }
    v
}

const CONSERVA: &[&str] = &["rbx", "rbp", "r12", "r13", "r14", "r15"];

/// Instrucciones que no escriben su operando destino.
fn no_escribe(m: &str) -> bool {
    m.starts_with('j') || m.starts_with("cmp") || m.starts_with("test") || m.starts_with("push") || m.starts_with("call") || m.starts_with("ret")
        || matches!(m, "bt" | "btl" | "btq" | "nop" | "out" | "outb" | "outw" | "outl" | "int" | "int3" | "hlt" | "cli" | "sti" | "lfence" | "mfence" | "sfence" | "pause" | "ud2" | "invlpg" | "lgdt" | "lidt" | "wrmsr")
}

struct Linea { n: u32, mnemo: String, ops: Vec<String>, etiqueta: Option<String> }

fn operandos(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    let (mut nivel, mut actual) = (0, String::new());
    for c in s.chars() {
        match c { '(' | '[' => { nivel += 1; actual.push(c); } ')' | ']' => { nivel -= 1; actual.push(c); }
                  ',' if nivel == 0 => { v.push(actual.trim().to_string()); actual.clear(); } _ => actual.push(c) }
    }
    if !actual.trim().is_empty() { v.push(actual.trim().to_string()); }
    v
}

/// Un archivo .s/.S/.asm entero: ASM-001 (pila al llamar), ASM-002 (registros que se
/// conservan) y ASM-005 (div sin preparar rdx), por funcion global.
fn asm_archivo(texto: &str, nasm: bool, cpp: bool) -> Vec<Sospecha> {
    let mut out = Vec::new();
    let bajo = texto.to_ascii_lowercase();
    // Codigo de 16 o 32 bits (arranque, modo real): otra ABI. No se juzga.
    if [".code16", ".code32", "bits 16", "bits 32", "use16", "use32"].iter().any(|x| bajo.contains(x)) { return out; }
    let intel = nasm || bajo.contains(".intel_syntax");
    let mut lineas: Vec<Linea> = Vec::new();
    let mut globales: BTreeSet<String> = BTreeSet::new();
    let mut en_comentario = false;
    for (k, crudo) in texto.lines().enumerate() {
        let n = k as u32 + 1;
        let mut l = String::from(crudo);
        if cpp {
            // /* */ y // del preprocesador.
            let mut limpio = String::new();
            let mut resto = l.as_str();
            loop {
                if en_comentario { match resto.find("*/") { Some(p) => { resto = &resto[p + 2..]; en_comentario = false; } None => { resto = ""; break; } } }
                match resto.find("/*") { Some(p) => { limpio.push_str(&resto[..p]); resto = &resto[p + 2..]; en_comentario = true; } None => break }
            }
            limpio.push_str(resto);
            if let Some(p) = limpio.find("//") { limpio.truncate(p); }
            if limpio.trim_start().starts_with('#') { continue; }
            l = limpio;
        }
        let corte = if nasm { l.find(';') } else { l.find('#') };
        if let Some(p) = corte { l.truncate(p); }
        let partes: Vec<&str> = if nasm { vec![l.as_str()] } else { l.split(';').collect() };
        for p in partes {
            let mut p = p.trim();
            let mut etiqueta = None;
            if let Some((e, resto)) = p.split_once(':').filter(|(e, _)| !e.is_empty() && e.bytes().all(|b| es_ident(b) || b == b'.')) {
                etiqueta = Some(e.to_string());
                p = resto.trim();
            }
            if let Some(e) = etiqueta.take() { lineas.push(Linea { n, mnemo: String::new(), ops: Vec::new(), etiqueta: Some(e) }); }
            if p.is_empty() { continue; }
            let (m, ops) = p.split_once([' ', '\t']).unwrap_or((p, ""));
            let m = m.to_ascii_lowercase();
            if matches!(m.as_str(), ".globl" | ".global" | "global") {
                for g in ops.split(',') { globales.insert(g.trim().trim_end_matches(":function").to_string()); }
                continue;
            }
            if m.starts_with('.') || m.starts_with('%') || matches!(m.as_str(), "section" | "extern" | "align" | "db" | "dw" | "dd" | "dq" | "default" | "bits") { continue; }
            lineas.push(Linea { n, mnemo: m, ops: operandos(ops), etiqueta: None });
        }
    }
    let reg = |op: &str| -> Option<&'static str> { familia(op.trim().trim_start_matches('%')) };
    let destino = |l: &Linea| -> Option<&'static str> {
        if no_escribe(&l.mnemo) || l.ops.is_empty() { return None; }
        let op = if intel { l.ops.first()? } else { l.ops.last()? };
        reg(op)
    };
    // Funciones: de cada etiqueta global a la siguiente etiqueta global.
    let inicios: Vec<usize> = lineas.iter().enumerate().filter(|(_, l)| l.etiqueta.as_ref().is_some_and(|e| globales.contains(e))).map(|(i, _)| i).collect();
    for (k, &ini) in inicios.iter().enumerate() {
        let fin = inicios.get(k + 1).copied().unwrap_or(lineas.len());
        let cuerpo = &lineas[ini + 1..fin];
        let nombre = lineas[ini].etiqueta.clone().unwrap_or_default();
        // No vuelve (ni con ret ni saltando de cola a otra funcion global): no es una funcion normal.
        let vuelve = cuerpo.iter().any(|l| l.mnemo.starts_with("ret") || (l.mnemo == "jmp" && l.ops.first().is_some_and(|o| globales.contains(o.trim()))));
        if !vuelve { continue; }
        // ASM-002
        let guardados: BTreeSet<&str> = cuerpo.iter().filter(|l| l.mnemo.starts_with("push")).filter_map(|l| l.ops.first().and_then(|o| reg(o))).collect();
        let mut avisados = BTreeSet::new();
        for l in cuerpo {
            let mut escritos: Vec<&'static str> = destino(l).into_iter().collect();
            if l.mnemo == "cpuid" { escritos.push("rbx"); }
            if l.mnemo.starts_with("xchg") { escritos.extend(l.ops.iter().filter_map(|o| reg(o))); }
            for f in escritos {
                if CONSERVA.contains(&f) && !guardados.contains(f) && avisados.insert(f) {
                    let visto = alloc::format!("{} {}", l.mnemo, l.ops.join(", "));
                    let motivo = alloc::format!("{nombre} cambia {f} sin guardarlo: en System V AMD64 la funcion llamada debe conservarlo");
                    out.push(Sospecha { ficha: "ASM-002", linea: l.n, columna: 1, fuerza: Fuerza::Revisar, visto, motivo });
                }
            }
        }
        // ASM-001: desde la entrada, recto hasta el primer salto o etiqueta.
        let mut delta: i64 = 0;
        for l in cuerpo {
            if l.etiqueta.is_some() || l.mnemo.starts_with('j') || l.mnemo == "enter" { break; }
            let rsp_destino = destino(l) == Some("rsp");
            if l.mnemo.starts_with("push") { delta += 8; continue; }
            if l.mnemo.starts_with("pop") { delta -= 8; continue; }
            if rsp_destino {
                let fuente = if intel { l.ops.get(1) } else { l.ops.first() };
                let imm = fuente.and_then(|f| inmediato(f));
                match (l.mnemo.trim_end_matches('q'), imm) { ("sub", Some(v)) => { delta += v; continue; } ("add", Some(v)) => { delta -= v; continue; } _ => break }
            }
            if l.mnemo.starts_with("call") {
                if (8 + delta).rem_euclid(16) != 0 {
                    let visto = alloc::format!("{} {}", l.mnemo, l.ops.join(", "));
                    let motivo = alloc::format!("en {nombre} la pila llega a call movida 8 + {delta} bytes desde la entrada: no es multiplo de 16");
                    out.push(Sospecha { ficha: "ASM-001", linea: l.n, columna: 1, fuerza: Fuerza::Revisar, visto, motivo });
                }
                break;
            }
        }
        // ASM-005
        for (j, l) in cuerpo.iter().enumerate() {
            let m = l.mnemo.as_str();
            let firma = matches!(m, "idiv" | "idivq" | "idivl");
            if !(firma || matches!(m, "div" | "divq" | "divl")) { continue; }
            if l.ops.first().is_some_and(|o| matches!(o.trim().trim_start_matches('%'), "bl" | "cl" | "dl" | "al" | "sil" | "dil") || o.starts_with("byte")) { continue; }
            let previas = cuerpo[..j].iter().rev().take_while(|p| p.etiqueta.is_none() && !p.mnemo.starts_with('j')).take(8);
            let prepara = previas.into_iter().any(|p| matches!(p.mnemo.as_str(), "cqo" | "cdq" | "cqto" | "cltd") || destino(p) == Some("rdx"));
            if !prepara {
                let visto = alloc::format!("{} {}", l.mnemo, l.ops.join(", "));
                let motivo = String::from("rdx no se prepara antes de dividir (xor edx, edx / cqo): #DE con divisor no nulo si el cociente no cabe");
                out.push(Sospecha { ficha: "ASM-005", linea: l.n, columna: 1, fuerza: Fuerza::Revisar, visto, motivo });
            }
        }
    }
    out
}

fn inmediato(op: &str) -> Option<i64> {
    let s = op.trim().trim_start_matches('$');
    if let Some(h) = s.strip_prefix("0x") { return i64::from_str_radix(h, 16).ok(); }
    s.parse().ok()
}

/// Resumen para la plataforma: cuantas por ficha (ordenado por id).
pub fn por_ficha<'a>(todas: impl Iterator<Item = &'a Sospecha>) -> BTreeMap<&'static str, u32> {
    let mut m = BTreeMap::new();
    for s in todas { *m.entry(s.ficha).or_insert(0) += 1; }
    m
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn lexer_quita_comentarios_y_cadenas() {
        let t = tokens("int a = 1; // free(p+1)\n/* free(p+1) */ char *s = \"free(p+1)\";\n#define X(a) (1 << 40)\n");
        assert!(!t.iter().any(|x| x.t == "free"));
        assert!(t.iter().any(|x| x.t == "<<" && x.linea == 3), "el cuerpo del define se lee");
    }
    #[test]
    fn literales() {
        assert_eq!(literal("1"), Some((1, 32, true)));
        assert_eq!(literal("0x10UL"), Some((16, 64, false)));
        assert_eq!(literal("1'000"), Some((1000, 32, true)));
    }
}
