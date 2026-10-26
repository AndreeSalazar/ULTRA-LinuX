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
use crate::compilacion::Compilador;
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
    /// Modo guardia: que archivos mantener revisados (el criterio, no la lista: un .c nuevo
    /// de la carpeta entra solo). None: apagado.
    guardia: Option<Guardia>,
    /// La firma de la carpeta tomada con el indice del ultimo analisis (antes de leer: si algo
    /// cambia mientras, la proxima firma sera distinta y se reanaliza).
    pub firma: (u64, usize),
    /// La cronologia de git (se lee al pedirla; se rehace si cambia el ultimo commit).
    crono: Option<Crono>,
    /// Por que esta pendiente cada una ("cambio a.c", "cambio base/x.h").
    razones: std::collections::BTreeMap<String, String>,
}

/// El criterio del modo guardia.
#[derive(Debug, Clone, PartialEq)]
pub struct Guardia {
    /// La carpeta a guardar ("" = todo el proyecto).
    carpeta: String,
    /// Tambien C++ (con el perfil generico: el de los guardianes es de C).
    cpp: bool,
    /// Tambien los casos a proposito (carpetas con CASOS-MUSEO.txt): sus minas fallarian.
    estudio: bool,
}

/// Cuantos archivos como mucho en guardia.
const MAX_GUARDIA: usize = 5000;

/// Una revision de GCC de un archivo: con que perfil, cuando, y que dijo.
#[derive(Debug, Clone, PartialEq)]
pub struct Revision {
    fecha: u128,
    tam: u64,
    /// "guardian" (perfil-gcc.sh, como ultra_revisar) o "generico".
    perfil: String,
    analizador: bool,
    extra: Vec<String>,
    /// Con que compilador se hizo (las guardadas sin el campo son de GCC).
    compilador: Compilador,
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
///
/// La firma y el indice con el que se hizo (para analizar con ese mismo, sin recorrer la
/// carpeta otra vez: con Chromium son 0,8 s).
pub fn firma_e_indice(raiz: &Path) -> ((u64, usize), Indice) {
    let (indice, _) = indexar(raiz, false);
    (firma_de(raiz, &indice), indice)
}

fn firma_de(raiz: &Path, indice: &Indice) -> (u64, usize) {
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
        Some(m) => {
            let mut v = json!({ "linea": m.linea, "escrito": m.escrito(desde), "condicion": m.condicion });
            if m.exportada { v["exportada"] = json!(true); }
            v
        }
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
        h("gcc_diagnosticos", "Revisa un archivo con GCC o Clang (compila a /dev/null: no escribe nada). compilador auto: el de compile_commands.json (Chromium: clang), si no gcc. En C con GCC usa el perfil de los guardianes del proyecto (perfil-gcc.sh: ULTRA_WARN, dos pasadas como ultra_revisar, -O0 -fanalyzer y -O2); si no, -Wall -Wextra -O2 (+ -fanalyzer de GCC o --analyze de Clang si se pide: dan el camino de cada fallo en pasos). La revision se guarda: sobrevive al reanalisis y se repite sola, con el mismo compilador, si el archivo cambia.",
          json!({ "archivo": ruta, "perfil": { "type": "string", "enum": ["guardian", "generico"] }, "analizador": { "type": "boolean" }, "compilador": compilador_esquema(), "flags": { "type": "array", "items": { "type": "string" }, "description": "Flags extra: solo -I, -D, -std= y -W" }, "limite": lim }), &["archivo"]),
 h("guardia", "Modo guardia: mantener revisados por los guardianes (GCC) todos los archivos de C de una carpeta (o del proyecto), por turnos y despues solo lo que cambie. Sin casos a proposito (CASOS-MUSEO.txt) salvo que se pida.",
          json!({ "carpeta": ruta, "activar": { "type": "boolean", "description": "false la apaga" }, "cpp": { "type": "boolean", "description": "tambien C++ (perfil generico)" }, "estudio": { "type": "boolean", "description": "tambien los casos a proposito" } }), &[]),
        h("guardianes", "El perfil de los guardianes (banderas de perfil-gcc.sh) y las revisiones de GCC guardadas: archivo, perfil, errores, avisos y si siguen al dia.", json!({ "limite": lim }), &[]),
        h("cobertura_ub", "Lee el informe historico del banco LLVM C17/C++20 por perfil y lenguaje; muestra fallos y pendientes sin ejecutar minas. No certifica ausencia de UB ni produccion.", json!({ "limite": lim }), &[]),
        h("revision_estado", "Revalida con el motor instalado de ULTRA los hashes de fuentes, includes y herramientas. Devuelve revisado, caducado, pendiente o diagnostico; no ejecuta programas del proyecto ni confirma UB de runtime.", json!({ "limite": lim }), &[]),
        h("gcc_deps", "Mide cuanto acierta el mapa lexico: para unos archivos C/C++ compara sus cabeceras del proyecto con las que dice gcc -M -MG (con #if resueltos); no mide los que piden cabeceras generadas que faltan. Pasar las -I reales del proyecto en flags.",
          json!({ "carpeta": ruta, "flags": { "type": "array", "items": { "type": "string" }, "description": "-I relativas, -D, -std=" }, "compilador": compilador_esquema(), "limite": lim }), &[]),
        h("exportar_sarif", "Escribe las sospechas del museo (y los diagnosticos de GCC leidos) en un archivo SARIF 2.1.0 dentro de la carpeta.", json!({ "salida": ruta }), &["salida"]),
        h("reanalizar", "Vuelve a leer la carpeta ahora (el servidor MCP ya reanaliza solo al ver cambios; incremental: solo relee lo que cambio).", json!({}), &[]),
        h("ventana", "Lo que la persona esta mirando ahora en la ventana de ULTRA Carpetas (vista, grupo elegido, seleccion, camino, archivo del editor), con las senales de su grupo. Sin capturas.", json!({ "limite": lim }), &[]),
        h("mostrar", "Ensena algo en la ventana de la persona (solo mueve la vista): accion grupo (rutas, modo ciudad|grafo), revelar (ruta, linea), camino (a, b), aislar (rutas) o limpiar.",
          json!({ "accion": { "type": "string", "enum": ["grupo", "revelar", "camino", "aislar", "limpiar"] }, "rutas": rutas, "ruta": ruta, "linea": { "type": "integer", "minimum": 1 },
                  "a": ruta, "b": ruta, "modo": { "type": "string", "enum": ["ciudad", "grafo"] } }), &["accion"]),
        h("cronologia", "Como se construyo el proyecto desde el primer commit (git), sin sacar versiones viejas: sin argumentos, como crecio por periodos (archivos, dependencias, ciclos, con el mapa de cada momento resuelto como hoy), en que orden nacieron las carpetas y una verificacion (lo reconstruido contra lo de hoy). ruta: cuando nacio un archivo o carpeta, quien, por que (asunto del commit), sus nombres anteriores y como crecio. a y b: que commit anadio la referencia que une a con b. ciclos: que commit cerro cada ciclo de hoy.",
          json!({ "ruta": ruta, "a": ruta, "b": ruta, "ciclos": { "type": "boolean" }, "periodos": { "type": "integer", "minimum": 1, "maximum": 100 },
                  "profundidad": { "type": "integer", "minimum": 1, "maximum": 4, "description": "carpetas de hasta tantos niveles en el orden de nacimiento (1 por defecto)" }, "limite": lim }), &[]),
        h("cambios", "Que cambio desde el analisis anterior: sospechas nuevas y arregladas, archivos que empiezan o dejan de fallar, la causa principal, ciclos nuevos y rotos.", json!({ "limite": lim }), &[]),
    ]
}

impl Mapa {
    pub fn abrir(raiz: &Path) -> Result<Mapa, String> {
        let raiz = raiz.canonicalize().map_err(|e| format!("{}: {e}", raiz.display()))?;
        if !raiz.is_dir() { return Err(format!("{}: no es una carpeta", raiz.display())); }
        let mut m = Mapa { raiz, indice: Indice::default(), estado: Estado::default(), micros: 0, cc: None, version: 0, previa: None, cuando: 0,
                           guardian: None, revisiones: Default::default(), pendientes: Vec::new(), razones: Default::default(), guardia: None, firma: (0, 0), crono: None };
        m.revisiones = m.cargar_revisiones();
        m.guardia = m.ruta_guardia().and_then(|r| std::fs::read(r).ok()).and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| Some(Guardia { carpeta: v["carpeta"].as_str()?.into(), cpp: v["cpp"].as_bool()?, estudio: v["estudio"].as_bool()? }));
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
        self.firma = firma_de(&self.raiz, &indice);
        self.analizar_con(indice, t);
    }

    /// Analiza con un indice ya hecho y su firma (el vigia los acaba de sacar).
    pub fn reanalizar_con(&mut self, indice: Indice, firma: (u64, usize)) {
        if self.version > 0 { self.previa = Some(self.foto()); }
        self.version += 1;
        self.cuando = ahora_s();
        self.firma = firma;
        self.analizar_con(indice, Instant::now());
    }

    fn analizar_con(&mut self, indice: Indice, t: Instant) {
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

    // ---------------- Cronologia (git) ----------------

    /// La cronologia al dia: se lee si no hay, si cambio el ultimo commit o si se piden
    /// otros periodos.
    fn crono(&mut self, periodos: usize) -> Result<&Crono, String> {
        let mut cabeza = String::new();
        let _ = std::process::Command::new("git").arg("-C").arg(&self.raiz).args(["rev-parse", "HEAD"])
            .env("GIT_OPTIONAL_LOCKS", "0").stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output()
            .map(|o| cabeza = String::from_utf8_lossy(&o.stdout).trim().to_string());
        let vale = self.crono.as_ref().is_some_and(|c| c.cabeza == cabeza && c.periodos == periodos);
        if !vale {
            let t = Instant::now();
            let (c, cortada, superficial) = crate::git::cronologia(&self.raiz)?;
            if c.commits.is_empty() { return Err("el repositorio no tiene commits".into()); }
            let rep = carpetas_nucleo::cronologia::reproducir(&c, periodos);
            self.crono = Some(Crono { cabeza, c, rep, periodos, cortada, superficial, ms: t.elapsed().as_millis() });
        }
        Ok(self.crono.as_ref().expect("recien hecha"))
    }

    fn cronologia(&mut self, a: &Value) -> Result<Value, String> {
        use carpetas_nucleo::cronologia::recorrer;
        let n = limite(a);
        let periodos = a.get("periodos").and_then(Value::as_u64).map(|x| (x as usize).clamp(1, 100)).unwrap_or(10);
        let profundidad = a.get("profundidad").and_then(Value::as_u64).map(|x| (x as usize).clamp(1, 4)).unwrap_or(1);
        // Lo que hay hoy (para verificar y para los ciclos), antes de tomar prestada la cronologia.
        let hoy: std::collections::BTreeSet<String> = (0..self.indice.len()).filter(|k| !self.indice.es_carpeta(*k)).map(|k| self.indice.ruta(k).to_string()).collect();
        let ciclos_hoy = self.estado.analisis.ciclos.clone();
        let dep_hoy = &self.estado.analisis.dep;
        let aristas_hoy: std::collections::BTreeSet<(String, String)> = dep_hoy.rutas.iter()
            .flat_map(|x| dep_hoy.usa(x).map(move |y| (x.clone(), y.to_string()))).collect();
        let cr = self.crono(periodos)?;
        let (c, rep) = (&cr.c, &cr.rep);
        let commit = |i: u32| { let x = &c.commits[i as usize]; json!({ "commit": x.id, "fecha": fecha_iso(x.fecha), "autor": c.autores[x.autor as usize], "asunto": x.asunto }) };
        let nota = "git sin versiones viejas: archivos por --name-status y referencias por las lineas #include/mod/use/import que entran y salen (-G). El mapa de cada momento se resuelve con las reglas de hoy. No se ve lo que un #include no escribe (macros, rutas generadas) ni el #if de una linea suelta; los merges no cuentan y el orden es el de git log.";
        let base = json!({ "commits": c.commits.len(), "autores": c.autores.len(), "desde": fecha_iso(c.commits[0].fecha), "hasta": fecha_iso(c.commits[c.commits.len() - 1].fecha),
                           "leida_ms": cr.ms, "cortada": cr.cortada, "superficial": cr.superficial, "nota": nota });
        let mut v = base;
        if cr.superficial { v["aviso"] = json!("repositorio superficial (clonado sin toda la historia): el primer commit no es el de verdad; git fetch --unshallow lo trae entero"); }
        // Una arista: que commit anadio la referencia que une a con b.
        if let (Some(x), Some(y)) = (a.get("a").and_then(Value::as_str), a.get("b").and_then(Value::as_str)) {
            let (x, y) = (relativa(x)?, relativa(y)?);
            let Some(m) = self.estado.analisis.dep.id(&x).zip(self.estado.analisis.dep.id(&y)).and_then(|(i, j)| self.estado.analisis.dep.motivo(i, j).cloned())
                else { return Err(format!("hoy {x} no usa a {y} (sin arista en el mapa)")); };
            let cr = self.crono.as_ref().expect("leida arriba");
            let c = &cr.c;
            let entra = c.id_ruta(&x).zip(c.id_texto(&m.texto)).and_then(|k| cr.rep.entra.get(&k).copied());
            v["arista"] = json!({ "a": x, "b": y, "escrito": m.escrito(&x),
                "entra": entra.map(|i| { let k = &c.commits[i as usize]; json!({ "commit": k.id, "fecha": fecha_iso(k.fecha), "autor": c.autores[k.autor as usize], "asunto": k.asunto }) }),
                "si_falta": "una referencia que git no vio entrar (escrita con una macro, o en una linea que la lectura no reconoce)" });
            return Ok(v);
        }
        if a.get("ciclos").and_then(Value::as_bool) == Some(true) {
            let lista: Vec<Value> = ciclos_hoy.lista.iter().take(n).map(|ci| {
                // El ciclo se cerro con la ultima referencia de su vuelta que entro.
                let entradas: Vec<(Option<u32>, &carpetas_nucleo::ciclos::Eslabon)> = ci.vuelta.iter().map(|e| {
                    let t = e.motivo.as_ref().map(|m| m.texto.as_str()).unwrap_or("");
                    (c.id_ruta(&e.desde).zip(c.id_texto(t)).and_then(|k| rep.entra.get(&k).copied()), e)
                }).collect();
                let cierre = entradas.iter().filter_map(|(i, e)| i.map(|i| (i, *e))).max_by_key(|(i, _)| *i);
                json!({ "archivos": ci.archivos.len(), "vuelta": ci.vuelta.iter().map(|e| e.desde.as_str()).collect::<Vec<_>>(),
                        "se_cerro": cierre.map(|(i, e)| json!({ "con": e.motivo.as_ref().map(|m| m.escrito(&e.desde)), "en": e.desde, "por": commit(i) })),
                        "sin_ver_entrar": entradas.iter().filter(|(i, _)| i.is_none()).count() })
            }).collect();
            v["ciclos"] = json!({ "hoy": ciclos_hoy.total, "lista": lista });
            return Ok(v);
        }
        if let Some(r) = a.get("ruta").and_then(Value::as_str) {
            let r = relativa(r)?;
            if let Some(k) = c.id_ruta(&r).filter(|k| rep.vidas.contains_key(k)) {
                let vida = &rep.vidas[&k];
                v["archivo"] = json!({ "ruta": r, "nace": commit(vida.nace), "cambios": vida.cambios, "vivo": vida.muere.is_none(),
                    "antes_se_llamaba": vida.antes.iter().map(|(x, i)| json!({ "ruta": c.ruta_de(*x), "hasta": commit(*i) })).collect::<Vec<_>>(),
                    "muere": vida.muere.map(commit) });
                return Ok(v);
            }
            let Some(&nace) = rep.carpetas.get(&r) else { return Err(format!("{r}: git no lo conoce (ni archivo ni carpeta con historia)")) };
            let prefijo = format!("{r}/");
            let mut crece = Vec::new();
            recorrer(c, periodos, |e, i| {
                let dentro = e.vivos.iter().filter(|k| c.ruta_de(**k).starts_with(&prefijo)).count();
                crece.push(json!({ "fecha": fecha_iso(c.commits[i as usize].fecha), "commit": c.commits[i as usize].id, "archivos": dentro }));
            });
            v["carpeta"] = json!({ "ruta": r, "nace": commit(nace), "crecimiento": crece });
            return Ok(v);
        }
        // Sin argumentos: el crecimiento, las carpetas en orden y la verificacion.
        v["crecimiento"] = json!(rep.fotos.iter().map(|f| {
            let x = &c.commits[f.commit as usize];
            json!({ "fecha": fecha_iso(x.fecha), "commit": x.id, "asunto": x.asunto, "archivos": f.archivos, "codigo": f.codigo,
                    "dependencias": f.dependencias, "ciclos": f.ciclos })
        }).collect::<Vec<_>>());
        let mut carpetas: Vec<(&String, &u32)> = rep.carpetas.iter().filter(|(k, _)| k.split('/').count() <= profundidad).collect();
        carpetas.sort_by_key(|(k, i)| (**i, (*k).clone()));
        v["carpetas_en_orden"] = json!(carpetas.iter().take(n).map(|(k, i)| json!({ "carpeta": k, "nace": commit(**i) })).collect::<Vec<_>>());
        v["carpetas_total"] = json!(carpetas.len());
        // Verificacion: lo reconstruido al final contra lo que hay hoy en la carpeta.
        // Lo oculto (.gitignore, .github/) no esta en el indice: no se compara.
        let reconstruidos: std::collections::BTreeSet<&str> = rep.vivos.iter().map(|k| c.ruta_de(*k)).filter(|r| !r.split('/').any(|p| p.starts_with('.'))).collect();
        let coinciden = reconstruidos.iter().filter(|r| hoy.contains(**r)).count();
        let solo_git: Vec<&&str> = reconstruidos.iter().filter(|r| !hoy.contains(**r)).take(5).collect();
        v["verificacion"] = json!({ "reconstruidos": reconstruidos.len(), "hoy": hoy.len(), "coinciden": coinciden,
            "pct_de_lo_reconstruido": if reconstruidos.is_empty() { 0.0 } else { (coinciden as f64 * 1000.0 / reconstruidos.len() as f64).round() / 10.0 },
            "solo_en_git": solo_git, "nota": "hoy cuenta tambien lo que git no sigue (ignorado o generado); solo_en_git deberia estar vacio" });
        // Y las dependencias: el mapa reconstruido al final contra el de hoy.
        let (_, con_texto) = rep.mapa(c);
        let aristas_git: std::collections::BTreeSet<(String, String)> = con_texto.iter().map(|(x, y, _)| (c.ruta_de(*x).to_string(), c.ruta_de(*y).to_string())).collect();
        let iguales = aristas_git.intersection(&aristas_hoy).count();
        v["verificacion"]["dependencias"] = json!({ "reconstruidas": aristas_git.len(), "hoy": aristas_hoy.len(), "coinciden": iguales,
            "solo_hoy": aristas_hoy.difference(&aristas_git).take(5).map(|(x, y)| format!("{x} -> {y}")).collect::<Vec<_>>(),
            "solo_en_git": aristas_git.difference(&aristas_hoy).take(5).map(|(x, y)| format!("{x} -> {y}")).collect::<Vec<_>>(),
            "nota": "solo_hoy: lo que aun no esta en un commit, o un use de Rust partido en varias lineas" });
        Ok(v)
    }

    // ---------------- Revisiones de GCC (guardianes) ----------------

    fn ruta_revisiones(&self) -> Option<PathBuf> {
        let c = crate::analisis::ruta_cache(&self.raiz)?;
        let nombre = c.file_name()?.to_string_lossy().replacen("analisis-", "revisiones-", 1);
        Some(c.with_file_name(nombre).with_extension("json"))
    }

    fn cargar_revisiones(&self) -> std::collections::BTreeMap<String, Revision> {
        let Some(doc) = self.ruta_revisiones().and_then(|r| std::fs::read(r).ok()).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return Default::default() };
        doc.as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.clone(), Revision {
            fecha: v["fecha"].as_str()?.parse().ok()?, tam: v["tam"].as_u64()?, perfil: v["perfil"].as_str()?.into(), analizador: v["analizador"].as_bool()?,
            compilador: v["compilador"].as_str().and_then(Compilador::de_nombre).unwrap_or_default(),
            extra: v["extra"].as_array()?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(), cuando: v["cuando"].as_u64()?,
            diags: v["diags"].as_array()?.iter().filter_map(diag_de_json).collect(),
            deps: v["deps"].as_array().into_iter().flatten().filter_map(|d| Some((d[0].as_str()?.to_string(), d[1].as_str()?.parse().ok()?, d[2].as_u64()?))).collect() }))).collect()
    }

    fn guardar_revisiones(&self) {
        let Some(ruta) = self.ruta_revisiones() else { return };
        let doc: serde_json::Map<String, Value> = self.revisiones.iter().map(|(k, r)| (k.clone(), json!({
            "fecha": r.fecha.to_string(), "tam": r.tam, "perfil": r.perfil, "analizador": r.analizador, "extra": r.extra, "compilador": r.compilador.programa(), "cuando": r.cuando,
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
        // Reparto justo bajo el tope (500): primero un error de cada archivo, despues el resto de
        // errores y al final los avisos. Asi un archivo con cientos no tapa el fallo de otro.
        let mut vistos: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let rango: Vec<u8> = leidos.iter().map(|(a, d)| if !d.grave { 2 } else if vistos.insert(a.clone()) { 0 } else { 1 }).collect();
        let mut con_rango: Vec<(u8, (String, Diagnostico))> = rango.into_iter().zip(leidos).collect();
        con_rango.sort_by_key(|(r, _)| *r);
        let leidos: Vec<(String, Diagnostico)> = con_rango.into_iter().map(|(_, x)| x).collect();
        // De cero (lista y cuenta): si solo se vaciara la lista, la cuenta llegaria al tope tras
        // unas rondas y dejaria de anotar.
        self.estado.errores.olvidar_diagnosticos();
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
            if let Ok((diags, _, _, deps, c)) = self.revisar(&archivo, perfil.as_ref(), r.analizador, &r.extra, Some(r.compilador)) {
                self.anotar_revision(&archivo, perfil.is_some(), r.analizador, r.extra.clone(), c, diags, deps);
                hechos.push(archivo);
            }
        }
        if !hechos.is_empty() { self.reaplicar(); self.guardar_revisiones(); }
        hechos
    }

    #[allow(clippy::too_many_arguments)]
    fn anotar_revision(&mut self, archivo: &str, guardian: bool, analizador: bool, extra: Vec<String>, compilador: Compilador, diags: Vec<DiagGcc>, deps: Vec<String>) {
        let (fecha, tam) = fecha_y_tam(&self.raiz.join(archivo)).unwrap_or((0, 0));
        let deps = deps.into_iter().filter_map(|d| { let (f, s) = fecha_y_tam(&self.raiz.join(&d))?; Some((d, f, s)) }).collect();
        self.revisiones.insert(archivo.to_string(), Revision { fecha, tam, perfil: if guardian { "guardian".into() } else { "generico".into() },
                                                              analizador, extra, compilador, cuando: ahora_s(), diags, deps });
    }

    /// Una revision de GCC: con el perfil de los guardianes, dos pasadas como `ultra_revisar`
    /// (-O0 -fanalyzer y -O2); si no, -Wall -Wextra -O2 (y -fanalyzer si se pide).
    /// Devuelve los diagnosticos (sin repetir), las ordenes y de donde salen las flags.
    #[allow(clippy::type_complexity)]
    fn revisar(&self, archivo: &str, guardian: Option<&crate::guardianes::Perfil>, analizador: bool, extra: &[String], pedido: Option<Compilador>)
        -> Result<(Vec<DiagGcc>, Vec<String>, &'static str, Vec<String>, Compilador), String> {
        let (carpeta, base, origen, entrada, propio) = self.base_gcc(archivo);
        // Los guardianes son de GCC (perfil-gcc.sh); si no, el pedido o el del archivo.
        let compilador = if guardian.is_some() { Compilador::Gcc } else { pedido.unwrap_or(propio) };
        if compilador == Compilador::Clang {
            let (diags, ordenes, deps) = self.revisar_clang(archivo, &carpeta, &base, &entrada, analizador, extra)?;
            return Ok((diags, ordenes, origen, deps, compilador));
        }
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
            let (salida, err) = self.compilar(Compilador::Gcc, &carpeta, &args)?;
            if i == 0 { deps = self.deps_de(&carpeta, &salida, archivo); }
            juntar(&mut diags, leer_sarif(&err, &self.raiz)?);
            ordenes.push(format!("gcc {}", args.join(" ")));
        }
        Ok((diags, ordenes, origen, deps, compilador))
    }

    /// Las cabeceras del proyecto que dice -MMD (sin el propio archivo).
    fn deps_de(&self, carpeta: &Path, salida: &str, archivo: &str) -> Vec<String> {
        let mut deps: Vec<String> = salida.split_once(':').map(|(_, d)| d).unwrap_or("").split_whitespace().filter(|x| *x != "\\")
            .filter_map(|x| self.del_proyecto(carpeta, x)).filter(|x| x != archivo).take(MAX_DEPS).collect();
        deps.sort();
        deps.dedup();
        deps
    }

    /// La revision con Clang: compilar a /dev/null con -Wall -Wextra -O2 (diagnosticos en
    /// texto: el SARIF de Clang 18 es "inestable" y no dice el -W de cada aviso) y, si se
    /// pide, su analizador estatico (--analyze) en SARIF, con el camino de cada fallo.
    #[allow(clippy::type_complexity)]
    fn revisar_clang(&self, archivo: &str, carpeta: &Path, base: &[String], entrada: &str, analizador: bool, extra: &[String])
        -> Result<(Vec<DiagGcc>, Vec<String>, Vec<String>), String> {
        let mut args: Vec<String> = base.to_vec();
        args.extend(extra.iter().cloned());
        args.extend(["-Wall", "-Wextra", "-O2", "-MMD", "-MF", "/dev/stdout", "-c", "-o", "/dev/null", "-fno-color-diagnostics", "-fdiagnostics-show-option"].map(String::from));
        args.push(entrada.to_string());
        let (salida, err) = self.compilar(Compilador::Clang, carpeta, &args)?;
        let deps = self.deps_de(carpeta, &salida, archivo);
        let mut diags = self.leer_clang(carpeta, &err);
        let mut ordenes = vec![format!("clang {}", args.join(" "))];
        if analizador {
            let mut a: Vec<String> = base.to_vec();
            a.extend(extra.iter().cloned());
            a.extend(["--analyze", "-Xclang", "-analyzer-output=sarif", "-o", "/dev/stdout"].map(String::from));
            a.push(entrada.to_string());
            let (sarif, _) = self.compilar(Compilador::Clang, carpeta, &a)?;
            // Sin SARIF (no compilo): quedan los diagnosticos de la compilacion.
            if !sarif.trim().is_empty() { juntar(&mut diags, leer_sarif(&sarif, &self.raiz)?); }
            ordenes.push(format!("clang {}", a.join(" ")));
        }
        Ok((diags, ordenes, deps))
    }

    /// Los diagnosticos de Clang en texto: `ruta:linea:columna: nivel: mensaje [-Wregla]`.
    /// Las notas van como pasos del diagnostico anterior.
    fn leer_clang(&self, carpeta: &Path, texto: &str) -> Vec<DiagGcc> {
        let mut v: Vec<DiagGcc> = Vec::new();
        for l in texto.lines() {
            let Some((ruta, resto)) = l.split_once(':') else { continue };
            let mut partes = resto.splitn(3, ':');
            let (Some(linea), Some(columna), Some(cola)) = (partes.next(), partes.next(), partes.next()) else { continue };
            let (Ok(linea), Ok(columna)) = (linea.trim().parse::<u32>(), columna.trim().parse::<u32>()) else { continue };
            let cola = cola.trim_start();
            let (nivel, mensaje) = if let Some(m) = cola.strip_prefix("fatal error: ") { ("error", m) }
                else if let Some(m) = cola.strip_prefix("error: ") { ("error", m) }
                else if let Some(m) = cola.strip_prefix("warning: ") { ("warning", m) }
                else if let Some(m) = cola.strip_prefix("note: ") { ("note", m) }
                else { continue };
            let (mensaje, regla) = match mensaje.rsplit_once(" [") {
                Some((m, r)) if r.ends_with(']') && r.starts_with("-W") => (m.to_string(), Some(r.trim_end_matches(']').to_string())),
                _ => (mensaje.to_string(), None),
            };
            let (archivo, dentro) = match self.del_proyecto(carpeta, ruta) { Some(r) => (r, true), None => (ruta.to_string(), false) };
            if nivel == "note" {
                if let Some(d) = v.last_mut() { if d.pasos.len() < 12 { d.pasos.push(format!("{archivo}:{linea}: {mensaje}")); } }
                continue;
            }
            juntar(&mut v, vec![DiagGcc { archivo, archivo_en_proyecto: dentro, linea, columna, nivel: nivel.into(), regla, mensaje, pasos: Vec::new() }]);
        }
        v
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

    fn ruta_guardia(&self) -> Option<PathBuf> {
        let r = self.ruta_revisiones()?;
        let nombre = r.file_name()?.to_string_lossy().replacen("revisiones-", "guardia-", 1);
        Some(r.with_file_name(nombre))
    }

    /// Los archivos que el modo guardia debe tener revisados, ahora mismo.
    fn lista_guardia(&self) -> Vec<String> {
        let Some(g) = &self.guardia else { return Vec::new() };
        let dentro = |r: &str| g.carpeta.is_empty() || r.starts_with(&format!("{}/", g.carpeta));
        let vale = |r: &str| matches!(r.rsplit('.').next(), Some("c")) || (g.cpp && matches!(r.rsplit('.').next(), Some("cc" | "cpp" | "cxx")));
        // Si hay compile_commands.json, lo que de verdad se compila; si no, el indice.
        let mut v: Vec<String> = match &self.cc {
            Some(cc) => cc.por_archivo.keys().filter(|r| dentro(r) && vale(r)).cloned().collect(),
            None => (0..self.indice.len()).filter(|k| !self.indice.es_carpeta(*k)).map(|k| self.indice.ruta(k)).filter(|r| dentro(r) && vale(r)).map(str::to_string).collect(),
        };
        if !g.estudio {
            let mut marcadas: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
            v.retain(|r| {
                let mut d = r.as_str();
                while let Some((arriba, _)) = d.rsplit_once('/') {
                    d = arriba;
                    let m = *marcadas.entry(d.to_string()).or_insert_with(|| self.raiz.join(d).join(carpetas_nucleo::analisis::MARCA_ESTUDIO).is_file());
                    if m { return false; }
                }
                true
            });
        }
        v.sort();
        v.truncate(MAX_GUARDIA);
        v
    }

    /// Lo que le falta a la guardia: sin revision, o con la revision pendiente (cambio).
    pub fn por_guardar(&self) -> Vec<String> {
        self.lista_guardia().into_iter().filter(|r| !self.revisiones.contains_key(r) || self.pendientes.contains(r)).collect()
    }

    /// Un turno de la guardia: revisa como mucho `max` de lo que falta. Devuelve cuantos hizo.
    pub fn revisar_guardia(&mut self, max: usize) -> usize {
        let lista: Vec<String> = self.por_guardar().into_iter().take(max).collect();
        let mut n = 0;
        for archivo in &lista {
            let es_c = archivo.ends_with(".c");
            let perfil = self.guardian.clone().filter(|_| es_c);
            let (analizador, extra) = self.revisiones.get(archivo).map(|r| (r.analizador, r.extra.clone())).unwrap_or((false, Vec::new()));
            match self.revisar(archivo, perfil.as_ref(), analizador, &extra, None) {
                Ok((diags, _, _, deps, c)) => { self.anotar_revision(archivo, perfil.is_some(), analizador, extra, c, diags, deps); n += 1; }
                // Que no falle es un fallo de GCC (no del codigo): se anota vacio para no reintentar sin fin.
                Err(e) => { self.anotar_revision(archivo, perfil.is_some(), analizador, extra, Compilador::Gcc, vec![DiagGcc { archivo: archivo.clone(), archivo_en_proyecto: true, linea: 1, columna: 1,
                    nivel: "warning".into(), regla: Some("guardia".into()), mensaje: format!("no se pudo revisar: {e}"), pasos: Vec::new() }], Vec::new()); n += 1; }
            }
        }
        if n > 0 { self.reaplicar(); self.guardar_revisiones(); }
        n
    }

    fn guardia(&mut self, a: &Value) -> Result<Value, String> {
        let activar = a.get("activar").and_then(Value::as_bool).unwrap_or(true);
        if !activar {
            self.guardia = None;
            if let Some(r) = self.ruta_guardia() { let _ = std::fs::remove_file(r); }
            return Ok(json!({ "guardia": "apagada", "nota": "Las revisiones hechas se quedan; ya no se revisa nada nuevo por turnos." }));
        }
        let carpeta = match a.get("carpeta").and_then(Value::as_str) { Some(c) if !c.is_empty() => { self.existe(c)?; relativa(c)? } _ => String::new() };
        let g = Guardia { carpeta, cpp: a.get("cpp").and_then(Value::as_bool).unwrap_or(false), estudio: a.get("estudio").and_then(Value::as_bool).unwrap_or(false) };
        if let Some(r) = self.ruta_guardia() {
            if let Some(d) = r.parent() { let _ = std::fs::create_dir_all(d); }
            let _ = std::fs::write(r, json!({ "carpeta": g.carpeta, "cpp": g.cpp, "estudio": g.estudio }).to_string());
        }
        self.guardia = Some(g);
        let total = self.lista_guardia().len();
        let falta = self.por_guardar().len();
        Ok(json!({ "guardia": "encendida", "archivos": total, "por_revisar": falta,
                   "perfil": self.guardian.as_ref().map(|p| p.origen.clone()).unwrap_or_else(|| "generico (sin perfil-gcc.sh)".into()),
                   "nota": "El servidor MCP revisa por turnos (2 cada vez, sin bloquear) y despues solo lo que cambie. `guardianes` dice el progreso." }))
    }

    fn guardianes(&self, n: usize) -> Value {
        let lista = self.lista_guardia();
        let falta = self.por_guardar();
        let mut con_errores: Vec<(&String, usize)> = lista.iter().filter_map(|r| self.revisiones.get(r).map(|x| (r, x.diags.iter().filter(|d| d.nivel == "error").count()))).filter(|(_, e)| *e > 0).collect();
        con_errores.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        json!({
            "guardia": self.guardia.as_ref().map(|g| json!({ "carpeta": if g.carpeta.is_empty() { "(todo)" } else { g.carpeta.as_str() }, "cpp": g.cpp, "casos_a_proposito": g.estudio,
                "archivos": lista.len(), "al_dia": lista.len() - falta.len(), "por_revisar": falta.len(), "con_errores": con_errores.len(),
                "peores": con_errores.iter().take(n).map(|(r, e)| json!({ "archivo": r, "errores": e })).collect::<Vec<_>>() })),
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
            "cronologia" => self.cronologia(a),
            "guardianes" => Ok(self.guardianes(limite(a))),
            "cobertura_ub" => crate::cobertura_ub::consultar(&self.raiz, limite(a)),
            "revision_estado" => crate::revision_ui::consultar(&self.raiz, limite(a)),
            "guardia" => self.guardia(a),
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
            "cabeceras_exportadas": (!an.exportadas.is_empty()).then(|| json!({
                "prefijos": an.exportadas.iter().map(|(p, k)| json!({ "prefijo": p, "aristas": k })).collect::<Vec<_>>(),
                "nota": "deducidas: el #include nombra una ruta que solo existe al compilar (mozilla/x.h es mfbt/x.h); se aprenden del proyecto",
            })),
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
    fn compilar(&self, c: Compilador, carpeta: &Path, args: &[String]) -> Result<(String, String), String> {
        let mut hijo = std::process::Command::new(c.programa()).args(args).current_dir(carpeta)
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
            .spawn().map_err(|e| format!("no se pudo ejecutar {}: {e}", c.programa()))?;
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
            if t.elapsed() > GCC_TIEMPO { let _ = hijo.kill(); let _ = hijo.wait(); return Err(format!("{} tardo mas de 60 s: cortado", c.programa())); }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok((salida.join().unwrap_or_default(), errores.join().unwrap_or_default()))
    }

    /// Donde y con que flags se compila un archivo: las de compile_commands.json si las tiene
    /// (su carpeta, sus -I, -D, -std...); si no, unas genericas desde la raiz. Devuelve
    /// (carpeta, flags, de donde salen) y la ruta del archivo tal como hay que pasarla.
    fn base_gcc(&self, archivo: &str) -> (PathBuf, Vec<String>, &'static str, String, Compilador) {
        if let Some(o) = self.cc.as_ref().and_then(|c| c.por_archivo.get(archivo)) {
            return (o.carpeta.clone(), o.flags.clone(), "compile_commands.json", self.raiz.join(archivo).display().to_string(), o.compilador);
        }
        // Una cabecera .h se compila como C++ (en C tambien vale casi siempre, al reves no).
        let cpp = matches!(archivo.rsplit('.').next(), Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "mm" | "h"));
        let mut v = vec![if cpp { "-std=c++20".into() } else { "-std=c17".into() }, "-I.".into()];
        if let Some((d, _)) = archivo.rsplit_once('/') { v.push(format!("-I{d}")); }
        if archivo.ends_with(".h") { v.push("-xc++".into()); } // las cabeceras de C++ suelen acabar en .h
        (self.raiz.clone(), v, "genericas", archivo.to_string(), Compilador::Gcc)
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
        let pedido = compilador_pedido(a)?;
        // Los guardianes del proyecto mandan en C (perfil-gcc.sh); en C++ o sin perfil, generico.
        let es_c = matches!(archivo.rsplit('.').next(), Some("c" | "h"));
        let perfil = match a.get("perfil").and_then(Value::as_str) {
            Some("generico") => None,
            Some("guardian") => Some(self.guardian.clone().ok_or_else(|| format!("este proyecto no tiene {}", crate::guardianes::LUGAR))?),
            Some(x) => return Err(format!("perfil: guardian o generico (no {x})")),
            None => self.guardian.clone().filter(|_| es_c && pedido != Some(Compilador::Clang)),
        };
        if perfil.is_some() && pedido == Some(Compilador::Clang) { return Err("los guardianes son de GCC: con clang, perfil generico".into()); }
        let t = Instant::now();
        let (diags, ordenes, origen, deps, compilador) = self.revisar(&archivo, perfil.as_ref(), analizador, &extra, pedido)?;
        let ms = t.elapsed().as_millis();
        let n_deps = deps.len();
        // Se guarda (con la fecha del archivo y de sus cabeceras) y vuelve al estado.
        self.anotar_revision(&archivo, perfil.is_some(), analizador, extra, compilador, diags.clone(), deps);
        self.reaplicar();
        self.guardar_revisiones();
        let errores = diags.iter().filter(|d| d.nivel == "error").count();
        Ok(json!({
            "archivo": archivo, "perfil": perfil.as_ref().map(|p| format!("guardianes ({})", p.origen)).unwrap_or_else(|| "generico".into()),
            "compilador": compilador.programa(), "ordenes": ordenes, "flags_de": origen, "ms": ms, "cabeceras_vigiladas": n_deps, "analizador": analizador || perfil.is_some(),
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
        let pedido = compilador_pedido(a)?;
        let (mut con_gcc, mut con_clang) = (0usize, 0usize);
        // Archivos con cabeceras que no existen (se generan al compilar: configure, make).
        // Con -MG, GCC las da por buenas pero deja de seguir otras: la medida no valdria.
        let mut incompletos = 0usize;
        let mut faltan: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        // Plantillas del proyecto (`x509.h.in`): si GCC encuentra `x509.h` fuera del proyecto,
        // toma la del sistema (otra version) en lugar de la que se generaria.
        let plantillas: std::collections::BTreeSet<String> = if elegidos.is_empty() { Default::default() } else {
            (0..self.indice.len()).filter(|k| !self.indice.es_carpeta(*k)).map(|k| self.indice.ruta(k))
                .filter_map(|r| r.strip_suffix(".in")).filter(|r| r.ends_with(".h") || r.ends_with(".hpp"))
                .map(|r| r.rsplit('/').next().unwrap_or(r).to_string()).collect()
        };
        let mut del_sistema: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let t = Instant::now();
        for r in &elegidos {
            let (dir, mut args, origen, entrada, propio) = self.base_gcc(r);
            if origen == "compile_commands.json" { con_cc += 1; }
            let c = pedido.unwrap_or(propio);
            if c == Compilador::Clang { con_clang += 1 } else { con_gcc += 1 }
            args.retain(|x| x != "-xc++");
            args.extend(extra.iter().cloned());
            // -M y no -MM: con -MM, GCC calla lo que entra a traves de una cabecera del sistema,
            // aunque sea del proyecto (y si falta una generada, la busca en el sistema).
            args.extend(["-M".into(), "-MG".into(), entrada]);
            let Ok((salida, _)) = self.compilar(c, &dir, &args) else { fallidos += 1; continue };
            if salida.trim().is_empty() { fallidos += 1; continue; }
            // "obj.o: a.c b.h \\\n c.h": las cabeceras que existen en el proyecto.
            let nombres: Vec<&str> = salida.split_once(':').map(|(_, d)| d).unwrap_or("")
                .split_whitespace().filter(|x| *x != "\\").skip(1).collect();
            let ausentes: Vec<&str> = nombres.iter().copied().filter(|x| !dir.join(x).exists()).collect();
            let tapadas: Vec<&str> = nombres.iter().copied()
                .filter(|x| self.del_proyecto(&dir, x).is_none() && x.rsplit('/').next().is_some_and(|n| plantillas.contains(n))).collect();
            if !ausentes.is_empty() || !tapadas.is_empty() {
                incompletos += 1;
                for x in ausentes { *faltan.entry(x.to_string()).or_default() += 1; }
                for x in tapadas { *del_sistema.entry(x.to_string()).or_default() += 1; }
                continue;
            }
            let deps: std::collections::BTreeSet<String> = nombres.into_iter()
                .filter_map(|x| self.del_proyecto(&dir, x)).filter(|x| dep.id(x).is_some()).collect();
            let Some(id) = dep.id(r) else { continue };
            // El mapa: todo lo que alcanza por #include (gcc -MM da el cierre transitivo).
            let mapa: std::collections::BTreeSet<String> = dep.alcance(&[id], false, 20_000).0.keys().filter(|x| **x != id).map(|x| dep.ruta(*x).to_string()).collect();
            for x in &mapa { if deps.contains(x) { aciertos += 1; } else { solo_mapa += 1; if ejemplos_mapa.len() < 5 { ejemplos_mapa.push(json!({ "archivo": r, "cabecera": x })); } } }
            for x in &deps { if !mapa.contains(x) { solo_gcc += 1; if ejemplos_gcc.len() < 5 { ejemplos_gcc.push(json!({ "archivo": r, "cabecera": x })); } } }
            for x in alcance_incondicional(dep, id) { if deps.contains(dep.ruta(x)) { aciertos_fijo += 1; } else { solo_fijo += 1; } }
        }
        let p = |a: usize, b: usize| if a + b == 0 { Value::Null } else { json!((a as f64 / (a + b) as f64 * 1000.0).round() / 10.0) };
        let mut faltan: Vec<(String, usize)> = faltan.into_iter().collect();
        faltan.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut del_sistema: Vec<(String, usize)> = del_sistema.into_iter().collect();
        del_sistema.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let sin_medir = (incompletos > 0).then(|| json!({
            "archivos": incompletos,
            "cabeceras_que_faltan": faltan.len(),
            "las_mas_pedidas": faltan.iter().take(n.min(10)).map(|(x, k)| json!({ "cabecera": x, "la_piden": k })).collect::<Vec<_>>(),
            "tomadas_del_sistema": del_sistema.len(),
            "del_sistema_en_vez_del_proyecto": del_sistema.iter().take(n.min(10)).map(|(x, k)| json!({ "cabecera": x, "la_piden": k })).collect::<Vec<_>>(),
            "aviso": "faltan cabeceras que se generan al compilar (configure, make), o falta una -I; o GCC uso la del sistema (otra version) en lugar de la plantilla .h.in del proyecto. Un archivo asi no se mide: generalas o pasa las -I y vuelve a medir.",
        }));
        Ok(json!({
            "archivos_medidos": elegidos.len() - fallidos - incompletos, "fallidos": fallidos, "sin_medir": sin_medir, "compiladores": { "gcc": con_gcc, "clang": con_clang }, "con_compile_commands": con_cc, "ms": t.elapsed().as_millis(),
            "aciertos": aciertos, "solo_en_el_mapa": solo_mapa, "solo_en_gcc": solo_gcc,
            "precision_pct": p(aciertos, solo_mapa), "cobertura_pct": p(aciertos, solo_gcc),
            "sin_los_if": { "aciertos": aciertos_fijo, "solo_en_el_mapa": solo_fijo, "precision_pct": p(aciertos_fijo, solo_fijo),
                            "nota": "el mapa contando solo los #include que no estan dentro de un #if" },
            "ejemplos_solo_mapa": ejemplos_mapa, "ejemplos_solo_gcc": ejemplos_gcc,
            "flags": extra,
            "como": "gcc -M -MG con -I., -I<carpeta del archivo> y las flags pedidas. Solo en el mapa: #include que GCC no siguio (otro #if, otra plataforma) o mal resuelto; solo en GCC: lo que el mapa no ve (macros, rutas -I).",
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

/// La cronologia leida y reproducida (con el commit de arriba cuando se leyo).
struct Crono { cabeza: String, c: carpetas_nucleo::cronologia::Cronologia, rep: carpetas_nucleo::cronologia::Reproduccion,
               periodos: usize, cortada: bool, superficial: bool, ms: u128 }

/// Fecha Unix (UTC) como AAAA-MM-DD (Howard Hinnant, days_from_civil al reves).
pub fn fecha_iso(s: u64) -> String {
    let z = (s / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// El compilador pedido: auto (None), gcc o clang.
fn compilador_pedido(a: &Value) -> Result<Option<Compilador>, String> {
    match a.get("compilador").and_then(Value::as_str) {
        None | Some("auto") => Ok(None),
        Some(x) => Compilador::de_nombre(x).map(Some).ok_or_else(|| format!("compilador: auto, gcc o clang (no {x})")),
    }
}

fn compilador_esquema() -> Value { json!({ "type": "string", "enum": ["auto", "gcc", "clang"], "description": "auto: el de compile_commands.json, si no gcc" }) }

/// Junta diagnosticos sin repetir (mismo sitio y mensaje).
fn juntar(diags: &mut Vec<DiagGcc>, nuevos: Vec<DiagGcc>) {
    for d in nuevos {
        if !diags.iter().any(|x| x.archivo == d.archivo && x.linea == d.linea && x.columna == d.columna && x.mensaje == d.mensaje) { diags.push(d); }
    }
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
            // El camino del fallo (-fanalyzer, --analyze). Puede cruzar archivos: entonces cada
            // paso dice el suyo.
            let pasos: Vec<String> = r["codeFlows"][0]["threadFlows"][0]["locations"].as_array().into_iter().flatten().take(12).map(|p| {
                let l = &p["location"];
                let n = l["physicalLocation"]["region"]["startLine"].as_u64().unwrap_or(0);
                let m = l["message"]["text"].as_str().unwrap_or("");
                match l["physicalLocation"]["artifactLocation"]["uri"].as_str() {
                    Some(u) if u != uri => {
                        let u = u.strip_prefix("file://").unwrap_or(u);
                        let u = Path::new(u).strip_prefix(raiz).map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|_| u.to_string());
                        format!("{u}:{n}: {m}")
                    }
                    _ => format!("linea {n}: {m}"),
                }
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
    fn clang_revisa_lo_que_compila_clang_y_da_el_camino_entre_archivos() {
        let hay = |p: &str| std::process::Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success());
        if !hay("clang") || !hay("gcc") { return; }
        let dir = std::env::temp_dir().join(format!("ultra-clang-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        // Se libera en util.h y se usa despues en a.c: el camino cruza los dos archivos.
        std::fs::write(dir.join("src/util.h"), "#include <stdlib.h>\nstatic inline void suelta(int *p) {\n    free(p);\n}\n").unwrap();
        std::fs::write(dir.join("src/a.c"), "#include \"util.h\"\nint usa(void) {\n    int *p = malloc(sizeof *p);\n    if (!p) return 0;\n    *p = 1;\n    suelta(p);\n    return *p;\n}\nint corta(long v) { return v; }\n").unwrap();
        // Como Chromium: compile_commands.json con un clang que no es el del sistema.
        std::fs::write(dir.join("compile_commands.json"), json!([{ "directory": dir.join("src").display().to_string(), "file": "a.c",
            "arguments": ["../../third_party/llvm-build/Release+Asserts/bin/clang", "-Wconversion", "-c", "a.c"] }]).to_string()).unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "src/a.c", "analizador": true })).unwrap();
        assert_eq!(r["compilador"], "clang", "auto: el de compile_commands.json: {r}");
        assert!(r["ordenes"][0].as_str().unwrap().starts_with("clang "), "el clang del sistema, no el del proyecto: {r}");
        let lista = r["lista"].as_array().unwrap();
        assert!(lista.iter().any(|d| d["regla"] == "-Wshorten-64-to-32" || d["regla"] == "-Wconversion"), "con su -W: {r}");
        // Clang cambia el texto entre versiones ("freed" / "released").
        // Comprobar la regla y el sitio del caso conocido, no esa frase inglesa.
        let libre = lista.iter().find(|d| d["regla"] == "unix.Malloc" && d["linea"] == 7).unwrap_or_else(|| panic!("el uso tras free: {r}"));
        assert_eq!((libre["archivo"].as_str(), libre["linea"].as_u64()), (Some("src/a.c"), Some(7)), "{libre}");
        let pasos: Vec<&str> = libre["pasos"].as_array().unwrap().iter().filter_map(|x| x.as_str()).collect();
        assert!(pasos.iter().any(|p| p.starts_with("src/util.h:3:")), "el camino pasa por util.h: {pasos:?}");
        // La revision recuerda su compilador y se repite con el mismo al cambiar una cabecera.
        assert_eq!(m.revisiones["src/a.c"].compilador, Compilador::Clang);
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.join("src/util.h"), "#include <stdlib.h>\nstatic inline void suelta(int *p) {\n    (void)p;\n}\n").unwrap();
        m.analizar();
        assert_eq!(m.revisar_pendientes(4), ["src/a.c"]);
        assert_eq!(m.revisiones["src/a.c"].compilador, Compilador::Clang);
        assert!(m.revisiones["src/a.c"].diags.iter().all(|d| !d.mensaje.contains("freed")), "arreglado en la cabecera");
        // Pedir gcc a mano: GCC, y el perfil de los guardianes no se mezcla con clang.
        let r = m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "src/a.c", "compilador": "gcc" })).unwrap();
        assert_eq!(r["compilador"], "gcc");
        assert!(m.ejecutar("gcc_diagnosticos", &json!({ "archivo": "src/a.c", "compilador": "llvm" })).is_err());
        let d = m.ejecutar("gcc_deps", &json!({})).unwrap();
        assert_eq!(d["compiladores"]["clang"], 1, "{d}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn la_cronologia_sale_de_git_y_se_verifica_sola() {
        let dir = std::env::temp_dir().join(format!("ultra-crono-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(&dir).args(["-c", "user.name=Ana", "-c", "user.email=a@x", "-c", "commit.gpgsign=false"])
            .args(args).env("GIT_AUTHOR_DATE", "1700000000 +0000").env("GIT_COMMITTER_DATE", "1700000000 +0000").output().is_ok_and(|o| o.status.success());
        if !git(&["init", "-q"]) { return; }
        std::fs::write(dir.join("src/a.c"), "#include \"b.h\"\nint main(void) { return 0; }\n").unwrap();
        std::fs::write(dir.join("src/b.h"), "int b;\n").unwrap();
        assert!(git(&["add", "-A"]) && git(&["commit", "-q", "-m", "Primer paso"]));
        std::fs::create_dir_all(dir.join("inc")).unwrap();
        std::fs::rename(dir.join("src/b.h"), dir.join("inc/b.h")).unwrap();
        std::fs::write(dir.join("src/a.c"), "#include \"../inc/b.h\"\nint main(void) { return 0; }\n").unwrap();
        std::fs::write(dir.join("inc/c.h"), "#include \"b.h\"\n").unwrap();
        assert!(git(&["add", "-A"]) && git(&["commit", "-q", "-m", "Mover b.h a inc y nace c.h"]));
        std::fs::write(dir.join("inc/b.h"), "#include \"c.h\"\nint b;\n").unwrap();
        assert!(git(&["add", "-A"]) && git(&["commit", "-q", "-m", "b.h usa c.h: ciclo"]));
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("cronologia", &json!({ "periodos": 3 })).unwrap();
        assert_eq!((r["commits"].as_u64(), r["superficial"].as_bool()), (Some(3), Some(false)), "{r}");
        let f: Vec<(u64, u64, u64)> = r["crecimiento"].as_array().unwrap().iter()
            .map(|x| (x["archivos"].as_u64().unwrap(), x["dependencias"].as_u64().unwrap(), x["ciclos"].as_u64().unwrap())).collect();
        assert_eq!(f, [(2, 1, 0), (3, 2, 0), (3, 3, 1)], "{r}");
        assert_eq!(r["carpetas_en_orden"][0]["carpeta"], "src");
        assert_eq!(r["carpetas_en_orden"][1]["carpeta"], "inc");
        assert_eq!(r["verificacion"]["pct_de_lo_reconstruido"], 100.0, "{r}");
        assert_eq!(r["verificacion"]["dependencias"]["coinciden"], r["verificacion"]["dependencias"]["hoy"], "{r}");
        // inc/b.h nacio como src/b.h en el primer commit.
        let b = m.ejecutar("cronologia", &json!({ "ruta": "inc/b.h", "periodos": 3 })).unwrap();
        assert_eq!(b["archivo"]["nace"]["asunto"], "Primer paso", "{b}");
        assert_eq!(b["archivo"]["antes_se_llamaba"][0]["ruta"], "src/b.h");
        // El ciclo se cerro en el tercer commit, con el #include "c.h" de inc/b.h.
        let c = m.ejecutar("cronologia", &json!({ "ciclos": true, "periodos": 3 })).unwrap();
        assert_eq!(c["ciclos"]["lista"][0]["se_cerro"]["por"]["asunto"], "b.h usa c.h: ciclo", "{c}");
        assert_eq!(c["ciclos"]["lista"][0]["se_cerro"]["con"], "#include \"c.h\"");
        let e = m.ejecutar("cronologia", &json!({ "a": "src/a.c", "b": "inc/b.h", "periodos": 3 })).unwrap();
        assert_eq!(e["arista"]["entra"]["asunto"], "Mover b.h a inc y nace c.h", "{e}");
        assert!(m.ejecutar("cronologia", &json!({ "ruta": "no/existe" })).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn el_compilador_de_cada_orden() {
        let o = |v: &[&str]| Compilador::de_orden(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        assert_eq!(o(&["../../third_party/llvm-build/Release+Asserts/bin/clang++", "-c"]), Compilador::Clang);
        assert_eq!(o(&["ccache", "clang-18", "-c"]), Compilador::Clang);
        assert_eq!(o(&["/usr/bin/x86_64-linux-gnu-gcc-13", "-c"]), Compilador::Gcc);
        assert_eq!(o(&["c++", "-c"]), Compilador::Gcc);
        assert_eq!(o(&["raro", "-c"]), Compilador::Gcc, "sin saberlo: GCC");
    }

    #[test]
    fn gcc_deps_no_mide_si_faltan_cabeceras_generadas() {
        if std::process::Command::new("gcc").arg("--version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("ultra-gcc-deps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.h"), "int b(void);\n").unwrap();
        // gen.h se generaria al compilar (como x509.h en OpenSSL): no existe.
        std::fs::write(dir.join("a.c"), "#include \"gen.h\"\n#include \"b.h\"\nint a(void) { return b(); }\n").unwrap();
        std::fs::write(dir.join("c.c"), "#include \"b.h\"\nint c(void) { return b(); }\n").unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_deps", &json!({})).unwrap();
        assert_eq!(r["archivos_medidos"], 1, "solo c.c: {r}");
        assert_eq!(r["sin_medir"]["archivos"], 1, "{r}");
        assert_eq!(r["sin_medir"]["las_mas_pedidas"][0]["cabecera"], "gen.h", "{r}");
        assert_eq!((r["precision_pct"].as_f64(), r["cobertura_pct"].as_f64()), (Some(100.0), Some(100.0)), "{r}");
        // La plantilla del proyecto (ssl.h.in) y una ssl.h "del sistema" (otra version) que la
        // tapa; la del sistema incluye una cabecera del proyecto (con -MM, GCC la callaba).
        std::fs::remove_file(dir.join("a.c")).unwrap();
        std::fs::create_dir_all(dir.join("sistema/x")).unwrap();
        std::fs::create_dir_all(dir.join("include/x")).unwrap();
        std::fs::write(dir.join("include/x/ssl.h.in"), "/* se genera */\n").unwrap();
        std::fs::write(dir.join("include/x/evp.h"), "int evp;\n").unwrap();
        let fuera = std::env::temp_dir().join(format!("ultra-gcc-deps-sistema-{}", std::process::id()));
        std::fs::create_dir_all(fuera.join("x")).unwrap();
        std::fs::write(fuera.join("x/ssl.h"), "#include <x/evp.h>\n").unwrap();
        std::fs::write(dir.join("d.c"), "#include <x/ssl.h>\n#include \"b.h\"\nint d;\n").unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("gcc_deps", &json!({ "flags": ["-Iinclude", format!("-I{}", fuera.display())] }));
        assert!(r.is_err(), "las -I de fuera del proyecto no se aceptan");
        // C_INCLUDE_PATH: para GCC, cabeceras del sistema (solo trae x/ssl.h: no molesta a
        // las otras pruebas).
        std::env::set_var("C_INCLUDE_PATH", &fuera);
        let r = m.ejecutar("gcc_deps", &json!({ "flags": ["-Iinclude"] })).unwrap();
        std::env::remove_var("C_INCLUDE_PATH");
        assert_eq!(r["archivos_medidos"], 1, "c.c; d.c usa la del sistema: {r}");
        assert_eq!(r["sin_medir"]["tomadas_del_sistema"], 1, "{r}");
        assert!(r["sin_medir"]["del_sistema_en_vez_del_proyecto"][0]["cabecera"].as_str().unwrap().ends_with("x/ssl.h"), "{r}");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&fuera).ok();
    }

    #[test]
    fn la_guardia_revisa_por_turnos_y_ve_los_nuevos() {
        if std::process::Command::new("gcc").arg("--version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("ultra-guardia-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/guardian/produccion")).unwrap();
        std::fs::create_dir_all(dir.join("lab")).unwrap();
        std::fs::write(dir.join(crate::guardianes::LUGAR), "ULTRA_WARN=(-std=c17 -Wall -Wextra -Werror -Wvla -Wconversion)\n").unwrap();
        std::fs::write(dir.join("a.c"), "int f(int n) { return n + 1; }\n").unwrap();
        // Solo el guardian lo ve (-Wconversion): el museo no tiene ficha para esto.
        std::fs::write(dir.join("b.c"), "int g(long n) { return n; }\n").unwrap();
        std::fs::write(dir.join("lab/CASOS-MUSEO.txt"), "minas a proposito\n").unwrap();
        std::fs::write(dir.join("lab/mina.c"), "int h(int n) { char y[n]; y[0] = 1; return y[0]; }\n").unwrap();
        let mut m = Mapa::abrir(&dir).unwrap();
        let r = m.ejecutar("guardia", &json!({})).unwrap();
        assert_eq!((r["archivos"].as_u64(), r["por_revisar"].as_u64()), (Some(2), Some(2)), "la mina del laboratorio no entra: {r}");
        assert_eq!(m.revisar_guardia(1), 1, "por turnos");
        assert_eq!(m.revisar_guardia(5), 1);
        assert_eq!(m.revisar_guardia(5), 0, "ya esta todo al dia");
        let g = m.ejecutar("guardianes", &json!({})).unwrap();
        assert_eq!(g["guardia"]["con_errores"], 1);
        assert_eq!(g["guardia"]["peores"][0]["archivo"], "b.c");
        assert!(m.estado.analisis.sospechas.get("b.c").is_none(), "el museo no ve nada en b.c");
        assert_eq!(m.estado.conectado.fallos, 1, "el guardian (y solo el) hace fallar b.c");
        // Volver a aplicar muchas veces no agota la cuenta de diagnosticos (fallo arreglado:
        // antes, tras unas rondas, la cuenta llegaba al tope y b.c dejaba de fallar).
        for _ in 0..3000 { m.reaplicar(); }
        assert!(m.estado.errores.por_nodo.contains_key("b.c"));
        assert_eq!(m.estado.conectado.fallos, 1, "tras 3000 rondas sigue fallando b.c");
        // Un archivo con mas errores que el tope (500), antes en el orden, no tapa el de b.c.
        let muchos: String = (0..600).map(|i| format!("int f{i}(int x) {{ int s = x; return x; }}\n")).collect();
        std::fs::write(dir.join("a.c"), muchos).unwrap();
        m.analizar();
        m.revisar_pendientes(4);
        let errores_a = m.revisiones["a.c"].diags.iter().filter(|d| d.nivel == "error").count();
        assert!(errores_a > 500, "a.c da {errores_a} errores (mas que el tope)");
        assert_eq!(m.estado.conectado.fallos, 2, "fallan a.c y b.c: el de b.c no queda fuera");
        std::fs::write(dir.join("a.c"), "int f(int n) { return n + 1; }\n").unwrap();
        m.analizar();
        m.revisar_pendientes(4);
        // Un .c nuevo entra solo (el criterio, no una lista).
        std::fs::write(dir.join("c.c"), "int k(void) { return 3; }\n").unwrap();
        m.analizar();
        assert_eq!(m.por_guardar(), ["c.c"]);
        // Y la guardia sobrevive a reiniciar.
        let otro = Mapa::abrir(&dir).unwrap();
        assert_eq!(otro.por_guardar(), ["c.c"]);
        assert_eq!(m.ejecutar("guardia", &json!({ "activar": false })).unwrap()["guardia"], "apagada");
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
