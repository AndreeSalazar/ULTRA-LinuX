// SPDX-License-Identifier: GPL-2.0-only
//! La vista declarativa: a partir del estado, que hay que dibujar. Ningun pixel aqui:
//! Linux lo pinta con egui/wgpu y BMO-X con su propio framebuffer.

use crate::comandos::buscar;
use crate::disposicion::{CELDA, ICONO};
use crate::estado::Estado;
use crate::tema::{Tema, TEMAS};
use crate::tipos::{Rgb, Tipo};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Nodo {
    /// Un punto de rejilla en cada celda.
    Rejilla { paso: [f32; 2] },
    Icono { nombre: String, pos: [f32; 2], tam: [f32; 2], tipo: Tipo, color: Rgb, seleccionado: bool, etiqueta: String },
    Banda { desde: [f32; 2], hasta: [f32; 2], color: Rgb },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VistaPaleta {
    pub texto: String,
    /// (titulo, atajo, elegido)
    pub filas: Vec<(&'static str, &'static str, bool)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VistaAjustes {
    /// (id, nombre, descripcion, muestras, elegido)
    pub temas: Vec<(&'static str, &'static str, &'static str, [Rgb; 5], bool)>,
    pub brillo: bool,
    pub acento_escritorio: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vista {
    /// El tema ya resuelto (con el acento y el brillo que eligio el usuario).
    pub tema: Tema,
    pub ajustes: Option<VistaAjustes>,
    pub nodos: Vec<Nodo>,
    /// Tamano del lienzo que ocupan los iconos.
    pub tam: [f32; 2],
    pub estado: String,
    pub paleta: Option<VistaPaleta>,
}

pub fn vista(e: &Estado) -> Vista {
    let tema = e.ajustes.tema(e.acento);
    let mut nodos = Vec::new();
    if e.disp.rejilla { nodos.push(Nodo::Rejilla { paso: CELDA }); }
    let mut tam = [0.0f32, 0.0f32];
    for entrada in &e.entradas {
        let Some(p) = e.disp.posiciones.get(&entrada.nombre) else { continue };
        tam = [tam[0].max(p[0] + CELDA[0]), tam[1].max(p[1] + CELDA[1])];
        let etiqueta = if entrada.tipo == Tipo::Carpeta { String::new() } else {
            entrada.nombre.rsplit_once('.').map(|(_, x)| x.to_uppercase()).filter(|x| x.len() <= 5).unwrap_or_default()
        };
        nodos.push(Nodo::Icono { nombre: entrada.nombre.clone(), pos: *p, tam: ICONO, tipo: entrada.tipo,
                                 color: tema.color_tipo(entrada.tipo), seleccionado: e.seleccion.contains(&entrada.nombre), etiqueta });
    }
    if let Some((desde, hasta)) = e.banda { nodos.push(Nodo::Banda { desde, hasta, color: tema.acento }); }
    let mut estado = format!("{} elementos · {} seleccionados", e.entradas.len(), e.seleccion.len());
    if e.disp.rejilla { estado.push_str(" · rejilla"); }
    if e.disp.bloqueada { estado.push_str(" · candado puesto"); }
    if e.disp.puede_deshacer() { estado.push_str(" · Ctrl+Z deshace"); }
    let paleta = e.paleta.as_ref().map(|p| VistaPaleta {
        texto: p.texto.clone(),
        filas: buscar(&p.texto).iter().enumerate().map(|(i, c)| (c.titulo, c.atajo, i == p.elegido)).collect(),
    });
    let ajustes = e.panel_ajustes.then(|| VistaAjustes {
        temas: TEMAS.iter().enumerate().map(|(i, t)| (t.id, t.nombre, t.descripcion, t.muestras(), i == e.ajustes.indice())).collect(),
        brillo: tema.brillo,
        acento_escritorio: e.ajustes.acento_escritorio,
    });
    Vista { tema, ajustes, nodos, tam, estado, paleta }
}
