// SPDX-License-Identifier: GPL-2.0-only
//! El organizador con nombres reales de Chromium (tests/datos/chromium.txt).
use carpetas_nucleo::organizar::{camino_a, es_meta, organizar, raiz_unidad, Clase, Hijo, Organizar};
use carpetas_nucleo::*;

fn carpeta(prefijo: &str) -> Vec<Entrada> {
    include_str!("datos/chromium.txt").lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.strip_prefix(prefijo))
        .filter(|r| !r.trim_end_matches('/').contains('/'))
        .map(|r| match r.strip_suffix('/') { Some(c) => Entrada { nombre: c.into(), tipo: Tipo::Carpeta, tam: 0 },
                                            None => Entrada { nombre: r.into(), tipo: tipo_por_extension(r), tam: 100 } }).collect()
}

fn nombres(h: &[Hijo]) -> Vec<String> { h.iter().map(|x| match x { Hijo::Real(e) => e.nombre.clone(), Hijo::Grupo(g) => format!("[{}]", g.nombre) }).collect() }

#[test]
fn raices_de_unidad_como_chromium() {
    for (n, r) in [("base_paths_android.cc", "base_paths"), ("base_paths_mac.mm", "base_paths"), ("at_exit_unittest.cc", "at_exit"),
                   ("check_deref_nocompile.nc", "check_deref"), ("base64_decode_fuzzer.cc", "base64_decode"), ("IDS_X.png.sha1", "IDS_X"),
                   (".clang-format", ".clang-format"), ("_test.cc", "_test"), ("file_path_watcher_linux.cc", "file_path_watcher")] {
        assert_eq!(raiz_unidad(n), r, "{n}");
    }
    for n in ["BUILD.gn", "OWNERS", "SECURITY_OWNERS", "DEPS", "DIR_METADATA", "README.md", "README.chromium", "features.gni", "LICENSE"] { assert!(es_meta(n), "{n}"); }
    assert!(!es_meta("main.cc") && !es_meta("Cargo.toml") && !es_meta("Makefile"));
}

#[test]
fn base_de_chromium_se_organiza() {
    let base = carpeta("base/");
    let archivos = base.iter().filter(|e| e.tipo != Tipo::Carpeta).count();
    let carpetas = base.len() - archivos;
    assert!(archivos > 250 && carpetas > 30, "{archivos} archivos, {carpetas} carpetas");
    let hijos = organizar(&base, Organizar::Auto, None);
    // Las carpetas primero, intactas; build y metadatos al final.
    assert!(hijos[..carpetas].iter().all(|h| matches!(h, Hijo::Real(e) if e.tipo == Tipo::Carpeta)));
    assert!(matches!(hijos.last(), Some(Hijo::Grupo(g)) if g.clase == Clase::Meta && g.miembros.iter().any(|m| m.nombre == "BUILD.gn")));
    let nodos = hijos.len() - carpetas;
    assert!(nodos * 2 < archivos, "{archivos} archivos -> {nodos} nodos: al menos la mitad");
    // base_paths: cabecera primero, codigo, plataformas, y su resumen.
    let bp = hijos.iter().find_map(|h| match h { Hijo::Grupo(g) if g.nombre == "base_paths" => Some(g), _ => None }).unwrap();
    assert_eq!(bp.miembros[0].nombre, "base_paths.h");
    assert_eq!(bp.miembros[1].nombre, "base_paths.cc");
    assert!(bp.resumen.starts_with("h · cc"), "{}", bp.resumen);
    assert!(bp.miembros.len() >= 10);
    // Nada se pierde ni se repite: cada archivo esta en un sitio exactamente.
    let mut vistos: Vec<&str> = Vec::new();
    for h in &hijos { match h { Hijo::Real(e) => vistos.push(&e.nombre), Hijo::Grupo(g) => vistos.extend(g.miembros.iter().map(|m| m.nombre.as_str())) } }
    vistos.sort();
    let mut todos: Vec<&str> = base.iter().map(|e| e.nombre.as_str()).collect();
    todos.sort();
    assert_eq!(vistos, todos);
    // Abrir un grupo da sus miembros; y se sabe el camino para revelar un archivo.
    let dentro = organizar(&base, Organizar::Auto, Some("u:base_paths"));
    assert_eq!(nombres(&dentro)[..2], ["base_paths.h", "base_paths.cc"]);
    assert_eq!(camino_a(&base, Organizar::Auto, "base_paths_win.cc"), ["u:base_paths"]);
    assert!(camino_a(&base, Organizar::Auto, "files").is_empty(), "una carpeta no esta en ningun grupo");
    // Nunca: como en disco.
    assert_eq!(organizar(&base, Organizar::Nunca, None).len(), base.len());
}

#[test]
fn net_http_y_carpeta_pequena() {
    let http = carpeta("net/http/");
    let hijos = organizar(&http, Organizar::Auto, None);
    assert!(hijos.len() * 2 < http.len(), "{} -> {}", http.len(), hijos.len());
    let pocos: Vec<Entrada> = http.iter().take(10).cloned().collect();
    assert_eq!(organizar(&pocos, Organizar::Auto, None).len(), 10, "pocas cosas: tal cual");
    assert!(organizar(&pocos, Organizar::Siempre, None).len() <= 10);
}

#[test]
fn carpeta_generada_gigante_por_prefijos() {
    // Como chrome/app/generated_resources_grd: miles de IDS_*.png.sha1.
    let mut v = Vec::new();
    for a in ["A11Y", "ACCOUNT", "BOOKMARK", "DOWNLOAD", "SETTINGS"] {
        for b in 0..300 { v.push(Entrada { nombre: format!("IDS_{a}_X{b}_LABEL.png.sha1"), tipo: Tipo::Otro, tam: 41 }); }
    }
    v.push(Entrada { nombre: "OWNERS".into(), tipo: Tipo::Otro, tam: 9 });
    let hijos = organizar(&v, Organizar::Auto, None);
    // Todo empieza por IDS: el primer nivel salta solo a IDS_A11Y, IDS_ACCOUNT...
    let n = nombres(&hijos);
    assert_eq!(n, ["[IDS_A11Y…]", "[IDS_ACCOUNT…]", "[IDS_BOOKMARK…]", "[IDS_DOWNLOAD…]", "[IDS_SETTINGS…]", "OWNERS"]);
    let Hijo::Grupo(g) = &hijos[0] else { panic!() };
    assert_eq!(g.resumen, "300 archivos");
    // Dentro, 300 son muchos: otro nivel por prefijo.
    let dentro = organizar(&v, Organizar::Auto, Some(&g.clave));
    assert!(dentro.len() < 300 && !dentro.is_empty());
    let total: usize = dentro.iter().map(|h| match h { Hijo::Real(_) => 1, Hijo::Grupo(g) => g.miembros.len() }).sum();
    assert_eq!(total, 300, "nada se pierde");
    let camino = camino_a(&v, Organizar::Auto, "IDS_BOOKMARK_X7_LABEL.png.sha1");
    assert!(camino.len() >= 1 && camino[0].ends_with("IDS_BOOKMARK"), "{camino:?}");
}
