// SPDX-License-Identifier: GPL-2.0-only
//! Seleccion grupal: como se combinan los elegidos (directas, lo que comparten, quien los
//! junta y los sueltos), y el reductor del grupo.

use carpetas_nucleo::combinar::{combinar, MAX_GRUPO};
use carpetas_nucleo::dependencias::Dependencias;
use carpetas_nucleo::estado::Lateral;
use carpetas_nucleo::{reducir, Accion, AccionGrupo, Estado};

/// main.c usa red/red.h y util.h; red/red.c usa red/red.h y util.h; disco.c usa util.h;
/// prueba.c usa main.c y disco.c; solo.c no usa nada.
fn proyecto() -> Dependencias {
    let rutas: Vec<String> = ["main.c", "red/red.h", "red/red.c", "util.h", "disco.c", "prueba.c", "solo.c"].iter().map(|s| s.to_string()).collect();
    Dependencias::nuevo(rutas, &[(0, 1), (0, 3), (2, 1), (2, 3), (4, 3), (5, 0), (5, 4)])
}

fn ids(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

#[test]
fn directas_compartidos_y_quien_los_junta() {
    let dep = proyecto();
    let c = combinar(&dep, &ids(&["main.c", "disco.c", "red/red.h"]));
    let m = |id: &str| c.miembro(id).unwrap();
    // main.c usa red/red.h: una directa.
    assert_eq!(c.directas, vec![(m("main.c"), m("red/red.h"), 1)]);
    // util.h lo usan main.c y disco.c.
    assert_eq!(c.comparten.len(), 1);
    assert_eq!(c.comparten[0].ruta, "util.h");
    assert_eq!(c.comparten[0].miembros, vec![m("main.c"), m("disco.c")]);
    // prueba.c usa a main.c y a disco.c: los junta. red/red.c usa red/red.h solo: no junta.
    assert_eq!(c.juntan.len(), 1);
    assert_eq!(c.juntan[0].ruta, "prueba.c");
    assert_eq!(c.piezas, 1);
    assert!(c.sueltos.is_empty());
    assert_eq!(c.miembros[m("main.c")].lazos, 2);
}

#[test]
fn una_carpeta_cuenta_con_sus_archivos_y_lo_suelto_se_dice() {
    let dep = proyecto();
    let c = combinar(&dep, &ids(&["red", "disco.c", "solo.c", "LEEME.md"]));
    let red = c.miembro("red").unwrap();
    assert_eq!(c.miembros[red].archivos, 2);
    // red/red.c usa util.h, como disco.c: comparten. Lo de dentro de red no es directa.
    assert!(c.directas.is_empty());
    assert_eq!(c.comparten[0].ruta, "util.h");
    // solo.c se conoce pero no toca a nadie: suelto. LEEME.md no tiene dependencias: sin datos.
    assert_eq!(c.sueltos, vec![c.miembro("solo.c").unwrap()]);
    assert_eq!(c.sin_datos, ids(&["LEEME.md"]));
    assert_eq!(c.piezas, 2);
}

#[test]
fn sin_nada_en_comun_son_piezas_separadas() {
    let dep = proyecto();
    let c = combinar(&dep, &ids(&["red/red.h", "disco.c"]));
    assert_eq!(c.piezas, 2);
    assert_eq!(c.sueltos.len(), 2);
}

#[test]
fn el_grupo_se_alterna_y_abre_el_panel_al_juntar_dos() {
    let mut e = Estado::default();
    reducir(&mut e, Accion::Grupo(AccionGrupo::Alternar("a.c".into())));
    assert_eq!(e.lateral, Lateral::Explorador);
    reducir(&mut e, Accion::Grupo(AccionGrupo::Alternar("b.c".into())));
    assert_eq!(e.lateral, Lateral::Combinar);
    // Despues manda quien lo usa: cambiar de panel y sumar otro no lo vuelve a abrir.
    reducir(&mut e, Accion::Lateral(Lateral::Museo));
    reducir(&mut e, Accion::Grupo(AccionGrupo::Sumar(ids(&["c.c", "a.c"]))));
    assert_eq!(e.lateral, Lateral::Museo);
    assert_eq!(e.grupo, ids(&["a.c", "b.c", "c.c"]));
    reducir(&mut e, Accion::Grupo(AccionGrupo::Alternar("a.c".into())));
    assert_eq!(e.grupo, ids(&["b.c", "c.c"]));
    let muchos: Vec<String> = (0..200).map(|i| format!("f{i}.c")).collect();
    reducir(&mut e, Accion::Grupo(AccionGrupo::Sumar(muchos)));
    assert_eq!(e.grupo.len(), MAX_GRUPO);
    reducir(&mut e, Accion::Grupo(AccionGrupo::Limpiar));
    assert!(e.grupo.is_empty());
}
