// SPDX-License-Identifier: GPL-2.0-only
//! Carriles con semaforo: cada parte del programa tiene su presupuesto y sus limites,
//! para que ninguna choque con otra ni crezca sin control. Datos puros (no_std): la
//! plataforma mide y pinta el semaforo; aqui se decide que es verde, amarillo o rojo.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Carril {
    /// Listar y vigilar carpetas. Corre en su propio hilo: nunca frena la interfaz.
    Disco,
    /// Calcular donde va cada nodo del grafo.
    Acomodo,
    /// Construir la Vista (lo que hay que dibujar) desde el Estado.
    Vista,
    /// Dibujar el cuadro con egui (sin contar la GPU).
    Pintado,
    /// Pasar SVG a textura. Solo la primera vez de cada icono y tamano.
    Iconos,
    /// Geometria del grafo para la GPU: la CPU la arma solo cuando cambia el contenido.
    Geometria,
    /// Lineas que llegan de un proceso a la terminal.
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Luz { Verde, Amarillo, Rojo }

pub struct Presupuesto {
    pub carril: Carril,
    pub nombre: &'static str,
    /// Hasta aqui verde; hasta `amarillo` amarillo; mas, rojo. En microsegundos.
    pub verde_us: u64,
    pub amarillo_us: u64,
    pub que_consume: &'static str,
}

/// Un cuadro a 60 Hz son 16 666 us. Disco va aparte (su hilo), por eso tolera mas.
pub const PRESUPUESTOS: &[Presupuesto] = &[
    Presupuesto { carril: Carril::Disco, nombre: "Disco", verde_us: 20_000, amarillo_us: 200_000,
                  que_consume: "E/S del disco en su hilo; la interfaz nunca la espera" },
    Presupuesto { carril: Carril::Acomodo, nombre: "Acomodo", verde_us: 1_000, amarillo_us: 4_000,
                  que_consume: "CPU: O(n log n) con n <= MAX_NODOS; se calcula solo si cambio el estado" },
    Presupuesto { carril: Carril::Vista, nombre: "Vista", verde_us: 1_500, amarillo_us: 5_000,
                  que_consume: "CPU y memoria: una lista de nodos; en cache por version del estado" },
    Presupuesto { carril: Carril::Pintado, nombre: "Pintado", verde_us: 6_000, amarillo_us: 12_000,
                  que_consume: "CPU: egui teselar formas cada cuadro; solo se repinta con eventos" },
    Presupuesto { carril: Carril::Iconos, nombre: "Iconos SVG", verde_us: 8_000, amarillo_us: 30_000,
                  que_consume: "CPU una vez por icono y tamano (resvg); luego textura en la GPU" },
    Presupuesto { carril: Carril::Geometria, nombre: "Geometria", verde_us: 3_000, amarillo_us: 10_000,
                  que_consume: "CPU solo cuando cambia el grafo; mover y zoom solo mandan 16 bytes de camara a la GPU" },
    Presupuesto { carril: Carril::Terminal, nombre: "Terminal", verde_us: 1_000, amarillo_us: 5_000,
                  que_consume: "Lineas de procesos; como mucho 512 por cuadro y 2000 guardadas" },
];

pub fn presupuesto(c: Carril) -> &'static Presupuesto {
    PRESUPUESTOS.iter().find(|p| p.carril == c).unwrap_or(&PRESUPUESTOS[0])
}

pub fn luz(c: Carril, micros: u64) -> Luz {
    let p = presupuesto(c);
    if micros <= p.verde_us { Luz::Verde } else if micros <= p.amarillo_us { Luz::Amarillo } else { Luz::Rojo }
}

/// Limites duros: lo que pase de aqui no entra, por diseno.
pub const MAX_ENTRADAS: usize = 5000;
pub const MAX_NODOS: usize = crate::grafo::MAX_NODOS;
pub const MAX_DESPLEGADAS: usize = 64;
pub const MAX_JSON: usize = crate::disposicion::MAX_JSON;
pub const MAX_LINEAS_POR_CUADRO: usize = 512;
