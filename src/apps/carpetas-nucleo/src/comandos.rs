// SPDX-License-Identifier: GPL-2.0-only
//! Comandos como en VS Code: cada cosa que se puede hacer tiene id, titulo y atajo.
//! La paleta (Ctrl+Shift+P) y el teclado usan la misma tabla.

use crate::estado::{Accion, AccionPaleta, Orden};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comando {
    Rejilla, Candado, Ocultos, Deshacer, Ordenar(Orden), SeleccionarTodo, LimpiarSeleccion,
    AbrirSeleccion, Subir, Atras, Releer, Paleta,
}

pub struct InfoComando {
    pub comando: Comando,
    pub id: &'static str,
    pub titulo: &'static str,
    pub atajo: &'static str,
}

pub const COMANDOS: &[InfoComando] = &[
    InfoComando { comando: Comando::Rejilla, id: "carpetas.rejilla", titulo: "Alternar rejilla", atajo: "G" },
    InfoComando { comando: Comando::Candado, id: "carpetas.candado", titulo: "Poner o quitar candado", atajo: "L" },
    InfoComando { comando: Comando::Ocultos, id: "carpetas.ocultos", titulo: "Mostrar u ocultar archivos ocultos", atajo: "H" },
    InfoComando { comando: Comando::Deshacer, id: "carpetas.deshacer", titulo: "Deshacer movimiento", atajo: "Ctrl+Z" },
    InfoComando { comando: Comando::Ordenar(Orden::Nombre), id: "carpetas.ordenarNombre", titulo: "Ordenar por nombre", atajo: "" },
    InfoComando { comando: Comando::Ordenar(Orden::Tipo), id: "carpetas.ordenarTipo", titulo: "Ordenar por tipo", atajo: "" },
    InfoComando { comando: Comando::Ordenar(Orden::Tamano), id: "carpetas.ordenarTamano", titulo: "Ordenar por tamano", atajo: "" },
    InfoComando { comando: Comando::SeleccionarTodo, id: "carpetas.seleccionarTodo", titulo: "Seleccionar todo", atajo: "Ctrl+A" },
    InfoComando { comando: Comando::LimpiarSeleccion, id: "carpetas.limpiar", titulo: "Quitar seleccion", atajo: "Esc" },
    InfoComando { comando: Comando::AbrirSeleccion, id: "carpetas.abrir", titulo: "Abrir lo seleccionado", atajo: "Enter" },
    InfoComando { comando: Comando::Subir, id: "carpetas.subir", titulo: "Ir a la carpeta de arriba", atajo: "Retroceso" },
    InfoComando { comando: Comando::Atras, id: "carpetas.atras", titulo: "Volver a la carpeta anterior", atajo: "Alt+Izquierda" },
    InfoComando { comando: Comando::Releer, id: "carpetas.releer", titulo: "Volver a leer la carpeta", atajo: "F5" },
    InfoComando { comando: Comando::Paleta, id: "carpetas.paleta", titulo: "Paleta de comandos", atajo: "Ctrl+Shift+P" },
];

/// Teclas que entiende el nucleo; cada plataforma traduce las suyas a estas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tecla { G, L, H, Z, A, P, Retroceso, Enter, Escape, F5, Arriba, Abajo, Izquierda }

/// Como el keybindings.json de VS Code: tecla + modificadores -> accion.
pub fn atajo(tecla: Tecla, ctrl: bool, shift: bool, alt: bool, paleta_abierta: bool) -> Option<Accion> {
    if paleta_abierta {
        return match tecla {
            Tecla::Escape => Some(Accion::Paleta(AccionPaleta::Cerrar)),
            Tecla::Enter => Some(Accion::Paleta(AccionPaleta::Aceptar)),
            Tecla::Arriba => Some(Accion::Paleta(AccionPaleta::Mover(-1))),
            Tecla::Abajo => Some(Accion::Paleta(AccionPaleta::Mover(1))),
            _ => None, // el resto lo escribe el cuadro de texto
        };
    }
    let comando = match (tecla, ctrl, shift, alt) {
        (Tecla::P, true, true, _) => Comando::Paleta,
        (Tecla::Z, true, _, _) => Comando::Deshacer,
        (Tecla::A, true, _, _) => Comando::SeleccionarTodo,
        (Tecla::Izquierda, _, _, true) => Comando::Atras,
        (Tecla::Arriba, _, _, true) | (Tecla::Retroceso, false, false, false) => Comando::Subir,
        (Tecla::G, false, false, false) => Comando::Rejilla,
        (Tecla::L, false, false, false) => Comando::Candado,
        (Tecla::H, false, false, false) => Comando::Ocultos,
        (Tecla::Enter, false, false, false) => Comando::AbrirSeleccion,
        (Tecla::Escape, false, false, false) => Comando::LimpiarSeleccion,
        (Tecla::F5, _, _, _) => Comando::Releer,
        _ => return None,
    };
    Some(Accion::Comando(comando))
}

/// Busqueda difusa: las letras del texto deben aparecer en orden en el titulo o el id.
/// Puntua mejor las coincidencias seguidas y al principio de palabra. Sin texto, todos.
pub fn buscar(texto: &str) -> Vec<&'static InfoComando> {
    let texto: Vec<char> = texto.chars().filter(|c| !c.is_whitespace()).flat_map(|c| c.to_lowercase()).collect();
    let mut puntuados: Vec<(i32, usize, &'static InfoComando)> = Vec::new();
    for (i, info) in COMANDOS.iter().enumerate() {
        let mejor = [info.titulo, info.id].iter().filter_map(|t| puntuar(&texto, t)).max();
        if let Some(p) = mejor {
            puntuados.push((p, i, info));
        }
    }
    puntuados.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    puntuados.into_iter().map(|(_, _, info)| info).collect()
}

fn puntuar(texto: &[char], candidato: &str) -> Option<i32> {
    let letras: Vec<char> = candidato.chars().flat_map(|c| c.to_lowercase()).collect();
    let (mut j, mut puntos, mut previo) = (0usize, 0i32, None::<usize>);
    for c in texto {
        while j < letras.len() && letras[j] != *c {
            j += 1;
        }
        if j == letras.len() {
            return None;
        }
        puntos += 1;
        if previo == Some(j.wrapping_sub(1)) { puntos += 3; }
        if j == 0 || matches!(letras[j - 1], ' ' | '.') { puntos += 2; }
        previo = Some(j);
        j += 1;
    }
    Some(puntos - letras.len() as i32 / 8)
}
