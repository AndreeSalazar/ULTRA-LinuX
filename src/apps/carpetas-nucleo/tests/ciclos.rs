// SPDX-License-Identifier: GPL-2.0-only
//! Ciclos de cabeceras (E-003): el caso aislado `mal` tiene una vuelta y `bien` no. Se leen
//! los archivos de verdad y se resuelven con el mismo resolvedor que la aplicacion.

use carpetas_nucleo::camino::camino;
use carpetas_nucleo::ciclos::ciclos;
use carpetas_nucleo::dependencias::{referencias, resolver, Dependencias, Motivo, Rutas};
use std::path::Path;

struct Lista(Vec<String>);
impl Rutas for Lista {
    fn existe(&self, r: &str) -> bool { self.0.iter().any(|x| x == r) }
    fn por_nombre(&self, n: &str) -> Vec<&str> { self.0.iter().map(String::as_str).filter(|r| r.rsplit('/').next() == Some(n)).collect() }
}

/// Las dependencias de una carpeta de casos, como las arma el hilo de analisis.
fn leer(caso: &str) -> Dependencias {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/datos/ciclos").join(caso);
    let mut rutas: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    rutas.sort();
    let lista = Lista(rutas.clone());
    let mut aristas = Vec::new();
    let mut motivos = Vec::new();
    for (a, r) in rutas.iter().enumerate() {
        let texto = std::fs::read_to_string(dir.join(r)).unwrap();
        for x in referencias(r, &texto) {
            if let Some(d) = resolver(r, &x, &lista) {
                let b = rutas.iter().position(|y| *y == d).unwrap();
                aristas.push((a as u32, b as u32));
                motivos.push(Motivo::de(&x));
            }
        }
    }
    let mut d = Dependencias::nuevo(rutas, &aristas);
    d.poner_motivos(&aristas, motivos);
    d
}

#[test]
fn el_caso_malo_tiene_una_vuelta_con_sus_motivos() {
    let c = ciclos(&leer("mal"));
    assert_eq!((c.total, c.archivos), (1, 3));
    let ciclo = &c.lista[0];
    assert_eq!(ciclo.archivos, ["a.h", "b.h", "c.h"]);
    assert_eq!(ciclo.vuelta.len(), 3);
    // La vuelta se cierra: cada paso empieza donde acabo el anterior.
    for i in 0..3 { assert_eq!(ciclo.vuelta[i].hasta, ciclo.vuelta[(i + 1) % 3].desde); }
    // El paso que vuelve a a.h lleva su linea (la 4 de c.h).
    let vuelve = ciclo.vuelta.iter().find(|e| e.desde == "c.h").unwrap();
    assert_eq!((vuelve.hasta.as_str(), vuelve.motivo.as_ref().unwrap().linea), ("a.h", 4));
    assert_eq!(vuelve.motivo.as_ref().unwrap().escrito("c.h"), "#include \"a.h\"");
    // Donde cortar: el incluido que menos usan. a.h lo usan main.c y c.h (2); b.h y c.h, 1.
    let corte = &ciclo.vuelta[ciclo.corte.unwrap()];
    assert_ne!(corte.hasta, "a.h");
}

#[test]
fn un_ciclo_dentro_de_un_if_se_dice() {
    let c = ciclos(&leer("condicional"));
    assert_eq!(c.total, 1);
    assert!(c.lista[0].condicional);
    assert_eq!(c.condicionales, 1);
    let e = c.lista[0].vuelta.iter().find(|e| e.desde == "b.h").unwrap();
    assert_eq!(e.motivo.as_ref().unwrap().condicion.as_deref(), Some("#if defined(_WIN32)"));
    // El del caso malo no depende de nada.
    assert!(!ciclos(&leer("mal")).lista[0].condicional);
}

#[test]
fn el_control_no_tiene_ciclos() {
    let c = ciclos(&leer("bien"));
    assert_eq!((c.total, c.archivos), (0, 0));
    assert!(c.lista.is_empty());
}

#[test]
fn el_camino_dice_si_hay_vuelta() {
    let mal = leer("mal");
    let c = camino(&mal, "a.h", "c.h");
    assert!(c.ciclo, "{}", c.por_que);
    assert!(c.por_que.contains("forman un ciclo"));
    let bien = leer("bien");
    assert!(!camino(&bien, "a.h", "c.h").ciclo);
}

#[test]
fn varios_ciclos_el_mayor_primero_y_rust_no_cuenta() {
    // x.h <-> y.h ; p.h -> q.h -> r.h -> p.h ; a.rs <-> b.rs (modulos de un crate: no es ciclo aqui)
    let rutas: Vec<String> = ["x.h", "y.h", "p.h", "q.h", "r.h", "a.rs", "b.rs", "solo.c"].iter().map(|s| s.to_string()).collect();
    let d = Dependencias::nuevo(rutas, &[(0, 1), (1, 0), (2, 3), (3, 4), (4, 2), (5, 6), (6, 5), (7, 0)]);
    let c = ciclos(&d);
    assert_eq!((c.total, c.archivos), (2, 5));
    assert_eq!(c.lista[0].archivos, ["p.h", "q.h", "r.h"]);
    assert_eq!(c.lista[1].archivos, ["x.h", "y.h"]);
    assert_eq!(c.lista[1].vuelta.len(), 2);
}

#[test]
fn un_grafo_grande_en_cadena_no_desborda_la_pila() {
    // 200 000 archivos en cadena y uno que vuelve al principio: un solo ciclo gigante.
    let n = 200_000u32;
    let rutas: Vec<String> = (0..n).map(|i| format!("h{i}.h")).collect();
    let mut aristas: Vec<(u32, u32)> = (0..n - 1).map(|i| (i, i + 1)).collect();
    aristas.push((n - 1, 0));
    let c = ciclos(&Dependencias::nuevo(rutas, &aristas));
    assert_eq!((c.total, c.archivos), (1, n));
    assert_eq!(c.lista[0].vuelta.len(), n as usize);
}
