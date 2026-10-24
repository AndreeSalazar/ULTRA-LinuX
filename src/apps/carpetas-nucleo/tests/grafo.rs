// SPDX-License-Identifier: GPL-2.0-only
//! Modo grafo (Blender/Houdini), explorador, iconos y carriles con semaforo.

use carpetas_nucleo::grafo::{acomodar, visibles, MAX_NODOS};
use carpetas_nucleo::iconos::icono;
use carpetas_nucleo::*;

fn e(n: &str, t: Tipo, tam: u64) -> Entrada { Entrada { nombre: n.into(), tipo: t, tam } }

fn estado_grafo() -> Estado {
    let mut s = Estado::default();
    reducir(&mut s, Accion::Cargar { nombre: "prueba".into(), entradas: vec![e("src", Tipo::Carpeta, 0), e("docs", Tipo::Carpeta, 0), e("Cargo.toml", Tipo::Codigo, 400)],
                                    disposicion: Disposicion::default() });
    reducir(&mut s, Accion::Comando(Comando::Modo));
    assert_eq!(s.ajustes.modo, Modo::Grafo);
    s
}

#[test]
fn desplegar_pide_disco_y_acepta_solo_lo_pedido() {
    let mut s = estado_grafo();
    assert_eq!(reducir(&mut s, Accion::Abrir("src".into())), vec![Efecto::LeerCarpeta("src".into())], "en grafo, abrir carpeta = desplegar");
    // Respuesta de algo no pedido o inventado: se ignora.
    reducir(&mut s, Accion::Contenido { ruta: "docs".into(), entradas: vec![e("x", Tipo::Otro, 1)] });
    reducir(&mut s, Accion::Contenido { ruta: "../etc".into(), entradas: vec![e("passwd", Tipo::Otro, 1)] });
    assert!(!s.arbol.contains_key("docs") && !s.arbol.contains_key("../etc"));
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![e("main.rs", Tipo::Codigo, 2000), e("app", Tipo::Carpeta, 0)] });
    let ids: Vec<String> = visibles(&s).into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["", "src", "src/main.rs", "src/app", "docs", "Cargo.toml"]);
    // Abrir un archivo de una subcarpeta pide su ruta relativa.
    assert_eq!(reducir(&mut s, Accion::Abrir("src/main.rs".into())), vec![Efecto::AbrirArchivo("src/main.rs".into())]);
    // Plegar no pide disco.
    assert!(reducir(&mut s, Accion::Expandir("src".into())).is_empty());
    assert_eq!(visibles(&s).len(), 4);
}

#[test]
fn acomodo_blender_y_houdini() {
    let mut s = estado_grafo();
    reducir(&mut s, Accion::Expandir("src".into()));
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![e("a.rs", Tipo::Codigo, 1), e("b.rs", Tipo::Codigo, 1)] });
    let p = acomodar(&s);
    // Blender: profundidad hacia la derecha; hermanos uno debajo de otro.
    assert!(p["src/a.rs"][0] > p["src"][0] && p["src"][0] > p[""][0]);
    assert!(p["src/b.rs"][1] > p["src/a.rs"][1]);
    assert_eq!(p["src"][1], (p["src/a.rs"][1] + p["src/b.rs"][1]) / 2.0, "el padre se centra sobre sus hijos");
    reducir(&mut s, Accion::Comando(Comando::Direccion));
    let h = acomodar(&s);
    // Houdini: profundidad hacia abajo; hermanos uno al lado de otro.
    assert!(h["src/a.rs"][1] > h["src"][1] && h["src/b.rs"][0] > h["src/a.rs"][0]);
    let v = vista(&s);
    let (nodos, cables) = v.grafo.unwrap();
    assert_eq!(nodos.len(), 6);
    assert_eq!(cables.len(), 5, "un cable por hijo");
    let c = &cables[0];
    assert!(c.hasta[1] > c.desde[1], "Houdini: los cables bajan");
}

#[test]
fn arrastrar_nodo_fija_posicion_respeta_zoom_y_deshace() {
    let mut s = estado_grafo();
    s.camara.zoom = 2.0;
    let antes = acomodar(&s)["docs"];
    reducir(&mut s, Accion::EmpezarArrastre("docs".into()));
    reducir(&mut s, Accion::Arrastrar([100.0, -40.0]));
    let ef = reducir(&mut s, Accion::Soltar);
    assert!(matches!(ef.as_slice(), [Efecto::Guardar(_)]));
    assert_eq!(acomodar(&s)["docs"], [antes[0] + 50.0, antes[1] - 20.0], "con zoom 2 el nodo se mueve la mitad en el mundo");
    let json = match &ef[0] { Efecto::Guardar(j) => j.clone(), _ => unreachable!() };
    assert_eq!(Disposicion::de_json(json.as_bytes()).grafo["docs"], acomodar(&s)["docs"], "se guarda y se vuelve a leer");
    reducir(&mut s, Accion::Comando(Comando::Deshacer));
    assert_eq!(acomodar(&s)["docs"], antes);
    // Reacomodar borra lo fijado a mano; con candado no.
    reducir(&mut s, Accion::EmpezarArrastre("docs".into())); reducir(&mut s, Accion::Arrastrar([10.0, 0.0])); reducir(&mut s, Accion::Soltar);
    reducir(&mut s, Accion::Comando(Comando::Candado));
    assert!(reducir(&mut s, Accion::Comando(Comando::Reacomodar)).is_empty());
    reducir(&mut s, Accion::Comando(Comando::Candado));
    reducir(&mut s, Accion::Comando(Comando::Reacomodar));
    assert_eq!(acomodar(&s)["docs"], antes);
}

#[test]
fn camara_zoom_ancla_y_encuadrar() {
    let mut s = estado_grafo();
    reducir(&mut s, Accion::Lienzo([800.0, 600.0]));
    s.camara = Camara { x: 0.0, y: 0.0, zoom: 1.0 };
    reducir(&mut s, Accion::Zoom { factor: 2.0, ancla: [400.0, 300.0] });
    let c = s.camara;
    assert_eq!(c.zoom, 2.0);
    assert_eq!([c.x + 400.0 / c.zoom, c.y + 300.0 / c.zoom], [400.0, 300.0], "el punto bajo el raton no se mueve");
    reducir(&mut s, Accion::Zoom { factor: 1000.0, ancla: [0.0, 0.0] });
    assert_eq!(s.camara.zoom, 3.0, "zoom con limites");
    reducir(&mut s, Accion::Zoom { factor: f32::NAN, ancla: [0.0, 0.0] });
    assert_eq!(s.camara.zoom, 3.0);
    let x0 = s.camara.x;
    reducir(&mut s, Accion::Desplazar([90.0, 0.0]));
    assert_eq!(s.camara.x, x0 - 30.0, "arrastrar 90 px con zoom 3 mueve la camara 30 en el mundo");
    reducir(&mut s, atajo(Tecla::Inicio, false, false, false, false).unwrap());
    let (min, max) = carpetas_nucleo::grafo::limites(&s);
    let z = s.camara.zoom;
    assert!(min[0] >= s.camara.x && max[0] <= s.camara.x + 800.0 / z && max[1] <= s.camara.y + 600.0 / z, "encuadrar deja todo a la vista");
}

#[test]
fn banda_en_grafo_y_explorador_sincronizado() {
    let mut s = estado_grafo();
    let p = acomodar(&s);
    reducir(&mut s, Accion::EmpezarBanda { punto: [p["src"][0] - 5.0, p["src"][1] - 5.0], sumar: false });
    reducir(&mut s, Accion::MoverBanda([p["src"][0] + 10.0, p["docs"][1] + 10.0]));
    assert!(s.seleccion.contains("src") && s.seleccion.contains("docs") && !s.seleccion.contains(""), "la raiz no se selecciona");
    // El explorador muestra lo mismo que el grafo, con niveles.
    reducir(&mut s, Accion::Expandir("docs".into()));
    reducir(&mut s, Accion::Contenido { ruta: "docs".into(), entradas: vec![e("LEEME.md", Tipo::Documento, 3)] });
    let filas = vista(&s).arbol.unwrap();
    let resumen: Vec<(String, usize, Option<bool>)> = filas.iter().map(|f| (f.id.clone(), f.nivel, f.expandida)).collect();
    assert_eq!(resumen, [("src".into(), 0, Some(false)), ("docs".into(), 0, Some(true)), ("docs/LEEME.md".into(), 1, None), ("Cargo.toml".into(), 0, None)]);
    reducir(&mut s, atajo(Tecla::B, true, false, false, false).unwrap());
    assert!(vista(&s).arbol.is_none(), "Ctrl+B oculta el explorador");
}

#[test]
fn peso_detalle_y_limites_duros() {
    let mut s = estado_grafo();
    reducir(&mut s, Accion::Expandir("src".into()));
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![e("grande.bin", Tipo::Programa, 8000), e("chico.rs", Tipo::Codigo, 2000)] });
    let (nodos, _) = vista(&s).grafo.unwrap();
    let n = |id: &str| nodos.iter().find(|n| n.id == id).unwrap().clone();
    assert_eq!(n("src/grande.bin").peso, 0.8, "ocupa el 80 % de su carpeta");
    assert_eq!(n("src/chico.rs").peso, 0.2);
    assert_eq!(n("src").peso, 10000.0 / 10400.0, "src pesa 10000 de los 10400 de la raiz (Cargo.toml 400)");
    assert_eq!(n("").nombre, "prueba", "el nodo raiz lleva el nombre de la carpeta");
    assert!(n("").detalle.ends_with("10.2 KiB") && n("").peso == 0.0, "la raiz suma lo leido debajo y no lleva barra");
    assert!(n("src").detalle.starts_with("2 elementos"));
    assert_eq!(n("docs").detalle, "doble clic: desplegar");
    // Limite de nodos: una carpeta gigante no pasa de MAX_NODOS.
    let enorme: Vec<Entrada> = (0..2000).map(|i| e(&format!("f{i}"), Tipo::Otro, 1)).collect();
    reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: enorme });
    assert!(visibles(&s).len() <= MAX_NODOS);
    // Limite de carpetas desplegadas (cada una se vigila en el hilo del disco).
    let muchas: Vec<Entrada> = (0..100).map(|i| e(&format!("d{i}"), Tipo::Carpeta, 0)).collect();
    reducir(&mut s, Accion::Refrescar(muchas));
    let pedidas = (0..100).filter(|i| !reducir(&mut s, Accion::Expandir(format!("d{i}"))).is_empty()).count();
    assert_eq!(pedidas, carpetas_nucleo::carriles::MAX_DESPLEGADAS);
}

#[test]
fn iconos_por_estilo() {
    assert_eq!(icono("main.rs", Tipo::Codigo, EstiloIconos::Material), "material/rust");
    assert_eq!(icono("Imágenes", Tipo::Carpeta, EstiloIconos::Material), "material/folder-images");
    assert_eq!(icono("README", Tipo::Otro, EstiloIconos::Material), "material/readme");
    assert_eq!(icono("cosa.xyz", Tipo::Otro, EstiloIconos::Material), "material/file");
    assert_eq!(icono("arranque.sh", Tipo::Codigo, EstiloIconos::Lucide), "lucide/file-terminal");
    assert_eq!(icono("foto.png", Tipo::Imagen, EstiloIconos::Neon), "");
    let mut s = Estado::default();
    reducir(&mut s, atajo(Tecla::I, false, false, false, false).unwrap());
    assert_eq!(s.ajustes.iconos, EstiloIconos::Material);
    assert_eq!(Ajustes::de_json(s.ajustes.a_json().as_bytes()).iconos, EstiloIconos::Material);
}

#[test]
fn semaforo_de_carriles() {
    for p in PRESUPUESTOS { assert!(p.verde_us < p.amarillo_us, "{}", p.nombre); }
    assert_eq!(luz(Carril::Pintado, 3_000), Luz::Verde);
    assert_eq!(luz(Carril::Pintado, 9_000), Luz::Amarillo);
    assert_eq!(luz(Carril::Pintado, 50_000), Luz::Rojo);
    let mut s = Estado::default();
    let v0 = s.version;
    reducir(&mut s, atajo(Tecla::F12, false, false, false, false).unwrap());
    assert!(s.carriles && s.version > v0, "cada accion sube la version (cache de la vista)");
}
