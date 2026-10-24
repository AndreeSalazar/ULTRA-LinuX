// SPDX-License-Identifier: GPL-2.0-only
//! Pruebas del nucleo al estilo Redux: accion -> estado nuevo + efectos. Sin pantalla ni disco.

use carpetas_nucleo::*;

fn e(nombre: &str, tipo: Tipo, tam: u64) -> Entrada { Entrada { nombre: nombre.into(), tipo, tam } }

fn estado() -> Estado {
    let mut s = Estado { ancho: CELDA[0] * 3.0, ..Estado::default() };
    let entradas = vec![e("Juegos", Tipo::Carpeta, 0), e("kernel.rs", Tipo::Codigo, 900), e("foto.png", Tipo::Imagen, 5000), e("notas.txt", Tipo::Documento, 10)];
    let ef = reducir(&mut s, Accion::Cargar { nombre: "prueba".into(), entradas, disposicion: Disposicion::default() });
    assert!(matches!(ef.as_slice(), [Efecto::Guardar(_), Efecto::Reindexar { .. }]), "lo nuevo se coloca y se guarda; y se indexa");
    s
}

fn pos(s: &Estado, n: &str) -> [f32; 2] { s.disp.posiciones[n] }

#[test]
fn cargar_coloca_por_filas() {
    let s = estado();
    assert_eq!(pos(&s, "Juegos"), [0.0, 0.0]);
    assert_eq!(pos(&s, "foto.png"), [2.0 * CELDA[0], 0.0]);
    assert_eq!(pos(&s, "notas.txt"), [0.0, CELDA[1]]);
}

#[test]
fn arrastrar_soltar_guarda_y_deshacer() {
    let mut s = estado();
    let antes = s.disp.posiciones.clone();
    assert!(reducir(&mut s, Accion::EmpezarArrastre("kernel.rs".into())).is_empty());
    reducir(&mut s, Accion::Arrastrar([10.0, 300.0]));
    reducir(&mut s, Accion::Arrastrar([5.0, 7.0]));
    let ef = reducir(&mut s, Accion::Soltar);
    assert_eq!(pos(&s, "kernel.rs"), [CELDA[0] + 15.0, 307.0], "sin rejilla queda donde lo sueltas");
    let Efecto::Guardar(json) = &ef[0] else { panic!("soltar guarda") };
    assert_eq!(Disposicion::de_json(json.as_bytes()).posiciones, s.disp.posiciones, "lo guardado se vuelve a leer igual");
    reducir(&mut s, Accion::Comando(Comando::Deshacer));
    assert_eq!(s.disp.posiciones, antes);
    assert!(reducir(&mut s, Accion::Comando(Comando::Deshacer)).is_empty(), "nada mas que deshacer");
}

#[test]
fn rejilla_encaja_y_candado_bloquea() {
    let mut s = estado();
    reducir(&mut s, Accion::Comando(Comando::Rejilla));
    reducir(&mut s, Accion::EmpezarArrastre("notas.txt".into()));
    reducir(&mut s, Accion::Arrastrar([150.0, 160.0]));
    reducir(&mut s, Accion::Soltar);
    assert_eq!(pos(&s, "notas.txt"), [CELDA[0], 3.0 * CELDA[1]]);
    reducir(&mut s, Accion::Comando(Comando::Candado));
    let antes = s.disp.posiciones.clone();
    reducir(&mut s, Accion::EmpezarArrastre("notas.txt".into()));
    reducir(&mut s, Accion::Arrastrar([500.0, 500.0]));
    assert!(reducir(&mut s, Accion::Soltar).is_empty());
    assert!(reducir(&mut s, Accion::Comando(Comando::Ordenar(Orden::Tamano))).is_empty());
    assert_eq!(s.disp.posiciones, antes, "con candado nada se mueve");
}

#[test]
fn seleccion_con_clic_ctrl_y_banda() {
    let mut s = estado();
    reducir(&mut s, Accion::Pulsar { nombre: "foto.png".into(), sumar: false });
    reducir(&mut s, Accion::Pulsar { nombre: "Juegos".into(), sumar: true });
    assert_eq!(s.seleccion.len(), 2);
    reducir(&mut s, Accion::Pulsar { nombre: "Juegos".into(), sumar: true });
    assert_eq!(s.seleccion.iter().collect::<Vec<_>>(), ["foto.png"]);
    reducir(&mut s, Accion::Pulsar { nombre: "inventado".into(), sumar: false });
    assert_eq!(s.seleccion.len(), 1, "un nombre que no existe no entra");
    // Banda que cubre la primera fila.
    reducir(&mut s, Accion::EmpezarBanda { punto: [-5.0, -5.0], sumar: false });
    reducir(&mut s, Accion::MoverBanda([3.0 * CELDA[0], 20.0]));
    assert_eq!(s.seleccion.len(), 3);
    assert!(matches!(vista(&s).nodos.last(), Some(Nodo::Banda { .. })));
    reducir(&mut s, Accion::TerminarBanda);
    assert!(s.banda.is_none());
    // Arrastrar uno de los seleccionados mueve el grupo.
    reducir(&mut s, Accion::EmpezarArrastre("Juegos".into()));
    reducir(&mut s, Accion::Arrastrar([0.0, 400.0]));
    reducir(&mut s, Accion::Soltar);
    assert_eq!(pos(&s, "foto.png")[1], 400.0);
    assert_eq!(pos(&s, "kernel.rs")[1], 400.0);
}

#[test]
fn abrir_pide_efectos_segun_tipo() {
    let mut s = estado();
    assert_eq!(reducir(&mut s, Accion::Abrir("Juegos".into())), vec![Efecto::EntrarCarpeta("Juegos".into())]);
    assert_eq!(reducir(&mut s, Accion::Abrir("foto.png".into())), vec![Efecto::AbrirArchivo("foto.png".into())]);
    assert!(reducir(&mut s, Accion::Abrir("../../etc/passwd".into())).is_empty(), "solo lo que esta en la carpeta");
    reducir(&mut s, Accion::Pulsar { nombre: "notas.txt".into(), sumar: false });
    assert_eq!(reducir(&mut s, Accion::Comando(Comando::AbrirSeleccion)), vec![Efecto::AbrirArchivo("notas.txt".into())]);
    assert_eq!(reducir(&mut s, Accion::Comando(Comando::Ocultos)), vec![Efecto::Releer { ocultos: true }, Efecto::Reindexar { ocultos: true }]);
}

#[test]
fn refrescar_olvida_lo_borrado_sin_mover_lo_demas() {
    let mut s = estado();
    let juegos = pos(&s, "Juegos");
    reducir(&mut s, Accion::Pulsar { nombre: "foto.png".into(), sumar: false });
    reducir(&mut s, Accion::Refrescar(vec![e("Juegos", Tipo::Carpeta, 0), e("nuevo.mp4", Tipo::Video, 1)]));
    assert_eq!(pos(&s, "Juegos"), juegos);
    assert!(!s.disp.posiciones.contains_key("foto.png") && s.seleccion.is_empty());
    assert_eq!(pos(&s, "nuevo.mp4"), [CELDA[0], 0.0], "lo nuevo va a la primera celda libre");
}

#[test]
fn paleta_de_comandos_como_vscode() {
    let mut s = estado();
    let ef = reducir(&mut s, atajo(Tecla::P, true, true, false, false).unwrap());
    assert!(ef.is_empty() && s.paleta.is_some());
    reducir(&mut s, Accion::Paleta(AccionPaleta::Escribir("rej".into())));
    let v = vista(&s).paleta.unwrap();
    assert_eq!(v.filas[0], ("Alternar rejilla", "G", true));
    reducir(&mut s, atajo(Tecla::Enter, false, false, false, true).unwrap());
    assert!(s.disp.rejilla && s.paleta.is_none(), "Enter ejecuta el comando y cierra");
    // Busqueda difusa por letras en orden y por id.
    assert_eq!(buscar("ordtam")[0].id, "carpetas.ordenarTamano");
    assert_eq!(buscar("carpetas.candado")[0].titulo, "Poner o quitar candado");
    assert!(buscar("zzzz").is_empty());
    assert_eq!(buscar("").len(), COMANDOS.len());
    // Flechas dan la vuelta; Esc cierra sin hacer nada.
    reducir(&mut s, Accion::Paleta(AccionPaleta::Abrir));
    reducir(&mut s, Accion::Paleta(AccionPaleta::Mover(-1)));
    assert_eq!(s.paleta.as_ref().unwrap().elegido, COMANDOS.len() - 1);
    reducir(&mut s, atajo(Tecla::Escape, false, false, false, true).unwrap());
    assert!(s.paleta.is_none() && s.disp.rejilla);
    // Atajos fuera de la paleta.
    assert_eq!(atajo(Tecla::G, false, false, false, false), Some(Accion::Comando(Comando::Rejilla)));
    assert_eq!(atajo(Tecla::Z, true, false, false, false), Some(Accion::Comando(Comando::Deshacer)));
    assert_eq!(atajo(Tecla::G, false, false, false, true), None, "con la paleta abierta las letras se escriben");
}

#[test]
fn json_raro_no_rompe_nada() {
    assert_eq!(Disposicion::de_json(b"{no es json"), Disposicion::default());
    let d = Disposicion::de_json(br#"{"posiciones":{"a":[1e30,2],"b":[-5,1],"c":[10,20]},"rejilla":true}"#);
    assert_eq!(d.posiciones.keys().collect::<Vec<_>>(), ["c"]);
    assert!(d.rejilla);
    assert_eq!(Disposicion::de_json(&vec![b' '; (1 << 20) + 1]), Disposicion::default());
    let mut s = estado();
    reducir(&mut s, Accion::EmpezarArrastre("Juegos".into()));
    reducir(&mut s, Accion::Arrastrar([f32::NAN, f32::INFINITY]));
    reducir(&mut s, Accion::Arrastrar([-1.0e9, 1.0e9]));
    reducir(&mut s, Accion::Soltar);
    assert_eq!(pos(&s, "Juegos"), [0.0, 1.0e6], "NaN no entra y todo queda en limites");
    reducir(&mut s, Accion::Ancho(f32::NAN));
    assert_eq!(s.ancho, CELDA[0] * 3.0);
}

#[test]
fn vista_describe_lo_que_hay_que_dibujar() {
    let mut s = estado();
    reducir(&mut s, Accion::Pulsar { nombre: "kernel.rs".into(), sumar: false });
    let v = vista(&s);
    let iconos: Vec<_> = v.nodos.iter().filter_map(|n| if let Nodo::Icono { nombre, etiqueta, seleccionado, color, .. } = n { Some((nombre.as_str(), etiqueta.as_str(), *seleccionado, *color)) } else { None }).collect();
    assert_eq!(iconos.len(), 4);
    assert!(iconos.contains(&("kernel.rs", "RS", true, tipos_cyan())));
    assert!(iconos.contains(&("Juegos", "", false, s.acento)));
    assert!(v.estado.starts_with("4 elementos · 1 seleccionados"));
    assert_eq!(v.tam, [3.0 * CELDA[0], 2.0 * CELDA[1]]);
}

fn tipos_cyan() -> Rgb { Tipo::Codigo.color([0, 0, 0]) }
