// SPDX-License-Identifier: GPL-2.0-only
//! El mapa de CASM para el SUPER GRAFO: que bytes x86-64 salen de cada linea de un
//! programa `.casm`. Puro (no_std): la plataforma lee los archivos de una compilacion
//! verificada (`build/CASM/run-*`) y aqui se arma y se vuelve a comprobar el mapa.
//!
//! Precision antes que comodidad:
//! - Solo vale una compilacion cuyo `source.casm` congelado es identico, byte a byte, al
//!   texto que se muestra (`fuente`). Si se edita sin recompilar, no se ensenan bytes viejos.
//! - Los tramos se reconstruyen desde el IR y se comprueban contra el HEX: cuerpo completo,
//!   tramos contiguos, `LEA RSI,[RIP+d] ; CALL [R15+16]` en las llamadas, `RET` al final.
//!   Si algo no cuadra, no hay mapa (con el motivo).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Deserialize;

/// El IR que escribe casmc (`program.ir.json`, esquema 1).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Ir {
    schema: u32,
    target: String,
    #[serde(rename = "ABI")]
    abi: String,
    constants: Vec<ConstanteIr>,
    calls: Vec<LlamadaIr>,
    #[serde(rename = "Return")]
    retorno: PosIr,
    body_bytes: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ConstanteIr { name: String, line: u32, offset: u32, hex: String }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LlamadaIr { constant: u32, line: u32, offset: u32 }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PosIr { line: u32, offset: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pieza {
    /// `metal_puts(nombre)`: LEA RSI,[RIP+d] ; CALL [R15+16].
    Llamada { constante: String },
    Retorno,
    /// Una cadena constante en la region de datos (con su cero final).
    Dato { nombre: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tramo {
    pub linea: u32,
    /// Desplazamiento dentro del cuerpo (el paquete anade 24 bytes de cabecera delante).
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub pieza: Pieza,
    /// Lo que hacen esos bytes, en ensamblador o como texto.
    pub lectura: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapaCasm {
    /// El `source.casm` congelado de la compilacion: el mapa solo vale para este texto.
    pub fuente: String,
    pub tramos: Vec<Tramo>,
    pub cuerpo: u32,
    /// Carpeta de la compilacion (build/CASM/run-...).
    pub run: String,
}

impl MapaCasm {
    pub fn de_linea(&self, linea: u32) -> impl Iterator<Item = &Tramo> { self.tramos.iter().filter(move |t| t.linea == linea) }
}

const LLAMADA: usize = 11;

/// Bytes de `program.hex`: lineas de pares hexadecimales; `;` empieza un comentario.
pub fn leer_hex(texto: &str) -> Result<Vec<u8>, String> {
    let mut v = Vec::new();
    for (n, l) in texto.lines().enumerate() {
        let l = l.split(';').next().unwrap_or("");
        for par in l.split_whitespace() {
            if par.len() != 2 { return Err(format!("program.hex linea {}: '{par}' no es un byte", n + 1)); }
            v.push(u8::from_str_radix(par, 16).map_err(|_| format!("program.hex linea {}: '{par}' no es hexadecimal", n + 1))?);
        }
    }
    Ok(v)
}

/// Arma y comprueba el mapa de una compilacion. `fuente`: su source.casm congelado.
pub fn mapa(fuente: &str, ir_json: &str, hex: &str, run: &str) -> Result<MapaCasm, String> {
    let ir: Ir = serde_json::from_str(ir_json).map_err(|e| format!("program.ir.json no se entiende: {e}"))?;
    if ir.schema != 1 || ir.target != "x86_64" || ir.abi != "metal64" {
        return Err(format!("solo x86_64 / metal64 esquema 1 (vino {} / {} esquema {})", ir.target, ir.abi, ir.schema));
    }
    let cuerpo = leer_hex(hex)?;
    if cuerpo.len() != ir.body_bytes as usize { return Err(format!("el HEX tiene {} bytes y el IR dice {}", cuerpo.len(), ir.body_bytes)); }
    // (offset, largo, linea, pieza)
    let mut piezas: Vec<(u32, usize, u32, Pieza)> = Vec::new();
    for c in &ir.calls {
        let nombre = ir.constants.get(c.constant as usize).map(|k| k.name.clone()).ok_or("una llamada apunta a una constante que no existe")?;
        piezas.push((c.offset, LLAMADA, c.line, Pieza::Llamada { constante: nombre }));
    }
    piezas.push((ir.retorno.offset, 1, ir.retorno.line, Pieza::Retorno));
    for k in &ir.constants {
        if k.hex.len() % 2 != 0 { return Err(format!("la constante {} tiene un HEX impar", k.name)); }
        piezas.push((k.offset, k.hex.len() / 2, k.line, Pieza::Dato { nombre: k.name.clone() }));
    }
    piezas.sort_by_key(|p| p.0);
    // Contiguos y cubriendo el cuerpo entero, sin huecos ni solapes.
    let mut esperado = 0u32;
    for (off, largo, _, _) in &piezas {
        if *off != esperado { return Err(format!("hueco o solape en el desplazamiento {off:#x} (se esperaba {esperado:#x})")); }
        esperado = off + *largo as u32;
    }
    if esperado as usize != cuerpo.len() { return Err(format!("los tramos cubren {esperado} bytes de {}", cuerpo.len())); }
    let mut tramos = Vec::with_capacity(piezas.len());
    for (off, largo, linea, pieza) in piezas {
        let bytes = cuerpo[off as usize..off as usize + largo].to_vec();
        let lectura = match &pieza {
            Pieza::Llamada { constante } => {
                if bytes[..3] != [0x48, 0x8D, 0x35] || bytes[7..] != [0x41, 0xFF, 0x57, 0x10] {
                    return Err(format!("la llamada de la linea {linea} no es LEA RSI,[RIP+d] ; CALL [R15+16]"));
                }
                let d = i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
                let destino = off as i64 + 7 + d as i64;
                let dato = ir.constants.iter().find(|k| k.name == *constante).map(|k| k.offset as i64);
                if dato != Some(destino) { return Err(format!("la llamada de la linea {linea} no apunta a '{constante}'")); }
                format!("LEA RSI, [RIP{d:+#x}] -> +{destino:#04x} ({constante}) ; CALL [R15+0x10] (metal_puts)")
            }
            Pieza::Retorno => {
                if bytes != [0xC3] { return Err(format!("el retorno de la linea {linea} no es C3")); }
                String::from("RET")
            }
            Pieza::Dato { nombre } => {
                if bytes.last() != Some(&0) { return Err(format!("la cadena '{nombre}' no termina en cero")); }
                let texto: String = bytes[..bytes.len() - 1].iter().map(|b| match b { b'\r' => "\\r".to_string(), b'\n' => "\\n".to_string(),
                    0x20..=0x7E => (*b as char).to_string(), _ => format!("\\x{b:02X}") }).collect();
                format!("\"{texto}\\0\" ({} bytes)", bytes.len())
            }
        };
        tramos.push(Tramo { linea, offset: off, bytes, pieza, lectura });
    }
    Ok(MapaCasm { fuente: fuente.to_string(), tramos, cuerpo: cuerpo.len() as u32, run: run.to_string() })
}

/// "48 8D 35 05" (como mucho `max` bytes; si hay mas, "… (N bytes)").
pub fn hex_corto(bytes: &[u8], max: usize) -> String {
    let mut s: String = bytes.iter().take(max).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
    if bytes.len() > max { s.push_str(&format!(" … ({} bytes)", bytes.len())); }
    s
}

// ====================================================================================
// El taller: las katanas de CASM (que regla corta y donde) y sus compilaciones.
// Todo sale de los propios archivos de CASM: nada se escribe a mano aqui.
// ====================================================================================

/// En que paso de la cadena corta una regla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Etapa {
    /// `main.c`: leer la fuente, memoria y escritura en el anfitrion.
    Anfitrion,
    /// `frontend.c`: lexico, sintaxis, nombres y tipos (rechaza antes de emitir).
    Fuente,
    /// `emitter.c`: el codificador x86-64 y los limites del cuerpo.
    Codificador,
    /// `verify.py`: reconstruye cada byte permitido y solo entonces publica el paquete.
    Verificador,
}

impl Etapa {
    pub fn nombre(self) -> &'static str {
        match self { Etapa::Anfitrion => "Anfitrion", Etapa::Fuente => "Frontend", Etapa::Codificador => "Codificador x86-64", Etapa::Verificador => "Verificador" }
    }
    fn de_archivo(ruta: &str) -> Option<Etapa> {
        let n = ruta.rsplit('/').next().unwrap_or(ruta);
        match n { "main.c" => Some(Etapa::Anfitrion), "frontend.c" => Some(Etapa::Fuente), "emitter.c" => Some(Etapa::Codificador), "verify.py" => Some(Etapa::Verificador), _ => None }
    }
}

/// Una katana: una regla que CORTA (nada se emite o nada se publica) y donde esta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Katana {
    pub regla: String,
    pub etapa: Etapa,
    /// (archivo, linea) de cada sitio que la aplica.
    pub sitios: Vec<(String, u32)>,
    /// El motivo que imprime, si esta escrito al lado.
    pub motivo: String,
    /// Casos de `test.py` que la provocan a proposito.
    pub casos: Vec<String>,
}

fn entre_comillas(s: &str) -> Vec<&str> {
    let mut v = Vec::new();
    let mut resto = s;
    while let Some(i) = resto.find(|c| c == '"' || c == '\'') {
        let q = resto.as_bytes()[i];
        let tras = &resto[i + 1..];
        let Some(f) = tras.find(q as char) else { break };
        v.push(&tras[..f]);
        resto = &tras[f + 1..];
    }
    v
}

fn es_regla(r: &str) -> bool { !r.is_empty() && r.len() <= 40 && r.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-') && r.bytes().any(|b| b.is_ascii_uppercase()) }

/// Las katanas de CASM leidas de sus fuentes: `fail(p, "REGLA", "motivo")` y `RED [REGLA]`
/// en C, `require(..., 'REGLA')` en el verificador, y los casos rechazados de `test.py`.
pub fn katanas(archivos: &[(&str, &str)]) -> Vec<Katana> {
    let mut mapa: alloc::collections::BTreeMap<(Etapa, String), Katana> = alloc::collections::BTreeMap::new();
    let mut casos: Vec<(String, String)> = Vec::new();
    for (ruta, texto) in archivos {
        let nombre = ruta.rsplit('/').next().unwrap_or(ruta);
        if nombre == "test.py" {
            // Fuente:      'caso': (b'...','REGLA'),        -> la primera etapa que la tiene
            // Anfitrion:   'host-x': ('REGLA', [...], ...), -> idem
            // Verificador: 'verify-x': ('REGLA', ...),     -> el verificador
            for l in texto.lines() {
                let l = l.trim();
                let Some((caso, resto)) = l.strip_prefix('\'').and_then(|x| x.split_once("':")) else { continue };
                let resto = resto.trim_start();
                let partes = entre_comillas(resto.trim_end_matches(','));
                let regla = if resto.starts_with("(b") { partes.last() } else if resto.starts_with("('") { partes.first() } else { None };
                if let Some(r) = regla.filter(|r| es_regla(r)) { casos.push((r.to_string(), caso.to_string())); }
            }
            continue;
        }
        let Some(etapa) = Etapa::de_archivo(ruta) else { continue };
        for (k, l) in texto.lines().enumerate() {
            let linea = k as u32 + 1;
            let mut hallado: Vec<(String, String)> = Vec::new();
            for trozo in l.split("fail(").skip(1) {
                let q = entre_comillas(trozo);
                if let Some(r) = q.first().filter(|r| es_regla(r)) { hallado.push((r.to_string(), q.get(1).map(|x| x.to_string()).unwrap_or_default())); }
            }
            for trozo in l.split("RED [").skip(1) {
                if let Some((r, resto)) = trozo.split_once(']') {
                    // El motivo termina donde acaba la cadena de C: en el \n escrito o en la comilla.
                    let motivo = resto.split("\\n").next().unwrap_or("").split('"').next().unwrap_or("").trim();
                    if es_regla(r) { hallado.push((r.to_string(), motivo.to_string())); }
                }
            }
            for trozo in l.split("require(").skip(1) {
                if let Some(r) = entre_comillas(trozo).last() {
                    let r = r.split(':').next().unwrap_or(r).trim();
                    if es_regla(r) { hallado.push((r.to_string(), String::new())); }
                }
            }
            for (r, motivo) in hallado {
                let e = mapa.entry((etapa, r.clone())).or_insert_with(|| Katana { regla: r, etapa, sitios: Vec::new(), motivo: String::new(), casos: Vec::new() });
                if !e.sitios.iter().any(|(a, b)| a == *ruta && *b == linea) { e.sitios.push((ruta.to_string(), linea)); }
                if e.motivo.is_empty() && !motivo.is_empty() { e.motivo = motivo; }
            }
        }
    }
    let mut v: Vec<Katana> = mapa.into_values().collect();
    for (r, caso) in casos {
        // Los "verify-" prueban el verificador; el resto se compila, y corta en la primera
        // etapa que tiene la regla (la que llega antes).
        let verificador = caso.starts_with("verify-");
        if let Some(k) = v.iter_mut().find(|k| k.regla == r && (k.etapa == Etapa::Verificador) == verificador) { k.casos.push(caso); }
    }
    v
}

/// Una compilacion de `build/CASM/run-*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compilacion {
    pub run: String,
    /// La fuente original (de origen.txt), si se sabe.
    pub origen: String,
    /// Hay paquete publicado: el verificador lo aprobo.
    pub verde: bool,
    pub paquete: u32,
    pub cuerpo: u32,
    pub sha_paquete: String,
    /// La regla que corto (si fue roja) y su linea completa.
    pub corte: Option<(String, String)>,
    /// Lineas YELLOW del registro.
    pub avisos: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Verificado { verified: bool, target: String, #[serde(rename = "ABI")] abi: String, #[serde(rename = "PacketSHA256")] packet_sha256: String, packet_bytes: u32, body_bytes: u32 }

/// Arma una compilacion con lo que haya en su carpeta (cada pieza puede faltar).
pub fn compilacion(run: &str, origen: &str, verificado: Option<&str>, registro: &str, hay_paquete: bool) -> Compilacion {
    let v: Option<Verificado> = verificado.and_then(|j| serde_json::from_str(j).ok()).filter(|v: &Verificado| v.verified && v.target == "x86_64" && v.abi == "metal64");
    let corte = registro.lines().find_map(|l| {
        let r = l.strip_prefix("RED [")?.split_once(']')?;
        (r.0 != "COMPILE").then(|| (r.0.to_string(), l.to_string()))
    });
    let avisos = registro.lines().filter(|l| l.starts_with("YELLOW [")).map(|l| l.to_string()).collect();
    let verde = hay_paquete && v.is_some() && corte.is_none();
    Compilacion { run: run.to_string(), origen: origen.trim().to_string(), verde,
                  paquete: v.as_ref().map(|v| v.packet_bytes).unwrap_or(0), cuerpo: v.as_ref().map(|v| v.body_bytes).unwrap_or(0),
                  sha_paquete: v.map(|v| v.packet_sha256).unwrap_or_default(), corte, avisos }
}

/// Lo que el SUPER GRAFO sabe del taller CASM de la carpeta abierta.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Taller {
    /// Carpeta de CASM, relativa a la abierta ("" si es la propia).
    pub carpeta: String,
    pub katanas: Vec<Katana>,
    /// La mas nueva primero.
    pub compilaciones: Vec<Compilacion>,
    /// build/CASM/host/toolchain.txt (compilador, flags y huellas).
    pub toolchain: String,
}
