// SPDX-License-Identifier: GPL-2.0-only
//! Errores atrapados en los nodos, procesos en vivo y editor de codigo (clic derecho).
use carpetas_nucleo::procesos::{arbol, totales};
use carpetas_nucleo::*;

fn e(n: &str, t: Tipo, tam: u64) -> Entrada { Entrada { nombre: n.into(), tipo: t, tam } }

fn proyecto() -> Estado {
    let mut s = Estado::default();
    reducir(&mut s, Accion::Cargar { nombre: "asteroids".into(), entradas: vec![e("src", Tipo::Carpeta, 0), e("Cargo.toml", Tipo::Codigo, 100),
        e("docs", Tipo::Carpeta, 0)], disposicion: Disposicion::default() });
    reducir(&mut s, Accion::RutaRaiz("/home/u/asteroids".into()));
    reducir(&mut s, Accion::Comando(Comando::Modo));
    s
}

fn nodo<'a>(v: &'a Vista, id: &str) -> &'a NodoGrafo { v.grafo.as_ref().unwrap().0.iter().find(|n| n.id == id).unwrap() }

#[test]
fn el_error_marca_el_nodo_y_despliega_el_camino() {
    let mut s = proyecto();
    assert!(matches!(reducir(&mut s, Accion::Ejecutar(String::new())).as_slice(), [Efecto::Ejecutar { .. }]));
    let antes = s.version_contenido;
    assert!(reducir(&mut s, Accion::Salida { linea: "   Compiling asteroids v0.1.0".into(), error: true }).is_empty());
    assert_eq!(s.version_contenido, antes, "una linea normal no rehace el grafo");
    reducir(&mut s, Accion::Salida { linea: "error[E0425]: cannot find value `naves`".into(), error: true });
    // src aun no esta desplegada: el error la despliega sola (y pide leerla).
    let ef = reducir(&mut s, Accion::Salida { linea: "  --> src/ship.rs:42:13".into(), error: true });
    assert_eq!(ef, vec![Efecto::LeerCarpeta("src".into())]);
    assert!(s.version_contenido > antes, "el nodo cambia de color");
    let v = vista(&s);
    assert_eq!(nodo(&v, "src").marcas.errores, 1, "la carpeta cuenta lo de dentro");
    assert_eq!(nodo(&v, "").marcas.errores, 1);
    assert_eq!(nodo(&v, "docs").marcas, Marcas::default());
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![e("main.rs", Tipo::Codigo, 50), e("ship.rs", Tipo::Codigo, 80)] });
    let v = vista(&s);
    let ship = nodo(&v, "src/ship.rs");
    assert_eq!((ship.marcas.errores, ship.marcas.avisos), (1, 0));
    assert!(ship.marcas.primero.as_deref().unwrap().starts_with("src/ship.rs:42:13  error[E0425]"));
    assert_eq!(nodo(&v, "src/main.rs").marcas.errores, 0);
    let fila = v.arbol.as_ref().unwrap().iter().find(|f| f.id == "src/ship.rs").unwrap();
    assert_eq!(fila.marcas.errores, 1, "el explorador tambien lo marca");
    // Repetido no cuenta dos veces; un aviso va aparte.
    reducir(&mut s, Accion::Salida { linea: "error[E0425]: cannot find value `naves`".into(), error: true });
    reducir(&mut s, Accion::Salida { linea: "  --> src/ship.rs:42:13".into(), error: true });
    reducir(&mut s, Accion::Salida { linea: "warning: unused import".into(), error: true });
    reducir(&mut s, Accion::Salida { linea: "  --> src/main.rs:1:5".into(), error: true });
    reducir(&mut s, Accion::Termino(Some(101)));
    assert_eq!(s.errores.conteo(""), (1, 1));
    assert!(s.terminal.lineas.back().unwrap().0.contains("1 error y 1 aviso en 2 archivos: el nodo en rojo"));
    // Si el usuario pliega src, no se vuelve a abrir sola con mas salida.
    reducir(&mut s, Accion::Expandir("src".into()));
    assert!(reducir(&mut s, Accion::Contenido { ruta: "docs".into(), entradas: vec![] }).is_empty());
    // Ejecutar otra vez limpia los errores.
    reducir(&mut s, Accion::Ejecutar(String::new()));
    assert_eq!(s.errores.total, 0);
    assert_eq!(nodo(&vista(&s), "").marcas.errores, 0);
}

#[test]
fn rutas_fuera_y_parar_en_el_primer_error() {
    let mut s = proyecto();
    reducir(&mut s, Accion::Comando(Comando::PararEnError));
    reducir(&mut s, Accion::Ejecutar(String::new()));
    // Un error en la biblioteca estandar (fuera de la carpeta) no marca nada ni para.
    reducir(&mut s, Accion::Salida { linea: "error: boom".into(), error: true });
    assert!(reducir(&mut s, Accion::Salida { linea: " --> /home/u/.rustup/lib/core.rs:3:1".into(), error: true }).is_empty());
    assert_eq!(s.errores.total, 0);
    // Un panic en tu codigo: se para el proceso (una vez).
    reducir(&mut s, Accion::Salida { linea: "thread 'main' panicked at Cargo.toml:1:1:".into(), error: true });
    let ef = reducir(&mut s, Accion::Salida { linea: "algo".into(), error: true });
    assert_eq!(ef, vec![Efecto::Detener]);
    reducir(&mut s, Accion::Salida { linea: "thread 'main' panicked at Cargo.toml:2:1:".into(), error: true });
    assert!(!reducir(&mut s, Accion::Salida { linea: "otro".into(), error: true }).contains(&Efecto::Detener));
}

#[test]
fn clic_derecho_abre_el_editor_en_el_error() {
    let mut s = proyecto();
    reducir(&mut s, Accion::Expandir("src".into()));
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![e("main.rs", Tipo::Codigo, 50), e("ship.rs", Tipo::Codigo, 80),
        e("enorme.rs", Tipo::Codigo, 3 * 1024 * 1024)] });
    reducir(&mut s, Accion::Ejecutar(String::new()));
    for l in ["error[E0308]: mismatched types", "  --> src/ship.rs:7:9"] { reducir(&mut s, Accion::Salida { linea: l.into(), error: true }); }
    reducir(&mut s, Accion::Termino(Some(101)));
    // Clic derecho en un archivo con error: se abre en su linea.
    assert_eq!(reducir(&mut s, Accion::AbrirEditor { id: "src/ship.rs".into(), linea: None }), vec![Efecto::LeerArchivo("src/ship.rs".into())]);
    assert_eq!(s.editor.as_ref().unwrap().ir_a, Some(7));
    reducir(&mut s, Accion::ArchivoLeido { id: "otro.rs".into(), resultado: Ok("x".into()) });
    assert!(!s.editor.as_ref().unwrap().cargado, "solo se acepta lo pedido");
    reducir(&mut s, Accion::ArchivoLeido { id: "src/ship.rs".into(), resultado: Ok("fn main() {}\n".into()) });
    assert!(s.editor.as_ref().unwrap().cargado);
    // Teclear no rehace el grafo; Ctrl+S guarda solo si hay cambios.
    assert!(reducir(&mut s, atajo(Tecla::S, true, false, false, false).unwrap()).is_empty());
    let antes = s.version_contenido;
    reducir(&mut s, Accion::Editar("fn main() { 1 }\n".into()));
    assert_eq!(s.version_contenido, antes);
    assert_eq!(reducir(&mut s, atajo(Tecla::S, true, false, false, false).unwrap()),
               vec![Efecto::GuardarArchivo { id: "src/ship.rs".into(), texto: "fn main() { 1 }\n".into() }]);
    // Con cambios sin guardar no se cambia de archivo ni se cierra a la primera.
    assert!(reducir(&mut s, Accion::AbrirEditor { id: "src/main.rs".into(), linea: None }).is_empty());
    let saltos = s.editor.as_ref().unwrap().saltos;
    reducir(&mut s, Accion::AbrirEditor { id: "src/ship.rs".into(), linea: Some(7) });
    assert_eq!(s.editor.as_ref().unwrap().saltos, saltos + 1, "pedir la misma linea otra vez vuelve a saltar");
    assert_eq!(s.editor.as_ref().unwrap().id, "src/ship.rs");
    reducir(&mut s, Accion::Comando(Comando::CerrarEditor));
    assert!(s.editor.is_some() && s.editor.as_ref().unwrap().cerrar_pedido);
    reducir(&mut s, Accion::Guardado { id: "src/ship.rs".into(), resultado: Ok(()) });
    assert!(!s.editor.as_ref().unwrap().sucio);
    reducir(&mut s, Accion::Comando(Comando::CerrarEditor));
    assert!(s.editor.is_none());
    // En una carpeta, el clic derecho va a su primer error.
    assert_eq!(reducir(&mut s, Accion::AbrirEditor { id: "src".into(), linea: None }), vec![Efecto::LeerArchivo("src/ship.rs".into())]);
    s.editor = None;
    // Lo que no se listo, lo demasiado grande y las carpetas sin errores no abren nada.
    assert!(reducir(&mut s, Accion::AbrirEditor { id: "../../etc/passwd".into(), linea: None }).is_empty());
    assert!(reducir(&mut s, Accion::AbrirEditor { id: "src/enorme.rs".into(), linea: None }).is_empty());
    assert!(reducir(&mut s, Accion::AbrirEditor { id: "docs".into(), linea: None }).is_empty());
    // F8 recorre los errores y abre el editor.
    assert_eq!(reducir(&mut s, atajo(Tecla::F8, false, false, false, false).unwrap()), vec![Efecto::LeerArchivo("src/ship.rs".into())]);
    assert!(s.seleccion.contains("src/ship.rs"));
}

#[test]
fn procesos_en_arbol() {
    let p = |pid, ppid, n: &str, cpu| ProcesoVivo { pid, ppid, nombre: n.into(), cpu, memoria_kb: 1000 };
    let lista = vec![p(30, 20, "rustc", 95.0), p(20, 1, "cargo", 2.0), p(31, 20, "rustc", 80.0), p(40, 30, "cc", 10.0)];
    let a: Vec<(usize, &str, u32)> = arbol(&lista).into_iter().map(|(n, x)| (n, x.nombre.as_str(), x.pid)).collect();
    assert_eq!(a, [(0, "cargo", 20), (1, "rustc", 30), (2, "cc", 40), (1, "rustc", 31)]);
    assert_eq!(totales(&lista), (187.0, 4000));
    let mut s = proyecto();
    reducir(&mut s, Accion::Procesos(lista.clone()));
    assert!(s.procesos.is_empty(), "sin proceso en marcha no se guarda nada");
    reducir(&mut s, Accion::Ejecutar(String::new()));
    let antes = s.version_contenido;
    reducir(&mut s, Accion::Procesos((0..100).map(|i| p(i + 2, 1, "x", 0.0)).collect()));
    assert_eq!(s.procesos.len(), carpetas_nucleo::procesos::MAX_PROCESOS);
    assert_eq!(s.version_contenido, antes, "las muestras de /proc no rehacen el grafo");
    reducir(&mut s, Accion::Termino(Some(0)));
    assert!(s.procesos.is_empty());
}

#[test]
fn resaltado_sin_huecos_y_con_utf8() {
    use carpetas_nucleo::editor::{lenguaje, resaltar, Lenguaje, Token};
    assert_eq!(lenguaje("src/main.rs"), Lenguaje::Rust);
    assert_eq!(lenguaje("juego.py"), Lenguaje::Python);
    assert_eq!(lenguaje("Makefile"), Lenguaje::Shell);
    let l = "    let naves = vec![1, 2]; // ñandú \"x\"";
    let t = resaltar(l, Lenguaje::Rust);
    // Cubre toda la linea, en orden, y cada tramo cae en limite de caracter.
    let mut pos = 0;
    for (a, z, _) in &t { assert_eq!(*a, pos); assert!(l.is_char_boundary(*z)); pos = *z; }
    assert_eq!(pos, l.len());
    // Tramos vecinos del mismo color se juntan: se busca por posicion.
    let de = |s: &str| { let i = l.find(s).unwrap(); t.iter().find(|(a, z, _)| *a <= i && i + s.len() <= *z).map(|x| x.2) };
    assert_eq!(de("let"), Some(Token::Clave));
    assert_eq!(de("vec"), Some(Token::Clave), "macro");
    assert_eq!(de("naves"), Some(Token::Normal));
    assert_eq!(t.last().unwrap().2, Token::Comentario);
    let t = resaltar("x = 'hola' # ñ", Lenguaje::Python);
    assert!(t.iter().any(|(a, z, k)| *k == Token::Texto && &"x = 'hola' # ñ"[*a..*z] == "'hola'"));
    assert!(resaltar("\"sin cerrar \\", Lenguaje::C).iter().all(|(_, z, _)| *z <= 13), "no se pasa del final");
    assert_eq!(resaltar("texto plano", Lenguaje::Otro), vec![(0, 11, Token::Normal)]);
}
