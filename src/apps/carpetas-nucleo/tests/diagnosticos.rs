// SPDX-License-Identifier: GPL-2.0-only
use carpetas_nucleo::diagnosticos::{leer, resolver, Lector};

fn todo(lineas: &[&str]) -> Vec<(String, u32, u32, bool, String)> {
    let mut l = Lector::default();
    lineas.iter().flat_map(|x| leer(&mut l, x)).map(|(a, d)| (a, d.linea, d.columna, d.grave, d.mensaje)).collect()
}

#[test]
fn rustc_errores_y_avisos() {
    let d = todo(&["   Compiling asteroids v0.1.0", "error[E0425]: cannot find value `naves` in this scope", " --> src/main.rs:3:5",
                   "  |", "3 |     naves.len();", "warning: unused variable: `x`", "  --> src/ship.rs:10:9", "error: could not compile `asteroids`"]);
    assert_eq!(d.len(), 2);
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].2, d[0].3), ("src/main.rs", 3, 5, true));
    assert!(d[0].4.contains("E0425"));
    assert_eq!((d[1].0.as_str(), d[1].1, d[1].3), ("src/ship.rs", 10, false));
}

#[test]
fn gcc_y_python() {
    let d = todo(&["main.c:12:7: error: 'x' undeclared (first use in this function)", "util.h:3:1: warning: no newline at end of file",
                   "make: *** [Makefile:2: todo] Error 1"]);
    assert_eq!(d.iter().map(|x| (x.0.as_str(), x.1, x.2, x.3)).collect::<Vec<_>>(), [("main.c", 12, 7, true), ("util.h", 3, 1, false)]);
    let d = todo(&["Traceback (most recent call last):", "  File \"/home/u/p/juego.py\", line 8, in <module>", "    mover()",
                   "  File \"/home/u/p/fisica.py\", line 21, in mover", "    x = y / 0", "ZeroDivisionError: division by zero"]);
    assert_eq!(d.len(), 2);
    assert_eq!((d[1].0.as_str(), d[1].1), ("/home/u/p/fisica.py", 21));
    assert_eq!(d[1].4, "ZeroDivisionError: division by zero");
    assert!(todo(&["hola mundo: todo bien", "fotograma 3", "x:y:z"]).is_empty(), "lineas normales no son errores");
}

#[test]
fn rutas_a_nodos_sin_salir() {
    assert_eq!(resolver("src/main.rs", "", "/home/u/asteroids"), Some("src/main.rs".into()));
    assert_eq!(resolver("src/main.rs", "juego", "/home/u/p"), Some("juego/src/main.rs".into()), "relativo a donde corrio");
    assert_eq!(resolver("/home/u/p/fisica.py", "", "/home/u/p"), Some("fisica.py".into()), "absoluto dentro de la carpeta");
    assert_eq!(resolver("./src/../src/a.rs", "", "/x"), Some("src/a.rs".into()));
    assert_eq!(resolver("/usr/lib/rustlib/core.rs", "", "/home/u/p"), None, "fuera de la carpeta abierta");
    assert_eq!(resolver("../../etc/passwd", "", "/home/u/p"), None);
}

#[test]
fn panic_de_rust_en_marcha() {
    let d = todo(&["     Running `target/debug/asteroids`", "thread 'main' panicked at src/ship.rs:42:13:", "index out of bounds: the len is 3 but the index is 7",
                   "note: run with `RUST_BACKTRACE=1`"]);
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].2), ("src/ship.rs", 42, 13));
    assert!(d[0].4.starts_with("panic: index out of bounds"));
    let d = todo(&["thread 'main' panicked at 'boom', src/main.rs:2:5"]);
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].4.as_str()), ("src/main.rs", 2, "panic: boom"));
    let mut l = Lector::default();
    assert!(leer(&mut l, "thread 'w' panicked at src/a.rs:1:1:").is_empty());
    assert_eq!(carpetas_nucleo::diagnosticos::cerrar(&mut l).len(), 1, "un panic sin mensaje sale al terminar");
}

#[test]
fn lineas_de_la_terminal_con_clic() {
    use carpetas_nucleo::diagnosticos::ubicacion;
    assert_eq!(ubicacion("   --> src/main.rs:3:5"), Some(("src/main.rs".into(), 3, 5)));
    assert_eq!(ubicacion("  File \"/p/juego.py\", line 8, in <module>"), Some(("/p/juego.py".into(), 8, 1)));
    assert_eq!(ubicacion("thread 'main' panicked at src/ship.rs:42:13:"), Some(("src/ship.rs".into(), 42, 13)));
    assert_eq!(ubicacion("main.c:12:7: error: x"), Some(("main.c".into(), 12, 7)));
    assert_eq!(ubicacion("   Compiling asteroids v0.1.0"), None);
}
