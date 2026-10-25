// SPDX-License-Identifier: GPL-2.0-only
//! El SUPER GRAFO sin ventana: preguntas con respuesta exacta y acotada, en JSON.
//!
//! Lo usan la linea de ordenes (`ultra-carpetas --consulta <herramienta> <carpeta> [json]`)
//! y el servidor MCP (`ultra-carpetas --mcp <carpeta>`), con las mismas herramientas.
//!
//! Regla: **cada respuesta tiene tope**, sea cual sea el tamano del proyecto. Primero las
//! cifras y lo peor; las listas llevan `limite` (por defecto 10, como mucho 100) y dicen
//! cuantos quedan. El trabajo pesado lo hace el SUPER GRAFO; quien pregunta recibe poco.
//!
//! Solo lee, salvo dos cosas que se piden a proposito: `gcc_diagnosticos` ejecuta GCC con
//! `-c -o /dev/null` (compila y no escribe nada) y `exportar_sarif` escribe el archivo que
//! se le pide, dentro de la carpeta.

use crate::analisis::analizar;
use crate::indice::indexar;
use carpetas_nucleo::camino::{camino, SinPaso, Union};
use carpetas_nucleo::ciclos::Ciclo;
use carpetas_nucleo::combinar::combinar;
use carpetas_nucleo::dependencias::{Motivo, Rol};
use carpetas_nucleo::museo::{eje, ficha, Eje, Fuerza};
use carpetas_nucleo::proyecto::{buscar_rutas, Indice};
use carpetas_nucleo::{reducir, Accion, Diagnostico, Estado};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

const LIMITE: usize = 10;
const LIMITE_MAX: usize = 100;
/// Tiempo maximo de una ejecucion de GCC.
const GCC_TIEMPO: Duration = Duration::from_secs(60);

/// El proyecto analizado: indice, dependencias, museo y fallo conectado.
pub struct Mapa {
    pub raiz: PathBuf,
    indice: Indice,
    pub estado: Estado,
    micros: u64,
    /// compile_commands.json, si lo hay: carpeta y flags reales de cada archivo.
    cc: Option<crate::compilacion::Compilacion>,
    /// Sube con cada analisis (para `cambios` y los avisos en vivo).
    pub version: u64,
    /// La foto del analisis anterior y cuando se hizo el actual (segundos UNIX).
    previa: Option<Foto>,
    cuando: u64,
    /// El perfil de los guardianes del proyecto (perfil-gcc.sh), si tiene.
    guardian: Option<crate::guardianes::Perfil>,
    /// Las revisiones de GCC por archivo: sobreviven al reanalisis (si el archivo no cambio)
    /// y a reiniciar (se guardan en la cache). Las de archivos que cambiaron, pendientes.
    revisiones: std::collections::BTreeMap<String, Revision>,
    pub pendientes: Vec<String>,
    /// Por que esta pendiente cada una ("cambio a.c", "cambio base/x.h").
    razones: std::collections::BTreeMap<String, String>,
}

/// Una revision de GCC de un archivo: con que perfil, cuando, y que dijo.
#[derive(Debug, Clone, PartialEq)]
pub struct Revision {
    fecha: u128,
    tam: u64,
    /// "guardian" (perfil-gcc.sh, como ultra_revisar) o "generico".
    perfil: String,
    analizador: bool,
    extra: Vec<String>,
    cuando: u64,
    diags: Vec<DiagGcc>,
    /// Las cabeceras del proyecto que GCC leyo de verdad (-MMD, con los #if resueltos), con
    /// su fecha y tamano: si cambia una, la revision ya no vale.
    deps: Vec<(String, u128, u64)>,
}

/// Como mucho tantas cabeceras guardadas por revision.
const MAX_DEPS: usize = 4000;

fn fecha_y_tam(p: &Path) -> Option<(u128, u64)> {
    let m = std::fs::symlink_metadata(p).ok()?;
    Some((m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos(), m.len()))
}

fn diag_a_json(d: &DiagGcc) -> Value {
    json!({ "archivo": d.archivo, "dentro": d.archivo_en_proyecto, "linea": d.linea, "columna": d.columna, "nivel": d.nivel, "regla": d.regla, "mensaje": d.mensaje, "pasos": d.pasos })
}
fn diag_de_json(v: &Value) -> Option<DiagGcc> {
    Some(DiagGcc { archivo: v["archivo"].as_str()?.into(), archivo_en_proyecto: v["dentro"].as_bool()?, linea: v["linea"].as_u64()? as u32, columna: v["columna"].as_u64()? as u32,
                   nivel: v["nivel"].as_str()?.into(), regla: v["regla"].as_str().map(str::to_string), mensaje: v["mensaje"].as_str()?.into(),
                   pasos: v["pasos"].as_array()?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect() })
}

/// Lo que se compara entre dos analisis para decir que cambio.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Foto {
    version: u64,
    /// (archivo, ficha, lo visto): la linea cambia al editar, lo visto no.
    sospechas: std::collections::BTreeSet<(String, String, String)>,
    fallan: std::collections::BTreeSet<String>,
    causas: Vec<String>,
    ciclos: std::collections::BTreeSet<Vec<String>>,
    dependencias: u64,
    archivos: u64,
}

/// La firma barata de la carpeta: rutas, tamanos y fechas de lo que se analiza. Si cambia,
/// toca reanalizar (y la cache hace que solo se relea lo que cambio).
pub fn firma(raiz: &Path) -> (u64, usize) {
    let (indice, _) = indexar(raiz, false);
    let mut h = 0xcbf29ce484222325u64;
    let mut mezclar = |b: &[u8]| for x in b { h = (h ^ *x as u64).wrapping_mul(0x100000001b3); };
    let mut n = 0;
    for k in 0..indice.len() {
        if indice.es_carpeta(k) { continue; }
        let r = indice.ruta(k);
        if !(carpetas_nucleo::dependencias::sigue(r) || carpetas_nucleo::museo::idioma(r).is_some() || crate::compilacion::LUGARES.contains(&r)) { continue; }
        let Ok(m) = std::fs::symlink_metadata(raiz.join(r)) else { continue };
        let fecha = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
        mezclar(r.as_bytes());
        mezclar(&m.len().to_le_bytes());
        mezclar(&fecha.to_le_bytes());
        n += 1;
    }
    (h, n)
}

fn ahora_s() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

fn limite(a: &Value) -> usize { a.get("limite").and_then(Value::as_u64).map(|n| (n as usize).clamp(1, LIMITE_MAX)).unwrap_or(LIMITE) }

fn texto<'a>(a: &'a Value, campo: &str) -> Result<&'a str, String> {
    a.get(campo).and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or_else(|| format!("falta \"{campo}\" (texto)"))
}

fn lista(a: &Value, campo: &str) -> Result<Vec<String>, String> {
    let v = a.get(campo).and_then(Value::as_array).ok_or_else(|| format!("falta \"{campo}\" (lista de rutas)"))?;
    let v: Vec<String> = v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
    if v.is_empty() { return Err(format!("\"{campo}\" esta vacia")); }
    Ok(v.into_iter().take(64).collect())
}

/// Una ruta relativa que no sale de la carpeta (sin `..` ni rutas absolutas).
fn relativa(r: &str) -> Result<String, String> {
    let p = Path::new(r);
    if p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("{r}: hace falta una ruta relativa dentro de la carpeta (sin .. ni /)"));
    }
    Ok(r.trim_start_matches("./").trim_end_matches('/').to_string())
}

fn motivo(m: &Option<Motivo>, desde: &str) -> Value {
    match m {
        Some(m) => json!({ "linea": m.linea, "escrito": m.escrito(desde), "condicion": m.condicion }),
        None => Value::Null,
    }
}

fn nombre_eje(e: Eje) -> &'static str { e.nombre() }

fn rol(r: Rol) -> &'static str {
    match r { Rol::Causa => "causa", Rol::Falla => "falla", Rol::Arrastra => "arrastra", Rol::Base => "base" }
}

/// Las herramientas: nombre, que hace y sus argumentos (JSON Schema, para MCP).
pub fn herramientas() -> Vec<Value> {
    let ruta = json!({ "type": "string", "description": "Ruta relativa a la carpeta analizada" });
    let rutas = json!({ "type": "array", "items": { "type": "string" }, "description": "Rutas relativas (archivos o carpetas), como mucho 64" });
    let lim = json!({ "type": "integer", "minimum": 1, "maximum": LIMITE_MAX, "description": "Cuantos como mucho (10 por defecto)" });
    let h = |nombre: &str, desc: &str, props: Value, req: &[&str]| json!({
        "name": nombre, "description": desc,
        "inputSchema": { "type": "object", "properties": props, "required": req },
    });
    vec![
        h("resumen", "Cifras del proyecto: archivos, dependencias, sospechas por eje (Seguridad, Rendimiento, Elegancia), fallo conectado, ciclos. Empezar por aqui.", json!({}), &[]),
        h("buscar", "Rutas del proyecto cuyo nombre se parece al texto (como Ctrl+P).", json!({ "texto": { "type": "string" }, "limite": lim }), &["texto"]),
        h("archivo", "Un archivo: lo que usa y quien lo usa (con la linea de cada #include), sus sospechas del museo y su papel en el fallo conectado.", json!({ "ruta": ruta, "limite": lim }), &["ruta"]),
        h("fallos", "El fallo conectado: causas probables (falla y nada de lo que usa falla), cuantos fallos explica cada una y a cuantos llega.", json!({ "limite": lim }), &[]),
        h("senales", "Lo que senalan unos archivos (museo y errores de GCC), en el orden para arreglar, con la ficha de cada una y por donde les llega el fallo.", json!({ "archivos": rutas, "limite": lim }), &["archivos"]),
        h("camino", "Por donde se unen dos archivos (A usa B, B usa A, comparten X, los junta X), con la linea de cada paso y que pasos son imprescindibles (donde cortar).", json!({ "a": ruta, "b": ruta }), &["a", "b"]),
        h("ciclos", "Ciclos de cabeceras (C, C++ y ASM): la vuelta mas corta de cada uno, sus lineas, donde cortar y si depende de un #if.", json!({ "limite": lim }), &[]),
        h("peso", "Si cambia este archivo, cuantos se recompilan (quien lo usa, directa o indirectamente) y los que lo usan directamente.", json!({ "ruta": ruta, "limite": lim }), &["ruta"]),
        h("combinar", "Como se combinan varios archivos: directas, lo que comparten, quien los junta, sueltos y piezas.", json!({ "archivos": rutas, "limite": lim }), &["archivos"]),
        h("ficha", "Una ficha del museo: regla, control valido, limite, referencia y eje.", json!({ "id": { "type": "string", "description": "C-001, UB-002, E-003..." } }), &["id"]),
        h("gcc_diagnosticos", "Revisa un archivo con GCC (compila a /dev/null: no escribe nada) y lee sus diagnosticos en SARIF. En C usa el perfil de los guardianes del proyecto (perfil-gcc.sh: ULTRA_WARN, dos pasadas como ultra_revisar, -O0 -fanalyzer y -O2); si no, -Wall -Wextra -O2 (+ -fanalyzer si se pide). La revision se guarda: sobrevive al reanalisis y se repite sola si el archivo cambia.",
          json!({ "archivo": ruta, "perfil": { "type": "string", "enum": ["guardian", "generico"] }, "analizador": { "type": "boolean" }, "flags": { "type": "array", "items": { "type": "string" }, "description": "Flags extra: solo -I, -D, -std= y -W" }, "limite": lim }), &["archivo"]),
        h("guardianes", "El perfil de los guardianes (banderas de perfil-gcc.sh) y las revisiones de GCC guardadas: archivo, perfil, errores, avisos y si siguen al dia.", json!({ "limite": lim }), &[]),
        h("gcc_deps", "Mide cuanto acierta el mapa lexico: para unos archivos C/C++ compara sus cabeceras del proyecto con las que dice gcc -MM -MG (con #if resueltos). Pasar las -I reales del proyecto en flags.",
          json!({ "carpeta": ruta, "flags": { "type": "array", "items": { "type": "string" }, "description": "-I relativas, -D, -std=" }, "limite": lim }), &[]),
        h("exportar_sarif", "Escribe las sospechas del museo (y los diagnosticos de GCC leidos) en un archivo SARIF 2.1.0 dentro de la carpeta.", json!({ "salida": ruta }), &["salida"]),
        h("reanalizar", "Vuelve a leer la carpeta ahora (el servidor MCP ya reanaliza solo al ver cambios; incremental: solo relee lo que cambio).", json!({}), &[]),
        h("ventana", "Lo que la persona esta mirando ahora en la ventana de ULTRA Carpetas (vista, grupo elegido, seleccion, camino, archivo del editor), con las senales de su grupo. Sin capturas.", json!({ "limite": lim }), &[]),
        h("mostrar", "Ensena algo en la ventana de la persona (solo mueve la vista): accion grupo (rutas, modo ciudad|grafo), revelar (ruta, linea), camino (a, b), aislar (rutas) o limpiar.",
          json!({ "accion": { "type": "string", "enum": ["grupo", "revelar", "camino", "aislar", "limpiar"] }, "rutas": rutas, "ruta": ruta, "linea": { "type": "integer", "minimum": 1 },
                  "a": ruta, "b": ruta, "modo": { "type": "string", "enum": ["ciudad", "grafo"] } }), &["accion"]),
        h("cambios", "Que cambio desde el analisis anterior: sospechas nuevas y arregladas, archivos que empiezan o dejan de fallar, la causa principal, ciclos nuevos y rotos.", json!({ "limite": lim }), &[]),
    ]
}

impl Mapa {
    pub fn abrir(raiz: &Path) -> Result<Mapa, String> {
        let raiz = raiz.canonicalize().map_err(|e| format!("{}: {e}", raiz.display()))?;
        if !raiz.is_dir() { return Err(format!("{}: no es una carpeta", raiz.display())); }
        let mut m = Mapa { raiz, indice: Indice::default(), estado: Estado::default(), micros: 0, cc: None, version: 0, previa: None, cuando: 0,
                           guardian: None, revisiones: Default::default(), pendientes: Vec::new(), razones: Default::default() };
        m.revisiones = m.cargar_revisiones();
        m.analizar();
        Ok(m)
    }

    /// Vuelve a analizar (incremental: la cache solo relee lo que cambio) y guarda la foto
    /// anterior para `cambios`.
    pub fn analizar(&mut self) {
        if self.version > 0 { self.previa = Some(self.foto()); }
        self.version += 1;
        self.cuando = ahora_s();
        let t = Instant::now();
        let (indice, _) = indexar(&self.raiz, false);
        let r = analizar(&indice, &self.raiz, &|| true);
        let mut e = Estado { raiz_absoluta: self.raiz.to_string_lossy().to_string(), ..Estado::default() };
        reducir(&mut e, Accion::Analisis(r));
        self.indice = indice;
        self.estado = e;
        self.micros = t.elapsed().as_micros() as u64;
        self.cc = crate::compilacion::leer(&self.raiz);
        self.guardian = crate::guardianes::leer(&self.raiz);
        self.reaplicar();
    }

    // ---------------- Revisiones de GCC (guardianes) ----------------

    fn ruta_revisiones(&self) -> Option<PathBuf> {
        let c = crate::analisis::ruta_cache(&self.raiz)?;
        let nombre = c.file_name()?.to_string_lossy().replacen("analisis-", "revisiones-", 1);
        Some(c.with_file_name(nombre))
    }

    fn cargar_revisiones(&self) -> std::collections::BTreeMap<String, Revision> {
        let Some(doc) = self.ruta_revisiones().and_then(|r| std::fs::read(r).ok()).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return Default::default() };
        doc.as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.clone(), Revision {
            fecha: v["fecha"].as_str()?.parse().ok()?, tam: v["tam"].as_u64()?, perfil: v["perfil"].as_str()?.into(), analizador: v["analizador"].as_bool()?,
            extra: v["extra"].as_array()?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(), cuando: v["cuando"].as_u64()?,
            diags: v["diags"].as_array()?.iter().filter_map(diag_de_json).collect(),
            deps: v["deps"].as_array().into_iter().flatten().filter_map(|d| Some((d[0].as_str()?.to_string(), d[1].as_str()?.parse().ok()?, d[2].as_u64()?))).collect() }))).collect()
    }

    fn guardar_revisiones(&self) {
        let Some(ruta) = self.ruta_revisiones() else { return };
        let doc: serde_json::Map<String, Value> = self.revisiones.iter().map(|(k, r)| (k.clone(), json!({
            "fecha": r.fecha.to_string(), "tam": r.tam, "perfil": r.perfil, "analizador": r.analizador, "extra": r.extra, "cuando": r.cuando,
            "diags": r.diags.iter().map(diag_a_json).collect::<Vec<_>>(),
            "deps": r.deps.iter().map(|(k, f, s)| json!([k, f.to_string(), s])).collect::<Vec<_>>() }))).collect();
        if let Some(d) = ruta.parent() { let _ = std::fs::create_dir_all(d); }
        let tmp = ruta.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, serde_json::to_vec(&doc).unwrap_or_default()).is_ok() { let _ = std::fs::rename(&tmp, &ruta); }
    }

    /// Vuelve a poner en el estado lo que dijo GCC de los archivos que no cambiaron; los que
    /// cambiaron quedan pendientes (su revision ya no vale).
    fn reaplicar(&mut self) {
        self.pendientes.clear();
        self.razones.clear();
        let mut leidos: Vec<(String, Diagnostico)> = Vec::new();
        for (archivo, r) in &self.revisiones {
            // Vale si ni el archivo ni ninguna de las cabeceras que GCC leyo cambiaron.
            let cambio = match fecha_y_tam(&self.raiz.join(archivo)) {
                None => continue, // ya no existe: se olvida abajo
                Some((f, s)) if f != r.fecha || s != r.tam => Some(archivo.clone()),
                Some(_) => r.deps.iter().find(|(d, f, s)| fecha_y_tam(&self.raiz.join(d)) != Some((*f, *s))).map(|(d, ..)| d.clone()),
            };
            match cambio {
                None => leidos.extend(r.diags.iter().filter(|d| d.archivo_en_proyecto).map(|d| (d.archivo.clone(),
                    Diagnostico { linea: d.linea, columna: d.columna, mensaje: d.texto(), grave: d.nivel == "error" }))),
                Some(c) => { self.razones.insert(archivo.clone(), format!("cambio {c}")); self.pendientes.push(archivo.clone()); }
            }
        }
        self.revisiones.retain(|k, _| self.raiz.join(k).exists());
        self.estado.errores.por_nodo.clear();
        self.estado.version_fallos += 1;
        self.estado.diagnosticos_externos(leidos);
    }

    /// Revisa otra vez los archivos que cambiaron (como mucho `max`), con el mismo perfil.
    pub fn revisar_pendientes(&mut self, max: usize) -> Vec<String> {
        let lista: Vec<String> = self.pendientes.iter().take(max).cloned().collect();
        let mut hechos = Vec::new();
        for archivo in lista {
            let Some(r) = self.revisiones.get(&archivo).cloned() else { continue };
            let perfil = if r.perfil == "guardian" { self.guardian.clone() } else { None };
            if let Ok((diags, _, _, deps)) = self.revisar(&archivo, perfil.as_ref(), r.analizador, &r.extra) {
                self.anotar_revision(&archivo, perfil.is_some(), r.analizador, r.extra.clone(), diags, deps);
                hechos.push(archivo);
            }
        }
        if !hechos.is_empty() { self.reaplicar(); self.guardar_revisiones(); }
        hechos
    }

    fn anotar_revision(&mut self, archivo: &str, guardian: bool, analizador: bool, extra: Vec<String>, diags: Vec<DiagGcc>, deps: Vec<String>) {
        let (fecha, tam) = fecha_y_tam(&self.raiz.join(archivo)).unwrap_or((0, 0));
        let deps = deps.into_iter().filter_map(|d| { let (f, s) = fecha_y_tam(&self.raiz.join(&d))?; Some((d, f, s)) }).collect();
        self.revisiones.insert(archivo.to_string(), Revision { fecha, tam, perfil: if guardian { "guardian".into() } else { "generico".into() },
                                                              analizador, extra, cuando: ahora_s(), diags, deps });
    }

    /// Una revision de GCC: con el perfil de los guardianes, dos pasadas como `ultra_revisar`
    /// (-O0 -fanalyzer y -O2); si no, -Wall -Wextra -O2 (y -fanalyzer si se pide).
    /// Devuelve los diagnosticos (sin repetir), las ordenes y de donde salen las flags.
    fn revisar(&self, archivo: &str, guardian: Option<&crate::guardianes::Perfil>, analizador: bool, extra: &[String]) -> Result<(Vec<DiagGcc>, Vec<String>, &'static str, Vec<String>), String> {
        let (carpeta, base, origen, entrada) = self.base_gcc(archivo);
        let pasadas: Vec<Vec<String>> = match guardian {
            Some(p) => {
                // El perfil es de C: una cabecera va como C, no como C++.
                let mut w = p.warn.clone();
                if archivo.ends_with(".h") { w.push("-xc".into()); }
                vec![[w.clone(), vec!["-O0".into(), "-fanalyzer".into()]].concat(), [w, vec!["-O2".into()]].concat()]
            }
            None => vec![[vec!["-Wall".into(), "-Wextra".into(), "-O2".into()], if analizador { vec!["-fanalyzer".into()] } else { vec![] }].concat()],
        };
        let mut diags: Vec<DiagGcc> = Vec::new();
        let mut ordenes = Vec::new();
        let mut deps: Vec<String> = Vec::new();
        for (i, pasada) in pasadas.into_iter().enumerate() {
            let mut args = base.clone();
            if guardian.is_some() { args.retain(|x| x != "-xc++" && !x.starts_with("-std=")); }
            args.extend(extra.iter().cloned());
            args.extend(pasada);
            // Compilar de verdad a /dev/null (no escribe nada): -fsyntax-only no ve los avisos
            // que necesitan optimizar ni puede correr -fanalyzer.
            // En la primera pasada, las cabeceras que GCC lee de verdad, por la salida (no al disco).
            if i == 0 { args.extend(["-MMD".into(), "-MF".into(), "/dev/stdout".into()]); }
            args.extend(["-c".into(), "-o".into(), "/dev/null".into(), "-fdiagnostics-format=sarif-stderr".into(), entrada.clone()]);
            let (salida, err) = self.gcc(&carpeta, &args)?;
            if i == 0 {
                deps = salida.split_once(':').map(|(_, d)| d).unwrap_or("").split_whitespace().filter(|x| *x != "\\")
                    .filter_map(|x| self.del_proyecto(&carpeta, x)).filter(|x| x != archivo).take(MAX_DEPS).collect();
                deps.sort();
                deps.dedup();
            }
            for d in leer_sarif(&err, &self.raiz)? {
                if !diags.iter().any(|x| x.archivo == d.archivo && x.linea == d.linea && x.columna == d.columna && x.mensaje == d.mensaje) { diags.push(d); }
            }
            ordenes.push(format!("gcc {}", args.join(" ")));
        }
        Ok((diags, ordenes, origen, deps))
    }

    /// Lo que se compara entre analisis.
    pub fn foto(&self) -> Foto {
        let an = &self.estado.analisis;
        Foto {
            version: self.version,
            sospechas: an.sospechas.iter().flat_map(|(k, v)| v.iter().map(move |s| (k.clone(), s.ficha.to_string(), s.visto.clone()))).collect(),
            fallan: carpetas_nucleo::analisis::fallos(&self.estado.errores, an),
            causas: self.estado.conectado.conexion.causas.iter().map(|c| c.ruta.clone()).collect(),
            ciclos: an.ciclos.lista.iter().map(|c| c.archivos.clone()).collect(),
            dependencias: an.dep.aristas,
            archivos: an.archivos,
        }
    }

    /// Que cambio entre el analisis anterior y el actual.
    fn cambios(&self, n: usize) -> Value {
        let Some(antes) = &self.previa else {
            return json!({ "version": self.version, "cambios": false, "nota": "es el primer analisis: aun no hay con que comparar" });
        };
        let ahora = self.foto();
        let lista = |v: Vec<Value>| { let total = v.len(); json!({ "total": total, "lista": v.into_iter().take(n).collect::<Vec<_>>() }) };
        let sospecha = |(a, f, v): &(String, String, String)| json!({ "archivo": a, "ficha": f, "eje": eje(f).nombre(), "visto": v });
        let nuevas: Vec<Value> = ahora.sospechas.difference(&antes.sospechas).map(sospecha).collect();
        let arregladas: Vec<Value> = antes.sospechas.difference(&ahora.sospechas).map(sospecha).collect();
        let hay = !nuevas.is_empty() || !arregladas.is_empty() || ahora.fallan != antes.fallan || ahora.ciclos != antes.ciclos || ahora.dependencias != antes.dependencias || ahora.archivos != antes.archivos;
        json!({
            "desde_version": antes.version, "version": self.version, "analizado_hace_s": ahora_s().saturating_sub(self.cuando), "cambios": hay,
            "sospechas_nuevas": lista(nuevas), "sospechas_arregladas": lista(arregladas),
            "fallan_ahora": lista(ahora.fallan.difference(&antes.fallan).map(|x| json!(x)).collect()),
            "ya_no_fallan": lista(antes.fallan.difference(&ahora.fallan).map(|x| json!(x)).collect()),
            "causa_principal": { "antes": antes.causas.first(), "ahora": ahora.causas.first() },
            "ciclos_nuevos": lista(ahora.ciclos.difference(&antes.ciclos).map(|x| json!(x)).collect()),
            "ciclos_rotos": lista(antes.ciclos.difference(&ahora.ciclos).map(|x| json!(x)).collect()),
            "dependencias": { "antes": antes.dependencias, "ahora": ahora.dependencias },
            "archivos": { "antes": antes.archivos, "ahora": ahora.archivos },
            "revisiones_gcc": { "pendientes": self.pendientes, "por_que": self.razones, "nota": "las de archivos que cambiaron se repiten solas (servidor MCP) o con gcc_diagnosticos" },
        })
    }

    fn guardianes(&self, n: usize) -> Value {
        json!({
            "perfil": self.guardian.as_ref().map(|p| json!({ "origen": p.origen, "banderas": p.warn, "pasadas": ["-O0 -fanalyzer", "-O2"] })),
            "revisiones": { "total": self.revisiones.len(), "pendientes": self.pendientes.len(),
                            "lista": self.revisiones.iter().take(n).map(|(k, r)| json!({ "archivo": k, "perfil": r.perfil,
                                "errores": r.diags.iter().filter(|d| d.nivel == "error").count(), "avisos": r.diags.iter().filter(|d| d.nivel != "error").count(),
                                "al_dia": !self.pendientes.contains(k), "pendiente_por": self.razones.get(k), "cabeceras_vigiladas": r.deps.len(),
                                "revisado_hace_s": ahora_s().saturating_sub(r.cuando) })).collect::<Vec<_>>() },
            "nota": "Una revision se repite sola (servidor MCP) cuando cambia su archivo o cualquiera de las cabeceras del proyecto que GCC leyo al revisarlo.",
        })
    }

    /// Lo que se ve en la ventana (por el puente), con las senales del grupo elegido.
    fn ventana(&self, n: usize) -> Value {
        let Some((v, hace)) = crate::puente::carpeta(&self.raiz).and_then(|d| crate::puente::ventana(&d)) else {
            return json!({ "abierta": false, "nota": "No hay una ventana de ULTRA Carpetas abierta en esta carpeta (o es de otra version)." });
        };
        let grupo: Vec<String> = v["grupo"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(str::to_string)).collect();
        let senales = (!grupo.is_empty()).then(|| {
            let s = carpetas_nucleo::senales::senales(&self.estado, &grupo);
            json!({ "total": s.total, "primeras": s.lista.iter().take(n).map(|x| json!({ "archivo": x.archivo, "linea": x.linea, "ficha": x.ficha, "eje": x.eje.nombre(), "texto": x.texto })).collect::<Vec<_>>() })
        });
        json!({ "abierta": hace <= 15, "escrito_hace_s": hace, "vista": v, "senales_del_grupo": senales })
    }

    /// Pide a la ventana que ensene algo (solo la vista).
    fn mostrar(&self, a: &Value) -> Result<Value, String> {
        let accion = texto(a, "accion")?;
        let mut p = json!({ "accion": accion });
        match accion {
            "grupo" | "aislar" => {
                let rutas: Vec<String> = lista(a, "rutas")?.iter().map(|r| self.existe(r)).collect::<Result<_, _>>()?;
                p["rutas"] = json!(rutas);
                if let Some(m) = a.get("modo").and_then(Value::as_str) { if !matches!(m, "ciudad" | "grafo") { return Err("modo: ciudad o grafo".into()); } p["modo"] = json!(m); }
            }
            "revelar" => {
                p["ruta"] = json!(self.existe(texto(a, "ruta")?)?);
                if let Some(l) = a.get("linea").and_then(Value::as_u64) { p["linea"] = json!(l.max(1)); }
            }
            "camino" => { p["a"] = json!(self.existe(texto(a, "a")?)?); p["b"] = json!(self.existe(texto(a, "b")?)?); }
            "limpiar" => {}
            otra => return Err(format!("accion desconocida: {otra} (grupo, revelar, camino, aislar o limpiar)")),
        }
        let dir = crate::puente::carpeta(&self.raiz).ok_or("no hay carpeta para el puente")?;
        let abierta = crate::puente::ventana(&dir).is_some_and(|(_, hace)| hace <= 15);
        if !crate::puente::pedir(&dir, &p) { return Err("no se pudo dejar la peticion".into()); }
        Ok(json!({ "enviado": p, "ventana_abierta": abierta,
                   "nota": if abierta { "La ventana lo aplica en menos de medio segundo; `ventana` dice lo que se ve despues." } else { "No hay ventana abierta: la peticion caduca en 2 minutos." } }))
    }

    /// Ejecuta una herramienta. El error es un texto para quien pregunta.
    pub fn ejecutar(&mut self, nombre: &str, a: &Value) -> Result<Value, String> {
        match nombre {
            "resumen" => Ok(self.resumen()),
            "buscar" => self.buscar(a),
            "archivo" => self.archivo(a),
            "fallos" => Ok(self.fallos(limite(a))),
            "senales" => self.senales(a),
            "camino" => self.camino(a),
            "ciclos" => Ok(self.ciclos(limite(a))),
            "peso" => self.peso(a),
            "combinar" => self.combinar(a),
            "ficha" => self.ficha(a),
            "gcc_diagnosticos" => self.gcc_diagnosticos(a),
            "gcc_deps" => self.gcc_deps(a),
            "exportar_sarif" => self.exportar_sarif(a),
            "reanalizar" => { self.analizar(); Ok(self.resumen()) }
            "cambios" => Ok(self.cambios(limite(a))),
            "guardianes" => Ok(self.guardianes(limite(a))),
            "ventana" => Ok(self.ventana(limite(a))),
            "mostrar" => self.mostrar(a),
            _ => Err(format!("no hay herramienta \"{nombre}\"")),
        }
    }

    fn existe(&self, r: &str) -> Result<String, String> {
        let r = relativa(r)?;
        if self.raiz.join(&r).exists() { Ok(r) } else { Err(format!("{r}: no existe en {}", self.raiz.display())) }
    }

    fn resumen(&self) -> Value {
        let an = &self.estado.analisis;
        let mut ejes = [0u32; 3];
        for s in an.sospechas.values().flatten() { ejes[eje(s.ficha).indice()] += 1; }
        let k = &self.estado.conectado;
        let gcc: usize = self.estado.errores.por_nodo.values().map(Vec::len).sum();
        json!({
            "carpeta": self.raiz.display().to_string(),
            "archivos_indexados": self.indice.len(),
            "archivos_leidos": an.archivos,
            "megabytes": (an.bytes as f64 / 1048576.0 * 10.0).round() / 10.0,
            "dependencias": an.dep.aristas,
            "sospechas": { "Seguridad": ejes[0], "Rendimiento": ejes[1], "Elegancia": ejes[2], "a_proposito": an.total_estudio },
            "diagnosticos_gcc": gcc,
            "fallo_conectado": { "archivos_que_fallan": k.fallos, "causas": k.conexion.causas.len(),
                                 "se_pueden_romper": k.conexion.roles.values().filter(|r| **r == Rol::Arrastra).count() },
            "ciclos": { "total": an.ciclos.total, "archivos": an.ciclos.archivos, "dependen_de_un_if": an.ciclos.condicionales },
            "analisis_ms": self.micros / 1000,
            "version": self.version,
            "reutilizados_de_la_cache": an.reutilizados,
            "guardianes": self.guardian.as_ref().map(|p| json!({ "perfil": p.origen, "banderas": p.warn.len() })),
            "revisiones_gcc": { "archivos": self.revisiones.len(), "pendientes": self.pendientes.len() },
            "compile_commands": an.compilacion.as_ref().map(|(o, b)| json!({ "archivo": o, "carpetas_de_busqueda": b,
                                                                             "archivos_con_orden": self.cc.as_ref().map(|c| c.por_archivo.len()) })),
            "recortado": an.truncado,
            "siguiente": "fallos, ciclos o senales para ver lo peor; camino/peso/archivo para bajar a un archivo",
        })
    }

    fn buscar(&self, a: &Value) -> Result<Value, String> {
        let t = texto(a, "texto")?;
        let n = limite(a);
        let v: Vec<Value> = buscar_rutas(&self.indice, t, n).into_iter().map(|i| json!({ "ruta": self.indice.ruta(i), "carpeta": self.indice.es_carpeta(i) })).collect();
        Ok(json!({ "texto": t, "rutas": v }))
    }

    fn archivo(&self, a: &Value) -> Result<Value, String> {
        let r = self.existe(texto(a, "ruta")?)?;
        let n = limite(a);
        let dep = &self.estado.analisis.dep;
        let Some(id) = dep.id(&r) else {
            return Ok(json!({ "ruta": r, "dependencias": "ninguna conocida (no es C, C++, ASM, Rust ni Python, o nada lo usa ni usa nada)",
                              "sospechas": self.sospechas_de(&r, n) }));
        };
        let usa: Vec<u32> = dep.usa_id(id).to_vec();
        let lo_usan: Vec<u32> = dep.usado_por_id(id).to_vec();
        let lista_usa: Vec<Value> = usa.iter().take(n).map(|&b| json!({ "ruta": dep.ruta(b), "motivo": motivo(&dep.motivo(id, b).cloned(), &r) })).collect();
        let lista_usan: Vec<Value> = lo_usan.iter().take(n).map(|&x| { let d = dep.ruta(x); json!({ "ruta": d, "motivo": motivo(&dep.motivo(x, id).cloned(), d) }) }).collect();
        Ok(json!({
            "ruta": r,
            "usa": { "total": usa.len(), "lista": lista_usa },
            "lo_usan": { "total": lo_usan.len(), "lista": lista_usan },
            "papel": self.estado.conectado.conexion.roles.get(&r).map(|x| rol(*x)),
            "sospechas": self.sospechas_de(&r, n),
        }))
    }

    fn sospechas_de(&self, r: &str, n: usize) -> Value {
        let v = self.estado.analisis.sospechas.get(r).map(|v| v.as_slice()).unwrap_or(&[]);
        json!({ "total": v.len(), "lista": v.iter().take(n).map(|s| json!({
            "ficha": s.ficha, "titulo": ficha(s.ficha).map(|f| f.titulo), "eje": nombre_eje(eje(s.ficha)), "linea": s.linea,
            "exacta": s.fuerza == Fuerza::Exacto, "visto": s.visto, "motivo": s.motivo })).collect::<Vec<_>>() })
    }

    fn fallos(&self, n: usize) -> Value {
        let k = &self.estado.conectado;
        json!({
            "archivos_que_fallan": k.fallos,
            "regla": "Solo la Seguridad (y los errores del compilador) es fallo. Causa probable: falla y nada de lo que usa falla.",
            "causas": { "total": k.conexion.causas.len(), "lista": k.conexion.causas.iter().take(n).map(|c| json!({
                "ruta": c.ruta, "explica": c.explica, "lo_usan": c.arrastra,
                "primera_senal": self.estado.analisis.primera(&c.ruta).map(|(_, s)| json!({ "ficha": s.ficha, "linea": s.linea, "motivo": s.motivo })) })).collect::<Vec<_>>() },
            "recortado": k.conexion.truncada,
        })
    }

    fn senales(&self, a: &Value) -> Result<Value, String> {
        let ids: Vec<String> = lista(a, "archivos")?.iter().map(|r| relativa(r)).collect::<Result<_, _>>()?;
        let n = limite(a);
        let s = carpetas_nucleo::senales::senales(&self.estado, &ids);
        Ok(json!({
            "total": s.total,
            "orden": "compilador, Seguridad exacta, Seguridad a revisar, avisos, Rendimiento, Elegancia",
            "lista": s.lista.iter().take(n).map(|x| {
                let f = x.ficha.and_then(ficha);
                json!({ "archivo": x.archivo, "linea": x.linea, "ficha": x.ficha, "eje": nombre_eje(x.eje), "exacta": x.exacta,
                        "texto": x.texto, "visto": x.visto, "titulo": f.map(|f| f.titulo), "control_valido": f.map(|f| f.control) })
            }).collect::<Vec<_>>(),
            "quedan": (s.total as usize).saturating_sub(n),
            "por_donde_llega": s.por_que.iter().map(|(id, (r, cadena))| json!({
                "archivo": id, "papel": rol(*r),
                "cadena": cadena.iter().map(|e| json!({ "desde": e.desde, "hasta": e.hasta, "motivo": motivo(&e.motivo, &e.desde) })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        }))
    }

    fn camino(&self, a: &Value) -> Result<Value, String> {
        let (x, y) = (relativa(texto(a, "a")?)?, relativa(texto(a, "b")?)?);
        let c = camino(&self.estado.analisis.dep, &x, &y);
        let union = c.union.map(|u| match u { Union::Usa => "A usa B", Union::EsUsado => "B usa A", Union::Comparten => "comparten", Union::Juntan => "los junta" });
        Ok(json!({
            "a": c.a, "b": c.b, "union": union, "por_que": c.por_que, "saltos": c.pasos.len(), "igual_de_cortos": c.igual_de_cortos,
            "punto_de_encuentro": c.punto.map(|i| c.nodos[i].clone()), "ciclo": c.ciclo,
            "pasos": c.pasos.iter().map(|p| json!({
                "desde": p.desde, "hasta": p.hasta, "motivo": motivo(&p.motivo, &p.desde), "lo_usan": p.lo_usan,
                "sin_este_paso": match p.sin { SinPaso::Imprescindible => "se separan: imprescindible".to_string(),
                                               SinPaso::Desvio(_, s) => format!("otro camino de {s} saltos"), SinPaso::SinAnalizar => "sin analizar".to_string() } })).collect::<Vec<_>>(),
        }))
    }

    fn ciclos(&self, n: usize) -> Value {
        let ci = &self.estado.analisis.ciclos;
        let uno = |c: &Ciclo| json!({
            "archivos": c.archivos.len(), "aristas": c.aristas, "depende_de_un_if": c.condicional,
            "vuelta": c.vuelta.iter().map(|e| json!({ "desde": e.desde, "hasta": e.hasta, "motivo": motivo(&e.motivo, &e.desde), "lo_usan": e.lo_usan })).collect::<Vec<_>>(),
            "cortar": c.corte.map(|k| json!({ "desde": c.vuelta[k].desde, "hasta": c.vuelta[k].hasta })),
        });
        json!({ "total": ci.total, "archivos": ci.archivos, "dependen_de_un_if": ci.condicionales,
                "ficha": "E-003 (Elegancia): compila por las guardas, pero ninguno se entiende sin los otros",
                "lista": ci.lista.iter().take(n).map(uno).collect::<Vec<_>>(), "quedan": (ci.total as usize).saturating_sub(n) })
    }

    fn peso(&self, a: &Value) -> Result<Value, String> {
        let r = self.existe(texto(a, "ruta")?)?;
        let n = limite(a);
        let dep = &self.estado.analisis.dep;
        let Some(id) = dep.id(&r) else { return Ok(json!({ "ruta": r, "se_recompilan": 0, "nota": "sin dependencias conocidas" })) };
        let (arriba, recortado) = dep.alcance(&[id], true, carpetas_nucleo::dependencias::MAX_ALCANCE);
        let directos = dep.usado_por_id(id);
        // Los que mas arrastran entre los directos: por donde se reparte el cambio.
        let mut d: Vec<(u32, usize)> = directos.iter().map(|&x| (x, dep.alcance(&[x], true, 5_000).0.len())).collect();
        d.sort_by_key(|(x, k)| (std::cmp::Reverse(*k), *x));
        Ok(json!({
            "ruta": r,
            "se_recompilan": arriba.len().saturating_sub(1),
            "lo_usan_directamente": directos.len(),
            "por_donde_se_reparte": d.iter().take(n).map(|(x, k)| json!({ "ruta": dep.ruta(*x), "arrastra": k.saturating_sub(1),
                                                                          "motivo": motivo(&dep.motivo(*x, id).cloned(), dep.ruta(*x)) })).collect::<Vec<_>>(),
            "recortado": recortado,
            "nota": "Cuenta saltos de #include del mapa lexico, no tiempo de compilacion.",
        }))
    }

    fn combinar(&self, a: &Value) -> Result<Value, String> {
        let ids: Vec<String> = lista(a, "archivos")?.iter().map(|r| relativa(r)).collect::<Result<_, _>>()?;
        let n = limite(a);
        let c = combinar(&self.estado.analisis.dep, &ids);
        let id = |i: usize| c.miembros[i].id.clone();
        Ok(json!({
            "miembros": c.miembros.iter().map(|m| json!({ "ruta": m.id, "archivos": m.archivos, "se_toca_con": m.lazos })).collect::<Vec<_>>(),
            "directas": c.directas.iter().take(n).map(|(x, y, k)| json!({ "a": id(*x), "usa": id(*y), "aristas": k })).collect::<Vec<_>>(),
            "comparten": { "total": c.total_comparten, "lista": c.comparten.iter().take(n).map(|p| json!({ "ruta": p.ruta, "lo_usan": p.miembros.iter().map(|i| id(*i)).collect::<Vec<_>>() })).collect::<Vec<_>>() },
            "los_juntan": { "total": c.total_juntan, "lista": c.juntan.iter().take(n).map(|p| json!({ "ruta": p.ruta, "usa_a": p.miembros.iter().map(|i| id(*i)).collect::<Vec<_>>() })).collect::<Vec<_>>() },
            "sueltos": c.sueltos.iter().map(|i| id(*i)).collect::<Vec<_>>(),
            "piezas": c.piezas,
            "sin_datos": c.sin_datos,
        }))
    }

    fn ficha(&self, a: &Value) -> Result<Value, String> {
        let id = texto(a, "id")?;
        let f = ficha(id).ok_or_else(|| format!("no hay ficha {id}"))?;
        Ok(json!({ "id": f.id, "titulo": f.titulo, "lenguaje": f.lenguaje, "eje": f.eje, "motivo_eje": f.motivo_eje, "regla": f.regla,
                   "control_valido": f.control, "limite": f.limite, "referencia": f.referencia, "url": f.url, "pagina": f.pagina }))
    }

    // ---------------- GCC ----------------

    /// Ejecuta GCC sin shell, en la carpeta, con tope de tiempo. Devuelve stdout y stderr.
    fn gcc(&self, carpeta: &Path, args: &[String]) -> Result<(String, String), String> {
        let mut hijo = std::process::Command::new("gcc").args(args).current_dir(carpeta)
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
            .spawn().map_err(|e| format!("no se pudo ejecutar gcc: {e}"))?;
        // Las dos salidas se leen mientras corre: si se esperara al final, GCC se bloquearia al
        // llenar la tuberia (64 KiB de SARIF o de -MM) y nunca acabaria.
        let leer = |r: Option<Box<dyn std::io::Read + Send>>| std::thread::spawn(move || {
            let mut v = Vec::new();
            if let Some(mut r) = r { let _ = std::io::Read::read_to_end(&mut r, &mut v); }
            String::from_utf8_lossy(&v).into_owned()
        });
        let salida = leer(hijo.stdout.take().map(|x| Box::new(x) as Box<dyn std::io::Read + Send>));
        let errores = leer(hijo.stderr.take().map(|x| Box::new(x) as Box<dyn std::io::Read + Send>));
        let t = Instant::now();
        loop {
            if hijo.try_wait().map_err(|e| e.to_string())?.is_some() { break; }
            if t.elapsed() > GCC_TIEMPO { let _ = hijo.kill(); let _ = hijo.wait(); return Err("gcc tardo mas de 60 s: cortado".into()); }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok((salida.join().unwrap_or_default(), errores.join().unwrap_or_default()))
    }

    /// Donde y con que flags se compila un archivo: las de compile_commands.json si las tiene
    /// (su carpeta, sus -I, -D, -std...); si no, unas genericas desde la raiz. Devuelve
    /// (carpeta, flags, de donde salen) y la ruta del archivo tal como hay que pasarla.
    fn base_gcc(&self, archivo: &str) -> (PathBuf, Vec<String>, &'static str, String) {
        if let Some(o) = self.cc.as_ref().and_then(|c| c.por_archivo.get(archivo)) {
            return (o.carpeta.clone(), o.flags.clone(), "compile_commands.json", self.raiz.join(archivo).display().to_string());
        }
        // Una cabecera .h se compila como C++ (en C tambien vale casi siempre, al reves no).
        let cpp = matches!(archivo.rsplit('.').next(), Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "mm" | "h"));
        let mut v = vec![if cpp { "-std=c++20".into() } else { "-std=c17".into() }, "-I.".into()];
        if let Some((d, _)) = archivo.rsplit_once('/') { v.push(format!("-I{d}")); }
        if archivo.ends_with(".h") { v.push("-xc++".into()); } // las cabeceras de C++ suelen acabar en .h
        (self.raiz.clone(), v, "genericas", archivo.to_string())
    }

    /// Una ruta que escribe GCC (relativa a su carpeta o absoluta) como ruta del proyecto.
    fn del_proyecto(&self, carpeta: &Path, x: &str) -> Option<String> {
        let p = Path::new(x);
        let abs = if p.is_absolute() { p.to_path_buf() } else { carpeta.join(p) };
        let abs = abs.canonicalize().unwrap_or(abs);
        abs.strip_prefix(&self.raiz).ok().map(|r| r.to_string_lossy().to_string())
    }

    fn gcc_diagnosticos(&mut self, a: &Value) -> Result<Value, String> {
        let archivo = self.existe(texto(a, "archivo")?)?;
        let n = limite(a);
        let extra = flags(a)?;
        let analizador = a.get("analizador").and_then(Value::as_bool).unwrap_or(false);
        // Los guardianes del proyecto mandan en C (perfil-gcc.sh); en C++ o sin perfil, generico.
        let es_c = matches!(archivo.rsplit('.').next(), Some("c" | "h"));
        let perfil = match a.get("perfil").and_then(Value::as_str) {
            Some("generico") => None,
            Some("guardian") => Some(self.guardian.clone().ok_or_else(|| format!("este proyecto no tiene {}", crate::guardianes::LUGAR))?),
            Some(x) => return Err(format!("perfil: guardian o generico (no {x})")),
            None => self.guardian.clone().filter(|_| es_c),
        };
        let t = Instant::now();
        let (diags, ordenes, origen, deps) = self.revisar(&archivo, perfil.as_ref(), analizador, &extra)?;
        let ms = t.elapsed().as_millis();
        let n_deps = deps.len();
        // Se guarda (con la fecha del archivo y de sus cabeceras) y vuelve al estado.
        self.anotar_revision(&archivo, perfil.is_some(), analizador, extra, diags.clone(), deps);
        self.reaplicar();
        self.guardar_revisiones();
        let errores = diags.iter().filter(|d| d.nivel == "error").count();
        Ok(json!({
            "archivo": archivo, "perfil": perfil.as_ref().map(|p| format!("guardianes ({})", p.origen)).unwrap_or_else(|| "generico".into()),
            "ordenes": ordenes, "flags_de": origen, "ms": ms, "cabeceras_vigiladas": n_deps, "analizador": analizador || perfil.is_some(),
            "errores": errores, "avisos": diags.len() - errores,
            "lista": diags.iter().take(n).map(|d| json!({ "archivo": d.archivo, "linea": d.linea, "columna": d.columna, "nivel": d.nivel, "regla": d.regla, "mensaje": d.mensaje, "pasos": d.pasos })).collect::<Vec<_>>(),
            "quedan": diags.len().saturating_sub(n),
            "fallo_conectado": { "archivos_que_fallan": self.estado.conectado.fallos },
        }))
    }

    fn gcc_deps(&self, a: &Value) -> Result<Value, String> {
        let carpeta = match a.get("carpeta").and_then(Value::as_str) { Some(c) if !c.is_empty() => relativa(c)?, _ => String::new() };
        let n = limite(a);
        let extra = flags(a)?;
        let dep = &self.estado.analisis.dep;
        // Archivos C/C++ de la carpeta con dependencias en el mapa, repartidos por toda ella.
        let todos: Vec<&String> = dep.rutas.iter()
            .filter(|r| (carpeta.is_empty() || r.starts_with(&format!("{carpeta}/"))) && matches!(r.rsplit('.').next(), Some("c" | "cc" | "cpp" | "cxx")))
            .collect();
        let paso = (todos.len() / n).max(1);
        let elegidos: Vec<&String> = todos.iter().step_by(paso).take(n).copied().collect();
        let (mut aciertos, mut solo_mapa, mut solo_gcc) = (0usize, 0usize, 0usize);
        // Lo mismo con el mapa sin los #include que estan dentro de un #if.
        let (mut aciertos_fijo, mut solo_fijo) = (0usize, 0usize);
        let mut ejemplos_mapa: Vec<Value> = Vec::new();
        let mut ejemplos_gcc: Vec<Value> = Vec::new();
        let mut fallidos = 0;
        let mut con_cc = 0;
        let t = Instant::now();
        for r in &elegidos {
            let (dir, mut args, origen, entrada) = self.base_gcc(r);
            if origen == "compile_commands.json" { con_cc += 1; }
            args.retain(|x| x != "-xc++");
            args.extend(extra.iter().cloned());
            args.extend(["-MM".into(), "-MG".into(), entrada]);
            let Ok((salida, _)) = self.gcc(&dir, &args) else { fallidos += 1; continue };
            if salida.trim().is_empty() { fallidos += 1; continue; }
            // "obj.o: a.c b.h \\\n c.h": las cabeceras que existen en el proyecto.
            let deps: std::collections::BTreeSet<String> = salida.split_once(':').map(|(_, d)| d).unwrap_or("")
                .split_whitespace().filter(|x| *x != "\\").skip(1)
                .filter_map(|x| self.del_proyecto(&dir, x)).filter(|x| dep.id(x).is_some()).collect();
            let Some(id) = dep.id(r) else { continue };
            // El mapa: todo lo que alcanza por #include (gcc -MM da el cierre transitivo).
            let mapa: std::collections::BTreeSet<String> = dep.alcance(&[id], false, 20_000).0.keys().filter(|x| **x != id).map(|x| dep.ruta(*x).to_string()).collect();
            for x in &mapa { if deps.contains(x) { aciertos += 1; } else { solo_mapa += 1; if ejemplos_mapa.len() < 5 { ejemplos_mapa.push(json!({ "archivo": r, "cabecera": x })); } } }
            for x in &deps { if !mapa.contains(x) { solo_gcc += 1; if ejemplos_gcc.len() < 5 { ejemplos_gcc.push(json!({ "archivo": r, "cabecera": x })); } } }
            for x in alcance_incondicional(dep, id) { if deps.contains(dep.ruta(x)) { aciertos_fijo += 1; } else { solo_fijo += 1; } }
        }
        let p = |a: usize, b: usize| if a + b == 0 { Value::Null } else { json!((a as f64 / (a + b) as f64 * 1000.0).round() / 10.0) };
        Ok(json!({
            "archivos_medidos": elegidos.len() - fallidos, "fallidos": fallidos, "con_compile_commands": con_cc, "ms": t.elapsed().as_millis(),
            "aciertos": aciertos, "solo_en_el_mapa": solo_mapa, "solo_en_gcc": solo_gcc,
            "precision_pct": p(aciertos, solo_mapa), "cobertura_pct": p(aciertos, solo_gcc),
            "sin_los_if": { "aciertos": aciertos_fijo, "solo_en_el_mapa": solo_fijo, "precision_pct": p(aciertos_fijo, solo_fijo),
                            "nota": "el mapa contando solo los #include que no estan dentro de un #if" },
            "ejemplos_solo_mapa": ejemplos_mapa, "ejemplos_solo_gcc": ejemplos_gcc,
            "flags": extra,
            "como": "gcc -MM -MG con -I., -I<carpeta del archivo> y las flags pedidas. Solo en el mapa: #include que GCC no siguio (otro #if, otra plataforma) o mal resuelto; solo en GCC: lo que el mapa no ve (macros, rutas -I).",
        }))
    }

    fn exportar_sarif(&self, a: &Value) -> Result<Value, String> {
        let salida = relativa(texto(a, "salida")?)?;
        if !salida.ends_with(".sarif") { return Err("la salida debe acabar en .sarif".into()); }
        let doc = sarif(&self.estado);
        let n = doc["runs"][0]["results"].as_array().map(Vec::len).unwrap_or(0);
        let destino = self.raiz.join(&salida);
        std::fs::write(&destino, serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", destino.display()))?;
        Ok(json!({ "salida": salida, "resultados": n, "formato": "SARIF 2.1.0" }))
    }
}

/// Lo que alcanza un archivo siguiendo solo los #include que no estan dentro de un #if.
fn alcance_incondicional(dep: &carpetas_nucleo::dependencias::Dependencias, id: u32) -> Vec<u32> {
    let mut vistos = std::collections::BTreeSet::from([id]);
    let mut cola = std::collections::VecDeque::from([id]);
    while let Some(x) = cola.pop_front() {
        if vistos.len() > 20_000 { break; }
        for &y in dep.usa_id(x) {
            if dep.motivo(x, y).is_some_and(|m| m.condicion.is_some()) { continue; }
            if vistos.insert(y) { cola.push_back(y); }
        }
    }
    vistos.remove(&id);
    vistos.into_iter().collect()
}

/// Flags extra de GCC pedidas: solo las que no escriben ni ejecutan nada (-I dentro de la
/// carpeta, -D, -std= y -W).
fn flags(a: &Value) -> Result<Vec<String>, String> {
    let mut v = Vec::new();
    for f in a.get("flags").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
        let bien = (f.starts_with("-I") && relativa(&f[2..]).is_ok()) || f.starts_with("-D") || f.starts_with("-std=") || (f.starts_with("-W") && !f.starts_with("-Wl,") && !f.starts_with("-Wa,") && !f.starts_with("-Wp,"));
        if !bien { return Err(format!("flag no permitida: {f} (solo -I relativa, -D, -std= y -W)")); }
        v.push(f.to_string());
    }
    Ok(v.into_iter().take(64).collect())
}

/// Un diagnostico leido del SARIF de GCC.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagGcc {
    pub archivo: String,
    pub archivo_en_proyecto: bool,
    pub linea: u32,
    pub columna: u32,
    pub nivel: String,
    pub regla: Option<String>,
    pub mensaje: String,
    /// -fanalyzer: los pasos del camino que lleva al problema (como mucho 12).
    pub pasos: Vec<String>,
}

impl DiagGcc {
    fn texto(&self) -> String { match &self.regla { Some(r) => format!("{} [{r}]", self.mensaje), None => self.mensaje.clone() } }
}

/// Lee el SARIF que GCC escribe con -fdiagnostics-format=sarif-stderr.
pub fn leer_sarif(texto: &str, raiz: &Path) -> Result<Vec<DiagGcc>, String> {
    let inicio = texto.find('{').ok_or("gcc no devolvio SARIF (¿GCC 13 o mas nuevo?)")?;
    let doc: Value = serde_json::from_str(texto[inicio..].trim()).map_err(|e| format!("SARIF ilegible: {e}"))?;
    let mut v = Vec::new();
    for run in doc["runs"].as_array().into_iter().flatten() {
        for r in run["results"].as_array().into_iter().flatten() {
            let loc = &r["locations"][0]["physicalLocation"];
            let uri = loc["artifactLocation"]["uri"].as_str().unwrap_or("");
            let ruta = uri.strip_prefix("file://").unwrap_or(uri);
            let abs = Path::new(ruta);
            let (archivo, dentro) = match abs.strip_prefix(raiz) {
                Ok(p) => (p.to_string_lossy().to_string(), true),
                Err(_) if abs.is_relative() => (relativa(ruta).unwrap_or_else(|_| ruta.to_string()), relativa(ruta).is_ok()),
                Err(_) => (ruta.to_string(), false),
            };
            let pasos: Vec<String> = r["codeFlows"][0]["threadFlows"][0]["locations"].as_array().into_iter().flatten().take(12).map(|p| {
                let l = &p["location"];
                format!("linea {}: {}", l["physicalLocation"]["region"]["startLine"].as_u64().unwrap_or(0), l["message"]["text"].as_str().unwrap_or(""))
            }).collect();
            v.push(DiagGcc {
                archivo, archivo_en_proyecto: dentro,
                linea: loc["region"]["startLine"].as_u64().unwrap_or(0) as u32,
                columna: loc["region"]["startColumn"].as_u64().unwrap_or(0) as u32,
                nivel: r["level"].as_str().unwrap_or("warning").to_string(),
                regla: r["ruleId"].as_str().map(str::to_string),
                mensaje: r["message"]["text"].as_str().unwrap_or("").to_string(),
                pasos,
            });
        }
    }
    Ok(v)
}

/// Una linea de la terminal con el SARIF de GCC (-fdiagnostics-format=sarif-stderr): se
/// convierte en lineas como las de siempre ("archivo:linea:columna: error: mensaje"), que
/// el lector de diagnosticos ya entiende y marca en sus nodos.
pub fn sarif_a_lineas(linea: &str) -> Option<Vec<String>> {
    let l = linea.trim_start();
    if !(l.starts_with('{') && l.contains("\"runs\"") && l.contains("2.1.0")) { return None; }
    let v = leer_sarif(l, Path::new("")).ok()?;
    Some(v.into_iter().map(|d| {
        let nivel = match d.nivel.as_str() { "error" => "error", "note" => "note", _ => "warning" };
        format!("{}:{}:{}: {nivel}: {}", d.archivo, d.linea, d.columna.max(1), d.texto())
    }).collect())
}

/// Las sospechas del museo (y los diagnosticos de GCC leidos) en SARIF 2.1.0.
pub fn sarif(e: &Estado) -> Value {
    let an = &e.analisis;
    let fichas: std::collections::BTreeSet<&str> = an.sospechas.values().flatten().map(|s| s.ficha).collect();
    let reglas: Vec<Value> = fichas.iter().filter_map(|id| ficha(id)).map(|f| json!({
        "id": f.id, "name": f.titulo, "shortDescription": { "text": f.titulo }, "fullDescription": { "text": f.regla },
        "help": { "text": format!("Control valido: {}\nLimite: {}", f.control, f.limite) }, "helpUri": f.url,
        "properties": { "eje": f.eje, "lenguaje": f.lenguaje, "referencia": f.referencia },
    })).collect();
    let mut resultados: Vec<Value> = Vec::new();
    for (archivo, v) in &an.sospechas {
        for s in v {
            let nivel = match (eje(s.ficha), s.fuerza) { (Eje::Seguridad, Fuerza::Exacto) => "error", (Eje::Seguridad, _) => "warning", _ => "note" };
            resultados.push(json!({
                "ruleId": s.ficha, "level": nivel, "message": { "text": s.motivo },
                "locations": [{ "physicalLocation": { "artifactLocation": { "uri": archivo }, "region": { "startLine": s.linea.max(1), "startColumn": s.columna.max(1), "snippet": { "text": s.visto } } } }],
                "properties": { "eje": nombre_eje(eje(s.ficha)), "fuerza": if s.fuerza == Fuerza::Exacto { "exacto" } else { "revisar" } },
            }));
        }
    }
    for (archivo, v) in &e.errores.por_nodo {
        for d in v {
            resultados.push(json!({ "ruleId": "gcc", "level": if d.grave { "error" } else { "warning" }, "message": { "text": d.mensaje },
                "locations": [{ "physicalLocation": { "artifactLocation": { "uri": archivo }, "region": { "startLine": d.linea.max(1), "startColumn": d.columna.max(1) } } }] }));
        }
    }
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json", "version": "2.1.0",
        "runs": [{ "tool": { "driver": { "name": "SUPER GRAFO (ULTRA Carpetas)", "informationUri": "https://github.com/AndreeSalazar/ULTRA-LinuX", "rules": reglas } },
                   "originalUriBaseIds": { "RAIZ": { "uri": format!("file://{}/", e.raiz_absoluta) } },
                   "results": resultados }],
    })
}

/// `ultra-carpetas --consulta <herramienta> <carpeta> [json]`: una respuesta y fuera.
pub fn linea_de_ordenes(args: &[String]) -> i32 {
    let (Some(h), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("uso: ultra-carpetas --consulta <herramienta> <carpeta> ['{{\"limite\": 5}}']\nherramientas: {}",
                  herramientas().iter().filter_map(|h| h["name"].as_str().map(str::to_string)).collect::<Vec<_>>().join(", "));
        return 2;
    };
    let a: Value = match args.get(2) { Some(t) => match serde_json::from_str(t) { Ok(v) => v, Err(e) => { eprintln!("argumentos JSON: {e}"); return 2; } }, None => json!({}) };
    let mut m = match Mapa::abrir(Path::new(dir)) { Ok(m) => m, Err(e) => { eprintln!("{e}"); return 1; } };
    match m.ejecutar(h, &a) {
        Ok(v) => { println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn mapa(caso: &str) -> Mapa { Mapa::abrir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../carpetas-nucleo/tests/datos/ciclos").join(caso)).unwrap() }

    #[test]
    fn las_consultas_contestan_exacto_y_con_tope() {
        let mut m = mapa("mal");
        let r = m.ejecutar("resumen", &json!({})).unwrap();
        assert_eq!(r["ciclos"]["total"], 1);
        assert_eq!(r["dependencias"], 4);
        let c = m.ejecutar("camino", &json!({ "a": "a.h", "b": "c.h" })).unwrap();
        assert_eq!(c["union"], "A usa B");
        assert_eq!(c["ciclo"], true);
        assert_eq!(c["pasos"][0]["motivo"]["escrito"], "#include \"b.h\"");
        let p = m.ejecutar("peso", &json!({ "ruta": "c.h" })).unwrap();
        assert_eq!(p["se_recompilan"], 3, "a.h, b.h y main.c (el ciclo y quien lo usa)");
        let b = m.ejecutar("buscar", &json!({ "texto": "main", "limite": 1 })).unwrap();
        assert_eq!(b["rutas"].as_array().unwrap().len(), 1);
        assert_eq!(m.ejecutar("ficha", &json!({ "id": "E-003" })).unwrap()["titulo"], "Ciclo de cabeceras");
        // Rutas fuera de la carpeta y flags peligrosas: no.
        assert!(m.ejecutar("archivo", &json!({ "ruta": "/etc/passwd" })).is_err());
        assert!(m.ejecutar("archivo", &json!({ "ruta": "../../x" })).is_err());
        assert!(m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "main.c", "flags": ["-o", "x"] })).is_err());
        assert!(m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "main.c", "flags": ["-I../.."] })).is_err());
        assert!(m.ejecutar("exportar_sarif", &json!({ "salida": "x.txt" })).is_err());
        assert!(m.ejecutar("nada", &json!({})).is_err());
    }

    #[test]
    fn los_guardianes_revisan_guardan_y_vuelven_a_revisar_solos() {
        if std::process::Command::new("gcc").arg("--version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("ultra-guardian-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/guardian/produccion")).unwrap();
        std::fs::write(dir.join(crate::guardianes::LUGAR), "ULTRA_WARN=(-std=c17 -Wall -Wextra -Werror -Wvla)\n").unwrap();
        std::fs::write(dir.join("a.c"), "int f(int n) { return n + 1; }\n").unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "a.c" })).unwrap();
        assert!(r["perfil"].as_str().unwrap().starts_with("guardianes"));
        assert_eq!(r["ordenes"].as_array().unwrap().len(), 2, "dos pasadas, como ultra_revisar");
        assert_eq!(r["errores"], 0);
        // Alguien mete un VLA: al reanalizar, la revision queda pendiente y se repite sola.
        std::fs::write(dir.join("a.c"), "int f(int n) { char b[n]; b[0] = 1; return b[0]; }\n").unwrap();
        m.analizar();
        assert_eq!(m.pendientes, ["a.c"]);
        assert_eq!(m.revisar_pendientes(4), ["a.c"]);
        let c = m.ejecutar("cambios", &json!({})).unwrap();
        assert_eq!(c["fallan_ahora"]["lista"][0], "a.c", "el guardian lo hace fallar: {c}");
        // Reanalizar sin cambios no borra lo que dijo el guardian.
        m.analizar();
        assert!(m.pendientes.is_empty());
        assert_eq!(m.ejecutar("fallos", &json!({})).unwrap()["archivos_que_fallan"], 1);
        // Y sobrevive a reiniciar (cache).
        let otro = Mapa::abrir(&dir).unwrap();
        assert_eq!(otro.revisiones.len(), 1);
        assert_eq!(otro.estado.conectado.fallos, 1);
        if let Some(r) = otro.ruta_revisiones() { std::fs::remove_file(r).ok(); }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn si_cambia_una_cabecera_el_guardian_revisa_a_quien_la_usa() {
        if std::process::Command::new("gcc").arg("--version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("ultra-guardian-h-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/guardian/produccion")).unwrap();
        std::fs::create_dir_all(dir.join("inc")).unwrap();
        std::fs::write(dir.join(crate::guardianes::LUGAR), "ULTRA_WARN=(-std=c17 -Wall -Wextra -Werror -Wvla)\n").unwrap();
        std::fs::write(dir.join("inc/util.h"), "static inline int doble(int n) { return 2 * n; }\n").unwrap();
        std::fs::write(dir.join("main.c"), "#include \"inc/util.h\"\nint f(int n) { return doble(n); }\n").unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "main.c" })).unwrap();
        assert_eq!((r["errores"].as_u64(), r["cabeceras_vigiladas"].as_u64()), (Some(0), Some(1)), "GCC dijo que lee inc/util.h: {r}");
        // Solo cambia la cabecera (un VLA): la revision de main.c queda pendiente por ella.
        std::fs::write(dir.join("inc/util.h"), "static inline int doble(int n) { char b[n]; b[0] = 2; return b[0] * n; }\n").unwrap();
        m.analizar();
        assert_eq!(m.pendientes, ["main.c"]);
        assert_eq!(m.razones["main.c"], "cambio inc/util.h");
        m.revisar_pendientes(4);
        let c = m.ejecutar("cambios", &json!({})).unwrap();
        assert!(c["fallan_ahora"]["lista"].as_array().unwrap().iter().any(|x| x == "inc/util.h"), "el guardian ve el VLA de la cabecera: {c}");
        if let Some(r) = m.ruta_revisiones() { std::fs::remove_file(r).ok(); }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn el_control_no_tiene_ciclos() {
        let mut m = mapa("bien");
        assert_eq!(m.ejecutar("ciclos", &json!({})).unwrap()["total"], 0);
    }

    #[test]
    fn lee_el_sarif_de_gcc() {
        let sarif = r#"{"$schema":"x","version":"2.1.0","runs":[{"results":[
            {"ruleId":"-Wfree-nonheap-object","level":"warning","message":{"text":"'free' called on pointer 'p' with nonzero offset 1"},
             "locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.c"},"region":{"startLine":5,"startColumn":5}}}]},
            {"level":"error","message":{"text":"expected ';'"},
             "locations":[{"physicalLocation":{"artifactLocation":{"uri":"/usr/include/stdio.h"},"region":{"startLine":1}}}],
             "codeFlows":[{"threadFlows":[{"locations":[{"location":{"physicalLocation":{"region":{"startLine":3}},"message":{"text":"allocated here"}}}]}]}]}]}]}"#;
        let v = leer_sarif(sarif, Path::new("/proyecto")).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].archivo.as_str(), v[0].linea, v[0].nivel.as_str(), v[0].regla.as_deref()), ("src/a.c", 5, "warning", Some("-Wfree-nonheap-object")));
        assert!(v[0].archivo_en_proyecto);
        assert!(!v[1].archivo_en_proyecto, "una cabecera del sistema no es del proyecto");
        assert_eq!(v[1].pasos, ["linea 3: allocated here"]);
        assert!(leer_sarif("nada", Path::new("/")).is_err());
        // En la terminal: el bloque SARIF pasa a lineas de siempre (rutas tal cual).
        let l = sarif_a_lineas(&sarif.replace('\n', " ")).unwrap();
        assert_eq!(l[0], "src/a.c:5:5: warning: 'free' called on pointer 'p' with nonzero offset 1 [-Wfree-nonheap-object]");
        assert_eq!(l[1], "/usr/include/stdio.h:1:1: error: expected ';'");
        assert!(sarif_a_lineas("hola").is_none());
    }

    #[test]
    fn gcc_confirma_el_museo_y_entra_en_el_fallo() {
        if std::process::Command::new("gcc").arg("--version").output().is_err() { return; }
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../carpetas-nucleo/tests/datos/museo/C-001");
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "mal.c" })).unwrap();
        let msgs: Vec<&str> = r["lista"].as_array().unwrap().iter().filter_map(|d| d["mensaje"].as_str()).collect();
        assert!(msgs.iter().any(|x| x.contains("nonzero offset 1")), "GCC ve el free(p + 1) de C-001: {msgs:?}");
        // El SARIF exportado lleva la sospecha del museo y el aviso de GCC.
        let doc = sarif(&m.estado);
        let reglas: Vec<&str> = doc["runs"][0]["results"].as_array().unwrap().iter().filter_map(|x| x["ruleId"].as_str()).collect();
        assert!(reglas.contains(&"C-001") && reglas.contains(&"gcc"), "{reglas:?}");
        assert_eq!(doc["version"], "2.1.0");
    }
}
