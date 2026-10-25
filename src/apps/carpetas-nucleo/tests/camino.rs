// SPDX-License-Identifier: GPL-2.0-only
//! El camino entre dos elegidos: la union mas fuerte, el motivo de cada paso y que pasaria
//! sin el (imprescindible o con desvio).

use carpetas_nucleo::camino::{camino, SinPaso, Union};
use carpetas_nucleo::dependencias::{ClaseRef, Dependencias, Motivo};

/// main.c -> red.h -> util.h ; main.c -> log.h -> util.h ; disco.c -> util.h ;
/// prueba.c -> main.c, prueba.c -> disco.c ; solo.c -> nada ; aislado.h <- solo.c
fn proyecto() -> Dependencias {
    let rutas: Vec<String> = ["main.c", "red.h", "util.h", "log.h", "disco.c", "prueba.c", "solo.c", "aislado.h", "otro.c"].iter().map(|s| s.to_string()).collect();
    let aristas = [(0, 1), (1, 2), (0, 3), (3, 2), (4, 2), (5, 0), (5, 4), (6, 7), (8, 1)];
    let mut d = Dependencias::nuevo(rutas, &aristas);
    let m = |l: u32, t: &str| Motivo { linea: l, texto: t.to_string(), clase: ClaseRef::Comillas };
    d.poner_motivos(&aristas, vec![m(3, "red.h"), m(1, "util.h"), m(4, "log.h"), m(2, "util.h"), m(7, "util.h"), m(1, "main.c"), m(2, "disco.c"), m(1, "aislado.h"), m(5, "red.h")]);
    d
}

#[test]
fn a_usa_b_con_motivos_y_dos_caminos_igual_de_cortos() {
    let c = camino(&proyecto(), "main.c", "util.h");
    assert_eq!(c.union, Some(Union::Usa));
    assert_eq!(c.nodos.len(), 3);
    assert_eq!(c.igual_de_cortos, 2);
    // El primer paso sale de main.c, con su linea y como se escribio.
    let p = &c.pasos[0];
    assert_eq!(p.desde, "main.c");
    let m = p.motivo.as_ref().unwrap();
    assert_eq!(m.escrito("main.c"), "#include \"red.h\"");
    // Hay dos rutas: quitar un paso deja la otra (desvio de 2), nada es imprescindible.
    assert!(c.pasos.iter().all(|p| p.sin == SinPaso::Desvio(Union::Usa, 2)));
    assert!(c.por_que.contains("si cambia util.h, main.c se recompila"), "{}", c.por_que);
}

#[test]
fn al_reves_y_con_un_paso_imprescindible() {
    let c = camino(&proyecto(), "red.h", "prueba.c");
    assert_eq!(c.union, Some(Union::EsUsado));
    // De A (red.h) a B (prueba.c), pero cada paso dice quien usa a quien de verdad.
    assert_eq!(c.nodos, ["red.h", "main.c", "prueba.c"]);
    assert_eq!((c.pasos[0].desde.as_str(), c.pasos[0].hasta.as_str()), ("main.c", "red.h"));
    assert!(c.pasos.iter().all(|p| p.sin == SinPaso::Imprescindible));
}

#[test]
fn comparten_el_punto_mas_cercano() {
    let c = camino(&proyecto(), "disco.c", "otro.c");
    assert_eq!(c.union, Some(Union::Comparten));
    let i = c.punto.unwrap();
    assert_eq!(c.nodos[i], "util.h");
    assert_eq!(c.nodos, ["disco.c", "util.h", "red.h", "otro.c"]);
    // disco.c usa util.h; red.h usa util.h; otro.c usa red.h.
    assert_eq!((c.pasos[1].desde.as_str(), c.pasos[1].hasta.as_str()), ("red.h", "util.h"));
    assert_eq!((c.pasos[2].desde.as_str(), c.pasos[2].hasta.as_str()), ("otro.c", "red.h"));
}

#[test]
fn los_junta_y_nada() {
    let d = proyecto();
    let c = camino(&d, "main.c", "disco.c");
    // main.c y disco.c comparten util.h (2 y 1 saltos) antes que juntarlos prueba.c (1 y 1):
    // compartir se busca antes que juntar.
    assert_eq!(c.union, Some(Union::Comparten));
    let c = camino(&d, "aislado.h", "main.c");
    assert_eq!(c.union, None);
    assert!(c.por_que.contains("no se tocan"));
    let c = camino(&d, "LEEME.md", "main.c");
    assert!(c.por_que.contains("no tiene dependencias conocidas"));
}

#[test]
fn los_junta_cuando_no_comparten_nada() {
    // x.c usa a.h y b.h; a.h y b.h no usan nada.
    let rutas: Vec<String> = ["x.c", "a.h", "b.h"].iter().map(|s| s.to_string()).collect();
    let d = Dependencias::nuevo(rutas, &[(0, 1), (0, 2)]);
    let c = camino(&d, "a.h", "b.h");
    assert_eq!(c.union, Some(Union::Juntan));
    assert_eq!(c.nodos, ["a.h", "x.c", "b.h"]);
    assert_eq!((c.pasos[0].desde.as_str(), c.pasos[0].hasta.as_str()), ("x.c", "a.h"));
    assert_eq!((c.pasos[1].desde.as_str(), c.pasos[1].hasta.as_str()), ("x.c", "b.h"));
}
