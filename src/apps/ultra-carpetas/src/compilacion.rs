// SPDX-License-Identifier: GPL-2.0-only
//! `compile_commands.json`: como se compila de verdad cada archivo (lo generan Bear, CMake,
//! Meson o GN con GCC). El SUPER GRAFO lo usa para dos cosas:
//!
//! - **El mapa:** las `-I` del proyecto son donde el compilador busca los `#include`; el
//!   resolvedor las prueba antes que adivinar por el final de la ruta.
//! - **GCC:** `gcc_diagnosticos` y `gcc_deps` compilan cada archivo en su carpeta y con sus
//!   flags, no con unas genericas.
//!
//! Solo se copian flags que no ejecutan ni escriben nada: `-I`, `-isystem`, `-iquote`,
//! `-D`, `-U`, `-std=`, `-include`, una lista de opciones semanticas y `-W…` (salvo
//! `-Wl,`, `-Wa,` y `-Wp,`). Nunca `-o`, `-B`, `-specs`, `-wrapper` ni `@archivo`.

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Donde se busca, en este orden.
pub const LUGARES: &[&str] = &["compile_commands.json", "build/compile_commands.json", "out/Default/compile_commands.json", "out/compile_commands.json"];
/// Como mucho tantas carpetas de busqueda para el mapa.
const MAX_BASES: usize = 256;

/// Con que compilador se revisa: el que compila de verdad el archivo (Chromium: Clang; el
/// codigo de ULTRA: GCC). Siempre se ejecuta el `gcc` o `clang` del sistema, nunca el
/// programa que diga compile_commands.json (podria ser cualquier cosa del proyecto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compilador { #[default] Gcc, Clang }

impl Compilador {
    pub fn programa(self) -> &'static str { match self { Compilador::Gcc => "gcc", Compilador::Clang => "clang" } }
    pub fn de_nombre(s: &str) -> Option<Compilador> { match s { "gcc" => Some(Compilador::Gcc), "clang" => Some(Compilador::Clang), _ => None } }
    /// El de una orden: el primer programa que parezca un compilador (`ccache clang++`,
    /// `.../llvm-build/Release+Asserts/bin/clang++`, `x86_64-linux-gnu-gcc-13`).
    pub fn de_orden(args: &[String]) -> Compilador {
        for a in args.iter().take(3) {
            let n = a.rsplit('/').next().unwrap_or(a);
            if n.contains("clang") { return Compilador::Clang; }
            if n.contains("gcc") || n.contains("g++") || n == "cc" || n == "c++" { return Compilador::Gcc; }
        }
        Compilador::Gcc
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Orden {
    /// La carpeta donde corre el compilador (absoluta).
    pub carpeta: PathBuf,
    /// Las flags seguras, con las rutas de -I ya absolutas.
    pub flags: Vec<String>,
    /// El compilador de la orden (gcc o clang).
    pub compilador: Compilador,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Compilacion {
    /// De donde salio (relativo a la raiz).
    pub origen: String,
    /// Por archivo (ruta relativa a la raiz).
    pub por_archivo: HashMap<String, Orden>,
    /// Las -I que caen dentro de la raiz, relativas, para el mapa (las mas usadas primero).
    pub bases: Vec<String>,
    /// Flags que se descartaron por no ser seguras (para decirlo).
    pub descartadas: usize,
}

/// Parte una orden de shell sencilla (comillas simples y dobles, barra invertida).
fn partir(s: &str) -> Vec<String> {
    let (mut v, mut cur, mut en, mut q, mut esc) = (Vec::new(), String::new(), false, None::<char>, false);
    for c in s.chars() {
        if esc { cur.push(c); esc = false; en = true; continue; }
        match (c, q) {
            ('\\', Some('\'')) => cur.push(c),
            ('\\', _) => esc = true,
            ('"' | '\'', None) => { q = Some(c); en = true; }
            (c2, Some(x)) if c2 == x => q = None,
            (c2, None) if c2.is_whitespace() => { if en { v.push(std::mem::take(&mut cur)); en = false; } }
            _ => { cur.push(c); en = true; }
        }
    }
    if en { v.push(cur); }
    v
}

/// Flags seguras de una orden (sin el compilador ni el archivo de entrada).
pub(crate) fn semantica_segura(a: &str) -> bool {
    // No aceptar familias enteras: Clang -fpass-plugin carga codigo y otras -f
    // escriben modulos, perfiles o informes aunque la salida principal sea /dev/null.
    let nombre = a.split('=').next().unwrap_or(a);
    let nombre = nombre.strip_prefix("-fno-").or_else(|| nombre.strip_prefix("-f"));
    if let Some(n) = nombre {
        return matches!(n, "exceptions" | "rtti" | "ms-extensions" | "ms-compatibility"
            | "ms-compatibility-version" | "delayed-template-parsing" | "char8_t"
            | "short-wchar" | "short-enums" | "signed-char" | "unsigned-char"
            | "strict-aliasing" | "wrapv" | "trapv" | "builtin" | "freestanding"
            | "hosted" | "gnu89-inline" | "gnu-keywords" | "operator-names"
            | "visibility" | "visibility-inlines-hidden" | "PIC" | "pic" | "PIE" | "pie"
            | "stack-protector" | "stack-protector-strong" | "stack-protector-all"
            | "stack-clash-protection" | "cf-protection" | "omit-frame-pointer"
            | "math-errno" | "fast-math" | "finite-math-only" | "rounding-math"
            | "trapping-math" | "signed-zeros" | "fp-contract");
    }
    matches!(a, "-m32" | "-m64" | "-msse" | "-msse2" | "-msse3" | "-mssse3"
        | "-msse4.1" | "-msse4.2" | "-mavx" | "-mavx2" | "-mfma" | "-mno-red-zone")
        || a.starts_with("-march=") || a.starts_with("-mtune=")
}

fn seguras(args: &[String], carpeta: &Path, archivo: &str, descartadas: &mut usize) -> Vec<String> {
    let abs = |p: &str| { let x = Path::new(p); if x.is_absolute() { x.to_path_buf() } else { carpeta.join(x) } };
    let mut v = Vec::new();
    let mut i = 1; // args[0] es el compilador
    while i < args.len() {
        let a = &args[i];
        let siguiente = args.get(i + 1);
        // Flags con la ruta pegada o separada.
        let mut con_ruta = |flag: &str| -> bool {
            if a == flag { if let Some(s) = siguiente { v.push(format!("{flag}{}", abs(s).display())); i += 1; } return true; }
            if let Some(r) = a.strip_prefix(flag) { v.push(format!("{flag}{}", abs(r).display())); return true; }
            false
        };
        if con_ruta("-isystem") || con_ruta("-iquote") || con_ruta("-I") { i += 1; continue; }
        if a == "-include" { if let Some(s) = siguiente { v.push("-include".into()); v.push(abs(s).display().to_string()); i += 2; continue; } }
        if (a == "-D" || a == "-U") && siguiente.is_some() { v.push(format!("{a}{}", siguiente.map(String::as_str).unwrap_or(""))); i += 2; continue; }
        let segura = a.starts_with("-D") || a.starts_with("-U") || a.starts_with("-std=")
            || semantica_segura(a)
            || (a.starts_with("-W") && !a.starts_with("-Wl,") && !a.starts_with("-Wa,") && !a.starts_with("-Wp,") && a != "-Werror");
        // -o, -MF, -MT y -MQ llevan un argumento que no es una flag: se salta junto.
        if matches!(a.as_str(), "-o" | "-MF" | "-MT" | "-MQ" | "-x" | "-Xclang" | "-arch" | "-target" | "-B" | "-specs" | "-wrapper") { i += 2; if a != "-o" && a != "-x" { *descartadas += 1; } continue; }
        if segura { v.push(a.clone()); }
        else if a.starts_with('-') && !matches!(a.as_str(), "-c" | "-S" | "-E" | "-MD" | "-MMD" | "-M" | "-MM" | "-g" | "-pipe") && !a.starts_with("-O") && !a.starts_with("-g") { *descartadas += 1; }
        else if !a.starts_with('-') && a != archivo && !abs(a).ends_with(archivo) { *descartadas += 1; }
        i += 1;
    }
    v
}

/// Lee el primer compile_commands.json que encuentre en la raiz (o None si no hay).
pub fn leer(raiz: &Path) -> Option<Compilacion> {
    let raiz = raiz.canonicalize().ok()?;
    let (origen, texto) = LUGARES.iter().find_map(|l| std::fs::read_to_string(raiz.join(l)).ok().map(|t| (l.to_string(), t)))?;
    let doc: Value = serde_json::from_str(&texto).ok()?;
    let mut c = Compilacion { origen, ..Compilacion::default() };
    let mut cuenta: HashMap<String, usize> = HashMap::new();
    for e in doc.as_array()? {
        let Some(dir) = e["directory"].as_str() else { continue };
        let Some(archivo) = e["file"].as_str() else { continue };
        let carpeta = PathBuf::from(dir);
        let carpeta = if carpeta.is_absolute() { carpeta } else { raiz.join(carpeta) };
        let abs = if Path::new(archivo).is_absolute() { PathBuf::from(archivo) } else { carpeta.join(archivo) };
        let abs = abs.canonicalize().unwrap_or(abs);
        let Ok(rel) = abs.strip_prefix(&raiz) else { continue };
        let args: Vec<String> = match (e["arguments"].as_array(), e["command"].as_str()) {
            (Some(a), _) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
            (None, Some(cmd)) => partir(cmd),
            _ => continue,
        };
        if args.iter().any(|a| a.starts_with('@')) { c.descartadas += 1; continue; } // un archivo de respuestas: no se sigue
        let flags = seguras(&args, &carpeta, archivo, &mut c.descartadas);
        for f in &flags {
            if let Some(d) = f.strip_prefix("-I").or_else(|| f.strip_prefix("-iquote")) {
                let d = Path::new(d);
                let d = d.canonicalize().unwrap_or_else(|_| d.to_path_buf());
                if let Ok(r) = d.strip_prefix(&raiz) { *cuenta.entry(r.to_string_lossy().to_string()).or_default() += 1; }
            }
        }
        c.por_archivo.insert(rel.to_string_lossy().to_string(), Orden { carpeta, flags, compilador: Compilador::de_orden(&args) });
    }
    let mut bases: Vec<(String, usize)> = cuenta.into_iter().collect();
    bases.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    c.bases = bases.into_iter().map(|(b, _)| b).filter(|b| !b.is_empty()).take(MAX_BASES).collect();
    Some(c)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn parte_ordenes_de_shell() {
        assert_eq!(partir(r#"gcc -DX="a b" -I'dir con espacio' -c x.c"#), ["gcc", "-DX=a b", "-Idir con espacio", "-c", "x.c"]);
        assert_eq!(partir("gcc -DY=\\\"z\\\" x.c"), ["gcc", "-DY=\"z\"", "x.c"]);
    }

    #[test]
    fn no_importa_plugins_salidas_ni_opciones_del_backend() {
        let peligrosas = ["-fpass-plugin=/tmp/plugin.so", "-fplugin=/tmp/plugin.so",
            "-fmodule-output=/tmp/mod.pcm", "-fmodules-cache-path=/tmp/cache",
            "-foptimization-record-file=/tmp/informe", "-fsave-optimization-record",
            "-fcallgraph-info", "-fstack-usage", "-fprofile-instr-generate=/tmp/prof",
            "-mllvm=-load=/tmp/plugin.so", "-fopcion-futura"];
        let mut args = vec!["clang".to_string()];
        args.extend(peligrosas.iter().map(|s| s.to_string()));
        args.extend(["-fno-rtti", "-fPIC", "-march=x86-64", "-mtune=znver3"].map(String::from));
        let mut descartadas = 0;
        assert_eq!(seguras(&args, Path::new("/p"), "a.cc", &mut descartadas),
            ["-fno-rtti", "-fPIC", "-march=x86-64", "-mtune=znver3"]);
        assert_eq!(descartadas, peligrosas.len());
    }

    #[test]
    fn solo_flags_seguras_y_rutas_absolutas() {
        let args: Vec<String> = ["gcc", "-Iinc", "-I", "otra", "-isystem", "/usr/x", "-DNDEBUG", "-D", "A=1", "-std=c17", "-O2", "-Wall", "-Werror",
                                 "-fplugin=mal.so", "-fno-exceptions", "-Wl,--mal", "-o", "x.o", "-B/tmp/mal", "-c", "src/x.c"].iter().map(|s| s.to_string()).collect();
        let mut d = 0;
        let f = seguras(&args, Path::new("/p/build"), "src/x.c", &mut d);
        assert_eq!(f, ["-I/p/build/inc", "-I/p/build/otra", "-isystem/usr/x", "-DNDEBUG", "-DA=1", "-std=c17", "-Wall", "-fno-exceptions"]);
        assert_eq!(d, 4, "-Werror, -fplugin, -Wl, y -B se descartan");
    }

    #[test]
    fn lee_el_archivo_y_saca_las_bases() {
        let dir = std::env::temp_dir().join(format!("ultra-cc-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src/inc")).unwrap();
        std::fs::write(dir.join("src/a.c"), "int a;\n").unwrap();
        let raiz = dir.canonicalize().unwrap();
        let cc = serde_json::json!([{ "directory": raiz.join("src").display().to_string(), "file": "a.c", "command": "gcc -Iinc -I/usr/include -DX -c a.c -o a.o" }]);
        std::fs::write(dir.join("compile_commands.json"), cc.to_string()).unwrap();
        let c = leer(&dir).unwrap();
        assert_eq!(c.origen, "compile_commands.json");
        let o = &c.por_archivo["src/a.c"];
        assert_eq!(o.carpeta, raiz.join("src"));
        assert_eq!(o.flags, [format!("-I{}", raiz.join("src/inc").display()), "-I/usr/include".into(), "-DX".into()]);
        assert_eq!(c.bases, ["src/inc"], "solo las -I de dentro de la raiz, relativas");
        std::fs::remove_dir_all(&dir).ok();
    }
}
