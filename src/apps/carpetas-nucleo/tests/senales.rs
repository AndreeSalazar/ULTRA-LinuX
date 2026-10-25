// SPDX-License-Identifier: GPL-2.0-only
//! Lo que senalan los del grupo: ordenado para arreglar (lo que rompe primero) y, si un
//! archivo falla por otro, la cadena de #include hasta la causa.

use carpetas_nucleo::analisis::ResultadoAnalisis;
use carpetas_nucleo::dependencias::{ClaseRef, Motivo, Rol};
use carpetas_nucleo::museo::{analizar, Eje};
use carpetas_nucleo::senales::senales;
use carpetas_nucleo::{reducir, Accion, Estado};

#[test]
fn primero_lo_que_rompe_y_por_donde_llega() {
    let larga = format!("int f(int x) {{\n{}    return x;\n}}\n", "    x = x + 1;\n".repeat(110));
    let util = "void g(char *p) {\n    free(p + 1);\n}\n";
    let mut e = Estado::default();
    let rutas: Vec<String> = ["main.c", "util.h", "red.h"].iter().map(|s| s.to_string()).collect();
    let m = |l: u32, t: &str| Motivo { linea: l, texto: t.into(), clase: ClaseRef::Comillas, condicion: None };
    reducir(&mut e, Accion::Analisis(ResultadoAnalisis {
        rutas, aristas: vec![(0, 2), (2, 1)], motivos: vec![m(3, "red.h"), m(7, "util.h")],
        sospechas: vec![("main.c".into(), analizar("main.c", &larga)), ("util.h".into(), analizar("util.h", util))],
        ..Default::default()
    }));
    let s = senales(&e, &["main.c".to_string(), "util.h".to_string()]);
    assert_eq!(s.total, 2);
    // Primero la de Seguridad exacta (free(p + 1)), despues la funcion larga (Elegancia).
    assert_eq!((s.lista[0].archivo.as_str(), s.lista[0].ficha, s.lista[0].linea, s.lista[0].eje), ("util.h", Some("C-001"), 2, Eje::Seguridad));
    assert_eq!((s.lista[1].archivo.as_str(), s.lista[1].ficha, s.lista[1].eje), ("main.c", Some("E-001"), Eje::Elegancia));
    // main.c no falla, pero arrastra: le llega por red.h:7 desde util.h (la causa).
    let (rol, cadena) = &s.por_que["main.c"];
    assert_eq!(*rol, Rol::Arrastra);
    let pasos: Vec<(&str, &str, u32)> = cadena.iter().map(|x| (x.desde.as_str(), x.hasta.as_str(), x.motivo.as_ref().unwrap().linea)).collect();
    assert_eq!(pasos, [("main.c", "red.h", 3), ("red.h", "util.h", 7)]);
    assert_eq!(s.por_que["util.h"].0, Rol::Causa);
    // Una carpeta del grupo cuenta lo de dentro.
    assert_eq!(senales(&e, &["".to_string()]).total, 0, "la raiz vacia no es una carpeta");
}
