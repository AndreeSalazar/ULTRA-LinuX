// SPDX-License-Identifier: GPL-2.0-only
//! El triangulo en el SUPER GRAFO: solo la Seguridad es un fallo; Rendimiento y Elegancia
//! se cuentan y se ensenan, pero no se conectan en rojo.
use carpetas_nucleo::analisis::{fallos, EstadoAnalisis, ResultadoAnalisis};
use carpetas_nucleo::museo::analizar;
use carpetas_nucleo::Errores;

fn estado(archivos: &[(&str, &str)]) -> EstadoAnalisis {
    let sospechas = archivos.iter().map(|(r, t)| (r.to_string(), analizar(r, t))).collect();
    let mut e = EstadoAnalisis::default();
    e.llegar(ResultadoAnalisis { sospechas, ..Default::default() });
    e
}

#[test]
fn solo_la_seguridad_es_un_fallo() {
    let larga = format!("int f(int x) {{\n{}    return x;\n}}\n", "    x = x + 1;\n".repeat(110));
    let e = estado(&[("src/larga.c", &larga), ("src/libera.c", "void g(char *p) {\n    free(p + 1);\n}\n")]);
    assert_eq!(e.conteo_ejes("src/larga.c"), [0, 0, 1], "la funcion larga es Elegancia");
    assert_eq!(e.conteo_ejes("src/libera.c"), [1, 0, 0], "free(p + 1) es Seguridad");
    assert_eq!(e.conteo_ejes("src"), [1, 0, 1], "la carpeta suma lo de dentro");
    assert_eq!(e.conteo("src"), (1, 0), "el conteo de fallos solo mira Seguridad");
    let f = fallos(&Errores::default(), &e);
    assert!(f.contains("src/libera.c"));
    assert!(!f.contains("src/larga.c"), "una funcion larga no rompe nada: no se conecta en rojo");
}

#[test]
fn la_primera_que_se_ensena_es_la_de_seguridad() {
    let e = estado(&[("a.c", &format!("void g(char *p) {{\n    free(p + 1);\n{}}}\n", "    p++;\n".repeat(110)))]);
    let (_, s) = e.primera("a.c").unwrap();
    assert_eq!(s.ficha, "C-001");
}
