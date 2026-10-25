// SPDX-License-Identifier: GPL-2.0-only
//! Servidor MCP (Model Context Protocol) del SUPER GRAFO: `ultra-carpetas --mcp <carpeta>`.
//!
//! Transporte stdio: un mensaje JSON-RPC 2.0 por linea en stdin, una respuesta por linea en
//! stdout (stdout es solo del protocolo; los avisos van a stderr). Las herramientas son las
//! mismas que `--consulta` (ver `consultas.rs`): respuestas exactas y con tope.
//!
//! El analisis se hace al arrancar, en otro hilo: `initialize` contesta al momento y la
//! primera herramienta espera a que termine.
//!
//! **En vivo:** el kernel avisa de cada cambio (inotify, ver `vigia.rs`); se junta la
//! rafaga 150 ms y se confirma con una firma barata (rutas, tamanos y fechas). Si cambio,
//! reanaliza (incremental: la cache solo relee lo que cambio) y avisa:
//! - `notifications/resources/updated` a quien se suscribio a un recurso que cambio
//!   (`super-grafo://resumen`, `fallos`, `ciclos`, `cambios`, `ventana`);
//! - `notifications/message` (si se pidio `logging/setLevel`) con lo que cambio.
//! Sin inotify, sondea: 20 veces lo que cuesta la firma (de 1 a 30 s). Con inotify, ademas,
//! una comprobacion cada 30 s por si se perdio un aviso.

use crate::consultas::{herramientas, Mapa};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Los recursos que se pueden leer y a los que suscribirse.
const RECURSOS: &[(&str, &str, &str)] = &[
    ("resumen", "Resumen del proyecto", "Cifras: archivos, dependencias, sospechas por eje, fallo conectado y ciclos."),
    ("fallos", "Fallo conectado", "Las causas probables y cuanto explica cada una."),
    ("ciclos", "Ciclos de cabeceras", "Los ciclos (E-003) con su vuelta y donde cortar."),
    ("cambios", "Cambios", "Que cambio en el ultimo analisis: sospechas nuevas y arregladas, fallos, ciclos."),
    ("ventana", "La ventana", "Lo que la persona esta mirando en ULTRA Carpetas (si esta abierta)."),
];

/// Las versiones del protocolo que se entienden (la mas nueva primero).
const VERSIONES: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCCIONES: &str = "SUPER GRAFO de ULTRA Carpetas: el mapa de un proyecto C/C++/ASM (tambien Rust y Python) \
con el museo de fallos (Seguridad, Rendimiento, Elegancia), el fallo conectado, caminos, ciclos y pesos. \
Empezar por `resumen`; despues `fallos`, `ciclos` o `senales` para lo peor, y `camino`, `peso` o `archivo` para bajar. \
Las rutas son relativas a la carpeta. Todas las respuestas tienen tope (`limite`, 10 por defecto). \
Es analisis lexico: `gcc_deps` mide cuanto acierta contra gcc -MM y `gcc_diagnosticos` pide la opinion de GCC (SARIF, -fanalyzer).";

/// El estado del servidor: el mapa (cuando llega) y si ya se inicializo.
pub struct Servidor {
    mapa: Option<Mapa>,
    llegando: Option<mpsc::Receiver<Result<Mapa, String>>>,
    error: Option<String>,
    suscritos: BTreeSet<String>,
    /// La ultima firma de la carpeta y lo que costo calcularla.
    firma: Option<(u64, usize)>,
    coste_firma: Duration,
    /// Huella del contenido de cada recurso la ultima vez que se aviso.
    huellas: HashMap<String, u64>,
    /// logging/setLevel: si se piden mensajes.
    registro: bool,
}

impl Servidor {
    pub fn nuevo(carpeta: PathBuf) -> Servidor {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || { let _ = tx.send(Mapa::abrir(&carpeta)); });
        Servidor { mapa: None, llegando: Some(rx), error: None, suscritos: BTreeSet::new(), firma: None, coste_firma: Duration::ZERO, huellas: HashMap::new(), registro: false }
    }

    /// Para pruebas: con el mapa ya hecho.
    #[cfg(test)]
    pub fn con_mapa(m: Mapa) -> Servidor { Servidor { mapa: Some(m), llegando: None, error: None, suscritos: BTreeSet::new(), firma: None, coste_firma: Duration::ZERO, huellas: HashMap::new(), registro: false } }

    fn mapa(&mut self) -> Result<&mut Mapa, String> {
        if let Some(rx) = self.llegando.take() {
            match rx.recv() {
                Ok(Ok(m)) => { self.firma = Some(crate::consultas::firma(&m.raiz)); self.mapa = Some(m); }
                Ok(Err(e)) => self.error = Some(e),
                Err(_) => self.error = Some("el analisis se interrumpio".into()),
            }
        }
        if let Some(e) = &self.error { return Err(e.clone()); }
        self.mapa.as_mut().ok_or_else(|| "sin mapa".into())
    }

    /// El contenido de un recurso (`super-grafo://nombre`).
    fn recurso(&mut self, uri: &str) -> Result<Value, String> {
        let nombre = uri.strip_prefix("super-grafo://").ok_or_else(|| format!("recurso desconocido: {uri}"))?;
        if !RECURSOS.iter().any(|(n, ..)| *n == nombre) { return Err(format!("recurso desconocido: {uri}")); }
        self.mapa()?.ejecutar(nombre, &json!({}))
    }

    /// Cada cuanto vigilar: 20 veces lo que cuesta la firma, entre 1 y 30 s.
    pub fn intervalo(&self) -> Duration { (self.coste_firma * 20).clamp(Duration::from_secs(1), Duration::from_secs(30)) }

    /// Mira si la carpeta cambio; si si, reanaliza y devuelve los avisos que tocan.
    pub fn vigilar(&mut self) -> Vec<Value> {
        let Some(mapa) = self.mapa.as_mut() else { return Vec::new() };
        let t = Instant::now();
        let f = crate::consultas::firma(&mapa.raiz);
        self.coste_firma = t.elapsed();
        let mut avisos = Vec::new();
        let antes = self.firma.replace(f);
        if antes.is_some_and(|a| a != f) {
            let t = Instant::now();
            mapa.analizar();
            // Los guardianes vuelven a revisar lo que cambio (antes de decir que cambio).
            let revisados = mapa.revisar_pendientes(4);
            let c = mapa.ejecutar("cambios", &json!({ "limite": 5 })).unwrap_or_default();
            if self.registro {
                avisos.push(json!({ "jsonrpc": "2.0", "method": "notifications/message", "params": { "level": "info", "logger": "super-grafo",
                    "data": { "reanalizado_ms": t.elapsed().as_millis(), "version": mapa.version, "revisados_otra_vez": revisados, "cambios": c } } }));
            }
        }
        // Avisar de los recursos suscritos cuyo contenido cambio (la ventana cambia sola).
        let suscritos: Vec<String> = self.suscritos.iter().cloned().collect();
        for uri in suscritos {
            let Ok(v) = self.recurso(&uri) else { continue };
            let mut sin_hora = v.clone();
            if let Some(o) = sin_hora.as_object_mut() { o.remove("escrito_hace_s"); o.remove("analizado_hace_s"); o.remove("analisis_ms"); }
            let h = serde_json::to_string(&sin_hora).unwrap_or_default().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
            if self.huellas.insert(uri.clone(), h).is_some_and(|x| x != h) {
                avisos.push(json!({ "jsonrpc": "2.0", "method": "notifications/resources/updated", "params": { "uri": uri } }));
            }
        }
        avisos
    }

    /// Un mensaje JSON-RPC; None si es una notificacion (no lleva respuesta).
    pub fn atender(&mut self, m: &Value) -> Option<Value> {
        let id = m.get("id").cloned();
        let metodo = m.get("method").and_then(Value::as_str).unwrap_or("");
        let id = id?; // notificaciones (notifications/initialized, cancelled...): nada que contestar
        let ok = |r: Value| json!({ "jsonrpc": "2.0", "id": id, "result": r });
        let mal = |codigo: i64, texto: &str| json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codigo, "message": texto } });
        Some(match metodo {
            "initialize" => {
                let pedida = m["params"]["protocolVersion"].as_str().unwrap_or(VERSIONES[0]);
                let version = VERSIONES.iter().find(|v| **v == pedida).copied().unwrap_or(VERSIONES[0]);
                ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false }, "resources": { "subscribe": true, "listChanged": false }, "logging": {} },
                    "serverInfo": { "name": "super-grafo", "title": "SUPER GRAFO (ULTRA Carpetas)", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": INSTRUCCIONES,
                }))
            }
            "ping" => ok(json!({})),
            "tools/list" => ok(json!({ "tools": herramientas() })),
            "tools/call" => {
                let nombre = m["params"]["name"].as_str().unwrap_or("");
                let args = m["params"].get("arguments").cloned().unwrap_or_else(|| json!({}));
                if !herramientas().iter().any(|h| h["name"] == nombre) { return Some(mal(-32602, &format!("no hay herramienta \"{nombre}\""))); }
                // Un error de la herramienta va dentro del resultado (isError), como pide MCP.
                let r = self.mapa().and_then(|mapa| mapa.ejecutar(nombre, &args));
                ok(match r {
                    Ok(v) => json!({ "content": [{ "type": "text", "text": serde_json::to_string(&v).unwrap_or_default() }], "structuredContent": v, "isError": false }),
                    Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                })
            }
            "resources/list" => ok(json!({ "resources": RECURSOS.iter().map(|(n, titulo, desc)| json!({
                "uri": format!("super-grafo://{n}"), "name": n, "title": titulo, "description": desc, "mimeType": "application/json" })).collect::<Vec<_>>() })),
            "resources/read" => {
                let uri = m["params"]["uri"].as_str().unwrap_or("").to_string();
                match self.recurso(&uri) {
                    Ok(v) => ok(json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": serde_json::to_string(&v).unwrap_or_default() }] })),
                    Err(e) => mal(-32002, &e),
                }
            }
            "resources/subscribe" | "resources/unsubscribe" => {
                let uri = m["params"]["uri"].as_str().unwrap_or("").to_string();
                if !RECURSOS.iter().any(|(n, ..)| format!("super-grafo://{n}") == uri) { return Some(mal(-32002, &format!("recurso desconocido: {uri}"))); }
                if metodo == "resources/subscribe" {
                    // La huella de ahora, para avisar solo cuando cambie.
                    if let Ok(v) = self.recurso(&uri) {
                        let mut s = v; if let Some(o) = s.as_object_mut() { o.remove("escrito_hace_s"); o.remove("analizado_hace_s"); o.remove("analisis_ms"); }
                        let h = serde_json::to_string(&s).unwrap_or_default().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
                        self.huellas.insert(uri.clone(), h);
                    }
                    self.suscritos.insert(uri);
                } else { self.suscritos.remove(&uri); self.huellas.remove(&uri); }
                ok(json!({}))
            }
            "logging/setLevel" => { self.registro = m["params"]["level"].as_str().is_some_and(|l| !matches!(l, "error" | "critical" | "alert" | "emergency")); ok(json!({})) }
            "prompts/list" => ok(json!({ "prompts": [] })),
            _ => mal(-32601, &format!("metodo desconocido: {metodo}")),
        })
    }
}

/// Bucle stdio: hasta que se cierre stdin. Un hilo aparte vigila la carpeta y avisa.
pub fn servir(carpeta: PathBuf) -> i32 {
    let raiz = carpeta.clone();
    let s = Arc::new(Mutex::new(Servidor::nuevo(carpeta)));
    let salida = Arc::new(Mutex::new(std::io::stdout()));
    let escribir = |salida: &Mutex<std::io::Stdout>, v: &Value| -> bool {
        let Ok(mut o) = salida.lock() else { return false };
        writeln!(o, "{}", serde_json::to_string(v).unwrap_or_default()).is_ok() && o.flush().is_ok()
    };
    {
        let (s, salida) = (s.clone(), salida.clone());
        std::thread::Builder::new().name("vigia".into()).spawn(move || {
            // Que el analisis inicial termine (y deje su firma) antes de vigilar.
            if let Ok(mut x) = s.lock() { if let Ok(m) = x.mapa() { m.revisar_pendientes(8); } }
            let avisos_kernel = crate::vigia::iniciar(&raiz);
            loop {
                match &avisos_kernel {
                    Some(rx) => { crate::vigia::esperar(rx, Duration::from_secs(30), Duration::from_millis(150)); }
                    None => { let espera = s.lock().map(|x| x.intervalo()).unwrap_or(Duration::from_secs(2)); std::thread::sleep(espera); }
                }
                let avisos = match s.lock() { Ok(mut x) => x.vigilar(), Err(_) => return };
                for a in avisos { if !escribir(&salida, &a) { return; } }
            }
        }).ok();
    }
    let entrada = std::io::stdin();
    for linea in entrada.lock().lines() {
        let Ok(linea) = linea else { break };
        if linea.trim().is_empty() { continue; }
        let respuesta = match serde_json::from_str::<Value>(&linea) {
            Ok(Value::Array(lote)) => {
                let v: Vec<Value> = match s.lock() { Ok(mut s) => lote.iter().filter_map(|m| s.atender(m)).collect(), Err(_) => break };
                if v.is_empty() { None } else { Some(Value::Array(v)) }
            }
            Ok(m) => match s.lock() { Ok(mut s) => s.atender(&m), Err(_) => break },
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON ilegible: {e}") } })),
        };
        if let Some(r) = respuesta { if !escribir(&salida, &r) { break; } }
    }
    0
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn servidor() -> Servidor {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../carpetas-nucleo/tests/datos/ciclos/mal");
        Servidor::con_mapa(Mapa::abrir(&dir).unwrap())
    }

    #[test]
    fn en_vivo_avisa_cuando_cambia_el_codigo() {
        let dir = std::env::temp_dir().join(format!("ultra-mcp-vivo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.c"), "#include \"a.h\"\nint f(void) { return 0; }\n").unwrap();
        std::fs::write(dir.join("a.h"), "#pragma once\nint f(void);\n").unwrap();
        let mut s = Servidor::con_mapa(Mapa::abrir(&dir).unwrap());
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" })).unwrap();
        assert_eq!(r["result"]["resources"].as_array().unwrap().len(), RECURSOS.len());
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/read", "params": { "uri": "super-grafo://resumen" } })).unwrap();
        assert!(r["result"]["contents"][0]["text"].as_str().unwrap().contains("\"dependencias\":1"));
        s.atender(&json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/subscribe", "params": { "uri": "super-grafo://resumen" } })).unwrap();
        s.atender(&json!({ "jsonrpc": "2.0", "id": 4, "method": "logging/setLevel", "params": { "level": "info" } })).unwrap();
        assert!(s.vigilar().is_empty(), "la primera vez solo toma la firma");
        assert!(s.vigilar().is_empty(), "sin cambios, sin avisos");
        // Alguien mete un free(p + 1): el vigia reanaliza y avisa.
        std::fs::write(dir.join("b.c"), "#include <stdlib.h>\nvoid g(char *p) {\n    free(p + 1);\n}\n").unwrap();
        let avisos = s.vigilar();
        let metodos: Vec<&str> = avisos.iter().filter_map(|a| a["method"].as_str()).collect();
        assert!(metodos.contains(&"notifications/resources/updated"), "{avisos:?}");
        assert!(metodos.contains(&"notifications/message"));
        let c = s.mapa().unwrap().ejecutar("cambios", &json!({})).unwrap();
        assert_eq!(c["sospechas_nuevas"]["total"], 1);
        assert_eq!(c["sospechas_nuevas"]["lista"][0]["ficha"], "C-001");
        assert_eq!(c["fallan_ahora"]["lista"][0], "b.c");
        // Se arregla: otra vez aviso, y la sospecha sale como arreglada.
        std::fs::write(dir.join("b.c"), "#include <stdlib.h>\nvoid g(char *p) {\n    free(p);\n}\n").unwrap();
        assert!(!s.vigilar().is_empty());
        let c = s.mapa().unwrap().ejecutar("cambios", &json!({})).unwrap();
        assert_eq!(c["sospechas_arregladas"]["total"], 1);
        assert_eq!(c["ya_no_fallan"]["lista"][0], "b.c");
        assert!(s.intervalo() >= Duration::from_secs(1));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn habla_mcp_por_json_rpc() {
        let mut s = servidor();
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": { "name": "prueba", "version": "0" } } })).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26", "contesta con la version pedida si la entiende");
        assert_eq!(r["result"]["serverInfo"]["name"], "super-grafo");
        assert!(r["result"]["capabilities"]["tools"].is_object());
        // Las notificaciones no llevan respuesta.
        assert!(s.atender(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).unwrap();
        let nombres: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
        for n in ["resumen", "camino", "ciclos", "senales", "peso", "gcc_diagnosticos", "gcc_deps", "exportar_sarif"] { assert!(nombres.contains(&n), "{n}"); }
        for h in r["result"]["tools"].as_array().unwrap() { assert_eq!(h["inputSchema"]["type"], "object"); }
        // Una herramienta: el resultado va en texto y estructurado.
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "ciclos", "arguments": {} } })).unwrap();
        assert_eq!(r["result"]["isError"], false);
        assert_eq!(r["result"]["structuredContent"]["total"], 1);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("\"total\":1"));
        // Un error de la herramienta va dentro del resultado; una herramienta que no existe, no.
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "camino", "arguments": { "a": "../fuera.h", "b": "a.h" } } })).unwrap();
        assert_eq!(r["result"]["isError"], true);
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "borrar_todo", "arguments": {} } })).unwrap();
        assert_eq!(r["error"]["code"], -32602);
        let r = s.atender(&json!({ "jsonrpc": "2.0", "id": 6, "method": "nada" })).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
