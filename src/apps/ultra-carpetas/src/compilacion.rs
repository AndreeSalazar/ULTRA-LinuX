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
//! `-D`, `-U`, `-std=`, `-include`, `-f…` (salvo `-fplugin`), `-m…` y `-W…` (salvo
//! `-Wl,`, `-Wa,` y `-Wp,`). Nunca `-o`, `-B`, `-specs`, `-wrapper` ni `@archivo`.

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Donde se busca, en este orden.
pub const LUGARES: &[&str] = &["compile_commands.json", "build/compile_commands.json", "out/Default/compile_commands.json", "out/compile_commands.json"];
/// Como mucho tantas carpetas de busqueda para el mapa.
const MAX_BASES: usize = 256;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Orden {
    /// La carpeta donde corre el compilador (absoluta).
    pub carpeta: PathBuf,
    /// Las flags seguras, con las rutas de -I ya absolutas.
    pub flags: Vec<String>,
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
            || (a.starts_with("-f") && !a.starts_with("-fplugin") && !a.starts_with("-fdump") && !a.starts_with("-fprofile") && !a.starts_with("-fdiagnostics"))
            || a.starts_with("-m") || (a.starts_with("-W") && !a.starts_with("-Wl,") && !a.starts_with("-Wa,") && !a.starts_with("-Wp,") && a != "-Werror");
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
        c.por_archivo.insert(rel.to_string_lossy().to_string(), Orden { carpeta, flags });
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
