// SPDX-License-Identifier: GPL-2.0-only
//! Procesos en vivo: lo que la plataforma lee de /proc (Linux) o de su planificador
//! (BMO-X) para el proceso lanzado y sus hijos (cargo -> rustc -> tu programa).
//! Aqui solo se ordenan en arbol; nada de E/S.

use alloc::string::String;
use alloc::vec::Vec;

pub const MAX_PROCESOS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcesoVivo {
    pub pid: u32,
    pub ppid: u32,
    pub nombre: String,
    /// Por ciento de un nucleo en el ultimo intervalo (200 = dos nucleos llenos).
    pub cpu: f32,
    pub memoria_kb: u64,
}

/// En arbol, en profundidad: (nivel, proceso). Raices = las que no tienen padre en la lista.
pub fn arbol(lista: &[ProcesoVivo]) -> Vec<(usize, &ProcesoVivo)> {
    let mut salida = Vec::new();
    fn bajar<'a>(lista: &'a [ProcesoVivo], padre: u32, nivel: usize, salida: &mut Vec<(usize, &'a ProcesoVivo)>) {
        let mut hijos: Vec<&ProcesoVivo> = lista.iter().filter(|p| p.ppid == padre && p.pid != padre).collect();
        hijos.sort_by_key(|p| p.pid);
        for h in hijos {
            if salida.len() >= MAX_PROCESOS || nivel > 16 { return; }
            salida.push((nivel, h));
            bajar(lista, h.pid, nivel + 1, salida);
        }
    }
    let mut raices: Vec<&ProcesoVivo> = lista.iter().filter(|p| !lista.iter().any(|q| q.pid == p.ppid && q.pid != p.pid)).collect();
    raices.sort_by_key(|p| p.pid);
    for r in raices {
        if salida.len() >= MAX_PROCESOS { break; }
        salida.push((0, r));
        bajar(lista, r.pid, 1, &mut salida);
    }
    salida
}

/// CPU y memoria sumadas.
pub fn totales(lista: &[ProcesoVivo]) -> (f32, u64) {
    lista.iter().fold((0.0, 0), |(c, m), p| (c + p.cpu, m + p.memoria_kb))
}
