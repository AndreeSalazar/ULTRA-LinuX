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
