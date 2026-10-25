// SPDX-License-Identifier: GPL-2.0-only
//! Estado, acciones y el reducer: la unica forma de cambiar el estado.
//! El reducer no hace E/S: devuelve Efectos y la plataforma los ejecuta.

use crate::ajustes::Ajustes;
use crate::carriles::MAX_DESPLEGADAS;
use crate::comandos::{buscar, Comando};
use crate::diagnosticos::{self, Errores};
use crate::editor::{AccionBuscar, Pestanas, MAX_EDITOR};
use crate::procesos::{ProcesoVivo, MAX_PROCESOS};
use crate::disposicion::{Disposicion, ICONO};
use crate::ejecutar::{lanzador, partir_linea, Terminal};
use crate::grafo::{acomodar, hijo, limites, raiz_vista, Camara, Direccion, Modo, NODO};
use crate::organizar::{camino_a, es_grupo, id_grupo, organizar, partes_grupo, Hijo};
use crate::proyecto::{AccionIrA, AccionProyecto, BusquedaProyecto, Coincidencia, IrA, MAX_RESULTADOS_IR_A};
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
    /// Pestanas del editor: traer al frente / cerrar (con aviso si hay cambios).
    Pestana(usize),
    CerrarPestana(usize),
    /// Buscar y reemplazar en la pestana activa.
    Buscar(AccionBuscar),
    /// Una carpeta pasa a ser la raiz del grafo y del explorador ("" = la abierta).
    Enfocar(String),
    /// Mostrar una ruta en el grafo y en el explorador (desplegando lo que haga falta).
    Revelar { id: String, editor: bool },
    /// Ctrl+P: ir a un archivo o carpeta de todo el proyecto.
    IrA(AccionIrA),
    ResultadosIrA { generacion: u64, resultados: Vec<(String, bool)> },
    /// Ctrl+Shift+F: buscar texto en todo el proyecto.
    Proyecto(AccionProyecto),
    ResultadosProyecto { generacion: u64, lote: Vec<(String, Vec<Coincidencia>)>, vistos: u64, terminada: bool, truncada: bool },
    /// El hilo del indice termino de recorrer el proyecto.
    Indice(InfoIndice),
    /// Panel lateral: explorador o busqueda.
    Lateral(Lateral),
    /// Los archivos de la ciudad (de la raiz de la vista), ya juntados en bloques si son muchos.
    DatosCiudad { generacion: u64, raiz: String, archivos: Vec<crate::ciudad::ArchivoCiudad> },
    /// El hilo de lineas termino de contar.
    LineasListas { contadas: u64 },
    /// Llego la historia de git (o por que no hay). `ahora`: la hora de la plataforma.
    Historia { resultado: Result<crate::historia::Historia, String>, ahora: u64 },
    /// Llego el analisis (museo + dependencias) de todo el proyecto.
    Analisis(crate::analisis::ResultadoAnalisis),
    /// Aislar: None apaga; Some("") los fallos; Some(id) lo que rodea a ese nodo.
    Aislar(Option<String>),
    /// Revelar un archivo y abrirlo en el editor en esa linea (sospecha del museo).
    IrALinea { id: String, linea: u32 },
}

/// La historia de git de la carpeta abierta.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EstadoHistoria {
    pub datos: crate::historia::Historia,
    pub lista: bool,
    /// Por que no hay historia ("no es un repositorio git"...).
    pub sin: Option<String>,
    /// Hora de la plataforma cuando llego (para "hace 3 dias").
    pub ahora: u64,
    pub version: u64,
}

/// Datos de la vista Ciudad (la plataforma los pide al hilo del indice).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatosCiudad {
    pub generacion: u64,
    /// Para que raiz y con que version del indice se pidieron.
    pub raiz: String,
    pub version_indice: u64,
    pub archivos: Vec<crate::ciudad::ArchivoCiudad>,
    pub cargando: bool,
    /// Sube cuando llegan datos nuevos (la plataforma rehace la ciudad).
    pub version: u64,
}

/// Lo que se esta revelando y que hacer al llegar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Revelar {
    pub id: String,
    /// Abrirlo en el editor (en esa linea y con ese tramo seleccionado).
    pub editor: bool,
    pub linea: Option<u32>,
    pub rango: Option<(usize, usize)>,
    /// Es una carpeta que pasa a ser la raiz.
    pub enfocar: bool,
}

impl Revelar {
    pub fn a(id: &str) -> Revelar { Revelar { id: id.to_string(), ..Revelar::default() } }
}

/// Que muestra el panel de la izquierda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lateral { #[default] Explorador, Buscar, Museo }

/// Estado del indice de todo el proyecto (lo lleva la plataforma en su hilo).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InfoIndice { pub archivos: u64, pub carpetas: u64, pub truncado: bool, pub micros: u64, pub listo: bool,
                         /// Sube con cada indice nuevo y cuando se cuentan las lineas.
                         pub version: u64, pub lineas: u64, pub lineas_listas: bool }

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
    /// Recorrer todo el proyecto (en su hilo) para Ctrl+P y la busqueda.
    Reindexar { ocultos: bool },
    /// Nombres del indice que se parecen a la consulta.
    BuscarArchivos { generacion: u64, consulta: String },
    /// Texto dentro de los archivos de `carpeta` (relativa), en su hilo; cancela la anterior.
    BuscarTexto { generacion: u64, carpeta: String, patron: String, mayusculas: bool, palabra: bool, filtro: String },
    CancelarTexto,
    /// Archivos (con tamano y lineas) de `raiz` para la vista Ciudad.
    PedirCiudad { generacion: u64, raiz: String },
    /// Leer la historia de git de la carpeta abierta (en su hilo, sin tocar el repositorio).
    LeerHistoria,
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
    /// Editor de codigo (clic derecho): pestanas y busqueda.
    pub editor: Pestanas,
    /// El proceso en marcha y sus hijos (de /proc).
    pub procesos: Vec<ProcesoVivo>,
    pub ir_a: Option<IrA>,
    pub proyecto: BusquedaProyecto,
    pub lateral: Lateral,
    pub indice: InfoIndice,
    /// Ruta que se esta revelando (se continua cuando llegan sus carpetas del disco).
    pub revelar: Option<Revelar>,
    /// Ultima ruta revelada y un contador: la plataforma centra el grafo y el explorador.
    pub revelado: (String, u64),
    /// Un aviso corto para la barra de estado.
    pub aviso: Option<String>,
    pub ciudad: DatosCiudad,
    pub historia: EstadoHistoria,
    /// Museo y dependencias del proyecto (del hilo de analisis).
    pub analisis: crate::analisis::EstadoAnalisis,
    /// Aislar (tecla A): None apagado; Some("") los fallos; Some(id) alrededor de un nodo.
    pub aislar: Option<String>,
    /// La conexion de fallos (o del nodo aislado), recalculada solo si cambia algo.
    pub conectado: crate::analisis::Conectado,
    /// Sube cuando cambian los errores atrapados (para recalcular la conexion).
    pub version_fallos: u64,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { entradas: Vec::new(), disp: Disposicion::default(), seleccion: BTreeSet::new(), ocultos: false,
                 ancho: 1100.0, lienzo: [1100.0, 700.0], acento: HIELO, arrastrando: false, banda: None, paleta: None,
                 ajustes: Ajustes::default(), panel_ajustes: false, carriles: false, arbol: BTreeMap::new(),
                 expandidas: BTreeSet::new(), camara: Camara::default(), version: 0, nombre_raiz: String::new(),
                 version_contenido: 0, terminal: Terminal::default(), errores: Errores::default(), raiz_absoluta: String::new(),
                 editor: Pestanas::default(), procesos: Vec::new(), ir_a: None, proyecto: BusquedaProyecto::default(),
                 lateral: Lateral::Explorador, indice: InfoIndice::default(), revelar: None, revelado: (String::new(), 0), aviso: None,
                 ciudad: DatosCiudad::default(), historia: EstadoHistoria::default(),
                 analisis: Default::default(), aislar: None, conectado: Default::default(), version_fallos: 0 }
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

    /// Existe en la vista: una entrada leida o un grupo de una carpeta leida.
    pub fn existe(&self, id: &str) -> bool {
        match partes_grupo(id) {
            Some((carpeta, _)) => self.arbol.contains_key(carpeta),
            None => self.entrada(id).is_some(),
        }
    }

    /// Carpetas reales desplegadas (sin los grupos virtuales): las que lee y vigila el disco.
    pub fn carpetas_desplegadas(&self) -> impl Iterator<Item = &String> { self.expandidas.iter().filter(|id| !es_grupo(id)) }

    fn abrir(&mut self, id: &str) -> Vec<Efecto> {
        if es_grupo(id) { return self.expandir(id); }
        match self.entrada(id).map(|e| e.tipo) {
            Some(Tipo::Carpeta) if self.en_grafo() => self.expandir(id),
            Some(Tipo::Carpeta) => vec![Efecto::EntrarCarpeta(id.into())],
            Some(_) => vec![Efecto::AbrirArchivo(id.into())],
            None => vec![],
        }
    }

    fn expandir(&mut self, id: &str) -> Vec<Efecto> {
        if es_grupo(id) {
            // Un grupo no lee nada del disco: solo se abre en la vista.
            if self.existe(id) && !self.expandidas.remove(id) && self.expandidas.len() < MAX_DESPLEGADAS * 4 { self.expandidas.insert(id.to_string()); }
            return vec![];
        }
        if self.entrada(id).map(|e| e.tipo) != Some(Tipo::Carpeta) { return vec![]; }
        if self.expandidas.remove(id) { return vec![]; }
        if self.carpetas_desplegadas().count() >= MAX_DESPLEGADAS {
            self.aviso = Some(alloc::format!("Hay {MAX_DESPLEGADAS} carpetas desplegadas (el limite): pliega alguna o enfoca una carpeta [E]"));
            return vec![];
        } // limite duro del carril de disco
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
            Comando::Ocultos => { self.ocultos = !self.ocultos; vec![Efecto::Releer { ocultos: self.ocultos }, Efecto::Reindexar { ocultos: self.ocultos }] }
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
                self.ajustes.modo = match self.ajustes.modo { Modo::Iconos => Modo::Grafo, Modo::Grafo => Modo::Ciudad, Modo::Ciudad => Modo::Iconos };
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
            Comando::Guardar => match self.editor.para_guardar() {
                Some((id, texto)) => vec![Efecto::GuardarArchivo { id, texto }],
                None => vec![],
            },
            Comando::CerrarEditor => { let i = self.editor.activa; self.editor.cerrar(i); vec![] }
            Comando::SiguientePestana => { self.editor.mover(1); vec![] }
            Comando::Enfocar => match self.seleccion.iter().next() {
                Some(id) if self.seleccion.len() == 1 => { let id = id.clone(); self.enfocar(&id) }
                _ => vec![],
            },
            Comando::SubirFoco => {
                let f = raiz_vista(self).to_string();
                if f.is_empty() { return vec![]; }
                let padre = padre_de(&f).0.to_string();
                self.seleccion = [f].into();
                self.enfocar(&padre)
            }
            Comando::IrA => self.ir_a(AccionIrA::Abrir),
            Comando::BuscarProyecto => self.proyecto(AccionProyecto::Abrir),
            Comando::Organizar => { self.ajustes.organizar = self.ajustes.organizar.siguiente(); vec![self.guardar_ajustes()] }
            Comando::Reindexar => vec![Efecto::Reindexar { ocultos: self.ocultos }, Efecto::LeerHistoria],
            Comando::Aislar => {
                self.aislar = if self.aislar.is_some() { None } else { Some(self.seleccion.iter().next().cloned().unwrap_or_default()) };
                vec![]
            }
            Comando::Ciudad => { self.ajustes.modo = Modo::Ciudad; vec![self.guardar_ajustes()] }
            Comando::AlturaCiudad => { self.ajustes.altura_ciudad = self.ajustes.altura_ciudad.siguiente(); vec![self.guardar_ajustes()] }
            Comando::HuellaCiudad => { self.ajustes.huella_ciudad = self.ajustes.huella_ciudad.siguiente(); vec![self.guardar_ajustes()] }
            Comando::ColorCiudad => { self.ajustes.color_ciudad = self.ajustes.color_ciudad.siguiente(); vec![self.guardar_ajustes()] }
            Comando::AnteriorPestana => { self.editor.mover(-1); vec![] }
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
        self.version_fallos += 1;
        self.procesos.clear();
        vec![Efecto::Ejecutar { titulo, carpeta, programa, args }]
    }

    /// El editor en un archivo leido (o en el primer error de una carpeta).
    fn abrir_editor(&mut self, id: &str, linea: Option<u32>) -> Vec<Efecto> {
        // Un grupo abre su pieza con errores, o la principal (la cabecera o el codigo).
        if let Some((carpeta, clave)) = partes_grupo(id) {
            let Some(entradas) = self.arbol.get(carpeta) else { return vec![] };
            let miembros: Vec<String> = organizar(entradas, self.ajustes.organizar, Some(clave)).into_iter()
                .filter_map(|h| match h { Hijo::Real(e) if e.tipo != Tipo::Carpeta => Some(hijo(carpeta, &e.nombre)), _ => None }).collect();
            let elegido = miembros.iter().find(|m| !self.errores.de(m).is_empty()).or(miembros.first()).cloned();
            return match elegido { Some(m) => self.abrir_editor(&m, linea), None => vec![] };
        }
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
        match self.editor.abrir(&id, linea) {
            Ok(true) => vec![Efecto::LeerArchivo(id)],
            Ok(false) => vec![],
            Err(aviso) => { self.terminal.escribir(&aviso, true); vec![] }
        }
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

    /// Centra la camara del grafo en un nodo (sin alejar mas de lo que se lee).
    fn centrar(&mut self, id: &str) {
        let Some(p) = acomodar(self).get(id).copied() else { return };
        let z = self.camara.zoom.max(0.8);
        self.camara = Camara { zoom: z, x: p[0] + NODO[0] / 2.0 - self.lienzo[0] / 2.0 / z, y: p[1] + NODO[1] / 2.0 - self.lienzo[1] / 2.0 / z };
    }

    /// Sigue abriendo el camino hacia lo que se revela; cada carpeta que falta se pide al
    /// disco y se continua cuando llega su contenido.
    fn avanzar_revelar(&mut self) -> Vec<Efecto> {
        let Some(Revelar { id, editor, linea, rango, enfocar }) = self.revelar.clone() else { return vec![] };
        let tramos: Vec<&str> = id.split('/').collect();
        // Las carpetas de encima (y la propia si hay que enfocarla).
        let hasta = if enfocar { tramos.len() } else { tramos.len() - 1 };
        let mut camino = String::new();
        for tramo in &tramos[..hasta] {
            let padre = camino.clone();
            camino = hijo(&camino, tramo);
            if !self.arbol.contains_key(&padre) { return vec![]; } // aun llegando
            if self.entrada(&camino).map(|e| e.tipo) != Some(Tipo::Carpeta) {
                self.revelar = None;
                self.aviso = Some(alloc::format!("{id}: ya no esta en el disco"));
                return vec![];
            }
            if !self.arbol.contains_key(&camino) {
                if self.expandidas.contains(&camino) { return vec![]; } // ya pedida
                let ef = self.expandir(&camino);
                if ef.is_empty() { self.revelar = None; return vec![]; } // limite: avisado
                return ef;
            }
            self.expandidas.insert(camino.clone());
        }
        self.revelar = None;
        // Cambia lo que se ve: aunque la accion venga de Ctrl+P (que no rehace la vista al teclear).
        self.version_contenido = self.version_contenido.wrapping_add(1);
        if enfocar {
            self.disp.foco = id.clone();
            if self.en_grafo() { self.encuadrar(); }
            return vec![self.guardar()];
        }
        let (padre, nombre) = padre_de(&id);
        let Some(entradas) = self.arbol.get(padre) else { return vec![] };
        if !entradas.iter().any(|e| e.nombre == nombre) { self.aviso = Some(alloc::format!("{id}: ya no esta en el disco")); return vec![]; }
        for clave in camino_a(entradas, self.ajustes.organizar, nombre) { self.expandidas.insert(id_grupo(padre, &clave)); }
        // Si quedo fuera de la carpeta enfocada, se quita el foco para poder verlo.
        let raiz = raiz_vista(self).to_string();
        if !raiz.is_empty() && !id.starts_with(&(raiz + "/")) { self.disp.foco.clear(); }
        self.seleccion = [id.clone()].into();
        self.revelado = (id.clone(), self.revelado.1 + 1);
        self.version_contenido = self.version_contenido.wrapping_add(1);
        if self.en_grafo() { self.centrar(&id); }
        let carpeta = self.entrada(&id).is_some_and(|e| e.tipo == Tipo::Carpeta);
        if !editor || carpeta { return vec![]; }
        let ef = self.abrir_editor(&id, linea);
        if let Some(r) = rango { self.editor.marcar(&id, r); }
        ef
    }

    fn enfocar(&mut self, id: &str) -> Vec<Efecto> {
        if id.is_empty() {
            self.version_contenido = self.version_contenido.wrapping_add(1);
            self.disp.foco.clear();
            if self.en_grafo() { self.encuadrar(); }
            return vec![self.guardar()];
        }
        // Una carpeta aun no leida (doble clic en un distrito de la ciudad) tambien vale: el
        // camino se valida al ir llegando del disco.
        if es_grupo(id) || self.entrada(id).is_some_and(|e| e.tipo != Tipo::Carpeta) { return vec![]; }
        self.seleccion.clear();
        self.revelar = Some(Revelar { enfocar: true, ..Revelar::a(id) });
        self.avanzar_revelar()
    }

    fn ir_a(&mut self, a: AccionIrA) -> Vec<Efecto> {
        match a {
            AccionIrA::Abrir => { self.ir_a = Some(IrA::default()); self.paleta = None; vec![] }
            AccionIrA::Cerrar => { self.ir_a = None; vec![] }
            AccionIrA::Texto(t) => {
                let Some(ir) = &mut self.ir_a else { return vec![] };
                ir.texto = t.chars().take(128).collect();
                ir.generacion += 1;
                ir.elegido = 0;
                if ir.texto.trim().is_empty() { ir.resultados.clear(); ir.buscando = false; return vec![]; }
                ir.buscando = true;
                vec![Efecto::BuscarArchivos { generacion: ir.generacion, consulta: ir.texto.clone() }]
            }
            AccionIrA::Mover(d) => {
                if let Some(ir) = &mut self.ir_a { let n = ir.resultados.len() as i32; if n > 0 { ir.elegido = (ir.elegido as i32 + d).rem_euclid(n) as usize; } }
                vec![]
            }
            AccionIrA::Elegir(i) => { if let Some(ir) = &mut self.ir_a { ir.elegido = i; } self.ir_a(AccionIrA::Aceptar { editor: false }) }
            AccionIrA::Aceptar { editor } => {
                // Enter mientras busca: se recuerda y se acepta el primero al llegar.
                if let Some(ir) = &mut self.ir_a { if ir.buscando { ir.aceptar_al_llegar = Some(editor); return vec![]; } }
                let Some(ir) = self.ir_a.take() else { return vec![] };
                let Some((ruta, carpeta)) = ir.resultados.get(ir.elegido).cloned() else { return vec![] };
                // Ctrl+Enter: un archivo se abre en el editor; una carpeta se enfoca.
                if carpeta && editor { return self.enfocar(&ruta); }
                self.revelar = Some(Revelar { editor, ..Revelar::a(&ruta) });
                self.avanzar_revelar()
            }
        }
    }

    fn lanzar_busqueda(&mut self) -> Vec<Efecto> {
        if !self.proyecto.por_archivo.is_empty() { self.version_contenido = self.version_contenido.wrapping_add(1); } // se van las insignias
        let p = &mut self.proyecto;
        p.generacion += 1;
        p.por_archivo.clear();
        p.total = 0;
        p.archivos_vistos = 0;
        p.truncada = false;
        if p.patron.is_empty() { p.buscando = false; return vec![Efecto::CancelarTexto]; }
        p.buscando = true;
        vec![Efecto::BuscarTexto { generacion: p.generacion, carpeta: raiz_vista(self).to_string(), patron: self.proyecto.patron.clone(),
                                   mayusculas: self.proyecto.mayusculas, palabra: self.proyecto.palabra, filtro: self.proyecto.filtro.clone() }]
    }

    fn proyecto(&mut self, a: AccionProyecto) -> Vec<Efecto> {
        match a {
            AccionProyecto::Abrir => {
                self.lateral = Lateral::Buscar;
                if !self.ajustes.explorador { self.ajustes.explorador = true; }
                self.proyecto.foco += 1;
                vec![]
            }
            AccionProyecto::Patron(t) => {
                self.proyecto.patron = t.chars().take(crate::editor::MAX_PATRON).collect();
                // Busca mientras escribes desde 2 letras; la busqueda anterior se cancela sola.
                if self.proyecto.patron.chars().count() >= 2 || self.proyecto.patron.is_empty() { self.lanzar_busqueda() } else { vec![] }
            }
            AccionProyecto::Mayusculas => { self.proyecto.mayusculas = !self.proyecto.mayusculas; self.lanzar_busqueda() }
            AccionProyecto::Palabra => { self.proyecto.palabra = !self.proyecto.palabra; self.lanzar_busqueda() }
            AccionProyecto::Filtro(f) => { self.proyecto.filtro = f.chars().take(256).collect(); self.lanzar_busqueda() }
            AccionProyecto::Lanzar => self.lanzar_busqueda(),
            AccionProyecto::Limpiar => { self.proyecto.patron.clear(); self.lanzar_busqueda() }
            AccionProyecto::Plegar(id) => { if !self.proyecto.plegados.remove(&id) { self.proyecto.plegados.insert(id); } vec![] }
            AccionProyecto::Ir { id, i } => {
                let Some(c) = self.proyecto.por_archivo.get(&id).and_then(|v| v.get(i)).cloned() else { return vec![] };
                // En las dos vistas: se revela el nodo y se abre el editor con la coincidencia elegida.
                self.revelar = Some(Revelar { editor: true, linea: Some(c.linea), rango: Some(c.rango), ..Revelar::a(&id) });
                self.avanzar_revelar()
            }
        }
    }

    fn poner_tema(&mut self, i: usize) -> Vec<Efecto> {
        self.ajustes.tema = TEMAS[i].id.to_string();
        vec![self.guardar_ajustes()]
    }

    fn sincronizar(&mut self) -> Vec<Efecto> {
        let nombres = self.nombres();
        // Lo que ya no existe se olvida: seleccion, desplegadas y posiciones del grafo.
        let quedan: BTreeSet<String> = self.seleccion.iter().filter(|id| self.existe(id)).cloned().collect();
        self.seleccion = quedan;
        let antes = self.disp.grafo.len();
        let raices: BTreeSet<&String> = nombres.iter().collect();
        // Los grupos de la raiz empiezan por NUL: siguen mientras siga la carpeta abierta.
        let sigue = |id: &str| { let t = id.split('/').next().unwrap_or(""); t.starts_with(crate::organizar::MARCA) || raices.contains(&String::from(t)) };
        self.disp.grafo.retain(|id, _| id.is_empty() || sigue(id));
        self.expandidas.retain(|id| sigue(id));
        if !self.disp.foco.is_empty() && !sigue(&self.disp.foco) { self.disp.foco.clear(); }
        let cambio_grafo = self.disp.grafo.len() != antes;
        if self.disp.sincronizar(&nombres, self.ancho) || cambio_grafo { vec![self.guardar()] } else { vec![] }
    }

    /// Ids de lo que esta en la banda: iconos (coordenadas del lienzo) o nodos (del mundo).
    fn en_banda(&self, caja: ([f32; 2], [f32; 2])) -> Vec<String> {
        if self.en_grafo() {
            let raiz = raiz_vista(self).to_string();
            acomodar(self).into_iter().filter(|(id, p)| *id != raiz && se_cruzan(caja, (*p, [p[0] + NODO[0], p[1] + NODO[1]]))).map(|(id, _)| id).collect()
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
    let mut efectos = reducir_sin_ciudad(e, accion);
    efectos.extend(e.pedir_ciudad());
    e.reconectar();
    efectos
}

impl Estado {
    /// Recalcula la conexion de fallos si cambiaron el analisis, los errores o lo aislado.
    fn reconectar(&mut self) {
        let clave = (self.analisis.version, self.version_fallos, self.aislar.clone());
        if clave == self.conectado.clave { return; }
        let aislado = self.aislar.as_deref().filter(|x| !x.is_empty()).map(|id| (id, self.archivos_de(id)));
        let (modo, conexion, fallos) = crate::analisis::conectar_estado(&self.errores, &self.analisis, aislado);
        self.conectado = crate::analisis::Conectado { modo, conexion, fallos, clave };
        self.version_contenido = self.version_contenido.wrapping_add(1);
    }

    /// Los archivos de un nodo: el mismo, los miembros de un grupo o lo que hay en una carpeta.
    pub fn archivos_de(&self, id: &str) -> Vec<String> {
        if let Some((carpeta, clave)) = partes_grupo(id) {
            let Some(entradas) = self.arbol.get(carpeta) else { return vec![] };
            let mut dentro: Option<String> = None;
            for _ in 0..8 {
                let mut siguiente = None;
                for h in organizar(entradas, self.ajustes.organizar, dentro.as_deref()) {
                    if let Hijo::Grupo(g) = h {
                        if g.clave == clave { return g.miembros.iter().map(|m| hijo(carpeta, &m.nombre)).collect(); }
                        if clave.starts_with(g.clave.as_str()) { siguiente = Some(g.clave.clone()); }
                    }
                }
                match siguiente { Some(c) => dentro = Some(c), None => break }
            }
            return vec![];
        }
        if self.analisis.dep.id(id).is_some() || self.entrada(id).is_some_and(|x| x.tipo != Tipo::Carpeta) { return vec![id.to_string()]; }
        let prefijo = if id.is_empty() { String::new() } else { alloc::format!("{id}/") };
        self.analisis.dep.rutas.iter().filter(|r| r.starts_with(&prefijo)).take(2000).cloned().collect()
    }

    /// En la vista Ciudad: si cambio la raiz o el indice, se piden los datos otra vez.
    fn pedir_ciudad(&mut self) -> Vec<Efecto> {
        if self.ajustes.modo != Modo::Ciudad || !self.indice.listo || self.ciudad.cargando { return vec![]; }
        let raiz = raiz_vista(self).to_string();
        if self.ciudad.version > 0 && self.ciudad.raiz == raiz && self.ciudad.version_indice == self.indice.version { return vec![]; }
        self.ciudad.generacion += 1;
        self.ciudad.cargando = true;
        self.ciudad.raiz = raiz.clone();
        self.ciudad.version_indice = self.indice.version;
        vec![Efecto::PedirCiudad { generacion: self.ciudad.generacion, raiz }]
    }
}

fn reducir_sin_ciudad(e: &mut Estado, accion: Accion) -> Vec<Efecto> {
    e.version = e.version.wrapping_add(1);
    // Mover la camara o cambiar el tamano no cambia lo que se dibuja: la GPU ya lo tiene.
    // Tampoco la salida de un proceso, /proc o teclear en el editor: si de ahi sale un error
    // nuevo en un nodo, se marca abajo a mano.
    if !matches!(accion, Accion::Desplazar(_) | Accion::Zoom { .. } | Accion::Lienzo(_) | Accion::Ancho(_) | Accion::Salida { .. }
                       | Accion::Procesos(_) | Accion::Editar(_) | Accion::RutaRaiz(_) | Accion::Buscar(_) | Accion::Pestana(_)
                       | Accion::CerrarPestana(_) | Accion::ArchivoLeido { .. } | Accion::Guardado { .. } | Accion::IrA(_)
                       | Accion::ResultadosIrA { .. } | Accion::Indice(_) | Accion::Lateral(_) | Accion::ResultadosProyecto { .. }
                       | Accion::DatosCiudad { .. } | Accion::LineasListas { .. } | Accion::Historia { .. }
                       | Accion::Proyecto(AccionProyecto::Patron(_) | AccionProyecto::Mayusculas | AccionProyecto::Palabra
                                          | AccionProyecto::Filtro(_) | AccionProyecto::Lanzar | AccionProyecto::Limpiar | AccionProyecto::Plegar(_))) {
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
            e.revelar = None;
            e.ir_a = None;
            e.proyecto = BusquedaProyecto { patron: core::mem::take(&mut e.proyecto.patron), mayusculas: e.proyecto.mayusculas,
                                            palabra: e.proyecto.palabra, filtro: core::mem::take(&mut e.proyecto.filtro), ..BusquedaProyecto::default() };
            e.indice = InfoIndice { version: e.indice.version + 1, ..InfoIndice::default() };
            e.ciudad = DatosCiudad { generacion: e.ciudad.generacion, ..DatosCiudad::default() };
            let mut efectos = e.sincronizar();
            if e.en_grafo() { e.encuadrar(); }
            efectos.push(Efecto::Reindexar { ocultos: e.ocultos });
            e.historia = EstadoHistoria { version: e.historia.version + 1, ..EstadoHistoria::default() };
            efectos.push(Efecto::LeerHistoria);
            e.analisis = crate::analisis::EstadoAnalisis { version: e.analisis.version + 1, ..Default::default() };
            e.aislar = None;
            // El foco guardado: se abre el camino hasta el (sus carpetas aun no estan leidas).
            let foco = e.disp.foco.clone();
            if !foco.is_empty() { e.revelar = Some(Revelar { enfocar: true, ..Revelar::a(&foco) }); efectos.extend(e.avanzar_revelar()); }
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
            let quedan: BTreeSet<String> = e.seleccion.iter().filter(|id| e.existe(id)).cloned().collect();
            e.seleccion = quedan;
            let mut ef = e.desplegar_errores();
            ef.extend(e.avanzar_revelar());
            ef
        }
        Accion::Ancho(a) => { if a.is_finite() && a > 0.0 { e.ancho = a; } vec![] }
        Accion::Lienzo(t) => { if t.iter().all(|v| v.is_finite() && *v > 0.0) { e.lienzo = t; } vec![] }
        Accion::Pulsar { nombre, sumar } => {
            if !e.existe(&nombre) { return vec![]; }
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
            if e.disp.bloqueada || !e.existe(&nombre) { return vec![]; }
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
        Accion::ArchivoLeido { id, resultado } => { e.editor.leido(&id, resultado); vec![] }
        Accion::Editar(t) => { e.editor.editar(t); vec![] }
        Accion::Guardado { id, resultado } => { e.editor.guardado(&id, resultado); vec![] }
        Accion::Pestana(i) => { e.editor.elegir(i); vec![] }
        Accion::CerrarPestana(i) => { e.editor.cerrar(i); vec![] }
        Accion::Buscar(b) => { e.editor.buscar(b); vec![] }
        Accion::Enfocar(id) => e.enfocar(&id),
        Accion::Revelar { id, editor } => { e.revelar = Some(Revelar { editor, ..Revelar::a(&id) }); e.avanzar_revelar() }
        Accion::IrA(a) => e.ir_a(a),
        Accion::ResultadosIrA { generacion, mut resultados } => {
            if let Some(ir) = &mut e.ir_a {
                if ir.generacion == generacion {
                    resultados.truncate(MAX_RESULTADOS_IR_A);
                    ir.resultados = resultados;
                    ir.buscando = false;
                    ir.elegido = 0;
                    if let Some(editor) = ir.aceptar_al_llegar.take() { return e.ir_a(AccionIrA::Aceptar { editor }); }
                }
            }
            vec![]
        }
        Accion::Proyecto(a) => e.proyecto(a),
        Accion::ResultadosProyecto { generacion, lote, vistos, terminada, truncada } => {
            let p = &mut e.proyecto;
            if p.generacion != generacion { return vec![]; }
            let habia = !lote.is_empty();
            for (id, v) in lote {
                if p.total >= crate::proyecto::MAX_COINCIDENCIAS { p.truncada = true; break; }
                p.total += v.len();
                p.por_archivo.insert(id, v);
            }
            p.archivos_vistos = vistos;
            p.buscando = !terminada;
            p.truncada |= truncada;
            // Los nodos con coincidencias cambian (insignia): se rehace la vista una vez por lote.
            if habia { e.version_contenido = e.version_contenido.wrapping_add(1); }
            vec![]
        }
        Accion::Indice(i) => { let v = e.indice.version + 1; e.indice = InfoIndice { version: v, ..i }; vec![] }
        Accion::Historia { resultado, ahora } => {
            let h = &mut e.historia;
            match resultado { Ok(d) => { h.datos = d; h.sin = None; } Err(m) => { h.datos = Default::default(); h.sin = Some(m); } }
            h.lista = true;
            h.ahora = ahora;
            h.version += 1;
            vec![]
        }
        Accion::Analisis(r) => { e.analisis.llegar(r); vec![] }
        Accion::Aislar(a) => { e.aislar = a; vec![] }
        Accion::IrALinea { id, linea } => {
            e.revelar = Some(Revelar { editor: true, linea: Some(linea), ..Revelar::a(&id) });
            e.avanzar_revelar()
        }
        Accion::LineasListas { contadas } => { e.indice.lineas = contadas; e.indice.lineas_listas = true; e.indice.version += 1; vec![] }
        Accion::DatosCiudad { generacion, raiz, archivos } => {
            if generacion == e.ciudad.generacion {
                e.ciudad.archivos = archivos;
                e.ciudad.raiz = raiz;
                e.ciudad.cargando = false;
                e.ciudad.version += 1;
            }
            vec![]
        }
        Accion::Lateral(l) => { e.lateral = l; if !e.ajustes.explorador { e.ajustes.explorador = true; } vec![] }
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
        self.version_fallos += 1;
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
