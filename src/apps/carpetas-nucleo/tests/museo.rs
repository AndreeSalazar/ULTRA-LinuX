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
    assert_eq!(FICHAS.len(), 49);
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
    assert!(analizar("a.rs", "free(p + 1)").is_empty(), "free de C no es una ficha de Rust");
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
                  "int a[", "*(", "((void (*)(", "memcpy(a,", "__asm__ (\"cpuid\" :", "return &", "for(i=0;i<=", "INT_MAX +",
                  "#if", "#elif", "#endif\n#endif", "#if (defined(", "#if !", "#else\nx[9]", "struct { int a[0]; } b[1];", "} x[3];",
                  "r#\"", "br##\"x\"#", "'", "b'", "'\\", "/* /* */", "unsafe {", "static mut", "#[cfg(test)]", "#[test] fn", "x.unwrap(", "::transmute::<"];
    for b in basura {
        for ext in ["c", "cc", "h", "S", "asm", "rs"] { let _ = analizar(&format!("x.{ext}"), b); }
    }
}

/// Patrones reales de Chromium que la primera pasada marco como errores y no lo son.
#[test]
fn casos_reales_de_chromium_que_no_son_fallos() {
    let ub = |ruta: &str, texto: &str| analizar(ruta, texto).into_iter().filter(|s| s.ficha.starts_with("UB-")).collect::<Vec<_>>();
    // Arreglo flexible al final de la clase (blink/.../line_info_list.h).
    let flexible = "class L {\n  int& Primero() { return line_infos_[0]; }\n  int n_;\n  int line_infos_[0];\n};\n";
    assert!(ub("a.h", flexible).is_empty(), "{:?}", ub("a.h", flexible));
    let flexible1 = "struct S { int n; char datos[1]; };\nchar f(struct S *s) { char *d = s->datos; return datos[3]; }\n";
    assert!(ub("a.c", flexible1).is_empty(), "{:?}", ub("a.c", flexible1));
    // `} fpregs[32];` es un miembro de una estructura anonima (crashpad signal_context.h).
    let anonima = "struct A {\n  struct { int x; } fpregs[32];\n};\nstruct B { double fpregs[32]; };\n";
    assert!(ub("a.h", anonima).is_empty(), "{:?}", ub("a.h", anonima));
    // El tamano real es una constante (crash_handler_host_linux.cc); otra funcion tiene un iov[2].
    let iov = "void a() {\n  struct iovec iov[kCrashIovSize];\n  iov[2].iov_len = 1;\n  iov[5].iov_len = 2;\n}\nvoid b() {\n  struct iovec iov[2];\n  iov[0].iov_len = 1;\n}\n";
    assert!(ub("a.cc", iov).is_empty(), "{:?}", ub("a.cc", iov));
    // Devolver un bufer local como String o std::string copia el texto (filter_operation.h, hwid_checker.cc).
    let copia = "class F {\n  String DebugString() const override {\n    char buf[256];\n    snprintf(buf, sizeof(buf), \"x\");\n    return buf;\n  }\n};\nstd::string Suma(int x) {\n  char checksum[5];\n  return checksum;\n}\n";
    assert!(ub("a.h", copia).is_empty(), "{:?}", ub("a.h", copia));
    // La escritura en el puntero nulo esta en un #else que ni GCC ni Clang compilan (safe_conversions_impl.h).
    let trampa = "struct CheckOnFailure {\n  template <typename T> static T HandleFailure() {\n#if defined(_MSC_VER)\n    __debugbreak();\n#elif defined(__GNUC__) || defined(__clang__)\n    __builtin_trap();\n#else\n    ((void)(*(volatile char*)0 = 0));\n#endif\n    return T();\n  }\n};\n";
    assert!(ub("a.h", trampa).is_empty(), "{:?}", ub("a.h", trampa));
    assert!(ub("a.c", "void f(void) {\n#if 0\n  *(int*)0 = 1;\n#endif\n}\n").is_empty());
    assert!(ub("a.c", "void f(void) {\n#if !defined(__clang__)\n  *(int*)0 = 1;\n#endif\n}\n").is_empty());
}

/// Los arreglos anteriores no apagan los fallos de verdad.
#[test]
fn los_fallos_de_verdad_siguen_saltando() {
    let tiene = |ruta: &str, texto: &str, id: &str| analizar(ruta, texto).iter().any(|s| s.ficha == id);
    assert!(tiene("a.c", "void a() { int v[kN]; v[9] = 0; }\nvoid b() { int v[2]; v[2] = 1; }\n", "UB-004"), "cada funcion mira su propio arreglo");
    assert!(tiene("a.c", "int g[4];\nvoid f(void) { g[4] = 1; }\n", "UB-004"), "un global sigue contando");
    assert!(tiene("a.cc", "std::string_view f() {\n  char buf[8];\n  return buf;\n}\n", "UB-005"), "una vista no copia");
    assert!(tiene("a.c", "const char *f(void) {\n  char buf[8];\n  return buf;\n}\n", "UB-005"));
    // Una macro del proyecto no se sabe: la rama se sigue leyendo.
    assert!(tiene("a.c", "void f(void) {\n#ifdef ALGO_DEL_PROYECTO\n  *(int*)0 = 1;\n#endif\n}\n", "UB-015"));
    assert!(tiene("a.c", "void f(void) {\n#if defined(_MSC_VER)\n  x();\n#else\n  *(int*)0 = 1;\n#endif\n}\n", "UB-015"), "clang-cl define _MSC_VER: no se sabe");
}

#[test]
fn el_lexer_de_rust_no_se_confunde() {
    // Vidas, cadenas crudas y comentarios anidados no esconden ni inventan codigo.
    let raro = "fn f<'a>(x: &'a str) -> &'a str {\n    let s = r#\"unsafe { transmute(x) }\"#;\n    /* static mut X /* anidado */ unsafe { } */\n    let c = '{';\n    let b = b'\\'';\n    'fuera: loop { break 'fuera; }\n    x\n}\n";
    assert!(analizar("a.rs", raro).is_empty(), "{:?}", analizar("a.rs", raro));
    // Despues de todo eso, el codigo real se sigue viendo con su linea.
    let despues = format!("{raro}static mut CUENTA: u32 = 0;\n");
    let s = analizar("a.rs", &despues);
    assert_eq!(s.iter().map(|x| (x.ficha, x.linea)).collect::<Vec<_>>(), [("RS-003", 9)]);
    // Las pruebas pueden usar unwrap.
    assert!(analizar("crate/tests/lee.rs", "fn t() { x.unwrap(); }").is_empty());
    assert!(analizar("src/lib.rs", "fn t() { x.unwrap(); }").iter().any(|s| s.ficha == "RS-006"));
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
    assert_eq!((por_eje("SEGURIDAD"), por_eje("RENDIMIENTO"), por_eje("ELEGANCIA")), (39, 3, 7));
    assert_eq!(eje("RS-001"), Eje::Elegancia, "falta el porque, no hay un fallo");
    assert_eq!(eje("RS-002"), Eje::Seguridad);
    assert_eq!(eje("RS-006"), Eje::Elegancia, "un unwrap es una guia, no un fallo");
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
