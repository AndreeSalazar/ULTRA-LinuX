// SPDX-License-Identifier: GPL-2.0-only
//! Estado, acciones y el reducer: la unica forma de cambiar el estado.
//! El reducer no hace E/S: devuelve Efectos y la plataforma los ejecuta.

use crate::ajustes::Ajustes;
use crate::carriles::MAX_DESPLEGADAS;
use crate::comandos::{buscar, Comando};
use crate::diagnosticos::{self, Errores};
use crate::editor::{Editor, MAX_EDITOR};
use crate::procesos::{ProcesoVivo, MAX_PROCESOS};
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
    /// Ruta absoluta de la carpeta abierta (para entender rutas absolutas de Python y gcc).
    RutaRaiz(String),
    /// Muestra de /proc: el proceso lanzado y sus hijos. No rehace la geometria.
    Procesos(Vec<ProcesoVivo>),
    /// Clic derecho: abrir el editor de codigo (en una carpeta, su primer error).
    AbrirEditor { id: String, linea: Option<u32> },
    ArchivoLeido { id: String, resultado: Result<String, String> },
    /// El texto del editor cambio (no rehace la geometria).
    Editar(String),
    Guardado { id: String, resultado: Result<(), String> },
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
    /// Leer un archivo de texto para el editor (ruta relativa ya validada contra lo leido).
    LeerArchivo(String),
    /// Guardar el editor: escritura atomica en la plataforma.
    GuardarArchivo { id: String, texto: String },
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
    /// Errores de la ultima ejecucion, por nodo.
    pub errores: Errores,
    pub raiz_absoluta: String,
    /// Editor de codigo abierto (clic derecho).
    pub editor: Option<Editor>,
    /// El proceso en marcha y sus hijos (de /proc).
    pub procesos: Vec<ProcesoVivo>,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { entradas: Vec::new(), disp: Disposicion::default(), seleccion: BTreeSet::new(), ocultos: false,
                 ancho: 1100.0, lienzo: [1100.0, 700.0], acento: HIELO, arrastrando: false, banda: None, paleta: None,
                 ajustes: Ajustes::default(), panel_ajustes: false, carriles: false, arbol: BTreeMap::new(),
                 expandidas: BTreeSet::new(), camara: Camara::default(), version: 0, nombre_raiz: String::new(),
                 version_contenido: 0, terminal: Terminal::default(), errores: Errores::default(), raiz_absoluta: String::new(),
                 editor: None, procesos: Vec::new() }
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
            Comando::PararEnError => { self.ajustes.parar_en_error = !self.ajustes.parar_en_error; vec![self.guardar_ajustes()] }
            Comando::SiguienteError => {
                let lista: Vec<(String, u32)> = self.errores.por_nodo.iter().flat_map(|(k, v)| v.iter().map(move |d| (k.clone(), d.linea))).collect();
                if lista.is_empty() { return vec![]; }
                let i = self.errores.cursor % lista.len();
                self.errores.cursor = i + 1;
                let (id, linea) = lista[i].clone();
                if self.entrada(&id).is_some() { self.seleccion = [id.clone()].into(); }
                self.abrir_editor(&id, Some(linea))
            }
            Comando::Guardar => match &self.editor {
                Some(ed) if ed.cargado && ed.sucio => vec![Efecto::GuardarArchivo { id: ed.id.clone(), texto: ed.texto.clone() }],
                _ => vec![],
            },
            Comando::CerrarEditor => {
                match &mut self.editor {
                    Some(ed) if ed.sucio && !ed.cerrar_pedido => {
                        ed.cerrar_pedido = true;
                        ed.aviso = Some(String::from("Cambios sin guardar: Ctrl+S guarda; cerrar otra vez los descarta"));
                    }
                    _ => self.editor = None,
                }
                vec![]
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
        self.terminal.carpeta = carpeta.clone();
        self.errores.empezar(&carpeta);
        self.procesos.clear();
        vec![Efecto::Ejecutar { titulo, carpeta, programa, args }]
    }

    /// El editor en un archivo leido (o en el primer error de una carpeta).
    fn abrir_editor(&mut self, id: &str, linea: Option<u32>) -> Vec<Efecto> {
        let tipo = if id.is_empty() { Some(Tipo::Carpeta) } else { self.entrada(id).map(|e| e.tipo) };
        let (id, mut linea) = if tipo == Some(Tipo::Carpeta) {
            match self.errores.primero(id) {
                Some((f, d)) => (f.clone(), linea.or(Some(d.linea))),
                None => return vec![],
            }
        } else {
            (String::from(id), linea) // puede ser un error en un archivo aun sin leer
        };
        let Some(entrada) = self.entrada(&id) else {
            self.terminal.escribir(&alloc::format!("{id}: su carpeta aun no se ha leido; despliegala y vuelve a intentar"), true);
            return vec![];
        };
        if entrada.tipo == Tipo::Carpeta { return vec![]; }
        if entrada.tam > MAX_EDITOR {
            self.terminal.escribir(&alloc::format!("{id}: demasiado grande para el editor (max 2 MiB)"), true);
            return vec![];
        }
        if linea.is_none() { linea = self.errores.de(&id).iter().find(|d| d.grave).or(self.errores.de(&id).first()).map(|d| d.linea); }
        let carga = match &mut self.editor {
            Some(ed) if ed.id == id => { ed.ir_a = linea.or(ed.ir_a); ed.saltos += 1; return vec![]; }
            Some(ed) if ed.sucio => {
                ed.aviso = Some(alloc::format!("Guarda (Ctrl+S) o descarta {} antes de abrir otro archivo", ed.id));
                return vec![];
            }
            Some(ed) => ed.carga + 1,
            None => 1,
        };
        self.editor = Some(Editor { id: id.clone(), ir_a: linea.or(Some(1)), saltos: 1, carga, ..Editor::default() });
        vec![Efecto::LeerArchivo(id)]
    }

    /// Despliega las carpetas que llevan a un archivo con error (una vez por ejecucion:
    /// si el usuario la pliega, no se vuelve a abrir sola).
    fn desplegar_errores(&mut self) -> Vec<Efecto> {
        let mut efectos = Vec::new();
        let ids: Vec<String> = self.errores.por_nodo.iter().filter(|(_, v)| v.iter().any(|d| d.grave)).map(|(k, _)| k.clone()).collect();
        for id in ids {
            let mut camino = String::new();
            for tramo in id.split('/').collect::<Vec<_>>().split_last().map(|(_, r)| r.to_vec()).unwrap_or_default() {
                camino = hijo(&camino, tramo);
                if self.entrada(&camino).map(|e| e.tipo) != Some(Tipo::Carpeta) { break; }
                if !self.expandidas.contains(&camino) {
                    if !self.errores.desplegadas.insert(camino.clone()) { break; }
                    efectos.extend(self.expandir(&camino.clone()));
                    break; // el resto, cuando llegue su contenido
                }
            }
        }
        efectos
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
    // Tampoco la salida de un proceso, /proc o teclear en el editor: si de ahi sale un error
    // nuevo en un nodo, se marca abajo a mano.
    if !matches!(accion, Accion::Desplazar(_) | Accion::Zoom { .. } | Accion::Lienzo(_) | Accion::Ancho(_) | Accion::Salida { .. }
                       | Accion::Procesos(_) | Accion::Editar(_) | Accion::RutaRaiz(_)) {
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
            e.desplegar_errores()
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
        Accion::Salida { linea, error } => {
            e.terminal.escribir(&linea, error);
            let leidos = diagnosticos::leer(&mut e.errores.lector, &linea);
            e.notar_errores(leidos)
        }
        Accion::Termino(codigo) => {
            let texto = match codigo { Some(0) => String::from("[termino bien]"), Some(c) => alloc::format!("[termino con codigo {c}]"), None => String::from("[detenido]") };
            e.terminal.escribir(&texto, codigo != Some(0));
            e.terminal.en_curso = None;
            e.procesos.clear();
            let quedan = diagnosticos::cerrar(&mut e.errores.lector);
            let efectos = e.notar_errores(quedan);
            if e.errores.total > 0 {
                let (err, av) = e.errores.conteo("");
                let archivos = e.errores.por_nodo.len() as u32;
                let resumen = match (err, av) {
                    (0, _) => diagnosticos::contar(av, "aviso", "avisos"),
                    (_, 0) => diagnosticos::contar(err, "error", "errores"),
                    _ => alloc::format!("{} y {}", diagnosticos::contar(err, "error", "errores"), diagnosticos::contar(av, "aviso", "avisos")),
                };
                let color = if err > 0 { "rojo" } else { "amarillo" };
                e.terminal.escribir(&alloc::format!("{resumen} en {}: el nodo en {color} lo marca; clic derecho o F8 abre el editor en la linea",
                    diagnosticos::contar(archivos, "archivo", "archivos")), err > 0);
            }
            efectos
        }
        Accion::RutaRaiz(r) => { e.raiz_absoluta = r.chars().take(4096).collect(); vec![] }
        Accion::Procesos(mut lista) => {
            if e.terminal.en_curso.is_some() { lista.truncate(MAX_PROCESOS); e.procesos = lista; }
            vec![]
        }
        Accion::AbrirEditor { id, linea } => e.abrir_editor(&id, linea),
        Accion::ArchivoLeido { id, resultado } => {
            if let Some(ed) = &mut e.editor {
                if ed.id == id && !ed.cargado {
                    match resultado {
                        Ok(t) => { ed.texto = t; ed.cargado = true; ed.carga += 1; }
                        Err(m) => ed.aviso = Some(m),
                    }
                }
            }
            vec![]
        }
        Accion::Editar(t) => {
            if let Some(ed) = &mut e.editor {
                if ed.cargado && t != ed.texto {
                    if t.len() as u64 > MAX_EDITOR { ed.aviso = Some(String::from("El texto pasa de 2 MiB: no se acepta")); return vec![]; }
                    ed.texto = t;
                    ed.sucio = true;
                    ed.cerrar_pedido = false;
                }
            }
            vec![]
        }
        Accion::Guardado { id, resultado } => {
            if let Some(ed) = &mut e.editor {
                if ed.id == id {
                    match resultado {
                        Ok(()) => { ed.sucio = false; ed.cerrar_pedido = false; ed.aviso = Some(String::from("Guardado. F5 para volver a ejecutar")); }
                        Err(m) => ed.aviso = Some(m),
                    }
                }
            }
            vec![]
        }
    }
}

impl Estado {
    /// Errores nuevos de la salida: se marcan en sus nodos y se despliega el camino.
    fn notar_errores(&mut self, leidos: Vec<(String, crate::diagnosticos::Diagnostico)>) -> Vec<Efecto> {
        if leidos.is_empty() { return vec![]; }
        let raiz = self.raiz_absoluta.clone();
        let (nuevo, grave) = self.errores.anotar(leidos, &raiz);
        if !nuevo { return vec![]; }
        self.version_contenido = self.version_contenido.wrapping_add(1); // el nodo cambia de color
        let mut efectos = self.desplegar_errores();
        if grave && self.ajustes.parar_en_error && self.terminal.en_curso.is_some() && !self.errores.parado {
            self.errores.parado = true;
            self.terminal.escribir("[parado en el primer error: Ajustes > Parar en el primer error]", true);
            efectos.push(Efecto::Detener);
        }
        efectos
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
