// SPDX-License-Identifier: GPL-2.0-only
//! La vista Ciudad (inspirada en CodeCity): el proyecto como una ciudad en 3D.
//!
//! - **Distritos = carpetas**, anidados (treemap "squarified", Bruls et al. 2000), en
//!   terrazas: cada nivel un poco mas alto. Cada uno sabe que parte del total ocupa.
//! - **Edificios = archivos.** Huella (area) = tamano en bytes; altura = lineas (u otra
//!   medida); color = lenguaje, salud (errores) o busqueda. Ver `Altura` y `Color`.
//! - Proyectos enormes (Chromium: 509 417 archivos): las carpetas profundas se juntan en
//!   **bloques** hasta que caben MAX_EDIFICIOS; un bloque dice cuantos archivos lleva.
//!
//! Puro (no_std, sin libm: la raiz cuadrada es de Newton). La plataforma lo dibuja.

use crate::tipos::{tipo_por_extension, Tipo};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub const LADO: f32 = 1000.0;
pub const MAX_EDIFICIOS: usize = 600_000;
/// Altura de cada terraza de distrito.
pub const TERRAZA: f32 = 3.0;

/// Que dice la altura de un edificio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Altura { #[default] Lineas, Cambios, Errores, Coincidencias, Plana }

/// Que dice la huella (el area): bytes (todo lo que ocupa) o lineas (solo el texto, como CodeCity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Huella { #[default] Bytes, Lineas }

impl Huella {
    pub fn siguiente(self) -> Self { match self { Huella::Bytes => Huella::Lineas, Huella::Lineas => Huella::Bytes } }
    pub fn nombre(self) -> &'static str { match self { Huella::Bytes => "tamano en bytes", Huella::Lineas => "lineas de texto" } }
}

/// Que dice el color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Color { #[default] Lenguaje, Calor, Edad, Salud, Triangulo, Busqueda }

impl Altura {
    pub fn siguiente(self) -> Self {
        match self { Altura::Lineas => Altura::Cambios, Altura::Cambios => Altura::Errores, Altura::Errores => Altura::Coincidencias, Altura::Coincidencias => Altura::Plana, Altura::Plana => Altura::Lineas }
    }
    pub fn nombre(self) -> &'static str {
        match self { Altura::Lineas => "lineas", Altura::Cambios => "cambios en git", Altura::Errores => "errores y avisos", Altura::Coincidencias => "resultados de busqueda", Altura::Plana => "plana (mapa)" }
    }
}
impl Color {
    pub fn siguiente(self) -> Self {
        match self { Color::Lenguaje => Color::Calor, Color::Calor => Color::Edad, Color::Edad => Color::Salud, Color::Salud => Color::Triangulo, Color::Triangulo => Color::Busqueda, Color::Busqueda => Color::Lenguaje }
    }
    pub fn nombre(self) -> &'static str {
        match self { Color::Lenguaje => "lenguaje", Color::Calor => "calor (cambios en git)", Color::Edad => "edad (ultimo cambio)", Color::Salud => "salud (errores)", Color::Triangulo => "triangulo (seguridad, rendimiento, elegancia)", Color::Busqueda => "busqueda" }
    }
}

/// Un archivo (o un bloque de una carpeta entera) para la ciudad.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchivoCiudad {
    /// Ruta relativa a la carpeta abierta (de un bloque: la carpeta).
    pub ruta: String,
    pub tam: u64,
    /// Lineas contadas (None: sin contar, binario o demasiado grande).
    pub lineas: Option<u32>,
    /// Archivos que representa (1 en un archivo).
    pub archivos: u32,
    /// Es un bloque: una carpeta honda entera juntada en un edificio.
    pub bloque: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Distrito {
    pub ruta: String,
    pub nombre: String,
    /// x, z, ancho, fondo (en el suelo).
    pub rect: [f32; 4],
    pub nivel: u32,
    /// Altura de su suelo (terraza).
    pub base: f32,
    pub tam: u64,
    pub archivos: u32,
    /// Parte del total de la ciudad (0..100).
    pub porcentaje: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edificio {
    pub ruta: String,
    pub rect: [f32; 4],
    pub base: f32,
    pub altura: f32,
    pub tam: u64,
    pub lineas: Option<u32>,
    pub archivos: u32,
    pub bloque: bool,
    pub tipo: Tipo,
    /// Parte de su distrito y del total (0..100).
    pub del_distrito: f32,
    pub del_total: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ciudad {
    pub raiz: String,
    pub distritos: Vec<Distrito>,
    pub edificios: Vec<Edificio>,
    pub tam: u64,
    pub archivos: u64,
    pub lineas: u64,
    /// Hay bloques (carpetas juntadas por ser demasiadas).
    pub con_bloques: bool,
}

/// Raiz cuadrada sin libm (Newton; suficiente para dibujar).
pub fn raiz2(x: f32) -> f32 {
    if x <= 0.0 { return 0.0; }
    let mut r = f32::from_bits((x.to_bits() >> 1) + 0x1fbd_1df5);
    for _ in 0..4 { r = 0.5 * (r + x / r); }
    r
}

/// Es texto (codigo, documentos, config): se le cuentan o estiman lineas. Lo demas (imagenes,
/// audio, binarios) no tiene lineas: queda bajito, que no parezca codigo.
fn es_texto(a: &ArchivoCiudad) -> bool {
    a.bloque || a.lineas.is_some() || matches!(tipo_por_extension(a.ruta.rsplit('/').next().unwrap_or("")), Tipo::Codigo | Tipo::Documento)
}

/// Lineas de un archivo: las contadas o, si es texto sin contar aun, estimadas por tamano.
/// (lineas, estimadas)
pub fn lineas_de(a: &ArchivoCiudad) -> (u64, bool) {
    match a.lineas {
        Some(l) => (l as u64, false),
        None if es_texto(a) => (a.tam / 40, true),
        None => (0, false),
    }
}

/// Peso para el area: hasta un archivo vacio ocupa algo.
fn peso(a: &ArchivoCiudad, huella: Huella) -> f64 {
    match huella {
        Huella::Bytes => a.tam.max(512 * a.archivos as u64) as f64,
        Huella::Lineas => (lineas_de(a).0 * 40).max(64 * a.archivos as u64) as f64,
    }
}

/// Junta carpetas profundas en bloques hasta que caben `max` edificios.
///
/// Con las rutas en orden alfabetico, todas las que empiezan por "a/b/" quedan seguidas:
/// contar las claves a una profundidad es una sola pasada, y la profundidad se busca a
/// saltos (binaria). Chromium entero: 509 418 archivos en una fraccion de segundo.
pub fn juntar(mut archivos: Vec<ArchivoCiudad>, raiz: &str, max: usize) -> Vec<ArchivoCiudad> {
    if archivos.len() <= max { return archivos; }
    let pre = if raiz.is_empty() { 0 } else { raiz.len() + 1 };
    archivos.sort_unstable_by(|a, b| a.ruta.cmp(&b.ruta));
    let cuantos = |d: usize| {
        let (mut n, mut previa): (usize, Option<&str>) = (0, None);
        for a in &archivos {
            let k = cortar(&a.ruta, pre, d);
            if previa != Some(k) { n += 1; previa = Some(k); }
        }
        n
    };
    let maxima = archivos.iter().map(|a| a.ruta[pre.min(a.ruta.len())..].matches('/').count()).max().unwrap_or(0);
    // La mayor profundidad con la que caben (cuantos crece con d).
    let (mut lo, mut hi) = (0usize, maxima);
    while lo < hi {
        let m = (lo + hi).div_ceil(2);
        if cuantos(m) <= max { lo = m; } else { hi = m - 1; }
    }
    let mut salida: Vec<ArchivoCiudad> = Vec::new();
    for a in archivos {
        let clave = cortar(&a.ruta, pre, lo);
        if clave.len() == a.ruta.len() { salida.push(a); continue; }
        match salida.last_mut() {
            Some(b) if b.bloque && b.ruta == clave => {
                b.tam = b.tam.saturating_add(a.tam);
                b.lineas = match (b.lineas, a.lineas) { (Some(x), Some(y)) => Some(x.saturating_add(y)), (x, None) => x, (None, y) => y };
                b.archivos += a.archivos;
            }
            _ => salida.push(ArchivoCiudad { ruta: clave.to_string(), tam: a.tam, lineas: a.lineas, archivos: a.archivos, bloque: true }),
        }
    }
    salida
}

/// La ruta hasta `d` carpetas por debajo de la raiz (si es mas honda), si no, entera.
fn cortar(ruta: &str, pre: usize, d: usize) -> &str {
    let resto = &ruta[pre.min(ruta.len())..];
    match resto.match_indices('/').nth(d) { Some((i, _)) => &ruta[..pre + i], None => ruta }
}

/// Coloca pesos en un rectangulo (squarified treemap). Devuelve un rect por peso, en orden.
pub fn squarify(pesos: &[f64], rect: [f32; 4]) -> Vec<[f32; 4]> {
    let mut salida = alloc::vec![[0.0f32; 4]; pesos.len()];
    let total: f64 = pesos.iter().sum();
    if pesos.is_empty() || total <= 0.0 || rect[2] <= 0.0 || rect[3] <= 0.0 { return salida; }
    let mut orden: Vec<usize> = (0..pesos.len()).collect();
    orden.sort_by(|a, b| pesos[*b].partial_cmp(&pesos[*a]).unwrap_or(core::cmp::Ordering::Equal));
    let escala = rect[2] as f64 * rect[3] as f64 / total;
    let areas: Vec<f64> = orden.iter().map(|i| pesos[*i] * escala).collect();
    let (mut x, mut y, mut w, mut h) = (rect[0] as f64, rect[1] as f64, rect[2] as f64, rect[3] as f64);
    let peor = |fila: &[f64], lado: f64| -> f64 {
        let s: f64 = fila.iter().sum();
        let (mx, mn) = fila.iter().fold((0.0f64, f64::MAX), |(a, b), v| (a.max(*v), b.min(*v)));
        if s <= 0.0 || mn <= 0.0 { return f64::MAX; }
        ((lado * lado * mx) / (s * s)).max((s * s) / (lado * lado * mn))
    };
    let mut i = 0;
    while i < areas.len() {
        let lado = w.min(h).max(1e-9);
        let mut j = i + 1;
        while j < areas.len() && peor(&areas[i..j + 1], lado) <= peor(&areas[i..j], lado) { j += 1; }
        let fila = &areas[i..j];
        let s: f64 = fila.iter().sum();
        if w >= h {
            // Columna a la izquierda.
            let ancho = (s / h.max(1e-9)).min(w);
            let mut yy = y;
            for (k, a) in fila.iter().enumerate() {
                let alto = if ancho > 0.0 { a / ancho } else { 0.0 };
                salida[orden[i + k]] = [x as f32, yy as f32, ancho as f32, alto as f32];
                yy += alto;
            }
            x += ancho;
            w -= ancho;
        } else {
            let alto = (s / w.max(1e-9)).min(h);
            let mut xx = x;
            for (k, a) in fila.iter().enumerate() {
                let ancho = if alto > 0.0 { a / alto } else { 0.0 };
                salida[orden[i + k]] = [xx as f32, y as f32, ancho as f32, alto as f32];
                xx += ancho;
            }
            y += alto;
            h -= alto;
        }
        i = j;
    }
    salida
}

/// Una medida extra por edificio (errores, coincidencias) para la altura.
pub type Medida<'a> = &'a dyn Fn(&str) -> u32;

/// La ciudad de `raiz` (relativa; "" = todo) con los archivos dados (rutas bajo la raiz).
pub fn construir(raiz: &str, archivos: &[ArchivoCiudad], huella: Huella, altura: Altura, medida: Medida) -> Ciudad {
    // Arbol de carpetas: hijos (carpeta) y archivos de cada una.
    let mut carpetas: BTreeMap<String, (Vec<String>, Vec<usize>)> = BTreeMap::new();
    carpetas.insert(raiz.to_string(), (Vec::new(), Vec::new()));
    for (i, a) in archivos.iter().enumerate() {
        if !(raiz.is_empty() || a.ruta.starts_with(raiz) && a.ruta.as_bytes().get(raiz.len()) == Some(&b'/')) { continue; }
        let padre = a.ruta.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let padre = if padre.len() < raiz.len() { raiz } else { padre };
        // La carpeta del archivo; si es nueva, se crea su cadena hasta una que ya existia
        // (asi cada carpeta se sube una sola vez, no una por archivo).
        let nueva = !carpetas.contains_key(padre);
        carpetas.entry(padre.to_string()).or_default().1.push(i);
        if nueva {
            let mut actual = padre.to_string();
            while actual.len() > raiz.len() {
                let arriba = actual.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default();
                let arriba = if arriba.len() < raiz.len() { raiz.to_string() } else { arriba };
                let existia = carpetas.contains_key(&arriba);
                carpetas.entry(arriba.clone()).or_default().0.push(actual.clone());
                if existia || arriba == actual { break; }
                actual = arriba;
            }
        }
    }
    // Peso de cada carpeta (de abajo arriba).
    fn peso_carpeta(c: &str, carpetas: &BTreeMap<String, (Vec<String>, Vec<usize>)>, archivos: &[ArchivoCiudad], huella: Huella,
                    memo: &mut BTreeMap<String, (f64, u64, u32)>) -> (f64, u64, u32) {
        if let Some(v) = memo.get(c) { return *v; }
        let Some((hijos, propios)) = carpetas.get(c) else { return (0.0, 0, 0) };
        let mut r = (0.0, 0u64, 0u32);
        for i in propios { r.0 += peso(&archivos[*i], huella); r.1 = r.1.saturating_add(archivos[*i].tam); r.2 += archivos[*i].archivos; }
        for h in hijos { let p = peso_carpeta(h, carpetas, archivos, huella, memo); r.0 += p.0; r.1 = r.1.saturating_add(p.1); r.2 += p.2; }
        memo.insert(c.to_string(), r);
        r
    }
    let mut memo = BTreeMap::new();
    let total = peso_carpeta(raiz, &carpetas, archivos, huella, &mut memo);
    let lineas_max = archivos.iter().map(|a| lineas_de(a).0 / a.archivos.max(1) as u64).max().unwrap_or(1).max(1);
    let medida_max = archivos.iter().map(|a| medida(&a.ruta)).max().unwrap_or(0).max(1);
    let mut ciudad = Ciudad { raiz: raiz.to_string(), tam: total.1, archivos: total.2 as u64, ..Ciudad::default() };
    ciudad.lineas = archivos.iter().map(|a| a.lineas.unwrap_or(0) as u64).sum();
    ciudad.con_bloques = archivos.iter().any(|a| a.bloque);
    // Recorrido: cada carpeta reparte su rect entre subcarpetas y archivos.
    let mut pila: Vec<(String, [f32; 4], u32)> = alloc::vec![(raiz.to_string(), [0.0, 0.0, LADO, LADO], 0)];
    let vacio: (Vec<String>, Vec<usize>) = (Vec::new(), Vec::new());
    while let Some((c, rect, nivel)) = pila.pop() {
        let (hijos, propios) = carpetas.get(&c).unwrap_or(&vacio);
        let (pc, tam_c, n_c) = memo.get(&c).copied().unwrap_or((0.0, 0, 0));
        let base = nivel as f32 * TERRAZA;
        ciudad.distritos.push(Distrito { ruta: c.clone(), nombre: if c.len() <= raiz.len() { String::from("(raiz)") } else { c.rsplit('/').next().unwrap_or(&c).to_string() },
                                         rect, nivel, base, tam: tam_c, archivos: n_c, porcentaje: if total.0 > 0.0 { (pc / total.0 * 100.0) as f32 } else { 0.0 } });
        // Margen: la calle alrededor (mas ancha en los distritos grandes).
        let m = (rect[2].min(rect[3]) * 0.03).clamp(0.4, 8.0);
        let dentro = [rect[0] + m, rect[1] + m, (rect[2] - 2.0 * m).max(0.0), (rect[3] - 2.0 * m).max(0.0)];
        let mut pesos: Vec<f64> = hijos.iter().map(|h| memo.get(h).map(|x| x.0).unwrap_or(0.0)).collect();
        pesos.extend(propios.iter().map(|i| peso(&archivos[*i], huella)));
        let rects = squarify(&pesos, dentro);
        for (k, h) in hijos.iter().enumerate() { pila.push((h.clone(), rects[k], nivel + 1)); }
        for (k, i) in propios.iter().enumerate() {
            let a = &archivos[*i];
            let r = rects[hijos.len() + k];
            // Un hueco entre edificios para que se distingan (mas pequeno que la calle).
            let g = (r[2].min(r[3]) * 0.12).min(1.5);
            let rect = [r[0] + g, r[1] + g, (r[2] - 2.0 * g).max(0.05), (r[3] - 2.0 * g).max(0.05)];
            let por_archivo = lineas_de(a).0 / a.archivos.max(1) as u64;
            let alto = match altura {
                Altura::Lineas => 1.0 + 140.0 * raiz2(por_archivo as f32 / lineas_max as f32),
                Altura::Cambios | Altura::Errores | Altura::Coincidencias => 0.6 + 140.0 * raiz2(medida(&a.ruta) as f32 / medida_max as f32),
                Altura::Plana => 0.6,
            };
            let nombre = a.ruta.rsplit('/').next().unwrap_or(&a.ruta);
            ciudad.edificios.push(Edificio {
                ruta: a.ruta.clone(), rect, base: base + 0.01, altura: alto, tam: a.tam, lineas: a.lineas, archivos: a.archivos, bloque: a.bloque,
                tipo: if a.bloque { Tipo::Carpeta } else { tipo_por_extension(nombre) },
                del_distrito: if pc > 0.0 { (peso(a, huella) / pc * 100.0) as f32 } else { 0.0 },
                del_total: if total.0 > 0.0 { (peso(a, huella) / total.0 * 100.0) as f32 } else { 0.0 },
            });
        }
    }
    ciudad
}

/// Color por lenguaje (inspirado en los de GitHub Linguist, aclarados para fondo oscuro):
/// asi Rust, C, C++ y sus cabeceras se distinguen de un vistazo.
pub fn color_lenguaje(nombre: &str, bloque: bool) -> crate::tipos::Rgb {
    if bloque { return [110, 122, 145]; }
    let ext = nombre.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    let e = |l: &[&str]| l.iter().any(|x| x.eq_ignore_ascii_case(ext));
    if e(&["rs"]) { [222, 140, 90] }
    else if e(&["c"]) { [90, 145, 235] }
    else if e(&["h"]) { [140, 185, 250] }
    else if e(&["cc", "cpp", "cxx", "c++", "cppm", "ixx", "mm", "m"]) { [240, 80, 130] }
    else if e(&["hpp", "hh", "hxx", "inl", "ipp", "tpp"]) { [250, 140, 175] }
    else if e(&["cu", "cuh"]) { [118, 185, 0] }
    else if e(&["py", "pyi"]) { [255, 212, 59] }
    else if e(&["js", "mjs", "cjs", "jsx"]) { [241, 224, 90] }
    else if e(&["ts", "tsx", "mts"]) { [70, 150, 230] }
    else if e(&["go"]) { [0, 190, 225] }
    else if e(&["java", "kt", "kts"]) { [200, 130, 60] }
    else if e(&["zig"]) { [236, 145, 92] }
    else if e(&["s", "asm", "nasm"]) { [180, 140, 90] }
    else if e(&["sh", "bash", "zsh", "ps1"]) { [137, 224, 81] }
    else if e(&["html", "htm"]) { [227, 96, 58] }
    else if e(&["css", "scss", "sass"]) { [150, 120, 220] }
    else if e(&["wgsl", "glsl", "hlsl", "vert", "frag", "comp"]) { [90, 210, 175] }
    else if e(&["md", "txt", "rst"]) { [190, 190, 200] }
    else if e(&["json", "toml", "yaml", "yml", "xml", "gn", "gni", "cmake"]) { [150, 155, 170] }
    else if e(&["png", "jpg", "jpeg", "svg", "gif", "webp"]) { [192, 90, 255] }
    else { [95, 100, 112] }
}

/// Familias para la leyenda: (nombre, color, archivos), de mas a menos.
pub fn leyenda_lenguajes(ciudad: &Ciudad) -> Vec<(&'static str, crate::tipos::Rgb, u64)> {
    let familias: &[(&str, &str)] = &[("Rust", "x.rs"), ("C", "x.c"), ("cabeceras .h", "x.h"), ("C++", "x.cc"), ("cabeceras C++", "x.hpp"),
        ("CUDA", "x.cu"), ("Python", "x.py"), ("JavaScript", "x.js"), ("TypeScript", "x.ts"), ("Go", "x.go"), ("Java/Kotlin", "x.java"),
        ("Zig", "x.zig"), ("ensamblador", "x.asm"), ("shell", "x.sh"), ("HTML", "x.html"), ("CSS", "x.css"), ("shaders", "x.wgsl"),
        ("texto", "x.md"), ("config", "x.json"), ("imagenes", "x.png")];
    let mut v: Vec<(&'static str, crate::tipos::Rgb, u64)> = familias.iter().map(|(n, ej)| (*n, color_lenguaje(ej, false), 0u64)).collect();
    v.push(("otros", color_lenguaje("x", false), 0));
    v.push(("bloques", color_lenguaje("", true), 0));
    for e in &ciudad.edificios {
        let c = color_lenguaje(e.ruta.rsplit('/').next().unwrap_or(""), e.bloque);
        if let Some(f) = v.iter_mut().find(|f| f.1 == c) { f.2 += e.archivos as u64; }
    }
    v.retain(|f| f.2 > 0);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    v
}

/// Calor: de frio (azul, casi no cambia) a caliente (rojo, cambia todo el rato), pasando
/// por amarillo. `t` en 0..1. Sin cambios en lo leido: gris oscuro.
pub fn color_calor(t: Option<f32>) -> crate::tipos::Rgb {
    let Some(t) = t else { return [52, 56, 68] };
    let t = t.clamp(0.0, 1.0);
    let paradas: [(f32, [f32; 3]); 4] = [(0.0, [40.0, 90.0, 200.0]), (0.4, [60.0, 190.0, 200.0]), (0.7, [250.0, 205.0, 60.0]), (1.0, [240.0, 60.0, 60.0])];
    let mut k = 0;
    while k + 2 < paradas.len() && t > paradas[k + 1].0 { k += 1; }
    let (a, b) = (paradas[k], paradas[k + 1]);
    let u = ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    [(a.1[0] + (b.1[0] - a.1[0]) * u) as u8, (a.1[1] + (b.1[1] - a.1[1]) * u) as u8, (a.1[2] + (b.1[2] - a.1[2]) * u) as u8]
}
