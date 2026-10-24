// SPDX-License-Identifier: GPL-2.0-only
//! Estado, acciones y el reducer: la unica forma de cambiar el estado.
//! El reducer no hace E/S: devuelve Efectos y la plataforma los ejecuta.

use crate::ajustes::Ajustes;
use crate::carriles::MAX_DESPLEGADAS;
use crate::comandos::{buscar, Comando};
use crate::disposicion::{Disposicion, ICONO};
use crate::ejecutar::{lanzador, partir_linea, Terminal};
use crate::grafo::{acomodar, hijo, limites, Camara, Direccion, Modo, NODO};
use crate::tema::TEMAS;
use crate::tipos::{Rgb, Tipo, HIELO};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    pub nombre: String,
    pub tipo: Tipo,
    pub tam: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orden { Nombre, Tipo, Tamano }

#[derive(Debug, Clone, PartialEq)]
pub enum AccionPaleta { Abrir, Cerrar, Escribir(String), Mover(i32), Aceptar, Elegir(usize) }

#[derive(Debug, Clone, PartialEq)]
pub enum Accion {
    /// Ajustes leidos al arrancar.
    CargarAjustes(Ajustes),
    /// Carpeta nueva: lo leido del disco y el orden guardado (ya convertido desde JSON).
    Cargar { nombre: String, entradas: Vec<Entrada>, disposicion: Disposicion },
    /// La carpeta abierta cambio en disco.
    Refrescar(Vec<Entrada>),
    /// Contenido de una subcarpeta (ruta relativa) que pidio el grafo o el explorador.
    Contenido { ruta: String, entradas: Vec<Entrada> },
    Ancho(f32),
    /// Tamano del lienzo en pixeles (para encuadrar el grafo).
    Lienzo([f32; 2]),
    /// id = ruta relativa ("foto.png", "src/main.rs").
    Pulsar { nombre: String, sumar: bool },
    Abrir(String),
    /// Desplegar o plegar una carpeta en el grafo y el explorador.
    Expandir(String),
    EmpezarArrastre(String),
    /// Delta en pixeles de pantalla.
    Arrastrar([f32; 2]),
    Soltar,
    /// Puntos en coordenadas del lienzo (iconos) o del mundo (grafo).
    EmpezarBanda { punto: [f32; 2], sumar: bool },
    MoverBanda([f32; 2]),
    TerminarBanda,
    /// Mover la camara del grafo (pixeles de pantalla).
    Desplazar([f32; 2]),
    /// Zoom alrededor de un punto de la pantalla (relativo al lienzo).
    Zoom { factor: f32, ancla: [f32; 2] },
    Comando(Comando),
    Paleta(AccionPaleta),
    /// Boton de play de un nodo (o de una fila del explorador).
    Ejecutar(String),
    /// Linea escrita en la terminal (sin shell).
    LineaComando(String),
    /// Lo que va saliendo del proceso.
    Salida { linea: String, error: bool },
    Termino(Option<i32>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Efecto {
    /// Guardar el orden de esta carpeta (texto JSON).
    Guardar(String),
    /// Guardar los ajustes de la app (texto JSON).
    GuardarAjustes(String),
    AbrirArchivo(String),
    EntrarCarpeta(String),
    /// Listar una subcarpeta (ruta relativa) en el hilo del disco.
    LeerCarpeta(String),
    Subir,
    Atras,
    Releer { ocultos: bool },
    /// Lanzar un proceso: programa y argumentos sin shell, en una carpeta relativa.
    Ejecutar { titulo: String, carpeta: String, programa: String, args: Vec<String> },
    Detener,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paleta {
    pub texto: String,
    pub elegido: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Estado {
    pub entradas: Vec<Entrada>,
    pub disp: Disposicion,
    pub seleccion: BTreeSet<String>,
    pub ocultos: bool,
    pub ancho: f32,
    pub lienzo: [f32; 2],
    pub acento: Rgb,
    pub arrastrando: bool,
    pub banda: Option<([f32; 2], [f32; 2])>,
    pub paleta: Option<Paleta>,
    pub ajustes: Ajustes,
    pub panel_ajustes: bool,
    pub carriles: bool,
    /// Contenido leido de cada carpeta ("" = la abierta).
    pub arbol: BTreeMap<String, Vec<Entrada>>,
    /// Carpetas desplegadas: las comparten el grafo y el explorador.
    pub expandidas: BTreeSet<String>,
    pub camara: Camara,
    /// Sube con cada accion: la plataforma solo recalcula la Vista si cambio.
    pub version: u64,
    /// Nombre de la carpeta abierta (para el nodo raiz).
    pub nombre_raiz: String,
    /// Sube solo cuando cambia lo que se dibuja (no al mover la camara): la geometria de
    /// la GPU y la Vista se rehacen solo con esta.
    pub version_contenido: u64,
    pub terminal: Terminal,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { entradas: Vec::new(), disp: Disposicion::default(), seleccion: BTreeSet::new(), ocultos: false,
                 ancho: 1100.0, lienzo: [1100.0, 700.0], acento: HIELO, arrastrando: false, banda: None, paleta: None,
                 ajustes: Ajustes::default(), panel_ajustes: false, carriles: false, arbol: BTreeMap::new(),
                 expandidas: BTreeSet::new(), camara: Camara::default(), version: 0, nombre_raiz: String::new(),
                 version_contenido: 0, terminal: Terminal::default() }
    }
}

fn padre_de(id: &str) -> (&str, &str) {
    id.rsplit_once('/').unwrap_or(("", id))
}

impl Estado {
    fn nombres(&self) -> Vec<String> { self.entradas.iter().map(|e| e.nombre.clone()).collect() }
    fn guardar(&self) -> Efecto { Efecto::Guardar(self.disp.a_json()) }
    fn guardar_ajustes(&self) -> Efecto { Efecto::GuardarAjustes(self.ajustes.a_json()) }

    /// La entrada de una ruta relativa, solo si esta en lo leido (nunca se inventan rutas).
    pub fn entrada(&self, id: &str) -> Option<&Entrada> {
        let (padre, nombre) = padre_de(id);
        self.arbol.get(padre)?.iter().find(|e| e.nombre == nombre)
    }

    fn en_grafo(&self) -> bool { self.ajustes.modo == Modo::Grafo }

    fn abrir(&mut self, id: &str) -> Vec<Efecto> {
        match self.entrada(id).map(|e| e.tipo) {
            Some(Tipo::Carpeta) if self.en_grafo() => self.expandir(id),
            Some(Tipo::Carpeta) => vec![Efecto::EntrarCarpeta(id.into())],
            Some(_) => vec![Efecto::AbrirArchivo(id.into())],
            None => vec![],
        }
    }

    fn expandir(&mut self, id: &str) -> Vec<Efecto> {
        if self.entrada(id).map(|e| e.tipo) != Some(Tipo::Carpeta) { return vec![]; }
        if self.expandidas.remove(id) { return vec![]; }
        if self.expandidas.len() >= MAX_DESPLEGADAS { return vec![]; } // limite duro del carril de disco
        self.expandidas.insert(id.to_string());
        vec![Efecto::LeerCarpeta(id.to_string())]
    }

    fn ordenar(&mut self, orden: Orden) -> Vec<Efecto> {
        if self.disp.bloqueada { return vec![]; }
        let mut lista: Vec<&Entrada> = self.entradas.iter().collect();
        match orden {
            Orden::Nombre => lista.sort_by_key(|e| (e.tipo != Tipo::Carpeta, e.nombre.to_lowercase())),
            Orden::Tipo => lista.sort_by_key(|e| (e.tipo, e.nombre.to_lowercase())),
            Orden::Tamano => lista.sort_by_key(|e| (e.tipo != Tipo::Carpeta, core::cmp::Reverse(e.tam))),
        }
        let nombres: Vec<String> = lista.iter().map(|e| e.nombre.clone()).collect();
        self.disp.ordenar(&nombres, self.ancho);
        vec![self.guardar()]
    }

    fn encuadrar(&mut self) {
        let (min, max) = limites(self);
        let (w, h) = ((max[0] - min[0]).max(1.0), (max[1] - min[1]).max(1.0));
        let zoom = ((self.lienzo[0] - 80.0) / w).min((self.lienzo[1] - 80.0) / h).clamp(0.2, 1.5);
        self.camara = Camara { zoom, x: min[0] - (self.lienzo[0] / zoom - w) / 2.0, y: min[1] - (self.lienzo[1] / zoom - h) / 2.0 };
    }

    fn comando(&mut self, c: Comando) -> Vec<Efecto> {
        match c {
            Comando::Rejilla => { self.disp.rejilla = !self.disp.rejilla; vec![self.guardar()] }
            Comando::Candado => { self.disp.bloqueada = !self.disp.bloqueada; vec![self.guardar()] }
            Comando::Ocultos => { self.ocultos = !self.ocultos; vec![Efecto::Releer { ocultos: self.ocultos }] }
            Comando::Deshacer => if self.disp.deshacer() { vec![self.guardar()] } else { vec![] },
            Comando::Ordenar(o) => self.ordenar(o),
            Comando::SeleccionarTodo => { self.seleccion = self.nombres().into_iter().collect(); vec![] }
            Comando::LimpiarSeleccion => { self.seleccion.clear(); vec![] }
            Comando::AbrirSeleccion => match self.seleccion.iter().next() {
                Some(n) if self.seleccion.len() == 1 => { let n = n.clone(); self.abrir(&n) }
                _ => vec![],
            },
            Comando::Subir => vec![Efecto::Subir],
            Comando::Atras => vec![Efecto::Atras],
            Comando::Releer => vec![Efecto::Releer { ocultos: self.ocultos }],
            Comando::Paleta => { self.paleta = Some(Paleta::default()); self.panel_ajustes = false; vec![] }
            Comando::Ajustes => { self.panel_ajustes = !self.panel_ajustes; self.paleta = None; vec![] }
            Comando::SiguienteTema => { let i = (self.ajustes.indice() + 1) % TEMAS.len(); self.poner_tema(i) }
            Comando::Tema(i) => if (i as usize) < TEMAS.len() { self.poner_tema(i as usize) } else { vec![] },
            Comando::Brillo => {
                let actual = self.ajustes.tema(self.acento).brillo;
                self.ajustes.brillo = Some(!actual);
                vec![self.guardar_ajustes()]
            }
            Comando::AcentoEscritorio => { self.ajustes.acento_escritorio = !self.ajustes.acento_escritorio; vec![self.guardar_ajustes()] }
            Comando::Modo => {
                self.ajustes.modo = if self.en_grafo() { Modo::Iconos } else { Modo::Grafo };
                self.seleccion.clear();
                if self.en_grafo() { self.encuadrar(); }
                vec![self.guardar_ajustes()]
            }
            Comando::Explorador => { self.ajustes.explorador = !self.ajustes.explorador; vec![self.guardar_ajustes()] }
            Comando::Direccion => {
                // Las posiciones a mano eran para la otra direccion: se vuelve al acomodo automatico.
                self.ajustes.direccion = match self.ajustes.direccion { Direccion::Blender => Direccion::Houdini, Direccion::Houdini => Direccion::Blender };
                self.disp.recordar();
                self.disp.grafo.clear();
                self.encuadrar();
                vec![self.guardar_ajustes(), self.guardar()]
            }
            Comando::Encuadrar => { self.encuadrar(); vec![] }
            Comando::Reacomodar => {
                if self.disp.bloqueada || self.disp.grafo.is_empty() { return vec![]; }
                self.disp.recordar();
                self.disp.grafo.clear();
                vec![self.guardar()]
            }
            Comando::Iconos => { self.ajustes.iconos = self.ajustes.iconos.siguiente(); vec![self.guardar_ajustes()] }
            Comando::Carriles => { self.carriles = !self.carriles; vec![] }
            Comando::Terminal => { self.terminal.abierta = !self.terminal.abierta; vec![] }
            Comando::Detener => if self.terminal.en_curso.is_some() { vec![Efecto::Detener] } else { vec![] },
            Comando::LimpiarTerminal => { self.terminal.lineas.clear(); vec![] }
            Comando::Dibujo => {
                self.ajustes.dibujo = match self.ajustes.dibujo { crate::ajustes::Dibujo::Gpu => crate::ajustes::Dibujo::Cpu, crate::ajustes::Dibujo::Cpu => crate::ajustes::Dibujo::Gpu };
                vec![self.guardar_ajustes()]
            }
            Comando::EjecutarSeleccion => match self.seleccion.iter().next() {
                Some(id) if self.seleccion.len() == 1 => { let id = id.clone(); self.ejecutar(&id) }
                _ if self.seleccion.is_empty() && lanzador(self, "").is_some() => self.ejecutar(""),
                _ => vec![Efecto::Releer { ocultos: self.ocultos }],
            },
        }
    }

    /// Play: un proceso a la vez; la terminal se abre y anota que se lanzo y donde.
    fn ejecutar(&mut self, id: &str) -> Vec<Efecto> {
        if self.terminal.en_curso.is_some() {
            self.terminal.abierta = true;
            self.terminal.escribir("ya hay un proceso en marcha: detenlo antes de lanzar otro", true);
            return vec![];
        }
        let Some(l) = lanzador(self, id) else { return vec![] };
        self.lanzar(l.titulo, l.carpeta, l.programa.to_string(), l.args)
    }

    fn lanzar(&mut self, titulo: String, carpeta: String, programa: String, args: Vec<String>) -> Vec<Efecto> {
        self.terminal.abierta = true;
        let donde = if carpeta.is_empty() { self.nombre_raiz.clone() } else { carpeta.clone() };
        self.terminal.escribir(&alloc::format!("$ {titulo}   (en {donde})"), false);
        self.terminal.en_curso = Some(titulo.clone());
        vec![Efecto::Ejecutar { titulo, carpeta, programa, args }]
    }

    fn poner_tema(&mut self, i: usize) -> Vec<Efecto> {
        self.ajustes.tema = TEMAS[i].id.to_string();
        vec![self.guardar_ajustes()]
    }

    fn sincronizar(&mut self) -> Vec<Efecto> {
        let nombres = self.nombres();
        // Lo que ya no existe se olvida: seleccion, desplegadas y posiciones del grafo.
        let arbol = &self.arbol;
        let existe = |id: &String| { let (p, n) = padre_de(id); arbol.get(p).is_some_and(|l| l.iter().any(|e| e.nombre == n)) };
        self.seleccion.retain(|id| existe(id));
        let antes = self.disp.grafo.len();
        let raices: BTreeSet<&String> = nombres.iter().collect();
        self.disp.grafo.retain(|id, _| id.is_empty() || raices.contains(&String::from(id.split('/').next().unwrap_or(""))));
        self.expandidas.retain(|id| raices.contains(&String::from(id.split('/').next().unwrap_or(""))));
        let cambio_grafo = self.disp.grafo.len() != antes;
        if self.disp.sincronizar(&nombres, self.ancho) || cambio_grafo { vec![self.guardar()] } else { vec![] }
    }

    /// Ids de lo que esta en la banda: iconos (coordenadas del lienzo) o nodos (del mundo).
    fn en_banda(&self, caja: ([f32; 2], [f32; 2])) -> Vec<String> {
        if self.en_grafo() {
            acomodar(self).into_iter().filter(|(id, p)| !id.is_empty() && se_cruzan(caja, (*p, [p[0] + NODO[0], p[1] + NODO[1]]))).map(|(id, _)| id).collect()
        } else {
            self.disp.posiciones.iter().filter(|(_, p)| se_cruzan(caja, rect_icono(**p))).map(|(n, _)| n.clone()).collect()
        }
    }
}

fn rect_icono(p: [f32; 2]) -> ([f32; 2], [f32; 2]) { (p, [p[0] + ICONO[0], p[1] + ICONO[1]]) }

fn se_cruzan(a: ([f32; 2], [f32; 2]), b: ([f32; 2], [f32; 2])) -> bool {
    a.0[0] <= b.1[0] && b.0[0] <= a.1[0] && a.0[1] <= b.1[1] && b.0[1] <= a.1[1]
}

/// El reducer: (estado, accion) -> estado nuevo + efectos. Puro: sin E/S, sin reloj.
pub fn reducir(e: &mut Estado, accion: Accion) -> Vec<Efecto> {
    e.version = e.version.wrapping_add(1);
    // Mover la camara o cambiar el tamano no cambia lo que se dibuja: la GPU ya lo tiene.
    if !matches!(accion, Accion::Desplazar(_) | Accion::Zoom { .. } | Accion::Lienzo(_) | Accion::Ancho(_)) {
        e.version_contenido = e.version_contenido.wrapping_add(1);
    }
    match accion {
        Accion::CargarAjustes(a) => { e.ajustes = a; vec![] }
        Accion::Cargar { nombre, entradas, disposicion } => {
            e.nombre_raiz = nombre.chars().take(64).collect();
            e.arbol.clear();
            e.arbol.insert(String::new(), entradas.clone());
            e.entradas = entradas;
            e.disp = disposicion;
            e.seleccion.clear();
            e.expandidas.clear();
            e.arrastrando = false;
            e.banda = None;
            let efectos = e.sincronizar();
            if e.en_grafo() { e.encuadrar(); }
            efectos
        }
        Accion::Refrescar(entradas) => {
            e.arbol.insert(String::new(), entradas.clone());
            e.entradas = entradas;
            e.sincronizar()
        }
        Accion::Contenido { ruta, entradas } => {
            if ruta.is_empty() { return reducir(e, Accion::Refrescar(entradas)); }
            // Solo se acepta lo que se pidio y sigue desplegado (una respuesta tardia no ensucia nada).
            if !e.expandidas.contains(&ruta) || e.entrada(&ruta).map(|x| x.tipo) != Some(Tipo::Carpeta) { return vec![]; }
            e.arbol.insert(ruta, entradas);
            let arbol = &e.arbol;
            e.seleccion.retain(|id| { let (p, n) = padre_de(id); arbol.get(p).is_some_and(|l| l.iter().any(|x| x.nombre == n)) });
            vec![]
        }
        Accion::Ancho(a) => { if a.is_finite() && a > 0.0 { e.ancho = a; } vec![] }
        Accion::Lienzo(t) => { if t.iter().all(|v| v.is_finite() && *v > 0.0) { e.lienzo = t; } vec![] }
        Accion::Pulsar { nombre, sumar } => {
            if e.entrada(&nombre).is_none() { return vec![]; }
            if sumar {
                if !e.seleccion.remove(&nombre) { e.seleccion.insert(nombre); }
            } else {
                e.seleccion = [nombre].into();
            }
            vec![]
        }
        Accion::Abrir(nombre) => e.abrir(&nombre),
        Accion::Expandir(id) => e.expandir(&id),
        Accion::EmpezarArrastre(nombre) => {
            if e.disp.bloqueada || e.entrada(&nombre).is_none() { return vec![]; }
            if !e.seleccion.contains(&nombre) { e.seleccion = [nombre].into(); }
            e.disp.recordar();
            if e.en_grafo() {
                // Lo que se arrastra queda fijado a mano desde donde estaba.
                let pos = acomodar(e);
                for id in e.seleccion.clone() {
                    if let Some(p) = pos.get(&id) { e.disp.grafo.insert(id, *p); }
                }
            }
            e.arrastrando = true;
            vec![]
        }
        Accion::Arrastrar(delta) => {
            if e.arrastrando {
                let g = e.seleccion.clone();
                if e.en_grafo() {
                    let z = e.camara.zoom;
                    e.disp.mover_grafo(&g, [delta[0] / z, delta[1] / z]);
                } else {
                    e.disp.mover(&g, delta);
                }
            }
            vec![]
        }
        Accion::Soltar => {
            if !e.arrastrando { return vec![]; }
            e.arrastrando = false;
            if !e.en_grafo() { let g = e.seleccion.clone(); e.disp.soltar(&g); }
            vec![e.guardar()]
        }
        Accion::EmpezarBanda { punto, sumar } => {
            if !sumar { e.seleccion.clear(); }
            e.banda = Some((punto, punto));
            vec![]
        }
        Accion::MoverBanda(punto) => {
            if let Some((inicio, _)) = e.banda {
                e.banda = Some((inicio, punto));
                let caja = ([inicio[0].min(punto[0]), inicio[1].min(punto[1])], [inicio[0].max(punto[0]), inicio[1].max(punto[1])]);
                let dentro = e.en_banda(caja);
                e.seleccion.extend(dentro);
            }
            vec![]
        }
        Accion::TerminarBanda => { e.banda = None; vec![] }
        Accion::Desplazar(d) => {
            if d.iter().all(|v| v.is_finite()) {
                e.camara.x = (e.camara.x - d[0] / e.camara.zoom).clamp(-1.0e6, 1.0e6);
                e.camara.y = (e.camara.y - d[1] / e.camara.zoom).clamp(-1.0e6, 1.0e6);
            }
            vec![]
        }
        Accion::Zoom { factor, ancla } => {
            if factor.is_finite() && factor > 0.0 && ancla.iter().all(|v| v.is_finite()) {
                let c = e.camara;
                let mundo = [c.x + ancla[0] / c.zoom, c.y + ancla[1] / c.zoom];
                let z = (c.zoom * factor).clamp(0.2, 3.0);
                e.camara = Camara { zoom: z, x: mundo[0] - ancla[0] / z, y: mundo[1] - ancla[1] / z };
            }
            vec![]
        }
        Accion::Comando(c) => e.comando(c),
        Accion::Paleta(p) => paleta(e, p),
        Accion::Ejecutar(id) => e.ejecutar(&id),
        Accion::LineaComando(linea) => {
            if e.terminal.en_curso.is_some() { e.terminal.escribir("ya hay un proceso en marcha", true); return vec![]; }
            match partir_linea(&linea) {
                Some((programa, args)) => {
                    let titulo = linea.split_whitespace().collect::<Vec<_>>().join(" ");
                    e.lanzar(titulo, String::new(), programa, args)
                }
                None => vec![],
            }
        }
        Accion::Salida { linea, error } => { e.terminal.escribir(&linea, error); vec![] }
        Accion::Termino(codigo) => {
            let texto = match codigo { Some(0) => String::from("[termino bien]"), Some(c) => alloc::format!("[termino con codigo {c}]"), None => String::from("[detenido]") };
            e.terminal.escribir(&texto, codigo != Some(0));
            e.terminal.en_curso = None;
            vec![]
        }
    }
}

/// Ruta relativa de un hijo (publica para la plataforma: explorador y grafo).
pub fn ruta_hijo(padre: &str, nombre: &str) -> String { hijo(padre, nombre) }

fn paleta(e: &mut Estado, accion: AccionPaleta) -> Vec<Efecto> {
    match accion {
        AccionPaleta::Abrir => { e.paleta = Some(Paleta::default()); vec![] }
        AccionPaleta::Cerrar => { e.paleta = None; vec![] }
        AccionPaleta::Escribir(texto) => {
            if let Some(p) = &mut e.paleta { p.texto = texto.chars().take(64).collect(); p.elegido = 0; }
            vec![]
        }
        AccionPaleta::Mover(d) => {
            if let Some(p) = &mut e.paleta {
                let n = buscar(&p.texto).len() as i32;
                if n > 0 { p.elegido = (p.elegido as i32 + d).rem_euclid(n) as usize; }
            }
            vec![]
        }
        AccionPaleta::Elegir(i) => {
            if let Some(p) = &mut e.paleta { p.elegido = i; }
            paleta(e, AccionPaleta::Aceptar)
        }
        AccionPaleta::Aceptar => {
            let Some(p) = e.paleta.take() else { return vec![] };
            match buscar(&p.texto).get(p.elegido) {
                Some(info) if info.comando != Comando::Paleta => e.comando(info.comando),
                _ => vec![],
            }
        }
    }
}
