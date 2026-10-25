// SPDX-License-Identifier: GPL-2.0-only
//! Servidor MCP (Model Context Protocol) del SUPER GRAFO: `ultra-carpetas --mcp <carpeta>`.
//!
//! Transporte stdio: un mensaje JSON-RPC 2.0 por linea en stdin, una respuesta por linea en
//! stdout (stdout es solo del protocolo; los avisos van a stderr). Las herramientas son las
//! mismas que `--consulta` (ver `consultas.rs`): respuestas exactas y con tope.
//!
//! El analisis se hace al arrancar, en otro hilo: `initialize` contesta al momento y la
//! primera herramienta espera a que termine.

use crate::consultas::{herramientas, Mapa};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;

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
}

impl Servidor {
    pub fn nuevo(carpeta: PathBuf) -> Servidor {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || { let _ = tx.send(Mapa::abrir(&carpeta)); });
        Servidor { mapa: None, llegando: Some(rx), error: None }
    }

    /// Para pruebas: con el mapa ya hecho.
    #[cfg(test)]
    pub fn con_mapa(m: Mapa) -> Servidor { Servidor { mapa: Some(m), llegando: None, error: None } }

    fn mapa(&mut self) -> Result<&mut Mapa, String> {
        if let Some(rx) = self.llegando.take() {
            match rx.recv() { Ok(Ok(m)) => self.mapa = Some(m), Ok(Err(e)) => self.error = Some(e), Err(_) => self.error = Some("el analisis se interrumpio".into()) }
        }
        if let Some(e) = &self.error { return Err(e.clone()); }
        self.mapa.as_mut().ok_or_else(|| "sin mapa".into())
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
                    "capabilities": { "tools": { "listChanged": false } },
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
            "resources/list" => ok(json!({ "resources": [] })),
            "prompts/list" => ok(json!({ "prompts": [] })),
            _ => mal(-32601, &format!("metodo desconocido: {metodo}")),
        })
    }
}

/// Bucle stdio: hasta que se cierre stdin.
pub fn servir(carpeta: PathBuf) -> i32 {
    let mut s = Servidor::nuevo(carpeta);
    let entrada = std::io::stdin();
    let mut salida = std::io::stdout().lock();
    for linea in entrada.lock().lines() {
        let Ok(linea) = linea else { break };
        if linea.trim().is_empty() { continue; }
        let respuesta = match serde_json::from_str::<Value>(&linea) {
            Ok(Value::Array(lote)) => {
                let v: Vec<Value> = lote.iter().filter_map(|m| s.atender(m)).collect();
                if v.is_empty() { None } else { Some(Value::Array(v)) }
            }
            Ok(m) => s.atender(&m),
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON ilegible: {e}") } })),
        };
        if let Some(r) = respuesta {
            if writeln!(salida, "{}", serde_json::to_string(&r).unwrap_or_default()).is_err() || salida.flush().is_err() { break; }
        }
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
