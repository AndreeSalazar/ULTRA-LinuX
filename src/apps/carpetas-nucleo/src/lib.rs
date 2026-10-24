// SPDX-License-Identifier: GPL-2.0-only
//! Nucleo de ULTRA Carpetas. `no_std` + `alloc`: sin archivos, hilos, reloj ni GPU.
//!
//! Arquitectura como React + Redux (y los comandos de VS Code):
//!
//! ```text
//!   evento (clic, tecla, arrastre)
//!        │  la plataforma lo traduce
//!        ▼
//!     Accion ──► reducir(&mut Estado, Accion) ──► Vec<Efecto>   (guardar, abrir, navegar…)
//!                       │                               │
//!                       ▼                               ▼
//!                vista(&Estado) ──► Vista        la plataforma los ejecuta
//!                (lista de lo que hay que dibujar)  (Linux: std + egui/wgpu; BMO-X: su propio framebuffer)
//! ```
//!
//! Todo lo que decide el programa vive aqui y se prueba sin pantalla. La plataforma
//! solo dibuja la `Vista`, traduce eventos a `Accion` y ejecuta los `Efecto`.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod ajustes;
pub mod carriles;
pub mod ciudad;
pub mod comandos;
pub mod diagnosticos;
pub mod ejecutar;
pub mod disposicion;
pub mod editor;
pub mod estado;
pub mod grafo;
pub mod historia;
pub mod iconos;
pub mod organizar;
pub mod procesos;
pub mod proyecto;
pub mod tema;
pub mod tipos;
pub mod vista;

pub use ajustes::Ajustes;
pub use tema::{Tema, TEMAS};
pub use comandos::{atajo, buscar, Comando, InfoComando, Tecla, COMANDOS};
pub use disposicion::{Disposicion, CELDA, ICONO};
pub use estado::{reducir, Accion, AccionPaleta, Efecto, Entrada, Estado, Orden};
pub use tipos::{legible, tipo_por_extension, Rgb, Tipo};
pub use vista::{vista, Nodo, Vista, VistaAjustes, VistaPaleta, VistaTerminal};
pub use grafo::{Cable, Camara, Direccion, FilaArbol, Marcas, Modo, NodoGrafo, NODO};
pub use iconos::EstiloIconos;
pub use carriles::{luz, Carril, Luz, PRESUPUESTOS};
pub use ejecutar::{lanzador, Lanzador, Terminal};
pub use ajustes::Dibujo;
pub use diagnosticos::{Diagnostico, Errores};
pub use editor::Editor;
pub use procesos::ProcesoVivo;
