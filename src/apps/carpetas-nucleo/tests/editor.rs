// SPDX-License-Identifier: GPL-2.0-only
//! Editor: pestanas y buscar y reemplazar (puro, sin pantalla).
use carpetas_nucleo::editor::{buscar, reemplazar_todo, AccionBuscar as B, MAX_HALLAZGOS, MAX_PESTANAS};
use carpetas_nucleo::*;

fn e(n: &str, t: Tipo, tam: u64) -> Entrada { Entrada { nombre: n.into(), tipo: t, tam } }

fn con_archivos(n: usize) -> Estado {
    let mut s = Estado::default();
    let mut lista: Vec<Entrada> = (0..n).map(|i| e(&format!("f{i}.rs"), Tipo::Codigo, 10)).collect();
    lista.push(e("motor.cpp", Tipo::Codigo, 10));
    reducir(&mut s, Accion::Cargar { nombre: "p".into(), entradas: lista, disposicion: Disposicion::default() });
    s
}

fn abrir(s: &mut Estado, id: &str, texto: &str) {
    reducir(s, Accion::AbrirEditor { id: id.into(), linea: None });
    reducir(s, Accion::ArchivoLeido { id: id.into(), resultado: Ok(texto.into()) });
}

fn trozos<'a>(t: &'a str, h: &[(usize, usize)]) -> Vec<&'a str> { h.iter().map(|(a, z)| &t[*a..*z]).collect() }

#[test]
fn buscar_mayusculas_palabras_y_utf8() {
    let t = "Nave nave NAVE naves; navegar\nñandú Ñandú ÑANDÚ";
    assert_eq!(trozos(t, &buscar(t, "nave", true, false, 100)), ["nave", "nave", "nave"], "con mayusculas: nave, naves, navegar");
    assert_eq!(buscar(t, "nave", false, false, 100).len(), 5);
    assert_eq!(trozos(t, &buscar(t, "nave", false, true, 100)), ["Nave", "nave", "NAVE"], "palabra entera");
    assert_eq!(trozos(t, &buscar(t, "ñandú", false, false, 100)), ["ñandú", "Ñandú", "ÑANDÚ"], "sin mayusculas tambien con ñ y tildes");
    assert_eq!(buscar(t, "ñandú", true, false, 100).len(), 1);
    assert!(buscar(t, "", false, false, 100).is_empty());
    assert_eq!(buscar("aaaa", "aa", true, false, 100), [(0, 2), (2, 4)], "sin solaparse");
    assert_eq!(buscar(&"x".repeat(50), "x", false, false, 7).len(), 7, "con limite");
    // Los tramos caen siempre en limites de letra.
    for (a, z) in buscar("añoaño", "o", false, false, 10) { assert!("añoaño".is_char_boundary(a) && "añoaño".is_char_boundary(z)); }
}

#[test]
fn reemplazar_uno_y_todos() {
    let mut s = con_archivos(1);
    abrir(&mut s, "f0.rs", "let vida = 3;\nvida += vida;\nsobrevida = 1;\n");
    reducir(&mut s, Accion::Buscar(B::Abrir { reemplazar: true }));
    reducir(&mut s, Accion::Buscar(B::Patron("vida".into())));
    reducir(&mut s, Accion::Buscar(B::Palabra));
    let b = s.editor.busqueda.as_ref().unwrap();
    assert_eq!(b.hallazgos.len(), 3, "sobrevida no cuenta como palabra");
    // Seleccionar una coincidencia salta sin robar el foco (rango, no linea).
    let ed = s.editor.activa().unwrap();
    assert_eq!((ed.rango, ed.ir_a), (Some((4, 8)), None));
    reducir(&mut s, Accion::Buscar(B::Siguiente));
    assert_eq!(s.editor.busqueda.as_ref().unwrap().actual, 1);
    reducir(&mut s, Accion::Buscar(B::Reemplazo("salud".into())));
    let carga = s.editor.activa().unwrap().carga;
    reducir(&mut s, Accion::Buscar(B::Reemplazar));
    let ed = s.editor.activa().unwrap();
    assert_eq!(ed.texto, "let vida = 3;\nsalud += vida;\nsobrevida = 1;\n");
    assert!(ed.sucio && ed.carga > carga, "cambia el texto desde el nucleo: la plataforma lo recarga");
    assert_eq!(s.editor.busqueda.as_ref().unwrap().hallazgos.len(), 2);
    reducir(&mut s, Accion::Buscar(B::ReemplazarTodo));
    assert_eq!(s.editor.activa().unwrap().texto, "let salud = 3;\nsalud += salud;\nsobrevida = 1;\n");
    assert_eq!(s.editor.activa().unwrap().aviso.as_deref(), Some("2 reemplazos"));
    assert!(s.editor.busqueda.as_ref().unwrap().hallazgos.is_empty());
    // Teclar vuelve a buscar; cerrar recuerda el patron para la proxima vez.
    reducir(&mut s, Accion::Editar("vida vida".into()));
    assert_eq!(s.editor.busqueda.as_ref().unwrap().hallazgos.len(), 2);
    reducir(&mut s, Accion::Buscar(B::Cerrar));
    assert!(s.editor.busqueda.is_none());
    reducir(&mut s, atajo(Tecla::F, true, false, false, false).unwrap());
    assert_eq!(s.editor.busqueda.as_ref().unwrap().patron, "vida");
    assert_eq!(reemplazar_todo("a-b-c", &[(1, 2), (3, 4)], "+"), "a+b+c");
}

#[test]
fn reemplazar_respeta_el_limite_y_mucho_hallazgo() {
    let mut s = con_archivos(1);
    abrir(&mut s, "f0.rs", &"e".repeat(20_000));
    reducir(&mut s, Accion::Buscar(B::Abrir { reemplazar: true }));
    reducir(&mut s, Accion::Buscar(B::Patron("e".into())));
    let b = s.editor.busqueda.as_ref().unwrap();
    assert_eq!(b.hallazgos.len(), MAX_HALLAZGOS);
    assert!(b.demasiados);
    reducir(&mut s, Accion::Buscar(B::Reemplazo("x".repeat(256))));
    reducir(&mut s, Accion::Buscar(B::ReemplazarTodo));
    assert!(s.editor.activa().unwrap().aviso.as_deref().unwrap().contains("2 MiB"), "no pasa del limite");
    assert_eq!(s.editor.activa().unwrap().texto.len(), 20_000);
}

#[test]
fn pestanas_no_pierden_cambios() {
    let mut s = con_archivos(MAX_PESTANAS + 2);
    abrir(&mut s, "f0.rs", "uno");
    reducir(&mut s, Accion::Editar("uno cambiado".into()));
    for i in 1..MAX_PESTANAS { abrir(&mut s, &format!("f{i}.rs"), "x"); }
    assert_eq!(s.editor.lista.len(), MAX_PESTANAS);
    // Llena: se recicla la mas vieja SIN cambios (f1), nunca f0 que tiene cambios.
    abrir(&mut s, "motor.cpp", "int main() {}");
    assert_eq!(s.editor.lista.len(), MAX_PESTANAS);
    assert!(s.editor.lista.iter().any(|e| e.id == "f0.rs" && e.texto == "uno cambiado"));
    assert!(!s.editor.lista.iter().any(|e| e.id == "f1.rs"));
    assert_eq!(s.editor.activa().unwrap().id, "motor.cpp");
    // Ctrl+Tab / Ctrl+Shift+Tab dan la vuelta.
    reducir(&mut s, atajo(Tecla::Tab, true, false, false, false).unwrap());
    assert_eq!(s.editor.activa, 0);
    reducir(&mut s, atajo(Tecla::Tab, true, true, false, false).unwrap());
    assert_eq!(s.editor.activa().unwrap().id, "motor.cpp");
    // Cerrar una de en medio deja la activa en su sitio.
    let activa = s.editor.activa().unwrap().id.clone();
    reducir(&mut s, Accion::CerrarPestana(3));
    assert_eq!(s.editor.activa().unwrap().id, activa);
    // La busqueda sigue a la pestana activa.
    reducir(&mut s, Accion::Buscar(B::Abrir { reemplazar: false }));
    reducir(&mut s, Accion::Buscar(B::Patron("main".into())));
    assert_eq!(s.editor.busqueda.as_ref().unwrap().hallazgos.len(), 1);
    reducir(&mut s, Accion::Pestana(0));
    assert!(s.editor.busqueda.as_ref().unwrap().hallazgos.is_empty(), "f0 no tiene 'main'");
    // Todas con cambios y llena: no se abre otra y se avisa.
    for ed in s.editor.lista.clone().iter().enumerate() { reducir(&mut s, Accion::Pestana(ed.0)); reducir(&mut s, Accion::Editar(format!("{} cambio", ed.1.texto))); }
    let antes = s.editor.lista.len();
    for i in 0..3 { reducir(&mut s, Accion::AbrirEditor { id: format!("f{}.rs", MAX_PESTANAS + i - 1), linea: None }); }
    assert!(s.editor.lista.len() <= MAX_PESTANAS && s.editor.lista.len() >= antes);
    assert!(s.editor.lista.iter().all(|e| e.texto.ends_with("cambio") || !e.sucio), "ningun cambio se perdio");
}

#[test]
fn volver_al_texto_de_disco_quita_el_sin_guardar() {
    let mut s = con_archivos(1);
    abrir(&mut s, "f0.rs", "fn main() {}");
    reducir(&mut s, Accion::Editar("fn main() { 1 }".into()));
    assert!(s.editor.activa().unwrap().sucio);
    reducir(&mut s, Accion::Editar("fn main() {}".into()));
    assert!(!s.editor.activa().unwrap().sucio, "deshacer hasta el principio = sin cambios");
    reducir(&mut s, Accion::Editar("v2".into()));
    let ef = reducir(&mut s, Accion::Comando(Comando::Guardar));
    assert_eq!(ef, vec![Efecto::GuardarArchivo { id: "f0.rs".into(), texto: "v2".into() }]);
    reducir(&mut s, Accion::Editar("v3".into())); // se teclea antes de que llegue el Guardado
    reducir(&mut s, Accion::Guardado { id: "f0.rs".into(), resultado: Ok(()) });
    assert!(s.editor.activa().unwrap().sucio, "v3 no esta en disco");
    reducir(&mut s, Accion::Editar("v2".into()));
    assert!(!s.editor.activa().unwrap().sucio, "v2 si");
}
