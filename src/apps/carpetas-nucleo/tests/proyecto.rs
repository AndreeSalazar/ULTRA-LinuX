// SPDX-License-Identifier: GPL-2.0-only
//! Todo el proyecto: Ctrl+P, buscar texto, enfocar carpetas, revelar y grupos en el grafo.
use carpetas_nucleo::grafo::{migas, visibles, Cosa};
use carpetas_nucleo::organizar::es_grupo;
use carpetas_nucleo::proyecto::*;
use carpetas_nucleo::*;

fn c(n: &str) -> Entrada { Entrada { nombre: n.into(), tipo: Tipo::Carpeta, tam: 0 } }
fn a(n: &str) -> Entrada { Entrada { nombre: n.into(), tipo: tipo_por_extension(n), tam: 100 } }

/// Un mini Chromium: base/ con muchos archivos (se organiza) y net/http/ dentro.
fn chromium() -> Estado {
    let mut s = Estado::default();
    reducir(&mut s, Accion::Cargar { nombre: "chromium".into(), entradas: vec![c("base"), c("net"), a("BUILD.gn"), a("DEPS")], disposicion: Disposicion::default() });
    reducir(&mut s, Accion::Comando(Comando::Modo));
    s
}

fn base() -> Vec<Entrada> {
    let mut v = vec![c("files"), a("BUILD.gn"), a("OWNERS"), a("DEPS")];
    for u in ["at_exit", "base64", "bits", "callback_list", "check", "location", "logging", "memory_pressure", "path_service", "rand_util"] {
        for suf in [".h", ".cc", "_unittest.cc"] { v.push(a(&format!("{u}{suf}"))); }
    }
    v.push(a("base_paths_win.cc"));
    v.push(a("base_paths.h"));
    v
}

fn llegar(s: &mut Estado, ef: Vec<Efecto>, disco: &dyn Fn(&str) -> Vec<Entrada>) {
    let mut cola = ef;
    for _ in 0..20 {
        let mut siguiente = Vec::new();
        for e in cola {
            if let Efecto::LeerCarpeta(r) = e { siguiente.extend(reducir(s, Accion::Contenido { ruta: r.clone(), entradas: disco(&r) })); }
        }
        if siguiente.is_empty() { return; }
        cola = siguiente;
    }
}

fn disco(r: &str) -> Vec<Entrada> {
    match r {
        "base" => base(),
        "base/files" => vec![a("file_path.h"), a("file_path.cc")],
        "net" => vec![c("http")],
        "net/http" => vec![a("http_cache.h"), a("http_cache.cc")],
        _ => vec![],
    }
}

#[test]
fn indice_compacto_y_ctrl_p() {
    let mut i = Indice::default();
    for (r, carpeta) in [("base", true), ("base/files/file_path.cc", false), ("base/files/file_path.h", false), ("net/http/http_cache.cc", false),
                         ("third_party/blink/renderer/core/dom/element.cc", false), ("chrome/browser/ui/views/frame/browser_view.cc", false),
                         ("base/file_path_watcher.cc", false)] {
        assert!(i.agregar(r, carpeta));
    }
    assert_eq!(i.len(), 7);
    assert_eq!(i.ruta(4), "third_party/blink/renderer/core/dom/element.cc");
    let nombres = |q: &str| buscar_rutas(&i, q, 5).into_iter().map(|k| i.ruta(k).rsplit('/').next().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(nombres("file_path.cc")[0], "file_path.cc", "exacto primero");
    assert_eq!(nombres("fpw")[0], "file_path_watcher.cc", "iniciales de palabras");
    assert_eq!(nombres("bv")[0], "browser_view.cc");
    assert_eq!(nombres("ELEMENT")[0], "element.cc", "sin mayusculas");
    assert_eq!(nombres("dom/elem")[0], "element.cc", "con barra mira la ruta");
    assert!(nombres("zzz").is_empty());
    assert_eq!(letras_marcadas("fpw", "file_path_watcher.cc"), [0, 5, 10]);
}

#[test]
fn filtro_y_coincidencias_con_lineas() {
    assert!(pasa_filtro("", "a/b.cc"));
    assert!(pasa_filtro("*.cc, *.h", "base/x.h") && !pasa_filtro("*.cc,*.h", "base/x.py"));
    assert!(pasa_filtro("base/", "base/x.cc") && pasa_filtro("files/", "base/files/x.cc") && !pasa_filtro("base/", "net/base.cc"));
    assert!(!pasa_filtro("!third_party/", "third_party/x.cc") && pasa_filtro("!third_party/", "base/x.cc"));
    assert!(!pasa_filtro("*.cc, !test", "base/x_test.cc"));
    let t = "uno\n  dos vga\ntres\nvga vga ñ\n";
    let h = carpetas_nucleo::editor::buscar(t, "vga", true, false, 100);
    let v = coincidencias(t, &h);
    assert_eq!(v.iter().map(|c| (c.linea, c.columna)).collect::<Vec<_>>(), [(2, 7), (4, 1), (4, 5)]);
    assert_eq!(v[0].vista, "dos vga", "sin la sangria");
    assert_eq!(&v[0].vista[v[0].en_vista.0..v[0].en_vista.1], "vga");
    assert_eq!(&t[v[2].rango.0..v[2].rango.1], "vga");
}

#[test]
fn grupos_en_el_grafo_y_el_explorador() {
    let mut s = chromium();
    let ef = reducir(&mut s, Accion::Expandir("base".into()));
    llegar(&mut s, ef, &disco);
    let v = visibles(&s);
    // base: files (carpeta) + 10 unidades + base_paths + build y metadatos = 13 en vez de 37.
    let hijos: Vec<_> = v.iter().filter(|x| x.padre.as_deref() == Some("base")).collect();
    assert_eq!(hijos.len(), 13, "{:?}", hijos.iter().map(|x| &x.id).collect::<Vec<_>>());
    let unidad = hijos.iter().find(|x| x.id.ends_with("u:at_exit")).unwrap();
    assert!(es_grupo(&unidad.id) && matches!(unidad.cosa, Cosa::Grupo(_)));
    let unidad_id = unidad.id.clone();
    // Abrir el grupo muestra sus 3 piezas colgando de el; doble clic tambien.
    reducir(&mut s, Accion::Abrir(unidad_id.clone()));
    let v = visibles(&s);
    let piezas: Vec<&str> = v.iter().filter(|x| x.padre.as_deref() == Some(unidad_id.as_str())).map(|x| x.id.as_str()).collect();
    assert_eq!(piezas, ["base/at_exit.h", "base/at_exit.cc", "base/at_exit_unittest.cc"]);
    // Grafo y explorador: el nodo del grupo lleva su resumen y su icono de C/C++.
    let vista_ = vista(&s);
    let (nodos, cables) = vista_.grafo.unwrap();
    let n = nodos.iter().find(|n| n.id == unidad_id).unwrap();
    assert_eq!(n.grupo.as_deref(), Some("h · cc · unittest"));
    assert_eq!(n.nombre, "at_exit");
    assert!(n.expandida == Some(true) && n.play.is_none());
    assert_eq!(cables.len(), nodos.len() - 1, "un cable por nodo menos la raiz");
    let fila = vista_.arbol.unwrap().into_iter().find(|f| f.id == unidad_id).unwrap();
    assert_eq!(fila.grupo.as_deref(), Some("h · cc · unittest"));
    // Seleccionar, arrastrar y clic derecho (editor en su pieza principal) funcionan en grupos.
    reducir(&mut s, Accion::Pulsar { nombre: unidad_id.clone(), sumar: false });
    assert!(s.seleccion.contains(&unidad_id));
    assert_eq!(reducir(&mut s, Accion::AbrirEditor { id: unidad_id.clone(), linea: None }), [Efecto::LeerArchivo("base/at_exit.h".into())]);
    // El disco nunca recibe un grupo para leer ni vigilar.
    assert!(s.carpetas_desplegadas().all(|id| !es_grupo(id)));
    // "Nunca" lo deja como en disco.
    reducir(&mut s, Accion::Comando(Comando::Organizar));
    reducir(&mut s, Accion::Comando(Comando::Organizar));
    assert_eq!(s.ajustes.organizar, organizar::Organizar::Nunca);
    assert_eq!(visibles(&s).iter().filter(|x| x.padre.as_deref() == Some("base")).count(), base().len());
}

#[test]
fn enfocar_una_carpeta_y_volver() {
    let mut s = chromium();
    let ef = reducir(&mut s, Accion::Expandir("net".into()));
    llegar(&mut s, ef, &disco);
    let ef = reducir(&mut s, Accion::Expandir("net/http".into()));
    llegar(&mut s, ef, &disco);
    reducir(&mut s, Accion::Pulsar { nombre: "net/http".into(), sumar: false });
    let ef = reducir(&mut s, atajo(Tecla::E, false, false, false, false).unwrap());
    assert!(ef.iter().any(|e| matches!(e, Efecto::Guardar(_))), "el foco se guarda con la carpeta");
    assert_eq!(s.disp.foco, "net/http");
    let v = visibles(&s);
    assert_eq!(v[0].id, "net/http");
    assert!(matches!(v[0].cosa, Cosa::Raiz));
    assert_eq!(v.len(), 3, "solo net/http y sus 2 archivos: nada de la raiz");
    assert_eq!(migas(&s), [("".to_string(), "chromium".to_string()), ("net".into(), "net".into()), ("net/http".into(), "http".into())]);
    let (nodos, _) = vista(&s).grafo.unwrap();
    assert!(nodos[0].raiz && nodos[0].nombre == "http");
    assert_eq!(vista(&s).arbol.unwrap().len(), 2, "el explorador tambien empieza en el foco");
    // U sube un nivel; Enfocar("") vuelve a la raiz.
    reducir(&mut s, atajo(Tecla::U, false, false, false, false).unwrap());
    assert_eq!(s.disp.foco, "net");
    reducir(&mut s, Accion::Enfocar(String::new()));
    assert_eq!(visibles(&s)[0].id, "");
    // Un foco guardado se recupera al abrir: se piden sus carpetas y se aplica al llegar.
    let mut d = Disposicion::default();
    d.foco = "net/http".into();
    let json = d.a_json();
    let mut t = Estado::default();
    let ef = reducir(&mut t, Accion::Cargar { nombre: "chromium".into(), entradas: vec![c("base"), c("net")], disposicion: Disposicion::de_json(json.as_bytes()) });
    assert!(ef.contains(&Efecto::LeerCarpeta("net".into())));
    llegar(&mut t, ef, &disco);
    assert_eq!(visibles(&t)[0].id, "net/http");
    // Focos raros guardados se descartan.
    for raro in ["../etc", "/abs", "a//b", "a/./b"] {
        let mut d = Disposicion::default();
        d.foco = raro.into();
        assert_eq!(Disposicion::de_json(d.a_json().as_bytes()).foco, "", "{raro}");
    }
}

#[test]
fn ctrl_p_revela_en_las_dos_vistas() {
    let mut s = chromium();
    assert!(reducir(&mut s, Accion::Cargar { nombre: "chromium".into(), entradas: vec![c("base"), c("net")], disposicion: Disposicion::default() })
        .contains(&Efecto::Reindexar { ocultos: false }), "abrir una carpeta la indexa");
    reducir(&mut s, atajo(Tecla::P, true, false, false, false).unwrap());
    assert!(s.ir_a.is_some());
    let ef = reducir(&mut s, Accion::IrA(AccionIrA::Texto("atexit".into())));
    let gen = s.ir_a.as_ref().unwrap().generacion;
    assert_eq!(ef, [Efecto::BuscarArchivos { generacion: gen, consulta: "atexit".into() }]);
    // Una respuesta vieja no pisa la nueva.
    reducir(&mut s, Accion::ResultadosIrA { generacion: gen - 1, resultados: vec![("viejo".into(), false)] });
    assert!(s.ir_a.as_ref().unwrap().resultados.is_empty());
    reducir(&mut s, Accion::ResultadosIrA { generacion: gen, resultados: vec![("base/at_exit_unittest.cc".into(), false)] });
    // Enter: despliega base, abre el grupo at_exit, selecciona y centra.
    let ef = reducir(&mut s, Accion::IrA(AccionIrA::Aceptar { editor: true }));
    assert_eq!(ef, [Efecto::LeerCarpeta("base".into())]);
    llegar(&mut s, ef, &disco);
    assert!(s.ir_a.is_none());
    assert!(s.expandidas.contains("base") && s.expandidas.iter().any(|x| x.ends_with("u:at_exit")));
    assert_eq!(s.seleccion.iter().next().unwrap(), "base/at_exit_unittest.cc");
    assert_eq!(s.revelado.0, "base/at_exit_unittest.cc");
    assert_eq!(s.editor.activa().unwrap().id, "base/at_exit_unittest.cc", "Ctrl+Enter tambien lo abre en el editor");
    assert!(visibles(&s).iter().any(|v| v.id == "base/at_exit_unittest.cc"));
}

#[test]
fn buscar_en_el_proyecto() {
    let mut s = chromium();
    reducir(&mut s, Accion::Comando(Comando::BuscarProyecto));
    assert_eq!(s.lateral, carpetas_nucleo::estado::Lateral::Buscar);
    assert!(reducir(&mut s, Accion::Proyecto(AccionProyecto::Patron("H".into()))).is_empty(), "una letra no lanza");
    let ef = reducir(&mut s, Accion::Proyecto(AccionProyecto::Patron("HttpCache".into())));
    let g = s.proyecto.generacion;
    assert!(matches!(&ef[..], [Efecto::BuscarTexto { patron, carpeta, .. }] if patron == "HttpCache" && carpeta.is_empty()));
    let antes = s.version_contenido;
    let co = |l| Coincidencia { linea: l, columna: 3, vista: "x".into(), en_vista: (0, 1), rango: (10, 19) };
    reducir(&mut s, Accion::ResultadosProyecto { generacion: g, lote: vec![("net/http/http_cache.cc".into(), vec![co(4), co(9)])], vistos: 10, terminada: false, truncada: false });
    reducir(&mut s, Accion::ResultadosProyecto { generacion: g - 1, lote: vec![("viejo.cc".into(), vec![co(1)])], vistos: 1, terminada: true, truncada: false });
    assert_eq!((s.proyecto.total, s.proyecto.buscando), (2, true));
    assert!(s.version_contenido > antes, "las insignias de los nodos cambian");
    assert_eq!(s.proyecto.conteo("net"), 2, "la carpeta cuenta lo de dentro");
    assert_eq!(s.proyecto.conteo("ne"), 0);
    // Clic en un resultado: se revela en grafo y explorador y se abre el editor con la coincidencia seleccionada.
    let ef = reducir(&mut s, Accion::Proyecto(AccionProyecto::Ir { id: "net/http/http_cache.cc".into(), i: 1 }));
    llegar(&mut s, ef, &disco);
    let ed = s.editor.lista.iter().find(|e| e.id == "net/http/http_cache.cc").unwrap();
    assert_eq!((ed.ir_a, ed.rango), (Some(9), Some((10, 19))));
    assert_eq!(s.seleccion.iter().next().unwrap(), "net/http/http_cache.cc");
    // Con foco, la busqueda es dentro de la carpeta enfocada.
    reducir(&mut s, Accion::Enfocar("net".into()));
    let ef = reducir(&mut s, Accion::Proyecto(AccionProyecto::Lanzar));
    assert!(matches!(&ef[..], [Efecto::BuscarTexto { carpeta, .. }] if carpeta == "net"));
    assert_eq!(reducir(&mut s, Accion::Proyecto(AccionProyecto::Limpiar)), [Efecto::CancelarTexto]);
}

#[test]
fn muchos_hijos_cerrados_van_en_rejilla_sin_pisarse() {
    let mut s = Estado::default();
    let mut raiz: Vec<Entrada> = (0..52).map(|i| c(&format!("dir{i:02}"))).collect();
    raiz.extend((0..10).map(|i| a(&format!("f{i}.txt"))));
    reducir(&mut s, Accion::Cargar { nombre: "grande".into(), entradas: raiz, disposicion: Disposicion::default() });
    reducir(&mut s, Accion::Comando(Comando::Modo));
    let ef = reducir(&mut s, Accion::Expandir("dir05".into()));
    llegar(&mut s, ef, &|_| vec![a("x.rs"), a("y.rs")]);
    let p = carpetas_nucleo::grafo::acomodar(&s);
    // 61 cerrados -> 4 columnas de 16 como mucho; la abierta (dir05) ramifica a su derecha.
    let xs: std::collections::BTreeSet<i32> = p.iter().filter(|(id, _)| !id.is_empty() && !id.contains('/')).map(|(_, q)| q[0] as i32).collect();
    assert!(xs.len() >= 4 && xs.len() <= 5, "{xs:?}");
    let alto = p.values().map(|q| q[1]).fold(f32::MIN, f32::max) - p.values().map(|q| q[1]).fold(f32::MAX, f32::min);
    assert!(alto < 20.0 * (NODO[1] + 22.0), "alto {alto}: nada de una columna de 62");
    // Ningun par de nodos se pisa.
    let v: Vec<[f32; 2]> = p.values().copied().collect();
    for i in 0..v.len() { for j in i + 1..v.len() {
        let solapan = (v[i][0] - v[j][0]).abs() < NODO[0] && (v[i][1] - v[j][1]).abs() < NODO[1];
        assert!(!solapan, "{:?} y {:?}", v[i], v[j]);
    } }
    assert!(p["dir05/x.rs"][0] > p["dir05"][0], "los hijos de la abierta van a su derecha");
}

#[test]
fn chromium_file_path_watcher_es_una_unidad() {
    use carpetas_nucleo::organizar::raiz_unidad;
    for n in ["file_path_watcher_inotify.cc", "file_path_watcher_kqueue.h", "file_path_watcher_fsevents.cc", "file_path_watcher_mac.cc"] {
        assert_eq!(raiz_unidad(n), "file_path_watcher", "{n}");
    }
}

#[test]
fn enter_antes_de_los_resultados_espera() {
    let mut s = chromium();
    reducir(&mut s, Accion::IrA(AccionIrA::Abrir));
    reducir(&mut s, Accion::IrA(AccionIrA::Texto("http_cache".into())));
    assert!(reducir(&mut s, Accion::IrA(AccionIrA::Aceptar { editor: false })).is_empty());
    assert!(s.ir_a.is_some(), "no se cierra vacio");
    let g = s.ir_a.as_ref().unwrap().generacion;
    let ef = reducir(&mut s, Accion::ResultadosIrA { generacion: g, resultados: vec![("net/http/http_cache.cc".into(), false)] });
    assert_eq!(ef, [Efecto::LeerCarpeta("net".into())], "al llegar se acepta el primero");
    llegar(&mut s, ef, &disco);
    assert_eq!(s.seleccion.iter().next().unwrap(), "net/http/http_cache.cc");
}

#[test]
fn ctrl_enter_enfoca_una_carpeta_ya_leida_y_rehace_la_vista() {
    let mut s = chromium();
    let ef = reducir(&mut s, Accion::Expandir("net".into()));
    llegar(&mut s, ef, &disco);
    reducir(&mut s, Accion::IrA(AccionIrA::Abrir));
    reducir(&mut s, Accion::IrA(AccionIrA::Texto("net".into())));
    let g = s.ir_a.as_ref().unwrap().generacion;
    reducir(&mut s, Accion::ResultadosIrA { generacion: g, resultados: vec![("net".into(), true)] });
    let antes = s.version_contenido;
    reducir(&mut s, Accion::IrA(AccionIrA::Aceptar { editor: true }));
    assert_eq!(s.disp.foco, "net");
    assert!(s.version_contenido > antes, "la plataforma rehace la vista (antes quedaba la vieja en cache)");
    assert_eq!(visibles(&s)[0].id, "net");
}
