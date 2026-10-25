// SPDX-License-Identifier: GPL-2.0-only
use carpetas_nucleo::cronologia::{marcas, recorrer, reproducir, Cronologia};

/// Una historia escrita a mano, con el mismo formato que git:
/// 1. nace src/a.c (incluye b.h) y src/b.h;
/// 2. nace src/c.h, y b.h pasa a incluir c.h;
/// 3. src/b.h se mueve a inc/b.h (y a.c lo incluye por su nuevo nombre);
/// 4. c.h incluye b.h: se cierra el ciclo b.h -> c.h -> b.h;
/// 5. muere src/a.c.
fn historia() -> Cronologia {
    let archivos = "\u{1}c1\u{2}100\u{2}Ana\u{2}Primer paso\n\nA\tsrc/a.c\nA\tsrc/b.h\n\
                    \u{1}c2\u{2}200\u{2}Ana\u{2}Anadir c.h\n\nA\tsrc/c.h\nM\tsrc/b.h\n\
                    \u{1}c3\u{2}300\u{2}Luis\u{2}Mover b.h a inc\n\nR100\tsrc/b.h\tinc/b.h\nM\tsrc/a.c\n\
                    \u{1}c4\u{2}400\u{2}Luis\u{2}c.h usa b.h\n\nM\tsrc/c.h\n\
                    \u{1}c5\u{2}500\u{2}Ana\u{2}Quitar a.c\n\nD\tsrc/a.c\n";
    let refs = "\u{1}c1\u{2}100\n\ndiff --git a/src/a.c b/src/a.c\n--- /dev/null\n+++ b/src/a.c\n@@ -0,0 +1,2 @@\n+#include \"b.h\"\n+int main(void) { return 0; }\n\
                diff --git a/src/b.h b/src/b.h\n--- /dev/null\n+++ b/src/b.h\n@@ -0,0 +1 @@\n+int b;\n\
                \u{1}c2\u{2}200\n\ndiff --git a/src/b.h b/src/b.h\n--- a/src/b.h\n+++ b/src/b.h\n@@ -0,0 +1 @@\n+#include \"c.h\"\n\
                \u{1}c3\u{2}300\n\ndiff --git a/src/a.c b/src/a.c\n--- a/src/a.c\n+++ b/src/a.c\n@@ -1 +1 @@\n-#include \"b.h\"\n+#include \"../inc/b.h\"\n\
                \u{1}c4\u{2}400\n\ndiff --git a/src/c.h b/src/c.h\n--- a/src/c.h\n+++ b/src/c.h\n@@ -0,0 +1 @@\n+#include \"../inc/b.h\"\n\
                \u{1}c5\u{2}500\n\ndiff --git a/src/a.c b/src/a.c\n--- a/src/a.c\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-#include \"../inc/b.h\"\n-int main(void) { return 0; }\n";
    let mut c = Cronologia::default();
    for l in archivos.lines() { c.linea_archivos(l); }
    for l in refs.lines() { c.linea_refs(l); }
    c
}

#[test]
fn reproduce_la_historia_paso_a_paso() {
    let c = historia();
    assert_eq!(c.commits.len(), 5);
    assert_eq!(c.autores, ["Ana", "Luis"]);
    assert_eq!(c.sueltos, 0);
    let r = reproducir(&c, 5);
    let f: Vec<(u32, u32, u32)> = r.fotos.iter().map(|f| (f.archivos, f.dependencias, f.ciclos)).collect();
    // archivos, dependencias, ciclos en cada commit.
    assert_eq!(f, [(2, 1, 0), (3, 2, 0), (3, 2, 0), (3, 3, 1), (2, 2, 1)], "{:?}", r.fotos);
    // inc/b.h nacio en c1 (con otro nombre) y se movio en c3.
    let b = c.id_ruta("inc/b.h").unwrap();
    let vida = &r.vidas[&b];
    assert_eq!(vida.nace, 0, "nace con su primer nombre");
    assert_eq!(vida.antes, [(c.id_ruta("src/b.h").unwrap(), 2)]);
    // El #include "c.h" de b.h entro en c2 y se fue con el con el cambio de nombre.
    assert_eq!(r.entra.get(&(b, c.id_texto("c.h").unwrap())), Some(&1));
    // El que cerro el ciclo: c.h incluye ../inc/b.h en c4.
    assert_eq!(r.entra.get(&(c.id_ruta("src/c.h").unwrap(), c.id_texto("../inc/b.h").unwrap())), Some(&3));
    // a.c murio en c5; las carpetas en orden: src (c1) e inc (c3).
    assert_eq!(r.vidas[&c.id_ruta("src/a.c").unwrap()].muere, Some(4));
    assert_eq!(r.carpetas.get("src"), Some(&0));
    assert_eq!(r.carpetas.get("inc"), Some(&2));
}

#[test]
fn cortes_repartidos_y_recorrido_propio() {
    assert_eq!(marcas(10, 3).into_iter().collect::<Vec<_>>(), [3, 6, 9], "el ultimo siempre");
    assert_eq!(marcas(2, 9).len(), 2, "no mas cortes que commits");
    assert!(marcas(0, 4).is_empty());
    let c = historia();
    let mut en_src = Vec::new();
    recorrer(&c, 5, |r, _| en_src.push(r.vivos.iter().filter(|k| c.ruta_de(**k).starts_with("src/")).count()));
    assert_eq!(en_src, [2, 3, 2, 2, 1]);
}

#[test]
fn una_linea_rara_no_rompe_nada() {
    let mut c = Cronologia::default();
    for l in ["sin cabecera", "A\tsuelto.c", "\u{1}", "\u{1}x\u{2}no-es-fecha\u{2}Ana", "Z\traro", "R100\tsolo-uno"] { c.linea_archivos(l); }
    for l in ["+#include \"x.h\"", "\u{1}desconocido\u{2}1", "+++ b/a.c", "+#include \"y.h\""] { c.linea_refs(l); }
    assert_eq!(c.commits.len(), 1);
    assert_eq!(c.commits[0].fecha, 0);
    assert_eq!(c.sueltos, 1, "el commit que no estaba en la primera lectura se cuenta");
    assert!(c.refs.iter().all(|v| v.is_empty()));
    let r = reproducir(&c, 3);
    assert_eq!(r.fotos.len(), 1);
}
