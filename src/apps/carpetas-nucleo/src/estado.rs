// SPDX-License-Identifier: GPL-2.0-only
//! Estado, acciones y el reducer: la unica forma de cambiar el estado.
//! El reducer no hace E/S: devuelve Efectos y la plataforma los ejecuta.

use crate::ajustes::Ajustes;
use crate::comandos::{buscar, Comando};
use crate::tema::TEMAS;
use crate::disposicion::{Disposicion, ICONO};
use crate::tipos::{Rgb, Tipo, HIELO};
use alloc::collections::BTreeSet;
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
    Cargar { entradas: Vec<Entrada>, disposicion: Disposicion },
    /// La misma carpeta cambio en disco.
    Refrescar(Vec<Entrada>),
    Ancho(f32),
    Pulsar { nombre: String, sumar: bool },
    Abrir(String),
    EmpezarArrastre(String),
    Arrastrar([f32; 2]),
    Soltar,
    EmpezarBanda { punto: [f32; 2], sumar: bool },
    MoverBanda([f32; 2]),
    TerminarBanda,
    Comando(Comando),
    Paleta(AccionPaleta),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Efecto {
    /// Guardar el orden de esta carpeta (texto JSON).
    Guardar(String),
    /// Guardar los ajustes de la app (texto JSON).
    GuardarAjustes(String),
    AbrirArchivo(String),
    EntrarCarpeta(String),
    Subir,
    Atras,
    Releer { ocultos: bool },
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
    pub acento: Rgb,
    pub arrastrando: bool,
    pub banda: Option<([f32; 2], [f32; 2])>,
    pub paleta: Option<Paleta>,
    pub ajustes: Ajustes,
    pub panel_ajustes: bool,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { entradas: Vec::new(), disp: Disposicion::default(), seleccion: BTreeSet::new(), ocultos: false,
                 ancho: 1100.0, acento: HIELO, arrastrando: false, banda: None, paleta: None,
                 ajustes: Ajustes::default(), panel_ajustes: false }
    }
}

impl Estado {
    fn nombres(&self) -> Vec<String> { self.entradas.iter().map(|e| e.nombre.clone()).collect() }
    fn guardar(&self) -> Efecto { Efecto::Guardar(self.disp.a_json()) }
    fn tipo_de(&self, nombre: &str) -> Option<Tipo> { self.entradas.iter().find(|e| e.nombre == nombre).map(|e| e.tipo) }

    fn abrir(&self, nombre: &str) -> Vec<Efecto> {
        match self.tipo_de(nombre) {
            Some(Tipo::Carpeta) => vec![Efecto::EntrarCarpeta(nombre.into())],
            Some(_) => vec![Efecto::AbrirArchivo(nombre.into())],
            None => vec![],
        }
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
                vec![Efecto::GuardarAjustes(self.ajustes.a_json())]
            }
            Comando::AcentoEscritorio => { self.ajustes.acento_escritorio = !self.ajustes.acento_escritorio; vec![Efecto::GuardarAjustes(self.ajustes.a_json())] }
        }
    }

    fn poner_tema(&mut self, i: usize) -> Vec<Efecto> {
        self.ajustes.tema = TEMAS[i].id.to_string();
        vec![Efecto::GuardarAjustes(self.ajustes.a_json())]
    }

    fn sincronizar(&mut self) -> Vec<Efecto> {
        let nombres = self.nombres();
        let presentes: BTreeSet<&String> = nombres.iter().collect();
        self.seleccion.retain(|n| presentes.contains(n));
        if self.disp.sincronizar(&nombres, self.ancho) { vec![self.guardar()] } else { vec![] }
    }
}

fn rect_icono(p: [f32; 2]) -> ([f32; 2], [f32; 2]) { (p, [p[0] + ICONO[0], p[1] + ICONO[1]]) }

fn se_cruzan(a: ([f32; 2], [f32; 2]), b: ([f32; 2], [f32; 2])) -> bool {
    a.0[0] <= b.1[0] && b.0[0] <= a.1[0] && a.0[1] <= b.1[1] && b.0[1] <= a.1[1]
}

/// El reducer: (estado, accion) -> estado nuevo + efectos. Puro: sin E/S, sin reloj.
pub fn reducir(e: &mut Estado, accion: Accion) -> Vec<Efecto> {
    match accion {
        Accion::CargarAjustes(a) => { e.ajustes = a; vec![] }
        Accion::Cargar { entradas, disposicion } => {
            e.entradas = entradas;
            e.disp = disposicion;
            e.seleccion.clear();
            e.arrastrando = false;
            e.banda = None;
            e.sincronizar()
        }
        Accion::Refrescar(entradas) => { e.entradas = entradas; e.sincronizar() }
        Accion::Ancho(a) => { if a.is_finite() && a > 0.0 { e.ancho = a; } vec![] }
        Accion::Pulsar { nombre, sumar } => {
            if e.tipo_de(&nombre).is_none() { return vec![]; }
            if sumar {
                if !e.seleccion.remove(&nombre) { e.seleccion.insert(nombre); }
            } else {
                e.seleccion = [nombre].into();
            }
            vec![]
        }
        Accion::Abrir(nombre) => e.abrir(&nombre),
        Accion::EmpezarArrastre(nombre) => {
            if e.disp.bloqueada || e.tipo_de(&nombre).is_none() { return vec![]; }
            if !e.seleccion.contains(&nombre) { e.seleccion = [nombre].into(); }
            e.disp.recordar();
            e.arrastrando = true;
            vec![]
        }
        Accion::Arrastrar(delta) => {
            if e.arrastrando { let g = e.seleccion.clone(); e.disp.mover(&g, delta); }
            vec![]
        }
        Accion::Soltar => {
            if !e.arrastrando { return vec![]; }
            e.arrastrando = false;
            let g = e.seleccion.clone();
            e.disp.soltar(&g);
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
                let dentro: Vec<String> = e.disp.posiciones.iter().filter(|(_, p)| se_cruzan(caja, rect_icono(**p))).map(|(n, _)| n.clone()).collect();
                e.seleccion.extend(dentro);
            }
            vec![]
        }
        Accion::TerminarBanda => { e.banda = None; vec![] }
        Accion::Comando(c) => e.comando(c),
        Accion::Paleta(p) => paleta(e, p),
    }
}

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
