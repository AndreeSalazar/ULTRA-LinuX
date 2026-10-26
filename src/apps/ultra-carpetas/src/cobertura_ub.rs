// SPDX-License-Identifier: GPL-2.0-only
//! Lee evidencia del banco LLVM; nunca compila ni ejecuta una mina desde MCP.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

const REPORT: &str = "build/llvm-ub/current.json";
const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn consultar(root: &Path, limit: usize) -> Result<Value, String> {
    let mut path = root.to_path_buf();
    for component in REPORT.split('/') {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!({
                "estado": "sin_evidencia", "informe": REPORT, "production_certified": false,
                "nota": "Ejecutar el banco LLVM en QEMU; no se ejecuta desde esta consulta."
            })),
            Err(e) => return Err(e.to_string()),
            Ok(m) if m.file_type().is_symlink() => return Err("informe LLVM redirigido por enlace".into()),
            Ok(_) => {}
        }
    }
    let mut options = std::fs::OpenOptions::new(); options.read(true);
    #[cfg(target_os = "linux")]
    { use std::os::unix::fs::OpenOptionsExt; options.custom_flags(0x20000 | 0x800); } // NOFOLLOW | NONBLOCK
    let file = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES { return Err("informe LLVM no regular o demasiado grande".into()); }
    let mut bytes = Vec::new(); file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES { return Err("informe LLVM demasiado grande".into()); }
    let value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    resumir(&value, limit)
}

fn resumir(v: &Value, limit: usize) -> Result<Value, String> {
    if v["schema"] != 1 || v["kind"] != "ultra-llvm-ub-lab" || v["production_certified"] != false {
        return Err("contrato de evidencia LLVM desconocido; no certifica produccion".into());
    }
    let rows = v["cases"].as_array().ok_or("cases debe ser un array")?;
    if rows.is_empty() || rows.len() > 5000 { return Err("cantidad de casos LLVM invalida".into()); }
    let mut ids = BTreeSet::new(); let mut counts = BTreeMap::<String, usize>::new();
    let mut groups = BTreeMap::<String, (usize, usize)>::new(); let mut details = Vec::new();
    for row in rows {
        let id = row["id"].as_str().ok_or("caso sin id")?;
        if id.len() > 160 || !ids.insert(id) { return Err("id LLVM duplicado o demasiado largo".into()); }
        let language = row["language"].as_str().ok_or("caso sin lenguaje")?;
        let profile = row["profile"].as_str().ok_or("caso sin perfil")?;
        let status = row["status"].as_str().ok_or("caso sin estado")?;
        if !["c17", "cpp20"].contains(&language) || !["asan", "ubsan", "tsan", "msan"].contains(&profile)
            || !["passed", "compile_failed", "timeout", "detection_missing", "control_failed", "unavailable", "known_gap"].contains(&status)
            || !["mal", "bien"].contains(&row["variant"].as_str().unwrap_or("")) {
            return Err("lenguaje, perfil, variante o estado LLVM desconocido".into());
        }
        *counts.entry(status.into()).or_default() += 1;
        let group = groups.entry(format!("{language}/{profile}")).or_default();
        group.0 += 1; if status == "passed" { group.1 += 1; }
        details.push(json!({"id": id, "lenguaje": language, "perfil": profile, "estado": status, "variante": row["variant"]}));
    }
    // Show failures first, keeping bounded output even for thousands of rows.
    details.sort_by_key(|r| r["estado"] == "passed");
    let shown = limit.clamp(1, 100).min(details.len()); details.truncate(shown);
    let groups: Vec<_> = groups.into_iter().map(|(k,(total,passed))| json!({"grupo":k,"total":total,"cumplidos":passed})).collect();
    Ok(json!({"estado":"evidencia_laboratorio", "informe":REPORT, "total": rows.len(), "estados":counts,
        "grupos":groups, "casos":details, "restantes":rows.len()-shown, "production_certified":false,
        "vigencia":"Historica: repetir al cambiar fuentes, entradas, compilador o banderas; no acredita el proyecto abierto.",
        "aislamiento":"Declarado por el informe; esta consulta no lo autentica ni ejecuta herramientas.",
        "pendientes":["tsan","msan","libfuzzer","cfi-lto"]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> Value { json!({"schema":1,"kind":"ultra-llvm-ub-lab","production_certified":false,"cases":[
        {"id":"control","language":"c17","profile":"ubsan","variant":"bien","status":"passed"},
        {"id":"missing","language":"cpp20","profile":"asan","variant":"mal","status":"unavailable"}]}) }
    #[test] fn unavailable_is_never_passed_and_output_is_bounded() {
        let r=resumir(&report(),1).unwrap(); assert_eq!(r["estados"]["passed"],1);
        assert_eq!(r["casos"][0]["estado"],"unavailable"); assert_eq!(r["restantes"],1);
        assert_eq!(r["production_certified"],false);
    }
    #[test] fn rejects_forged_green_unknown_states_and_duplicate_ids() {
        let mut v=report();v["production_certified"]=json!(true);assert!(resumir(&v,10).is_err());
        v=report();v["cases"][0]["status"]=json!("all_ub_safe");assert!(resumir(&v,10).is_err());
        v=report();v["cases"][1]["id"]=json!("control");assert!(resumir(&v,10).is_err());
    }
    #[test] fn absent_evidence_is_explicit() {
        let root=std::env::temp_dir().join(format!("ultra-no-llvm-report-{}",std::process::id()));
        assert_eq!(consultar(&root,10).unwrap()["estado"],"sin_evidencia");
    }
    #[test] fn known_gap_is_visible_and_never_counts_as_passed() {
        let mut v=report(); v["cases"][1]["status"]=json!("known_gap");
        let r=resumir(&v,1).unwrap(); assert_eq!(r["estados"]["passed"],1);
        assert_eq!(r["estados"]["known_gap"],1); assert_eq!(r["casos"][0]["estado"],"known_gap");
    }
    #[cfg(unix)]
    #[test] fn rejects_redirected_report() {
        let root=std::env::temp_dir().join(format!("ultra-llvm-link-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let dir=root.join("build/llvm-ub");std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink("/dev/null",dir.join("current.json")).unwrap();
        assert!(consultar(&root,10).unwrap_err().contains("enlace"));
        std::fs::remove_file(dir.join("current.json")).unwrap();std::fs::remove_dir_all(root).unwrap();
    }
}
