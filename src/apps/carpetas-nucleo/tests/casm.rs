// SPDX-License-Identifier: GPL-2.0-only
//! El mapa .casm -> .hex con la salida real de casmc (copiada de build/CASM/run-*).
use carpetas_nucleo::casm::{hex_corto, leer_hex, mapa, Pieza};

const FUENTE: &str = include_str!("datos/casm/hello.casm");
const IR: &str = include_str!("datos/casm/hello.ir.json");
const HEX: &str = include_str!("datos/casm/hello.hex");

#[test]
fn cada_linea_con_sus_bytes() {
    let m = mapa(FUENTE, IR, HEX, "run-x").unwrap();
    assert_eq!(m.cuerpo, 40);
    // Linea 4: metal_puts(greeting) -> los 11 bytes de la llamada.
    let l4: Vec<_> = m.de_linea(4).collect();
    assert_eq!(l4.len(), 1);
    assert_eq!(hex_corto(&l4[0].bytes, 16), "48 8D 35 05 00 00 00 41 FF 57 10");
    assert_eq!(l4[0].pieza, Pieza::Llamada { constante: "greeting".into() });
    assert!(l4[0].lectura.contains("-> +0x0c (greeting)"), "{}", l4[0].lectura);
    // Linea 5: return; -> C3 en el desplazamiento 11.
    let l5: Vec<_> = m.de_linea(5).collect();
    assert_eq!((l5[0].offset, l5[0].bytes.as_slice(), l5[0].lectura.as_str()), (11, &[0xC3u8][..], "RET"));
    // Linea 3: la cadena con su cero.
    let l3: Vec<_> = m.de_linea(3).collect();
    assert_eq!(l3[0].offset, 12);
    assert!(l3[0].lectura.starts_with("\"Hola mundo desde mi .hex!\\r\\n\\0\" (28 bytes)"));
    assert!(m.de_linea(1).next().is_none(), "el comentario no emite bytes");
}

#[test]
fn rechaza_lo_que_no_cuadra() {
    // Un byte cambiado en la llamada, un byte de mas, otra ABI, un IR que no cubre el cuerpo.
    assert!(mapa(FUENTE, IR, &HEX.replacen("48 8D 35", "48 89 35", 1), "r").unwrap_err().contains("no es LEA"));
    assert!(mapa(FUENTE, IR, &format!("{HEX}90\n"), "r").unwrap_err().contains("41 bytes"));
    assert!(mapa(FUENTE, &IR.replace("metal64", "windows"), HEX, "r").unwrap_err().contains("solo x86_64"));
    assert!(mapa(FUENTE, &IR.replace("\"Offset\":11", "\"Offset\":12"), HEX, "r").is_err());
    assert!(mapa(FUENTE, &IR.replace("05 00 00 00", "06 00 00 00"), &HEX.replacen("35 05", "35 06", 1), "r").unwrap_err().contains("no apunta"));
    assert!(leer_hex("4G").is_err());
}

#[test]
fn las_katanas_salen_de_las_fuentes_de_casm() {
    use carpetas_nucleo::casm::{katanas, Etapa};
    let raiz = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../CASM/");
    let leer = |r: &str| std::fs::read_to_string(format!("{raiz}{r}")).unwrap();
    let archivos: Vec<(String, String)> = ["src/main.c", "src/frontend.c", "src/emitter.c", "verify.py", "test.py"].iter().map(|r| (format!("CASM/{r}"), leer(r))).collect();
    let refs: Vec<(&str, &str)> = archivos.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let k = katanas(&refs);
    let de = |r: &str, e: Etapa| k.iter().find(|x| x.regla == r && x.etapa == e).unwrap_or_else(|| panic!("falta {r} en {e:?}"));
    let indef = de("NAME-UNDEFINED", Etapa::Fuente);
    assert_eq!(indef.motivo, "String must be declared before use.");
    assert!(indef.sitios.iter().any(|(a, _)| a == "CASM/src/frontend.c"));
    assert_eq!(indef.casos, ["undefined-name"]);
    assert!(de("EMISSION-MISMATCH", Etapa::Verificador).sitios[0].0.ends_with("verify.py"));
    assert!(de("BODY-LIMIT", Etapa::Codificador).motivo.contains("4072"));
    assert!(de("SOURCE-SIZE", Etapa::Anfitrion).casos.contains(&"source-too-large".to_string()));
    // Cada caso rechazado de test.py demuestra una katana que existe.
    let casos: usize = k.iter().map(|x| x.casos.len()).sum();
    assert_eq!(casos, 23, "{k:#?}");
    assert!(k.len() >= 30, "{}", k.len());
}

#[test]
fn una_compilacion_verde_o_roja() {
    use carpetas_nucleo::casm::compilacion;
    let v = r#"{"Schema":1,"Verified":true,"Target":"x86_64","ABI":"metal64","PacketSHA256":"b3fd","PacketBytes":64,"BodyBytes":40}"#;
    let c = compilacion("run-1", "/x/hello.casm\n", Some(v), "GREEN [VERIFY] 64 bytes\nYELLOW [UNUSED-CONSTANT] line 2: a\n", true);
    assert!(c.verde && c.paquete == 64 && c.cuerpo == 40 && c.sha_paquete == "b3fd" && c.origen == "/x/hello.casm");
    assert_eq!(c.avisos.len(), 1);
    let r = compilacion("run-2", "", None, "RED [NAME-UNDEFINED] line 1: String must be declared before use.\nRED [COMPILE] Fuente rechazada.\n", false);
    assert!(!r.verde);
    assert_eq!(r.corte.as_ref().map(|x| x.0.as_str()), Some("NAME-UNDEFINED"));
    // Sin paquete no hay verde aunque el informe diga que si.
    assert!(!compilacion("run-3", "", Some(v), "", false).verde);
}
