// SPDX-License-Identifier: GPL-2.0-only
//! Comandos como en VS Code: cada cosa que se puede hacer tiene id, titulo y atajo.
//! La paleta (Ctrl+Shift+P) y el teclado usan la misma tabla.

use crate::editor::AccionBuscar;
use crate::estado::{Accion, AccionPaleta, Orden};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comando {
    Rejilla, Candado, Ocultos, Deshacer, Ordenar(Orden), SeleccionarTodo, LimpiarSeleccion,
    AbrirSeleccion, Subir, Atras, Releer, Paleta,
    Ajustes, SiguienteTema, Tema(u8), Brillo, AcentoEscritorio,
    Modo, Explorador, Direccion, Encuadrar, Reacomodar, Iconos, Carriles,
    Terminal, Detener, LimpiarTerminal, Dibujo, EjecutarSeleccion,
    SiguienteError, PararEnError, Guardar, CerrarEditor, SiguientePestana, AnteriorPestana,
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
    InfoComando { comando: Comando::Releer, id: "carpetas.releer", titulo: "Volver a leer la carpeta", atajo: "" },
    InfoComando { comando: Comando::Paleta, id: "carpetas.paleta", titulo: "Paleta de comandos", atajo: "Ctrl+Shift+P" },
    InfoComando { comando: Comando::Modo, id: "vista.modo", titulo: "Cambiar entre iconos y grafo de nodos", atajo: "Tab" },
    InfoComando { comando: Comando::Explorador, id: "vista.explorador", titulo: "Mostrar u ocultar el explorador", atajo: "Ctrl+B" },
    InfoComando { comando: Comando::Direccion, id: "grafo.direccion", titulo: "Grafo: Blender (izq-der) o Houdini (arriba-abajo)", atajo: "D" },
    InfoComando { comando: Comando::Encuadrar, id: "grafo.encuadrar", titulo: "Grafo: encuadrar todo", atajo: "Inicio" },
    InfoComando { comando: Comando::Reacomodar, id: "grafo.reacomodar", titulo: "Grafo: volver al acomodo automatico", atajo: "" },
    InfoComando { comando: Comando::Iconos, id: "ajustes.iconos", titulo: "Estilo de iconos: Neon, Material o Lucide", atajo: "I" },
    InfoComando { comando: Comando::EjecutarSeleccion, id: "terminal.ejecutar", titulo: "Ejecutar lo seleccionado (play)", atajo: "F5" },
    InfoComando { comando: Comando::Terminal, id: "terminal.mostrar", titulo: "Mostrar u ocultar la terminal", atajo: "Ctrl+`" },
    InfoComando { comando: Comando::Detener, id: "terminal.detener", titulo: "Detener el proceso", atajo: "Ctrl+C en terminal" },
    InfoComando { comando: Comando::SiguienteError, id: "errores.siguiente", titulo: "Ir al siguiente error (abre el editor)", atajo: "F8" },
    InfoComando { comando: Comando::PararEnError, id: "errores.parar", titulo: "Parar el proceso en el primer error", atajo: "" },
    InfoComando { comando: Comando::Guardar, id: "editor.guardar", titulo: "Editor: guardar", atajo: "Ctrl+S" },
    InfoComando { comando: Comando::CerrarEditor, id: "editor.cerrar", titulo: "Editor: cerrar la pestana", atajo: "Ctrl+W" },
    InfoComando { comando: Comando::SiguientePestana, id: "editor.siguientePestana", titulo: "Editor: siguiente pestana", atajo: "Ctrl+Tab" },
    InfoComando { comando: Comando::AnteriorPestana, id: "editor.anteriorPestana", titulo: "Editor: pestana anterior", atajo: "Ctrl+Shift+Tab" },
    InfoComando { comando: Comando::LimpiarTerminal, id: "terminal.limpiar", titulo: "Limpiar la terminal", atajo: "" },
    InfoComando { comando: Comando::Dibujo, id: "grafo.dibujo", titulo: "Dibujo del grafo: GPU retenido o CPU", atajo: "" },
    InfoComando { comando: Comando::Carriles, id: "vista.carriles", titulo: "Carriles: semaforo de rendimiento", atajo: "F12" },
    InfoComando { comando: Comando::Ajustes, id: "ajustes.abrir", titulo: "Abrir ajustes y temas", atajo: "Ctrl+," },
    InfoComando { comando: Comando::SiguienteTema, id: "ajustes.siguienteTema", titulo: "Siguiente tema", atajo: "T" },
    InfoComando { comando: Comando::Brillo, id: "ajustes.brillo", titulo: "Brillo neon: encender o apagar", atajo: "" },
    InfoComando { comando: Comando::AcentoEscritorio, id: "ajustes.acento", titulo: "Usar el acento del escritorio", atajo: "" },
    InfoComando { comando: Comando::Tema(0), id: "tema.neon-bmox", titulo: "Tema: Neon BMO-X", atajo: "" },
    InfoComando { comando: Comando::Tema(1), id: "tema.titan-taller", titulo: "Tema: TITAN Taller", atajo: "" },
    InfoComando { comando: Comando::Tema(2), id: "tema.synthwave-84", titulo: "Tema: Synthwave '84", atajo: "" },
    InfoComando { comando: Comando::Tema(3), id: "tema.tokyo-night", titulo: "Tema: Tokyo Night", atajo: "" },
    InfoComando { comando: Comando::Tema(4), id: "tema.catppuccin-mocha", titulo: "Tema: Catppuccin Mocha", atajo: "" },
    InfoComando { comando: Comando::Tema(5), id: "tema.rose-pine", titulo: "Tema: Rose Pine", atajo: "" },
    InfoComando { comando: Comando::Tema(6), id: "tema.kanagawa", titulo: "Tema: Kanagawa", atajo: "" },
    InfoComando { comando: Comando::Tema(7), id: "tema.dracula", titulo: "Tema: Dracula", atajo: "" },
    InfoComando { comando: Comando::Tema(8), id: "tema.nord", titulo: "Tema: Nord", atajo: "" },
];

/// Teclas que entiende el nucleo; cada plataforma traduce las suyas a estas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tecla { G, L, H, Z, A, P, T, B, D, I, Tab, Inicio, F12, Coma, Retroceso, Enter, Escape, F5, F8, S, F, W, F3, Arriba, Abajo, Izquierda, Acento }

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
        (Tecla::Coma, true, _, _) => Comando::Ajustes,
        (Tecla::T, false, false, false) => Comando::SiguienteTema,
        (Tecla::B, true, _, _) => Comando::Explorador,
        (Tecla::Tab, false, false, false) => Comando::Modo,
        (Tecla::D, false, false, false) => Comando::Direccion,
        (Tecla::I, false, false, false) => Comando::Iconos,
        (Tecla::Inicio, _, _, _) => Comando::Encuadrar,
        (Tecla::F12, _, _, _) => Comando::Carriles,
        (Tecla::Acento, true, _, _) => Comando::Terminal,
        (Tecla::Z, true, _, _) => Comando::Deshacer,
        (Tecla::A, true, _, _) => Comando::SeleccionarTodo,
        (Tecla::Izquierda, _, _, true) => Comando::Atras,
        (Tecla::Arriba, _, _, true) | (Tecla::Retroceso, false, false, false) => Comando::Subir,
        (Tecla::G, false, false, false) => Comando::Rejilla,
        (Tecla::L, false, false, false) => Comando::Candado,
        (Tecla::H, false, false, false) => Comando::Ocultos,
        (Tecla::Enter, false, false, false) => Comando::AbrirSeleccion,
        (Tecla::Escape, false, false, false) => Comando::LimpiarSeleccion,
        (Tecla::F5, _, _, _) => Comando::EjecutarSeleccion,
        (Tecla::F8, _, _, _) => Comando::SiguienteError,
        (Tecla::S, true, _, _) => Comando::Guardar,
        (Tecla::W, true, _, _) => Comando::CerrarEditor,
        (Tecla::Tab, true, false, _) => Comando::SiguientePestana,
        (Tecla::Tab, true, true, _) => Comando::AnteriorPestana,
        // Buscar: no son comandos de la tabla, son acciones del editor.
        (Tecla::F, true, _, _) => return Some(Accion::Buscar(AccionBuscar::Abrir { reemplazar: false })),
        (Tecla::H, true, _, _) => return Some(Accion::Buscar(AccionBuscar::Abrir { reemplazar: true })),
        (Tecla::F3, _, true, _) => return Some(Accion::Buscar(AccionBuscar::Anterior)),
        (Tecla::F3, _, false, _) => return Some(Accion::Buscar(AccionBuscar::Siguiente)),
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
        // El titulo manda; el id (carpetas.rejilla) solo ayuda, con penalizacion.
        let mejor = [puntuar(&texto, info.titulo), puntuar(&texto, info.id).map(|p| p - 3)].into_iter().flatten().max();
        if let Some(p) = mejor {
            puntuados.push((p, i, info));
        }
    }
    puntuados.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    puntuados.into_iter().map(|(_, _, info)| info).collect()
}

fn puntuar(texto: &[char], candidato: &str) -> Option<i32> {
    let letras: Vec<char> = candidato.chars().flat_map(|c| c.to_lowercase()).collect();
    let Some(primera) = texto.first() else { return Some(0) };
    // Se prueba cada sitio donde puede empezar y se queda el mejor (no solo el primero).
    let mut mejor: Option<i32> = None;
    for inicio in (0..letras.len()).filter(|i| letras[*i] == *primera) {
        let (mut j, mut puntos, mut previo) = (inicio, 0i32, None::<usize>);
        let mut ok = true;
        for c in texto {
            while j < letras.len() && letras[j] != *c { j += 1; }
            if j == letras.len() { ok = false; break; }
            puntos += 1;
            if previo == Some(j.wrapping_sub(1)) { puntos += 3; }
            if j == 0 || matches!(letras[j - 1], ' ' | '.' | ':' | '(') { puntos += 2; }
            previo = Some(j);
            j += 1;
        }
        if ok { mejor = Some(mejor.map_or(puntos, |m: i32| m.max(puntos))); }
    }
    mejor.map(|p| p - letras.len() as i32 / 8)
}
