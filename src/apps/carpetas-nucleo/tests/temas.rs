// SPDX-License-Identifier: GPL-2.0-only
//! Temas y ajustes: elegir, guardar, validar y que la vista use los colores del tema.

use carpetas_nucleo::*;

fn estado() -> Estado {
    let mut s = Estado { acento: [1, 2, 3], ..Estado::default() };
    reducir(&mut s, Accion::Cargar { entradas: vec![Entrada { nombre: "a.png".into(), tipo: Tipo::Imagen, tam: 1 }], disposicion: Disposicion::default() });
    s
}

/// Contraste WCAG 2 entre dos colores sRGB.
fn contraste(a: Rgb, b: Rgb) -> f64 {
    let lum = |c: Rgb| {
        let f = |v: u8| { let v = v as f64 / 255.0; if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) } };
        0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn ajustes_guardados(ef: &[Efecto]) -> Ajustes {
    match ef { [Efecto::GuardarAjustes(j)] => Ajustes::de_json(j.as_bytes()), otro => panic!("esperaba GuardarAjustes: {otro:?}") }
}

#[test]
fn tabla_de_temas_bien_formada() {
    let mut ids = std::collections::BTreeSet::new();
    for (i, t) in TEMAS.iter().enumerate() {
        assert!(ids.insert(t.id), "id repetido: {}", t.id);
        // Contraste WCAG: texto 7:1 (AAA) y texto secundario 4,5:1 (AA), sobre fondo y paneles.
        for (fondo, donde) in [(t.fondo, "fondo"), (t.panel, "panel")] {
            assert!(contraste(t.texto, fondo) >= 7.0, "{}: texto sobre {donde} {:.1}", t.nombre, contraste(t.texto, fondo));
            assert!(contraste(t.tenue, fondo) >= 4.5, "{}: tenue sobre {donde} {:.1}", t.nombre, contraste(t.tenue, fondo));
        }
        assert!(contraste(t.sobre_acento, t.acento) >= 4.5, "{}: texto sobre el acento", t.nombre);
        // Cada tema tiene su comando en la paleta.
        assert!(COMANDOS.iter().any(|c| c.comando == Comando::Tema(i as u8) && c.titulo.ends_with(t.nombre)));
    }
    assert!(TEMAS.len() >= 9);
}

#[test]
fn neon_usa_el_acento_del_escritorio() {
    let s = estado();
    let v = vista(&s);
    assert_eq!(v.tema.id, "neon-bmox");
    assert_eq!(v.tema.acento, [1, 2, 3], "Neon BMO-X siempre usa tu acento");
}

#[test]
fn elegir_tema_por_comando_paleta_y_tecla() {
    let mut s = estado();
    let a = ajustes_guardados(&reducir(&mut s, Accion::Comando(Comando::Tema(3))));
    assert_eq!(a.tema, "tokyo-night");
    let v = vista(&s);
    assert_eq!(v.tema.acento, [0x7a, 0xa2, 0xf7], "otros temas traen su acento");
    let foto = v.nodos.iter().find_map(|n| if let Nodo::Icono { color, .. } = n { Some(*color) } else { None }).unwrap();
    assert_eq!(foto, v.tema.tipos[0], "los iconos usan los colores del tema");
    // Con 'acento del escritorio' cualquier tema toma tu acento.
    reducir(&mut s, Accion::Comando(Comando::AcentoEscritorio));
    assert_eq!(vista(&s).tema.acento, [1, 2, 3]);
    // T pasa al siguiente y da la vuelta.
    for _ in 0..TEMAS.len() { reducir(&mut s, atajo(Tecla::T, false, false, false, false).unwrap()); }
    assert_eq!(vista(&s).tema.id, "tokyo-night");
    // Desde la paleta, como en VS Code.
    reducir(&mut s, Accion::Comando(Comando::Paleta));
    reducir(&mut s, Accion::Paleta(AccionPaleta::Escribir("tema drac".into())));
    reducir(&mut s, Accion::Paleta(AccionPaleta::Aceptar));
    assert_eq!(vista(&s).tema.id, "dracula");
    assert!(reducir(&mut s, Accion::Comando(Comando::Tema(200))).is_empty(), "un indice fuera de la tabla no hace nada");
}

#[test]
fn panel_de_ajustes_y_brillo() {
    let mut s = estado();
    assert!(vista(&s).ajustes.is_none());
    reducir(&mut s, atajo(Tecla::Coma, true, false, false, false).unwrap());
    let v = vista(&s);
    let panel = v.ajustes.unwrap();
    assert_eq!(panel.temas.len(), TEMAS.len());
    assert!(panel.temas[0].4 && panel.brillo, "Neon elegido y con brillo");
    let a = ajustes_guardados(&reducir(&mut s, Accion::Comando(Comando::Brillo)));
    assert_eq!(a.brillo, Some(false));
    assert!(!vista(&s).tema.brillo);
    reducir(&mut s, Accion::Comando(Comando::Paleta));
    assert!(!s.panel_ajustes, "abrir la paleta cierra los ajustes");
}

#[test]
fn ajustes_raros_vuelven_al_defecto() {
    assert_eq!(Ajustes::de_json(b"basura"), Ajustes::default());
    let a = Ajustes::de_json(br#"{"tema":"../../etc","brillo":true}"#);
    assert_eq!(a.tema, "neon-bmox");
    assert_eq!(a.brillo, Some(true));
    let mut s = estado();
    reducir(&mut s, Accion::CargarAjustes(Ajustes::de_json(br#"{"tema":"nord"}"#)));
    assert_eq!(vista(&s).tema.nombre, "Nord");
}
