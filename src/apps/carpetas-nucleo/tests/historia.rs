// SPDX-License-Identifier: GPL-2.0-only
use carpetas_nucleo::historia::*;
use carpetas_nucleo::ciudad::color_calor;

fn log() -> String {
    // Tres commits (del mas nuevo al mas viejo, como git log).
    let mut s = String::new();
    s += "\u{1}3000\u{2}Eddi\n\nsrc/motor.cpp\nsrc/vga.c\n";
    s += "\u{1}2000\u{2}Ana\n\nsrc/motor.cpp\n../fuera.c\n/etc/passwd\n";
    s += "\u{1}1000\u{2}Eddi\n\nsrc/motor.cpp\nREADME.md\n";
    s
}

#[test]
fn cuenta_cambios_fechas_y_autores() {
    let h = leer_log(&log(), MAX_COMMITS);
    assert_eq!((h.commits, h.desde, h.hasta, h.autores, h.truncada), (3, 1000, 3000, 2, false));
    let m = h.por_ruta["src/motor.cpp"];
    assert_eq!((m.veces, m.ultimo, m.primero, m.autores), (3, 3000, 1000, 2));
    assert_eq!(h.por_ruta["src/vga.c"].veces, 1);
    assert!(!h.por_ruta.contains_key("../fuera.c") && !h.por_ruta.contains_key("/etc/passwd"), "rutas raras fuera");
    // Una carpeta suma lo de dentro; "sr" no es "src".
    assert_eq!(h.de("src").veces, 4);
    assert_eq!(h.de("sr").veces, 0);
    assert_eq!(h.de("").veces, 5);
    assert_eq!(h.max_veces(), 3);
    // Limite de commits: se queda con los ultimos y lo dice.
    let h2 = leer_log(&log(), 2);
    assert!(h2.truncada && h2.commits == 2 && h2.por_ruta["src/motor.cpp"].veces == 2);
}

#[test]
fn puntos_calientes_grande_y_cambiante_primero() {
    let h = leer_log(&log(), MAX_COMMITS);
    let lineas = [("src/motor.cpp", 400u64), ("src/vga.c", 5000), ("README.md", 10), ("nuevo.rs", 900)];
    let p = puntos_calientes(&h, lineas.iter().map(|(r, l)| (*r, *l)), 10);
    assert_eq!(p[0].0, "src/motor.cpp", "3 cambios x 400 lineas gana a 1 cambio x 5000");
    assert!(p.iter().all(|x| x.0 != "nuevo.rs"), "sin historia no es punto caliente");
    assert_eq!(hace(1000, 1000 + 3 * 86_400), "hace 3 dias");
    assert_eq!(hace(0, 5), "nunca (en lo leido)");
    assert_eq!(hace(0 + 1, 1 + 400 * 86_400), "hace 1 ano");
}

#[test]
fn abrir_pide_la_historia_y_la_guarda() {
    use carpetas_nucleo::*;
    let mut s = Estado::default();
    let ef = reducir(&mut s, Accion::Cargar { nombre: "p".into(), entradas: vec![], disposicion: Disposicion::default() });
    assert!(ef.contains(&Efecto::LeerHistoria));
    let antes = s.version_contenido;
    reducir(&mut s, Accion::Historia { resultado: Ok(leer_log(&log(), MAX_COMMITS)), ahora: 5000 });
    assert!(s.historia.lista && s.historia.sin.is_none() && s.historia.datos.commits == 3 && s.historia.ahora == 5000);
    assert_eq!(s.version_contenido, antes, "no rehace el grafo");
    reducir(&mut s, Accion::Historia { resultado: Err("no es un repositorio git".into()), ahora: 1 });
    assert_eq!(s.historia.sin.as_deref(), Some("no es un repositorio git"));
    assert_eq!(color_calor(None), [52, 56, 68]);
    assert_ne!(color_calor(Some(0.0)), color_calor(Some(1.0)));
}
