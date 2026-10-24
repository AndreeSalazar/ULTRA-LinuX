// SPDX-License-Identifier: GPL-2.0-only
//! Procesos en vivo, leidos de /proc en su propio hilo (carril Procesos): el proceso que
//! lanzo el play y todo su grupo (cargo -> rustc -> tu programa), con CPU y memoria.
//! Solo lectura de /proc; cada 500 ms; como mucho 64 procesos; nunca frena la interfaz.

use carpetas_nucleo::procesos::MAX_PROCESOS;
use carpetas_nucleo::ProcesoVivo;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// /proc cuenta en "ticks" de USER_HZ, que en Linux es 100 en todas las arquitecturas.
const TICKS: f32 = 100.0;
const CADA: Duration = Duration::from_millis(500);

pub struct Muestra { pub procesos: Vec<ProcesoVivo>, pub micros: u64 }

pub struct Muestreador { tx: Sender<Option<u32>>, rx: Receiver<Muestra> }

/// Campos de /proc/PID/stat que hacen falta: (ppid, grupo, ticks de CPU). El nombre va
/// entre parentesis y puede tener espacios: se corta por el ultimo ')'.
pub fn leer_stat(texto: &str) -> Option<(String, u32, u32, u64)> {
    let abre = texto.find('(')?;
    let cierra = texto.rfind(')')?;
    let nombre = texto.get(abre + 1..cierra)?.chars().take(32).collect();
    let campos: Vec<&str> = texto.get(cierra + 1..)?.split_whitespace().collect();
    // Tras ')': 0 estado, 1 ppid, 2 grupo, ... 11 utime, 12 stime.
    let n = |i: usize| campos.get(i).and_then(|x| x.parse::<u64>().ok());
    Some((nombre, n(1)? as u32, n(2)? as u32, n(11)? + n(12)?))
}

fn memoria_kb(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status")).ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
        .unwrap_or(0)
}

/// Una pasada por /proc: los procesos del grupo, con sus ticks acumulados.
fn pasada(grupo: u32) -> Vec<(ProcesoVivo, u64)> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
    let mut v = Vec::new();
    for e in dir.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { continue };
        let Some((nombre, ppid, g, ticks)) = leer_stat(&stat) else { continue };
        if g != grupo { continue; }
        v.push((ProcesoVivo { pid, ppid, nombre, cpu: 0.0, memoria_kb: memoria_kb(pid) }, ticks));
        if v.len() >= MAX_PROCESOS { break; }
    }
    v
}

impl Muestreador {
    pub fn nuevo(avisar: impl Fn() + Send + 'static) -> Muestreador {
        let (tx, rx_orden) = mpsc::channel::<Option<u32>>();
        let (tx_muestra, rx) = mpsc::channel();
        std::thread::Builder::new().name("ultra-procesos".into()).spawn(move || {
            let mut grupo: Option<u32> = None;
            let mut antes: HashMap<u32, u64> = HashMap::new();
            let mut ultima = Instant::now();
            loop {
                match rx_orden.recv_timeout(if grupo.is_some() { CADA } else { Duration::from_secs(3600) }) {
                    Ok(g) => { grupo = g; antes.clear(); ultima = Instant::now(); if grupo.is_none() { continue; } }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                let Some(g) = grupo else { continue };
                let inicio = Instant::now();
                let segundos = ultima.elapsed().as_secs_f32().max(0.05);
                ultima = Instant::now();
                let lista = pasada(g);
                let mut ahora = HashMap::new();
                let procesos = lista.into_iter().map(|(mut p, ticks)| {
                    // CPU = ticks gastados desde la ultima pasada / tiempo (100 = un nucleo lleno).
                    if let Some(t0) = antes.get(&p.pid) { p.cpu = ticks.saturating_sub(*t0) as f32 / TICKS / segundos * 100.0; }
                    ahora.insert(p.pid, ticks);
                    p
                }).collect();
                antes = ahora;
                if tx_muestra.send(Muestra { procesos, micros: inicio.elapsed().as_micros() as u64 }).is_err() { return; }
                avisar();
            }
        }).expect("hilo de procesos");
        Muestreador { tx, rx }
    }

    /// Seguir a un grupo de procesos (el del play) o dejar de mirar.
    pub fn seguir(&self, grupo: Option<u32>) { let _ = self.tx.send(grupo); }

    /// La ultima muestra que llego (las viejas se tiran).
    pub fn recoger(&self) -> Option<Muestra> { self.rx.try_iter().last() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn stat_con_espacios_y_parentesis_en_el_nombre() {
        let s = "1234 (mi (raro) prog) S 1200 1200 1200 0 -1 4194304 100 0 0 0 250 50 0 0 20 0 1 0 100 1000 200";
        assert_eq!(leer_stat(s), Some(("mi (raro) prog".into(), 1200, 1200, 300)));
        assert_eq!(leer_stat("basura"), None);
    }

    #[test]
    fn ve_el_grupo_del_proceso_lanzado() {
        use std::os::unix::process::CommandExt;
        // Un grupo propio con un padre y un hijo que gasta CPU.
        let mut hijo = std::process::Command::new("sh").args(["-c", "(while :; do :; done) & sleep 3"]).process_group(0).spawn().unwrap();
        let m = Muestreador::nuevo(|| {});
        m.seguir(Some(hijo.id()));
        let limite = Instant::now() + Duration::from_secs(4);
        let mut vista = None;
        while Instant::now() < limite {
            std::thread::sleep(Duration::from_millis(100));
            if let Some(x) = m.recoger() { if x.procesos.len() >= 2 && x.procesos.iter().any(|p| p.cpu > 20.0) { vista = Some(x); break; } }
        }
        let _ = std::process::Command::new("kill").args(["-TERM", "--", &format!("-{}", hijo.id())]).status();
        let _ = hijo.wait();
        let x = vista.expect("no vio el grupo con CPU");
        assert!(x.procesos.iter().any(|p| p.pid == hijo.id() && p.nombre == "sh"));
        assert!(x.procesos.iter().all(|p| p.memoria_kb > 0));
        let arbol = carpetas_nucleo::procesos::arbol(&x.procesos);
        assert_eq!(arbol[0].1.pid, hijo.id(), "el lanzado es la raiz del arbol");
    }
}
