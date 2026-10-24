// SPDX-License-Identifier: GPL-2.0-only
//! Procesos de la terminal (play y linea de comandos), aislados en sus propios hilos.
//! Sin shell: programa + argumentos tal cual. Sin entrada (stdin cerrado). Su propio grupo
//! de procesos, para que "Detener" pare tambien a los hijos (cargo -> rustc -> tu programa).

use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};

pub enum Mensaje { Linea(String, bool), Fin(Option<i32>) }

pub struct Proceso { rx: Receiver<Mensaje>, grupo: u32 }

fn leer<R: Read + Send + 'static>(fuente: R, error: bool, tx: mpsc::Sender<Mensaje>, avisar: impl Fn() + Send + 'static) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut lector = BufReader::new(fuente);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            // Como mucho 64 KiB por linea: una linea gigante no se come la memoria.
            match (&mut lector).take(64 * 1024).read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let linea = String::from_utf8_lossy(&buf).trim_end_matches(['\n', '\r']).to_string();
                    if tx.send(Mensaje::Linea(linea, error)).is_err() { return; }
                    avisar();
                }
            }
        }
    })
}

impl Proceso {
    pub fn lanzar(carpeta: &Path, programa: &str, args: &[String], avisar: impl Fn() + Send + Sync + Clone + 'static) -> Result<Proceso, String> {
        let mut hijo = Command::new(programa).args(args).current_dir(carpeta)
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
            // Salida limpia en la terminal: sin colores ANSI.
            .env("TERM", "dumb").env("NO_COLOR", "1").env("CARGO_TERM_COLOR", "never").env("CLICOLOR", "0")
            .process_group(0)
            .spawn().map_err(|e| format!("no se pudo lanzar {programa}: {e}"))?;
        let grupo = hijo.id();
        let (tx, rx) = mpsc::channel();
        let a = leer(hijo.stdout.take().expect("stdout"), false, tx.clone(), avisar.clone());
        let b = leer(hijo.stderr.take().expect("stderr"), true, tx.clone(), avisar.clone());
        std::thread::spawn(move || {
            let _ = (a.join(), b.join());
            let codigo = hijo.wait().ok().and_then(|s| s.code());
            let _ = tx.send(Mensaje::Fin(codigo));
            avisar();
        });
        Ok(Proceso { rx, grupo })
    }

    pub fn recoger(&self, maximo: usize) -> Vec<Mensaje> { self.rx.try_iter().take(maximo).collect() }

    /// SIGTERM a todo el grupo (sin shell: el programa `kill` con el grupo en negativo).
    pub fn detener(&self) {
        let _ = Command::new("kill").args(["-TERM", "--", &format!("-{}", self.grupo)]).stdin(Stdio::null())
            .stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::time::{Duration, Instant};

    fn esperar(p: &Proceso, segundos: u64) -> (Vec<(String, bool)>, Option<Option<i32>>) {
        let (mut lineas, mut fin) = (Vec::new(), None);
        let limite = Instant::now() + Duration::from_secs(segundos);
        while fin.is_none() && Instant::now() < limite {
            for m in p.recoger(512) { match m { Mensaje::Linea(l, e) => lineas.push((l, e)), Mensaje::Fin(c) => fin = Some(c) } }
            std::thread::sleep(Duration::from_millis(20));
        }
        (lineas, fin)
    }

    #[test]
    fn salida_errores_y_codigo() {
        let p = Proceso::lanzar(Path::new("/tmp"), "sh", &["-c".into(), "echo hola; echo malo >&2; exit 3".into()], || {}).unwrap();
        let (lineas, fin) = esperar(&p, 10);
        assert!(lineas.contains(&("hola".into(), false)) && lineas.contains(&("malo".into(), true)));
        assert_eq!(fin, Some(Some(3)));
    }

    #[test]
    fn detener_para_tambien_a_los_hijos() {
        let p = Proceso::lanzar(Path::new("/tmp"), "sh", &["-c".into(), "sleep 30 & sleep 30; echo nunca".into()], || {}).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let inicio = Instant::now();
        p.detener();
        let (lineas, fin) = esperar(&p, 10);
        assert!(inicio.elapsed() < Duration::from_secs(5), "tardo {:?}", inicio.elapsed());
        assert_eq!(fin, Some(None), "terminado por senal");
        assert!(!lineas.iter().any(|(l, _)| l == "nunca"));
    }

    #[test]
    fn programa_que_no_existe() {
        assert!(Proceso::lanzar(Path::new("/tmp"), "no-existe-ultra", &[], || {}).is_err());
    }
}
