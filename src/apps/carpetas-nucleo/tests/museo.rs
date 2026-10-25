// SPDX-License-Identifier: GPL-2.0-only
//! Cada ficha con detector tiene su caso aislado: `mal` debe saltar con esa ficha y
//! `bien` (el control valido del museo) no debe saltar con ninguna.

use carpetas_nucleo::fichas::FICHAS;
use carpetas_nucleo::museo::{analizar, ficha, Fuerza};
use std::path::Path;

fn casos() -> Vec<(String, String, String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/datos/museo");
    let mut v = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        if !e.file_type().unwrap().is_dir() { continue; }
        let id = e.file_name().to_string_lossy().to_string();
        let mut mal = None;
        let mut bien = None;
        for f in std::fs::read_dir(e.path()).unwrap().flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            let texto = std::fs::read_to_string(f.path()).unwrap();
            if n.starts_with("mal.") { mal = Some((n, texto)); } else if n.starts_with("bien.") { bien = Some((n, texto)); }
        }
        let (nm, tm) = mal.unwrap_or_else(|| panic!("{id}: falta mal.*"));
        let (nb, tb) = bien.unwrap_or_else(|| panic!("{id}: falta bien.*"));
        v.push((id.clone(), format!("{id}/{nm}"), tm, format!("{id}/{nb}:{tb}")));
    }
    v.sort();
    v
}

#[test]
fn cada_ficha_salta_en_su_caso_malo_y_calla_en_su_control() {
    let casos = casos();
    assert!(casos.len() >= 28, "{} casos", casos.len());
    let mut fallos = Vec::new();
    for (id, ruta_mal, mal, bien) in &casos {
        assert!(ficha(id).is_some(), "{id} no esta en el museo");
        let s = analizar(ruta_mal, mal);
        if !s.iter().any(|x| x.ficha == id) { fallos.push(format!("{ruta_mal}: no salto {id}; dio {:?}", s.iter().map(|x| x.ficha).collect::<Vec<_>>())); }
        let (ruta_bien, texto) = bien.split_once(':').unwrap();
        let s = analizar(ruta_bien, texto);
        if !s.is_empty() { fallos.push(format!("{ruta_bien}: el control no debe saltar y dio {:?}", s.iter().map(|x| (x.ficha, x.linea, x.visto.as_str())).collect::<Vec<_>>())); }
    }
    assert!(fallos.is_empty(), "\n{}", fallos.join("\n"));
}

#[test]
fn cada_sospecha_dice_donde_que_y_por_que() {
    let s = analizar("x.c", "void f(char *p) {\n    free(p + 1);\n}\n");
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].ficha, s[0].linea, s[0].columna, s[0].fuerza), ("C-001", 2, 5, Fuerza::Exacto));
    assert_eq!(s[0].visto, "free(p + 1)");
    assert!(s[0].motivo.contains("free solo acepta"));
    let f = ficha("C-001").unwrap();
    assert_eq!(f.titulo, "Liberar un puntero interior");
    assert!(f.pagina.starts_with("Museo/README.md#"));
}

#[test]
fn las_fichas_son_las_del_museo() {
    let ub = include_str!("../../../../docs/museo-ub.json");
    let nuevo = include_str!("../../../../Museo/catalogo.json");
    let triangulo = include_str!("../../../../Museo/triangulo.json");
    assert_eq!(FICHAS.len(), 43);
    for f in FICHAS {
        let id = format!("\"id\": \"{}\"", f.id);
        let id2 = format!("\"id\":\"{}\"", f.id);
        assert!([ub, nuevo, triangulo].iter().any(|c| c.contains(&id) || c.contains(&id2) || c.contains(&format!("\"id\": \"{}\"", f.id))), "{} no esta en los catalogos", f.id);
        assert!([ub, nuevo, triangulo].iter().any(|c| c.contains(f.titulo)), "{}: titulo distinto", f.id);
    }
}

#[test]
fn comentarios_cadenas_y_otros_lenguajes_no_saltan() {
    assert!(analizar("a.c", "/* free(p + 1); */\nconst char *s = \"1 << 40\";\n// x = x++;\n").is_empty());
    assert!(analizar("a.rs", "free(p + 1)").is_empty(), "solo C, C++ y ASM");
    assert!(analizar("a.py", "x = 1 << 40").is_empty());
    // obj.free(p + 1) es un metodo propio, no el free de C.
    assert!(analizar("a.cc", "void f(A a, char *p) { a.free(p + 1); }").is_empty());
    // En C++20 1 << 31 esta definido; en C17 no.
    assert!(analizar("a.cc", "int x = 1 << 31;").is_empty());
    assert!(analizar("a.c", "int x = 1 << 31;").iter().any(|s| s.ficha == "UB-003"));
    // Los cuerpos de #define tambien se leen.
    assert!(analizar("a.h", "#define BIT(n) (1 << 32)\n").iter().any(|s| s.ficha == "UB-003" && s.linea == 1));
}

#[test]
fn asm_de_arranque_no_se_juzga_con_la_abi_de_64_bits() {
    let arranque = ".code32\n.globl inicio\ninicio:\n    mov %eax, %ebx\n    ret\n";
    assert!(analizar("boot.S", arranque).is_empty());
    // Sin ret no es una funcion normal (el _start del kernel): tampoco.
    assert!(analizar("e.S", ".globl _start\n_start:\n    mov %rdi, %rbx\n    hlt\n").is_empty());
}

#[test]
fn nunca_entra_en_panico_con_basura() {
    let basura = ["", "(", ")))", "\"sin cerrar", "'", "#define", "asm(", "free(", "for (", "x[", "R\"(", "\u{1F600} << 99", "1 <<", "va_arg(",
                  "int a[", "*(", "((void (*)(", "memcpy(a,", "__asm__ (\"cpuid\" :", "return &", "for(i=0;i<=", "INT_MAX +"];
    for b in basura {
        for ext in ["c", "cc", "h", "S", "asm"] { let _ = analizar(&format!("x.{ext}"), b); }
    }
}

#[test]
fn el_triangulo_reparte_las_fichas() {
    use carpetas_nucleo::museo::{eje, Eje};
    assert_eq!(eje("UB-001"), Eje::Seguridad);
    assert_eq!(eje("R-001"), Eje::Rendimiento);
    assert_eq!(eje("E-002"), Eje::Elegancia);
    assert_eq!(eje("NOUB-002"), Eje::Elegancia, "el orden no especificado es claridad, no un fallo");
    assert_eq!(eje("no-existe"), Eje::Seguridad, "lo desconocido, por prudencia, cuenta como seguridad");
    let por_eje = |e: &str| FICHAS.iter().filter(|f| f.eje == e).count();
    assert_eq!((por_eje("SEGURIDAD"), por_eje("RENDIMIENTO"), por_eje("ELEGANCIA")), (35, 3, 5));
}

#[test]
fn recibir_para_mover_no_es_copiar_de_mas() {
    use carpetas_nucleo::museo::analizar;
    // El sumidero de C++ moderno: por valor y std::move. Y una declaracion sin cuerpo.
    let sumidero = "#include <string>\n#include <utility>\nstruct A { std::string n; void pon(std::string x) { n = std::move(x); } };\nvoid f(std::string s);\n";
    assert!(analizar("a.cc", sumidero).iter().all(|s| s.ficha != "R-002"), "{:?}", analizar("a.cc", sumidero));
}

#[test]
fn una_clase_final_no_es_una_funcion() {
    use carpetas_nucleo::museo::analizar;
    // Una clase final con 120 lineas de miembros: ninguna funcion es larga.
    let miembros: String = (0..120).map(|i| format!("    int m{i};\n")).collect();
    let fuente = format!("class Grande final {{\n{miembros}    int f() const {{ return 1; }}\n}};\n");
    assert!(analizar("a.h", &fuente).iter().all(|s| s.ficha != "E-001"), "{:?}", analizar("a.h", &fuente));
}
