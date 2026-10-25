// SPDX-License-Identifier: GPL-2.0-only
//! El modo grafo (como los editores de nodos de Blender y Houdini) y el arbol del
//! explorador. Comparten las carpetas desplegadas: abrir una en un lado la abre en el otro.

use crate::dependencias::Rol;
use crate::estado::{Entrada, Estado};
use crate::iconos::icono;
use crate::organizar::{id_grupo, organizar, Clase, Grupo, Hijo};
use crate::tipos::{legible, Rgb, Tipo};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub const NODO: [f32; 2] = [210.0, 74.0];
pub const MAX_NODOS: usize = 600;

/// Blender: el flujo va de izquierda a derecha. Houdini: de arriba abajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direccion { #[default] Blender, Houdini }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modo { #[default] Iconos, Grafo, Ciudad }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camara { pub x: f32, pub y: f32, pub zoom: f32 }

impl Default for Camara { fn default() -> Self { Camara { x: -40.0, y: -40.0, zoom: 1.0 } } }

#[derive(Debug, Clone, PartialEq)]
pub struct NodoGrafo {
    /// Ruta relativa a la carpeta abierta ("" es la raiz).
    pub id: String,
    pub nombre: String,
    pub tipo: Tipo,
    pub pos: [f32; 2],
    pub tam: [f32; 2],
    pub color: Rgb,
    pub icono: &'static str,
    pub detalle: String,
    /// 0..1: que parte de su carpeta ocupa (lo que leyo el disco); para optimizar espacio.
    pub peso: f32,
    /// None en archivos; Some(abierta) en carpetas.
    pub expandida: Option<bool>,
    pub seleccionado: bool,
    /// Boton de play: que arranca ("cargo run", "make"...).
    pub play: Option<String>,
    /// Errores atrapados de la ultima ejecucion (en carpetas, todo lo de dentro).
    pub marcas: Marcas,
    /// Es la raiz de la vista (la carpeta abierta o la enfocada).
    pub raiz: bool,
    /// Grupo virtual del organizador: su resumen ("h · cc · unittest").
    pub grupo: Option<String>,
    /// SUPER GRAFO: su papel en el fallo conectado (o alrededor del nodo aislado).
    pub rol: Option<Rol>,
    /// Aislado: no esta conectado con lo que se aislo (se pinta apagado).
    pub atenuado: bool,
}

/// Lo que se atrapo en la salida para un nodo: rojo con errores, amarillo con avisos.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Marcas {
    pub errores: u32,
    pub avisos: u32,
    /// "src/main.rs:12 error[E0425]: ..." del primero (para el tooltip).
    pub primero: Option<String>,
    /// Coincidencias de la busqueda en todo el proyecto.
    pub coincidencias: u32,
    /// Sospechas del museo: patron exacto de la ficha / para revisar.
    pub exactas: u32,
    pub revisar: u32,
    /// "UB-003 Desplazar fuera del ancho · src/a.c:12\nmotivo" de la primera.
    pub museo: Option<String>,
}

impl Marcas {
    pub fn museo_total(&self) -> u32 { self.exactas + self.revisar }
}

pub fn marcas(e: &Estado, id: &str) -> Marcas {
    let (errores, avisos) = e.errores.conteo(id);
    let coincidencias = e.proyecto.conteo(id);
    let (exactas, revisar) = e.analisis.conteo(id);
    if errores + avisos + coincidencias + exactas + revisar == 0 { return Marcas::default(); }
    let primero = e.errores.primero(id).map(|(f, d)| format!("{f}:{}:{}  {}", d.linea, d.columna, d.mensaje));
    let museo = e.analisis.primera(id).map(|(f, s)| {
        let titulo = crate::museo::ficha(s.ficha).map(|x| x.titulo).unwrap_or("");
        let fuerza = if s.fuerza == crate::museo::Fuerza::Exacto { "patron exacto" } else { "revisar" };
        format!("Museo {} · {titulo} ({fuerza})\n{f}:{}  {}\n{}", s.ficha, s.linea, s.visto, s.motivo)
    });
    Marcas { errores, avisos, primero, coincidencias, exactas, revisar, museo }
}

/// Que une un cable: el arbol de carpetas, un fallo que se propaga o una dependencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClaseCable { #[default] Arbol, Fallo, Dependencia }

#[derive(Debug, Clone, PartialEq)]
pub struct Cable { pub desde: [f32; 2], pub hasta: [f32; 2], pub color: Rgb, pub clase: ClaseCable }

#[derive(Debug, Clone, PartialEq)]
pub struct FilaArbol {
    pub id: String,
    pub nombre: String,
    pub tipo: Tipo,
    pub nivel: usize,
    pub expandida: Option<bool>,
    pub seleccionado: bool,
    pub icono: &'static str,
    pub color: Rgb,
    pub play: Option<String>,
    pub marcas: Marcas,
    pub grupo: Option<String>,
}

pub fn hijo(padre: &str, nombre: &str) -> String {
    if padre.is_empty() { nombre.to_string() } else { format!("{padre}/{nombre}") }
}

/// Que es cada cosa visible: la raiz (la carpeta abierta o la enfocada), una entrada real
/// del disco o un grupo virtual del organizador.
#[derive(Debug, Clone, PartialEq)]
pub enum Cosa<'a> { Raiz, Real(&'a Entrada), Grupo(Grupo<'a>) }

#[derive(Debug, Clone, PartialEq)]
pub struct Visible<'a> {
    pub id: String,
    pub nivel: usize,
    /// Padre en la vista (una carpeta o un grupo); None en la raiz.
    pub padre: Option<String>,
    /// Carpeta real donde esta (para los grupos y sus miembros).
    pub carpeta: String,
    pub cosa: Cosa<'a>,
}

/// La carpeta que hace de raiz: la enfocada si se puede ver (leida y existente), si no "".
pub fn raiz_vista(e: &Estado) -> &str {
    let f = e.disp.foco.as_str();
    if f.is_empty() || (e.entrada(f).is_some_and(|x| x.tipo == Tipo::Carpeta) && e.arbol.contains_key(f)) { f } else { "" }
}

/// Recorre en profundidad lo que se ve: la raiz y lo que cuelga de carpetas y grupos abiertos.
pub fn visibles(e: &Estado) -> Vec<Visible<'_>> {
    let raiz = raiz_vista(e);
    let mut salida = vec![Visible { id: raiz.to_string(), nivel: 0, padre: None, carpeta: raiz.to_string(), cosa: Cosa::Raiz }];
    fn bajar<'a>(e: &'a Estado, carpeta: &str, dentro: Option<&str>, padre: &str, nivel: usize, salida: &mut Vec<Visible<'a>>) {
        let Some(entradas) = e.arbol.get(carpeta) else { return };
        for h in organizar(entradas, e.ajustes.organizar, dentro) {
            if salida.len() >= MAX_NODOS { return; }
            match h {
                Hijo::Real(en) => {
                    let id = hijo(carpeta, &en.nombre);
                    let abre = en.tipo == Tipo::Carpeta && e.expandidas.contains(&id);
                    salida.push(Visible { id: id.clone(), nivel, padre: Some(padre.to_string()), carpeta: carpeta.to_string(), cosa: Cosa::Real(en) });
                    if abre { bajar(e, &id, None, &id, nivel + 1, salida); }
                }
                Hijo::Grupo(g) => {
                    let id = id_grupo(carpeta, &g.clave);
                    let abre = e.expandidas.contains(&id);
                    let clave = g.clave.clone();
                    salida.push(Visible { id: id.clone(), nivel, padre: Some(padre.to_string()), carpeta: carpeta.to_string(), cosa: Cosa::Grupo(g) });
                    if abre { bajar(e, carpeta, Some(&clave), &id, nivel + 1, salida); }
                }
            }
        }
    }
    bajar(e, raiz, None, raiz, 1, &mut salida);
    salida
}

/// Hijos cerrados de una carpeta que pasan a rejilla (columnas de este alto) en vez de una
/// columna interminable: la raiz de Chromium son 77 nodos, 5 columnas de 16.
pub const FILAS_REJILLA: usize = 16;

/// Acomodo de arbol: cada hoja ocupa un hueco; un padre se centra sobre sus hijos. Si un
/// padre tiene mas de FILAS_REJILLA hijos cerrados, esos van en rejilla (columnas hacia
/// fuera) despues de los abiertos, que siguen ramificando como siempre.
pub fn acomodar(e: &Estado) -> BTreeMap<String, [f32; 2]> {
    let lista = visibles(e);
    let mut hijos_de: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for v in &lista {
        if let Some(p) = &v.padre { hijos_de.entry(p.clone()).or_default().push(v.id.clone()); }
    }
    struct Hueco { fila: f32, columna: f32 }
    fn colocar(id: &str, hijos_de: &BTreeMap<String, Vec<String>>, huecos: &mut BTreeMap<String, Hueco>, siguiente: &mut f32) -> f32 {
        let fila = match hijos_de.get(id) {
            Some(hijos) if !hijos.is_empty() => {
                let (abiertos, cerrados): (Vec<&String>, Vec<&String>) = hijos.iter().partition(|h| hijos_de.get(*h).is_some_and(|x| !x.is_empty()));
                let mut filas: Vec<f32> = Vec::new();
                if cerrados.len() > FILAS_REJILLA {
                    for h in &abiertos { filas.push(colocar(h, hijos_de, huecos, siguiente)); }
                    // Rejilla: columna c, fila r; ocupa FILAS_REJILLA huecos de alto.
                    let base = *siguiente;
                    for (k, h) in cerrados.iter().enumerate() {
                        let (c, r) = (k / FILAS_REJILLA, k % FILAS_REJILLA);
                        huecos.insert((*h).clone(), Hueco { fila: base + r as f32, columna: c as f32 });
                        if c == 0 { filas.push(base + r as f32); }
                    }
                    *siguiente = base + FILAS_REJILLA.min(cerrados.len()) as f32;
                } else {
                    for h in hijos { filas.push(colocar(h, hijos_de, huecos, siguiente)); }
                }
                let (min, max) = filas.iter().fold((f32::MAX, f32::MIN), |(a, b), f| (a.min(*f), b.max(*f)));
                (min + max) / 2.0
            }
            _ => { let s = *siguiente; *siguiente += 1.0; s }
        };
        huecos.entry(id.to_string()).or_insert(Hueco { fila, columna: 0.0 });
        fila
    }
    let mut huecos: BTreeMap<String, Hueco> = BTreeMap::new();
    let mut siguiente = 0.0f32;
    colocar(&lista[0].id, &hijos_de, &mut huecos, &mut siguiente);
    let mut pos = BTreeMap::new();
    for v in &lista {
        let h = huecos.get(&v.id).map(|h| (h.fila, h.columna)).unwrap_or((0.0, 0.0));
        let nivel = v.nivel as f32 + h.1;
        let automatica = match e.ajustes.direccion {
            Direccion::Blender => [nivel * (NODO[0] + 90.0) - h.1 * 60.0, h.0 * (NODO[1] + 22.0)],
            Direccion::Houdini => [h.0 * (NODO[0] + 30.0), nivel * (NODO[1] + 80.0) - h.1 * 50.0],
        };
        pos.insert(v.id.clone(), e.disp.grafo.get(&v.id).copied().unwrap_or(automatica));
    }
    pos
}

/// Marcas de un grupo: la suma de las de sus miembros.
fn marcas_grupo(e: &Estado, carpeta: &str, miembros: &[&Entrada]) -> Marcas {
    let mut m = Marcas::default();
    for x in miembros {
        let mx = marcas(e, &hijo(carpeta, &x.nombre));
        m.errores += mx.errores;
        m.avisos += mx.avisos;
        m.coincidencias += mx.coincidencias;
        m.exactas += mx.exactas;
        m.revisar += mx.revisar;
        if m.primero.is_none() { m.primero = mx.primero; }
        if m.museo.is_none() { m.museo = mx.museo; }
    }
    m
}

/// Que dibujar de cada visible: nombre, tipo, icono, detalle, bytes propios y si es grupo.
struct Datos { nombre: String, tipo: Tipo, icono: &'static str, detalle: String, propio: u64, grupo: Option<String>, expandida: Option<bool>, marcas: Marcas }

fn datos(e: &Estado, v: &Visible) -> Datos {
    match &v.cosa {
        Cosa::Raiz => {
            let nombre = if v.id.is_empty() { if e.nombre_raiz.is_empty() { String::from("/") } else { e.nombre_raiz.clone() } }
                         else { v.id.rsplit('/').next().unwrap_or(&v.id).to_string() };
            let propio = peso_leido(e, &v.id);
            let detalle = format!("{} elementos · {}", e.arbol.get(&v.id).map(|h| h.len()).unwrap_or(0), legible(propio));
            Datos { nombre, tipo: Tipo::Carpeta, icono: "", detalle, propio, grupo: None, expandida: Some(true), marcas: marcas(e, &v.id) }
        }
        Cosa::Real(en) => {
            let hijos = e.arbol.get(&v.id);
            let propio = match (en.tipo, hijos) { (Tipo::Carpeta, Some(_)) => peso_leido(e, &v.id), (Tipo::Carpeta, None) => 0, _ => en.tam };
            let detalle = match (en.tipo, hijos) {
                (Tipo::Carpeta, Some(h)) => format!("{} elementos · {}", h.len(), legible(propio)),
                (Tipo::Carpeta, None) => String::from("doble clic: desplegar"),
                _ => legible(en.tam),
            };
            Datos { nombre: en.nombre.clone(), tipo: en.tipo, icono: icono(&en.nombre, en.tipo, e.ajustes.iconos), detalle, propio, grupo: None,
                    expandida: (en.tipo == Tipo::Carpeta).then(|| e.expandidas.contains(&v.id)), marcas: marcas(e, &v.id) }
        }
        Cosa::Grupo(g) => {
            // El icono y el color de su pieza principal (la cabecera o el codigo).
            let principal = g.miembros.iter().find(|m| m.tipo == Tipo::Codigo).or(g.miembros.first()).copied();
            let (tipo, icono_g) = match (g.clase, principal) {
                (Clase::Meta, _) => (Tipo::Documento, icono("BUILD.gn.conf", Tipo::Documento, e.ajustes.iconos)),
                (_, Some(m)) => (m.tipo, icono(&m.nombre, m.tipo, e.ajustes.iconos)),
                _ => (Tipo::Otro, ""),
            };
            let propio = g.miembros.iter().map(|m| m.tam).fold(0u64, |a, b| a.saturating_add(b));
            let detalle = format!("{} archivos · {}", g.miembros.len(), legible(propio));
            Datos { nombre: g.nombre.clone(), tipo, icono: icono_g, detalle, propio, grupo: Some(g.resumen.clone()),
                    expandida: Some(e.expandidas.contains(&v.id)), marcas: marcas_grupo(e, &v.carpeta, &g.miembros) }
        }
    }
}

pub fn nodos_y_cables(e: &Estado, tema: &crate::tema::Tema) -> (Vec<NodoGrafo>, Vec<Cable>) {
    let lista = visibles(e);
    let pos = acomodar(e);
    let mut nodos = Vec::with_capacity(lista.len());
    // Bytes de cada padre visible, para la barra de peso (que parte de su padre ocupa).
    let mut propios: BTreeMap<&str, u64> = BTreeMap::new();
    let todos: Vec<Datos> = lista.iter().map(|v| datos(e, v)).collect();
    for (v, d) in lista.iter().zip(&todos) { propios.insert(&v.id, d.propio); }
    for (v, d) in lista.iter().zip(todos) {
        let total = v.padre.as_deref().and_then(|p| propios.get(p)).copied().unwrap_or(0);
        let peso = if total == 0 { 0.0 } else { (d.propio as f32 / total as f32).min(1.0) };
        let raiz = matches!(v.cosa, Cosa::Raiz);
        nodos.push(NodoGrafo {
            id: v.id.clone(), nombre: d.nombre, tipo: d.tipo,
            pos: pos.get(&v.id).copied().unwrap_or([0.0, 0.0]), tam: NODO, color: tema.color_tipo(d.tipo),
            icono: d.icono, detalle: d.detalle, peso, expandida: d.expandida,
            seleccionado: e.seleccion.contains(&v.id),
            play: if matches!(v.cosa, Cosa::Grupo(_)) { None } else { crate::ejecutar::lanzador(e, &v.id).map(|l| l.titulo) },
            marcas: d.marcas, raiz, grupo: d.grupo, rol: None, atenuado: false,
        });
    }
    // Cables del padre al hijo: salida a la derecha (Blender) o abajo (Houdini).
    let mut cables = Vec::new();
    for (n, v) in nodos.iter().zip(&lista) {
        let Some(p) = v.padre.as_ref().and_then(|p| pos.get(p)) else { continue };
        let (desde, hasta) = match e.ajustes.direccion {
            Direccion::Blender => ([p[0] + NODO[0], p[1] + NODO[1] / 2.0], [n.pos[0], n.pos[1] + NODO[1] / 2.0]),
            Direccion::Houdini => ([p[0] + NODO[0] / 2.0, p[1] + NODO[1]], [n.pos[0] + NODO[0] / 2.0, n.pos[1]]),
        };
        cables.push(Cable { desde, hasta, color: n.color, clase: ClaseCable::Arbol });
    }
    conectar_nodos(e, tema, &lista, &mut nodos, &mut cables);
    (nodos, cables)
}

pub const MAX_CABLES_FALLO: usize = 1500;

/// SUPER GRAFO: lleva la conexion (archivo -> papel) a los nodos que se ven. Un archivo
/// que no se ve cuenta en su grupo cerrado o en la carpeta visible mas cercana.
fn conectar_nodos(e: &Estado, tema: &crate::tema::Tema, lista: &[Visible], nodos: &mut [NodoGrafo], cables: &mut Vec<Cable>) {
    use crate::analisis::Modo as M;
    let c = &e.conectado;
    if c.modo == M::Ninguno { return; }
    let visibles: BTreeMap<&str, usize> = lista.iter().enumerate().map(|(i, v)| (v.id.as_str(), i)).collect();
    // Miembros de grupos cerrados -> el grupo.
    let mut miembro: BTreeMap<String, usize> = BTreeMap::new();
    for (i, v) in lista.iter().enumerate() {
        if let Cosa::Grupo(g) = &v.cosa {
            if !e.expandidas.contains(&v.id) { for m in &g.miembros { miembro.insert(hijo(&v.carpeta, &m.nombre), i); } }
        }
    }
    let raiz = lista[0].id.as_str();
    let de = |archivo: &str| -> Option<usize> {
        if !raiz.is_empty() && !(archivo.starts_with(raiz) && archivo.as_bytes().get(raiz.len()) == Some(&b'/')) { return None; }
        if let Some(i) = visibles.get(archivo) { return Some(*i); }
        if let Some(i) = miembro.get(archivo) { return Some(*i); }
        let mut p = archivo;
        while let Some((d, _)) = p.rsplit_once('/') {
            if let Some(i) = visibles.get(d).or_else(|| miembro.get(d)) { return Some(*i); }
            p = d;
        }
        visibles.get("").copied()
    };
    let mut roles: Vec<Option<Rol>> = alloc::vec![None; nodos.len()];
    for (archivo, rol) in &c.conexion.roles {
        if let Some(i) = de(archivo) { if roles[i].is_none_or(|r| *rol > r) { roles[i] = Some(*rol); } }
    }
    let aislado = e.aislar.is_some();
    for (n, r) in nodos.iter_mut().zip(&roles) {
        n.rol = *r;
        n.atenuado = aislado && r.is_none();
    }
    let (clase, color) = if c.modo == M::Fallos { (ClaseCable::Fallo, tema.error) } else { (ClaseCable::Dependencia, tema.acento) };
    let mut hechos: alloc::collections::BTreeSet<(usize, usize)> = alloc::collections::BTreeSet::new();
    for (a, b) in &c.conexion.aristas {
        // a usa b: el fallo viaja de b (lo usado) hacia a (quien lo usa).
        let (Some(desde), Some(hasta)) = (de(b), de(a)) else { continue };
        if desde == hasta || !hechos.insert((desde, hasta)) { continue; }
        if hechos.len() > MAX_CABLES_FALLO { break; }
        let (p, q) = (nodos[desde].pos, nodos[hasta].pos);
        let (d, h) = match e.ajustes.direccion {
            Direccion::Blender => ([p[0] + NODO[0], p[1] + NODO[1] / 2.0], [q[0], q[1] + NODO[1] / 2.0]),
            Direccion::Houdini => ([p[0] + NODO[0] / 2.0, p[1] + NODO[1]], [q[0] + NODO[0] / 2.0, q[1]]),
        };
        cables.push(Cable { desde: d, hasta: h, color, clase });
    }
}

/// Bytes de una carpeta contando lo ya leido de sus subcarpetas (hasta 32 niveles).
pub fn peso_leido(e: &Estado, carpeta: &str) -> u64 {
    fn bajar(e: &Estado, carpeta: &str, nivel: usize) -> u64 {
        let Some(lista) = e.arbol.get(carpeta) else { return 0 };
        lista.iter().map(|x| if x.tipo == Tipo::Carpeta && nivel < 32 { bajar(e, &hijo(carpeta, &x.nombre), nivel + 1) } else { x.tam }).fold(0u64, |a, b| a.saturating_add(b))
    }
    bajar(e, carpeta, 0)
}

pub fn filas_arbol(e: &Estado, tema: &crate::tema::Tema) -> Vec<FilaArbol> {
    visibles(e).into_iter().filter(|v| !matches!(v.cosa, Cosa::Raiz)).map(|v| {
        let d = datos(e, &v);
        let es_grupo = matches!(v.cosa, Cosa::Grupo(_));
        FilaArbol { nombre: d.nombre, tipo: d.tipo, nivel: v.nivel - 1, expandida: d.expandida,
                    seleccionado: e.seleccion.contains(&v.id), icono: d.icono, color: tema.color_tipo(d.tipo),
                    play: if es_grupo { None } else { crate::ejecutar::lanzador(e, &v.id).map(|l| l.titulo) },
                    marcas: d.marcas, grupo: d.grupo, id: v.id }
    }).collect()
}

/// Migas de pan desde la carpeta abierta hasta la enfocada: (id, nombre).
pub fn migas(e: &Estado) -> Vec<(String, String)> {
    let mut v = vec![(String::new(), if e.nombre_raiz.is_empty() { String::from("/") } else { e.nombre_raiz.clone() })];
    let raiz = raiz_vista(e);
    if raiz.is_empty() { return v; }
    let mut camino = String::new();
    for tramo in raiz.split('/') {
        camino = hijo(&camino, tramo);
        v.push((camino.clone(), tramo.to_string()));
    }
    v
}

/// Rectangulo que encierra todos los nodos (para encuadrar con Inicio, como en Blender).
pub fn limites(e: &Estado) -> ([f32; 2], [f32; 2]) {
    let pos = acomodar(e);
    let mut min = [f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN];
    for p in pos.values() {
        min = [min[0].min(p[0]), min[1].min(p[1])];
        max = [max[0].max(p[0] + NODO[0]), max[1].max(p[1] + NODO[1])];
    }
    if min[0] > max[0] { ([0.0, 0.0], NODO) } else { (min, max) }
}
