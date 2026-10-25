// SPDX-License-Identifier: GPL-2.0-only
//! La ciudad: areas proporcionales, nada se pisa, porcentajes y bloques.
use carpetas_nucleo::ciudad::*;

fn a(r: &str, tam: u64, lineas: u32) -> ArchivoCiudad { ArchivoCiudad { ruta: r.into(), tam, lineas: Some(lineas), archivos: 1, bloque: false } }
fn area(r: &[f32; 4]) -> f64 { r[2] as f64 * r[3] as f64 }
fn dentro(a: &[f32; 4], b: &[f32; 4]) -> bool { a[0] >= b[0] - 1e-2 && a[1] >= b[1] - 1e-2 && a[0] + a[2] <= b[0] + b[2] + 1e-2 && a[1] + a[3] <= b[1] + b[3] + 1e-2 }
fn pisan(a: &[f32; 4], b: &[f32; 4]) -> bool { a[0] + a[2] > b[0] + 1e-3 && b[0] + b[2] > a[0] + 1e-3 && a[1] + a[3] > b[1] + 1e-3 && b[1] + b[3] > a[1] + 1e-3 }

#[test]
fn squarify_reparte_proporcional_y_sin_pisar() {
    let pesos = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
    let r = squarify(&pesos, [0.0, 0.0, 6.0, 4.0]);
    let total: f64 = pesos.iter().sum();
    for (k, p) in pesos.iter().enumerate() {
        assert!((area(&r[k]) - p / total * 24.0).abs() < 1e-3, "{k}: {:?}", r[k]);
        assert!(dentro(&r[k], &[0.0, 0.0, 6.0, 4.0]));
        for j in k + 1..pesos.len() { assert!(!pisan(&r[k], &r[j]), "{k} y {j}"); }
    }
    // "Squarified": casi cuadrados (ningun rectangulo mas de 3 veces mas largo que ancho aqui).
    assert!(r.iter().all(|q| q[2].max(q[3]) / q[2].min(q[3]) < 3.0), "{r:?}");
    assert!((raiz2(2.0) - 1.41421).abs() < 1e-4 && raiz2(0.0) == 0.0 && (raiz2(1e6) - 1000.0).abs() < 0.1);
}

#[test]
fn distritos_edificios_y_porcentajes() {
    let archivos = vec![a("src/main.rs", 4000, 120), a("src/motor/motor.cpp", 12000, 400), a("src/motor/motor.hpp", 2000, 60),
                        a("docs/guia.md", 2000, 50), a("Cargo.toml", 500, 20)];
    let c = construir("", &archivos, Huella::Bytes, Altura::Lineas, &|_| 0);
    assert_eq!(c.edificios.len(), 5);
    assert_eq!((c.tam, c.archivos, c.lineas), (20500, 5, 650));
    let d = |r: &str| c.distritos.iter().find(|x| x.ruta == r).unwrap();
    // src = 18 000 de 20 500 bytes (con el minimo de 512 por archivo).
    assert!((d("src").porcentaje - 18000.0 / 20512.0 * 100.0).abs() < 0.5, "{}", d("src").porcentaje);
    assert!((d("").porcentaje - 100.0).abs() < 1e-3);
    assert_eq!(d("src/motor").nivel, 2);
    assert!(d("src/motor").base > d("src").base, "terrazas");
    // Cada edificio dentro de su distrito y ninguno pisa a otro.
    for e in &c.edificios {
        let padre = e.ruta.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        assert!(dentro(&e.rect, &d(padre).rect), "{} fuera de {padre}", e.ruta);
    }
    for i in 0..c.edificios.len() { for j in i + 1..c.edificios.len() { assert!(!pisan(&c.edificios[i].rect, &c.edificios[j].rect)); } }
    // Huella proporcional al tamano; altura por lineas (el que mas tiene, el mas alto).
    let e = |r: &str| c.edificios.iter().find(|x| x.ruta == r).unwrap();
    assert!(area(&e("src/motor/motor.cpp").rect) > area(&e("src/main.rs").rect) * 2.0);
    assert!(e("src/motor/motor.cpp").altura > e("src/main.rs").altura);
    assert!((e("src/motor/motor.cpp").del_distrito - 12000.0 / 14000.0 * 100.0).abs() < 0.5);
    // Altura por otra medida (errores): solo sube lo que la tiene.
    let c = construir("", &archivos, Huella::Bytes, Altura::Errores, &|r| if r == "src/main.rs" { 3 } else { 0 });
    let alto = |r: &str| c.edificios.iter().find(|x| x.ruta == r).unwrap().altura;
    assert!(alto("src/main.rs") > 10.0 * alto("Cargo.toml"));
    // Solo una carpeta (el foco): el resto no esta.
    let c = construir("src", &archivos, Huella::Bytes, Altura::Plana, &|_| 0);
    assert_eq!(c.edificios.len(), 3);
    assert_eq!(c.distritos[0].ruta, "src");
}

#[test]
fn proyecto_enorme_se_junta_en_bloques() {
    let mut v = Vec::new();
    for x in 0..30 { for y in 0..20 { for z in 0..10 { v.push(a(&format!("third_party/l{x}/d{y}/f{z}.cc"), 100, 5)); } } }
    v.push(a("README.md", 10, 1));
    let total: u64 = v.iter().map(|x| x.tam).sum();
    let j = juntar(v.clone(), "", 1000);
    assert!(j.len() <= 1000, "{}", j.len());
    assert_eq!(j.iter().map(|x| x.archivos as usize).sum::<usize>(), v.len(), "ningun archivo se pierde");
    assert_eq!(j.iter().map(|x| x.tam).sum::<u64>(), total);
    assert!(j.iter().any(|x| x.ruta == "README.md" && x.archivos == 1 && !x.bloque));
    assert!(j.iter().any(|x| x.bloque && x.archivos == 10 && x.ruta.starts_with("third_party/l3/d4")), "las carpetas hondas son bloques");
    let c = construir("", &j, Huella::Bytes, Altura::Lineas, &|_| 0);
    assert!(c.con_bloques && c.archivos as usize == v.len());
    assert_eq!(juntar(v.clone(), "", 100_000).len(), v.len(), "si caben, no se toca");
}

#[test]
fn la_ciudad_pide_sus_datos_cuando_toca() {
    use carpetas_nucleo::*;
    use carpetas_nucleo::estado::InfoIndice;
    let mut s = Estado::default();
    let c = |n: &str| Entrada { nombre: n.into(), tipo: Tipo::Carpeta, tam: 0 };
    reducir(&mut s, Accion::Cargar { nombre: "p".into(), entradas: vec![c("src"), c("docs")], disposicion: Disposicion::default() });
    // Tab: iconos -> grafo -> ciudad -> iconos.
    reducir(&mut s, Accion::Comando(Comando::Modo));
    assert_eq!(s.ajustes.modo, Modo::Grafo);
    let ef = reducir(&mut s, Accion::Comando(Comando::Modo));
    assert_eq!(s.ajustes.modo, Modo::Ciudad);
    assert!(!ef.iter().any(|e| matches!(e, Efecto::PedirCiudad { .. })), "sin indice aun no se pide");
    let ef = reducir(&mut s, Accion::Indice(InfoIndice { archivos: 3, listo: true, ..InfoIndice::default() }));
    let g = s.ciudad.generacion;
    assert_eq!(ef, [Efecto::PedirCiudad { generacion: g, raiz: String::new() }]);
    assert!(reducir(&mut s, Accion::Comando(Comando::AlturaCiudad)).iter().all(|e| !matches!(e, Efecto::PedirCiudad { .. })), "cargando: no se repite");
    reducir(&mut s, Accion::DatosCiudad { generacion: g - 1, raiz: "viejo".into(), archivos: vec![] });
    assert!(s.ciudad.cargando, "una respuesta vieja no cuenta");
    reducir(&mut s, Accion::DatosCiudad { generacion: g, raiz: String::new(), archivos: vec![a("src/main.rs", 10, 1)] });
    assert_eq!((s.ciudad.archivos.len(), s.ciudad.cargando, s.ciudad.version), (1, false, 1));
    assert!(reducir(&mut s, Accion::Comando(Comando::ColorCiudad)).iter().all(|e| !matches!(e, Efecto::PedirCiudad { .. })), "color: no hace falta pedir");
    // Enfocar src (aun sin leer: doble clic en un distrito): se lee y luego se pide la ciudad de src.
    let ef = reducir(&mut s, Accion::Enfocar("src".into()));
    assert!(ef.contains(&Efecto::LeerCarpeta("src".into())));
    let ef = reducir(&mut s, Accion::Contenido { ruta: "src".into(), entradas: vec![] });
    assert!(ef.iter().any(|e| matches!(e, Efecto::PedirCiudad { raiz, .. } if raiz == "src")), "{ef:?}");
    // Al contar las lineas, se vuelve a pedir (las alturas cambian).
    let g = s.ciudad.generacion;
    reducir(&mut s, Accion::DatosCiudad { generacion: g, raiz: "src".into(), archivos: vec![] });
    assert!(reducir(&mut s, Accion::LineasListas { contadas: 99 }).iter().any(|e| matches!(e, Efecto::PedirCiudad { .. })));
    reducir(&mut s, Accion::Comando(Comando::Modo));
    assert_eq!(s.ajustes.modo, Modo::Iconos);
}

#[test]
fn colores_de_lenguaje_distintos_para_c_y_cpp() {
    let c = color_lenguaje("vga.c", false);
    let h = color_lenguaje("vga.h", false);
    let cc = color_lenguaje("motor.cc", false);
    let hpp = color_lenguaje("motor.hpp", false);
    let rs = color_lenguaje("main.rs", false);
    let todos = [c, h, cc, hpp, rs];
    for i in 0..5 { for j in i + 1..5 { assert_ne!(todos[i], todos[j]); } }
    assert_eq!(color_lenguaje("x.cpp", false), cc);
    assert_ne!(color_lenguaje("x", true), color_lenguaje("x", false), "un bloque se distingue");
}

#[test]
fn imagenes_bajitas_y_huella_por_lineas() {
    let foto = ArchivoCiudad { ruta: "docs/captura.png".into(), tam: 1_200_000, lineas: None, archivos: 1, bloque: false };
    let codigo = a("src/motor.cpp", 30_000, 900);
    let sin_contar = ArchivoCiudad { ruta: "src/nuevo.rs".into(), tam: 4000, lineas: None, archivos: 1, bloque: false };
    assert_eq!(lineas_de(&foto), (0, false), "una imagen no tiene lineas");
    assert_eq!(lineas_de(&sin_contar), (100, true), "texto aun sin contar: estimado y marcado");
    let v = vec![foto.clone(), codigo.clone(), sin_contar];
    let c = construir("", &v, Huella::Bytes, Altura::Lineas, &|_| 0);
    let e = |r: &str| c.edificios.iter().find(|x| x.ruta == r).unwrap().clone();
    assert!(e("docs/captura.png").altura < 2.0, "la foto pesa mucho pero es bajita");
    assert!(e("src/motor.cpp").altura > 50.0);
    assert!(e("docs/captura.png").del_total > 90.0, "en bytes, la foto ocupa casi todo");
    let c = construir("", &v, Huella::Lineas, Altura::Lineas, &|_| 0);
    let e = |r: &str| c.edificios.iter().find(|x| x.ruta == r).unwrap().clone();
    assert!(e("docs/captura.png").del_total < 1.0 && e("src/motor.cpp").del_total > 85.0, "en lineas, manda el codigo");
}
