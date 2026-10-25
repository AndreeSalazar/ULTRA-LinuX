// SPDX-License-Identifier: GPL-2.0-only
use carpetas_nucleo::dependencias::{aislar, conectar, referencias, resolver, unir, ClaseRef, Dependencias, Rol, Rutas};
use std::collections::BTreeSet;

struct Proyecto(Vec<&'static str>);
impl Rutas for Proyecto {
    fn existe(&self, r: &str) -> bool { self.0.contains(&r) }
    fn por_nombre(&self, n: &str) -> Vec<&str> { self.0.iter().copied().filter(|r| r.rsplit('/').next() == Some(n)).collect() }
}

fn r(desde: &str, texto: &str, p: &Proyecto) -> Vec<String> {
    referencias(desde, texto).iter().filter_map(|x| resolver(desde, x, p)).collect()
}

#[test]
fn c_cpp_y_asm() {
    let p = Proyecto(vec!["src/main.c", "src/util.h", "include/linux/kernel.h", "tools/include/linux/kernel.h", "base/files/file.h", "base/files/file.cc", "boot/entrada.asm", "boot/macros.inc", "src/x.S"]);
    assert_eq!(r("src/main.c", "#include \"util.h\"\n#include <stdio.h>\n  #  include <linux/kernel.h>\n", &p), ["src/util.h", "include/linux/kernel.h"]);
    assert_eq!(r("base/files/file.cc", "#include \"base/files/file.h\"\n// #include \"src/util.h\" comentado tambien cuenta? no:\n", &p), ["base/files/file.h"]);
    assert_eq!(r("tools/perf.c", "#include <linux/kernel.h>\n", &p), ["tools/include/linux/kernel.h"], "el mas cercano al que incluye");
    assert_eq!(r("boot/entrada.asm", "%include \"macros.inc\"\n", &p), ["boot/macros.inc"]);
    assert_eq!(r("src/x.S", "#include \"util.h\"\n.include \"util.h\"\n", &p), ["src/util.h", "src/util.h"]);
    // <features.h> sin carpeta es de la libc: no es base/features.h (el camino de Chromium lo
    // destapo). Con comillas, o con carpeta, si se busca por el final.
    let q = Proyecto(vec!["base/features.h", "pa/build_config.h", "base/memory/raw_ptr.h"]);
    assert!(r("pa/build_config.h", "#include <features.h>\n", &q).is_empty());
    assert_eq!(r("pa/build_config.h", "#include <memory/raw_ptr.h>\n", &q), ["base/memory/raw_ptr.h"]);
    // No sale de la raiz.
    assert_eq!(unir("a", "../../etc/passwd"), None);
    assert!(r("src/main.c", "#include \"../../../etc/passwd\"\n", &p).is_empty());
}

#[test]
fn cada_include_sabe_de_que_if_depende() {
    let texto = "// licencia\n#ifndef X_H\n#define X_H\n#include \"siempre.h\"\n#if BUILDFLAG(IS_WIN)\n#include \"win.h\"\n#elif BUILDFLAG(IS_POSIX)\n#include \"posix.h\"\n#else\n#include \"otro.h\"\n#endif\n#ifdef DEBUG\n#  include <a/depura.h>\n#endif\n#endif\n";
    let c: Vec<(String, Option<String>)> = referencias("x.h", texto).into_iter().map(|r| (r.texto, r.condicion)).collect();
    let s = |x: &str| x.to_string();
    // La guarda (#ifndef X_H / #define X_H) no cuenta: siempre.h no depende de nada.
    assert_eq!(c, vec![(s("siempre.h"), None), (s("win.h"), Some(s("#if BUILDFLAG(IS_WIN)"))), (s("posix.h"), Some(s("#elif BUILDFLAG(IS_POSIX)"))),
                       (s("otro.h"), Some(s("#else de #elif BUILDFLAG(IS_POSIX)"))), (s("a/depura.h"), Some(s("#ifdef DEBUG")))]);
    // Sin guarda (#pragma once), un #ifndef de verdad si cuenta.
    let c = referencias("y.h", "#pragma once\n#ifndef NO_RED\n#include \"red.h\"\n#endif\n");
    assert_eq!(c[0].condicion.as_deref(), Some("#ifndef NO_RED"));
    // Un #ifndef con codigo antes no es la guarda.
    let c = referencias("z.c", "int x;\n#ifndef SOLO\n#include \"z.h\"\n#endif\n");
    assert_eq!(c[0].condicion.as_deref(), Some("#ifndef SOLO"));
}

#[test]
fn rust_mod_y_use() {
    let p = Proyecto(vec!["app/src/main.rs", "app/src/vista.rs", "app/src/vista/boton.rs", "app/src/red/mod.rs", "nucleo/src/lib.rs", "nucleo/src/ciudad.rs", "nucleo/tests/a.rs", "nucleo/tests/comun.rs"]);
    let main = "mod vista;\npub(crate) mod red;\nuse crate::vista::Cosa;\nuse carpetas_nucleo::{ciudad::construir, Otra};\nuse std::io;\nmod en_linea { }\n";
    let carpeta = Proyecto(p.0.iter().map(|x| *x).chain(["app/carpetas-nucleo/src/lib.rs", "app/carpetas-nucleo/src/ciudad.rs"]).collect());
    let v = r("app/src/main.rs", main, &carpeta);
    assert_eq!(v, ["app/src/vista.rs", "app/src/red/mod.rs", "app/src/vista.rs", "app/carpetas-nucleo/src/ciudad.rs", "app/carpetas-nucleo/src/lib.rs"]);
    assert_eq!(r("app/src/vista.rs", "mod boton;\n", &p), ["app/src/vista/boton.rs"]);
    assert_eq!(r("nucleo/tests/a.rs", "mod comun;\n", &p), ["nucleo/tests/comun.rs"]);
    assert!(referencias("x.rs", "/* mod oculto;\n*/\n").is_empty());
}

#[test]
fn python() {
    let p = Proyecto(vec!["scripts/museo.py", "scripts/comun.py", "pkg/__init__.py", "pkg/a.py", "pkg/sub/b.py"]);
    assert_eq!(r("scripts/museo.py", "import comun, os\nfrom pkg.sub import b\n", &p), ["scripts/comun.py", "pkg/sub/b.py"]);
    assert_eq!(r("pkg/sub/b.py", "from .. import a\nfrom ..a import f\n", &p), ["pkg/a.py", "pkg/a.py"]);
    assert_eq!(referencias("x.py", "from . import (a, b as c)\n").iter().map(|x| (x.texto.as_str(), x.clase)).collect::<Vec<_>>(),
               [("a", ClaseRef::Python(1)), ("b", ClaseRef::Python(1))]);
}

/// util.h falla; main.c y red.c lo incluyen y fallan por el: la causa es util.h.
#[test]
fn conecta_el_fallo_y_aisla_la_causa() {
    let rutas: Vec<String> = ["util.h", "main.c", "red.c", "solo.c", "base.h", "app.c"].iter().map(|x| x.to_string()).collect();
    // main.c -> util.h, red.c -> util.h, util.h -> base.h, app.c -> main.c
    let d = Dependencias::nuevo(rutas, &[(1, 0), (2, 0), (0, 4), (5, 1), (1, 0)]);
    assert_eq!(d.aristas, 5);
    assert_eq!(d.usa("main.c").collect::<Vec<_>>(), ["util.h"], "sin aristas repetidas");
    let fallos: BTreeSet<String> = ["util.h", "main.c", "red.c", "solo.c"].iter().map(|x| x.to_string()).collect();
    let c = conectar(&d, &fallos, &BTreeSet::new());
    assert_eq!(c.causas[0].ruta, "util.h");
    assert_eq!(c.causas[0].explica, 2, "main.c y red.c fallan por util.h");
    assert_eq!(c.roles["util.h"], Rol::Causa);
    assert_eq!(c.roles["main.c"], Rol::Falla);
    assert_eq!(c.roles["solo.c"], Rol::Causa, "falla por su cuenta");
    assert_eq!(c.roles["app.c"], Rol::Arrastra, "usa main.c, que usa util.h");
    assert!(!c.roles.contains_key("base.h"));
    assert!(c.aristas.contains(&("main.c".into(), "util.h".into())));
    // Con una sospecha del museo en main.c, main.c es causa propia (no consecuencia de util.h).
    let propios: BTreeSet<String> = ["main.c".to_string()].into();
    let c2 = conectar(&d, &fallos, &propios);
    assert_eq!(c2.roles["main.c"], Rol::Causa);
    assert_eq!(c2.roles["red.c"], Rol::Falla);
    // Aislar un archivo: lo que usa y quien lo usa.
    let a = aislar(&d, "util.h");
    assert_eq!(a.roles["base.h"], Rol::Base);
    assert_eq!(a.roles["app.c"], Rol::Arrastra);
    assert_eq!(a.roles.len(), 5, "todo menos solo.c");
    assert!(!a.roles.contains_key("solo.c"));
}
