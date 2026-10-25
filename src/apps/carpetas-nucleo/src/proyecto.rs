// SPDX-License-Identifier: GPL-2.0-only
//! Todo el proyecto, no solo lo desplegado: el indice de rutas, "Ir a archivo" (Ctrl+P,
//! busqueda difusa de nombres) y "Buscar en el proyecto" (Ctrl+Shift+F, texto dentro de
//! los archivos). Aqui va la logica pura; la plataforma recorre el disco en sus hilos.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Rutas como mucho en el indice (Chromium entero son 511 951 archivos + 46 370 carpetas).
pub const MAX_INDICE: usize = 2_000_000;
pub const MAX_RESULTADOS_IR_A: usize = 50;
/// Coincidencias de texto como mucho en todo el proyecto, y por archivo.
pub const MAX_COINCIDENCIAS: usize = 20_000;
pub const MAX_POR_ARCHIVO: usize = 500;
pub const MAX_VISTA: usize = 240;

/// Rutas relativas en un solo bloque de texto (sin una reserva por ruta) y su copia en
/// minusculas ASCII, para puntuar sin convertir letra a letra (Chromium entero: ~55 MB).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Indice { texto: String, minus: Vec<u8>, inicios: Vec<u32>, nombres: Vec<u32>, carpetas: Vec<bool>, tams: Vec<u64> }

impl Indice {
    pub fn agregar(&mut self, ruta: &str, carpeta: bool) -> bool { self.agregar_con(ruta, carpeta, 0) }

    /// Con su tamano en bytes (para la vista Ciudad).
    pub fn agregar_con(&mut self, ruta: &str, carpeta: bool, tam: u64) -> bool {
        if self.inicios.len() >= MAX_INDICE || self.texto.len() + ruta.len() > u32::MAX as usize { return false; }
        let inicio = self.texto.len();
        self.inicios.push(inicio as u32);
        self.nombres.push((inicio + ruta.rfind('/').map(|i| i + 1).unwrap_or(0)) as u32);
        self.texto.push_str(ruta);
        self.minus.extend(ruta.bytes().map(|b| b.to_ascii_lowercase()));
        self.carpetas.push(carpeta);
        self.tams.push(tam);
        true
    }
    pub fn tam(&self, i: usize) -> u64 { self.tams[i] }
    pub fn len(&self) -> usize { self.inicios.len() }
    pub fn is_empty(&self) -> bool { self.inicios.is_empty() }
    fn fin(&self, i: usize) -> usize { self.inicios.get(i + 1).map(|x| *x as usize).unwrap_or(self.texto.len()) }
    pub fn ruta(&self, i: usize) -> &str { &self.texto[self.inicios[i] as usize..self.fin(i)] }
    pub fn es_carpeta(&self, i: usize) -> bool { self.carpetas[i] }
    pub fn bytes(&self) -> usize { self.texto.len() * 2 + self.inicios.len() * 9 }
}

fn inicio_de_palabra(previo: Option<u8>, c: u8) -> bool {
    match previo { None => true, Some(p) => matches!(p, b'/' | b'_' | b'-' | b'.' | b' ') || (p.is_ascii_lowercase() && c.is_ascii_uppercase()) }
}

/// Puntos de `q` (minusculas) como subsecuencia de un texto (bytes originales y en
/// minusculas): comienzos de palabra y letras seguidas suben; huecos dentro de una
/// palabra bajan un poco. None si no esta.
fn difuso(q: &[u8], orig: &[u8], minus: &[u8], es_nombre: bool) -> Option<i32> {
    let (mut puntos, mut k, mut seguida, mut saltadas) = (0i32, 0usize, false, 0i32);
    for i in 0..minus.len() {
        if k == q.len() { break; }
        if minus[i] == q[k] {
            puntos += 1;
            if seguida { puntos += 3; }
            if inicio_de_palabra(i.checked_sub(1).map(|j| orig[j]), orig[i]) { puntos += 4; } else { puntos -= saltadas.min(4); }
            if i == 0 && es_nombre { puntos += 3; }
            k += 1;
            seguida = true;
            saltadas = 0;
        } else {
            seguida = false;
            if k > 0 { saltadas += 1; }
        }
    }
    (k == q.len()).then_some(puntos)
}

pub fn preparar(consulta: &str) -> Vec<char> {
    consulta.chars().filter(|c| !c.is_whitespace()).flat_map(|c| c.to_lowercase()).take(128).collect()
}

/// Puntos de la ruta `i`: manda el nombre; con "/" en la consulta, la ruta entera.
fn puntuar(indice: &Indice, q: &[u8], con_barra: bool, i: usize) -> Option<i32> {
    let (a, n, z) = (indice.inicios[i] as usize, indice.nombres[i] as usize, indice.fin(i));
    let orig = indice.texto.as_bytes();
    let profundidad = indice.minus[a..z].iter().filter(|b| **b == b'/').count() as i32;
    // "dom/element": "element" contra el nombre y "dom" contra las carpetas.
    let (q_dir, q) = match q.iter().rposition(|b| *b == b'/') { Some(k) if con_barra => (&q[..k], &q[k + 1..]), _ => (&q[..0], q) };
    let extra = if q_dir.is_empty() { 0 } else {
        let dir = &indice.minus[a..n];
        // Una carpeta que empieza asi ("dom" -> .../dom/) vale mucho mas que letras sueltas.
        let segmento = dir.windows(q_dir.len()).enumerate().any(|(k, w)| w == q_dir && (k == 0 || dir[k - 1] == b'/'));
        difuso(q_dir, &orig[a..n], dir, false)? + if segmento { 20 + q_dir.len() as i32 } else { 0 }
    };
    if q.is_empty() { return Some(extra - profundidad); }
    let nombre = &indice.minus[n..z];
    let mut p = difuso(q, &orig[n..z], nombre, true)? * 2 + extra - profundidad;
    if nombre.starts_with(q) { p += 12; }
    let base = nombre.split(|b| *b == b'.').next().unwrap_or(nombre);
    if base == q { p += 10; }
    Some(p)
}

/// Los mejores (puntos, i) de un tramo del indice; la plataforma puede repartir tramos en hilos.
pub fn buscar_en_tramo(indice: &Indice, consulta: &str, tramo: core::ops::Range<usize>, max: usize) -> Vec<(i32, usize)> {
    let q: Vec<u8> = consulta.bytes().filter(|b| !b.is_ascii_whitespace()).map(|b| b.to_ascii_lowercase()).take(128).collect();
    if q.is_empty() { return Vec::new(); }
    let con_barra = q.contains(&b'/');
    let mut mejores: Vec<(i32, usize)> = Vec::new();
    let mut umbral = i32::MIN;
    for i in tramo {
        let Some(p) = puntuar(indice, &q, con_barra, i) else { continue };
        if p < umbral { continue; }
        mejores.push((p, i));
        if mejores.len() >= max * 4 {
            // Poda: se queda con los `max` mejores y sube el listón.
            ordenar(indice, &mut mejores);
            mejores.truncate(max);
            umbral = mejores.last().map(|m| m.0).unwrap_or(i32::MIN);
        }
    }
    mejores
}

/// De mejor a peor; empate: ruta mas corta y luego alfabetica.
pub fn ordenar(indice: &Indice, v: &mut [(i32, usize)]) {
    v.sort_by(|a, b| b.0.cmp(&a.0).then(indice.ruta(a.1).len().cmp(&indice.ruta(b.1).len())).then(indice.ruta(a.1).cmp(indice.ruta(b.1))));
}

/// Los mejores `max` del indice para la consulta (indices, de mejor a peor).
pub fn buscar_rutas(indice: &Indice, consulta: &str, max: usize) -> Vec<usize> {
    let mut v = buscar_en_tramo(indice, consulta, 0..indice.len(), max);
    ordenar(indice, &mut v);
    v.truncate(max);
    v.into_iter().map(|(_, i)| i).collect()
}

/// Que letras del nombre marcar en la lista (indices de caracter).
pub fn letras_marcadas(consulta: &str, texto: &str) -> Vec<usize> {
    let q = preparar(consulta);
    let mut v = Vec::new();
    let mut k = 0;
    for (i, c) in texto.chars().enumerate() {
        if k < q.len() && c.to_lowercase().next() == Some(q[k]) { v.push(i); k += 1; }
    }
    if k == q.len() { v } else { Vec::new() }
}

/// Ctrl+P: el cuadro de "Ir a archivo o carpeta".
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IrA {
    pub texto: String,
    /// (ruta, es carpeta)
    pub resultados: Vec<(String, bool)>,
    pub elegido: usize,
    pub generacion: u64,
    pub buscando: bool,
    /// Se pulso Enter antes de que llegaran los resultados: se acepta al llegar (Some(editor)).
    pub aceptar_al_llegar: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccionIrA { Abrir, Cerrar, Texto(String), Mover(i32), Elegir(usize), Aceptar { editor: bool },
                     /// Alt+Enter: meter el elegido en el grupo (panel Combinar) sin moverse.
                     AlGrupo }

/// Una coincidencia de texto en un archivo.
#[derive(Debug, Clone, PartialEq)]
pub struct Coincidencia {
    pub linea: u32,
    pub columna: u32,
    /// La linea (recortada a MAX_VISTA letras) y donde cae la coincidencia en ella (bytes).
    pub vista: String,
    pub en_vista: (usize, usize),
    /// Bytes en el archivo (para seleccionarla en el editor).
    pub rango: (usize, usize),
}

/// Convierte coincidencias en bytes a lineas y columnas, con su linea para mostrar.
pub fn coincidencias(texto: &str, hallazgos: &[(usize, usize)]) -> Vec<Coincidencia> {
    let mut v = Vec::with_capacity(hallazgos.len().min(MAX_POR_ARCHIVO));
    let (mut linea, mut inicio_linea, mut pos) = (1u32, 0usize, 0usize);
    for &(a, z) in hallazgos.iter().take(MAX_POR_ARCHIVO) {
        // Avanza hasta la linea de `a` contando saltos (los hallazgos vienen en orden).
        for (i, b) in texto.as_bytes()[pos..a].iter().enumerate() {
            if *b == b'\n' { linea += 1; inicio_linea = pos + i + 1; }
        }
        pos = a;
        let fin_linea = texto[a..].find('\n').map(|i| a + i).unwrap_or(texto.len());
        let entera = &texto[inicio_linea..fin_linea];
        // Recorta alrededor de la coincidencia: hasta 60 bytes antes y MAX_VISTA en total.
        let sangria = entera.len() - entera.trim_start().len();
        let mut desde = (inicio_linea + sangria).max(a.saturating_sub(60));
        while !texto.is_char_boundary(desde) { desde += 1; }
        let mut hasta = (desde + MAX_VISTA).min(fin_linea).max(z.min(fin_linea));
        while !texto.is_char_boundary(hasta) { hasta -= 1; }
        let vista = texto[desde..hasta].to_string();
        let columna = texto[inicio_linea..a].chars().count() as u32 + 1;
        v.push(Coincidencia { linea, columna, en_vista: (a - desde, z.min(hasta) - desde), vista, rango: (a, z) });
    }
    v
}

/// Ctrl+Shift+F: buscar texto en todo el proyecto (o en la carpeta enfocada).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BusquedaProyecto {
    pub patron: String,
    pub mayusculas: bool,
    pub palabra: bool,
    /// "*.cc, *.h" o "src/" (vacio = todo).
    pub filtro: String,
    pub generacion: u64,
    pub por_archivo: BTreeMap<String, Vec<Coincidencia>>,
    pub total: usize,
    pub archivos_vistos: u64,
    pub buscando: bool,
    pub truncada: bool,
    /// Archivos con resultados plegados en la lista.
    pub plegados: alloc::collections::BTreeSet<String>,
    pub foco: u64,
}

impl BusquedaProyecto {
    /// Coincidencias en un nodo (en una carpeta, todo lo de dentro).
    pub fn conteo(&self, id: &str) -> u32 {
        if self.por_archivo.is_empty() { return 0; }
        self.por_archivo.range::<str, _>((core::ops::Bound::Included(id), core::ops::Bound::Unbounded))
            .take_while(|(k, _)| k.starts_with(id))
            .filter(|(k, _)| id.is_empty() || k.len() == id.len() || k.as_bytes()[id.len()] == b'/')
            .map(|(_, v)| v.len() as u32).sum()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccionProyecto { Abrir, Patron(String), Mayusculas, Palabra, Filtro(String), Lanzar, Limpiar, Plegar(String), Ir { id: String, i: usize } }

/// Filtro de la busqueda: patrones separados por comas. "*.cc" por extension, "src/" por
/// carpeta, y "!third_party/" para excluir. Vacio = todo.
pub fn pasa_filtro(filtro: &str, ruta: &str) -> bool {
    let mut incluye = false;
    let mut hay_incluir = false;
    for p in filtro.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (fuera, p) = match p.strip_prefix('!') { Some(r) => (true, r.trim()), None => (false, p) };
        let casa = if let Some(ext) = p.strip_prefix("*.") { ruta.rsplit_once('.').is_some_and(|(_, e)| e.eq_ignore_ascii_case(ext)) }
                   else if let Some(c) = p.strip_suffix('/') { ruta.starts_with(&(c.to_string() + "/")) || ruta.contains(&alloc::format!("/{c}/")) }
                   else { ruta.contains(p) };
        if fuera { if casa { return false; } } else { hay_incluir = true; incluye |= casa; }
    }
    !hay_incluir || incluye
}
