// SPDX-License-Identifier: GPL-2.0-only
//! Leer y guardar archivos del editor de codigo. Reglas:
//! - Solo rutas relativas limpias dentro de la carpeta abierta, sin enlaces simbolicos
//!   en ningun tramo (ni en las carpetas de encima).
//! - Se comprueba que el archivo abierto es el mismo que se miro (dispositivo + inodo):
//!   nadie lo cambia por un enlace entre medias.
//! - Hasta 2 MiB y solo texto UTF-8 (un NUL = binario: no se abre).
//! - Guardar es atomico (temporal + rename) y conserva los permisos. Si el archivo cambio
//!   en disco desde que se abrio, no se pisa: se avisa.

use crate::disco::ruta_limpia;
use carpetas_nucleo::carriles::MAX_EDITOR;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Huella de lo que se abrio: si cambia, alguien mas toco el archivo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Huella { modificado: SystemTime, tam: u64, inodo: u64 }

fn huella(m: &fs::Metadata) -> Huella { Huella { modificado: m.modified().unwrap_or(SystemTime::UNIX_EPOCH), tam: m.len(), inodo: m.ino() } }

/// La ruta, solo si es un archivo normal y ningun tramo es un enlace.
pub fn archivo_seguro(base: &Path, rel: &str) -> Result<PathBuf, String> {
    if !ruta_limpia(rel) { return Err(format!("ruta rechazada: {rel}")); }
    let mut actual = base.to_path_buf();
    let tramos: Vec<&str> = rel.split('/').collect();
    for (i, t) in tramos.iter().enumerate() {
        actual.push(t);
        let m = fs::symlink_metadata(&actual).map_err(|e| format!("{rel}: {e}"))?;
        if m.file_type().is_symlink() { return Err(format!("{rel}: es (o pasa por) un enlace simbolico; no se abre")); }
        let ultimo = i + 1 == tramos.len();
        if ultimo && !m.is_file() { return Err(format!("{rel}: no es un archivo normal")); }
        if !ultimo && !m.is_dir() { return Err(format!("{rel}: ruta rara")); }
    }
    Ok(actual)
}

pub fn leer(base: &Path, rel: &str) -> Result<(String, Huella), String> {
    let ruta = archivo_seguro(base, rel)?;
    let antes = fs::symlink_metadata(&ruta).map_err(|e| e.to_string())?;
    if antes.len() > MAX_EDITOR { return Err(format!("{rel}: pasa de 2 MiB; el editor no lo abre")); }
    let mut f = fs::File::open(&ruta).map_err(|e| format!("{rel}: {e}"))?;
    let abierto = f.metadata().map_err(|e| e.to_string())?;
    if (abierto.dev(), abierto.ino()) != (antes.dev(), antes.ino()) { return Err(format!("{rel}: cambio mientras se abria; vuelve a intentar")); }
    let mut bytes = Vec::with_capacity(abierto.len() as usize);
    Read::by_ref(&mut f).take(MAX_EDITOR + 1).read_to_end(&mut bytes).map_err(|e| format!("{rel}: {e}"))?;
    if bytes.len() as u64 > MAX_EDITOR { return Err(format!("{rel}: pasa de 2 MiB; el editor no lo abre")); }
    if bytes.contains(&0) { return Err(format!("{rel}: parece binario; el editor solo abre texto")); }
    let texto = String::from_utf8(bytes).map_err(|_| format!("{rel}: no es texto UTF-8"))?;
    Ok((texto, huella(&abierto)))
}

pub fn guardar(base: &Path, rel: &str, texto: &str, esperada: Option<Huella>) -> Result<Huella, String> {
    if texto.len() as u64 > MAX_EDITOR { return Err("el texto pasa de 2 MiB".into()); }
    let ruta = archivo_seguro(base, rel)?;
    let actual = fs::symlink_metadata(&ruta).map_err(|e| e.to_string())?;
    if esperada.is_some_and(|h| h != huella(&actual)) {
        return Err(format!("{rel} cambio en disco desde que lo abriste: no se pisa. Copia tus cambios, cierra y vuelve a abrirlo"));
    }
    let carpeta = ruta.parent().ok_or("ruta sin carpeta")?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temporal = carpeta.join(format!(".{nombre}.ultra-{}.tmp", std::process::id()));
    let escribir = || -> std::io::Result<()> {
        let mut f = fs::OpenOptions::new().write(true).create_new(true).mode(actual.permissions().mode() & 0o7777).open(&temporal)?;
        f.write_all(texto.as_bytes())?;
        f.sync_all()?;
        fs::set_permissions(&temporal, fs::Permissions::from_mode(actual.permissions().mode() & 0o7777))?;
        fs::rename(&temporal, &ruta)
    };
    if let Err(e) = escribir() { let _ = fs::remove_file(&temporal); return Err(format!("no se pudo guardar {rel}: {e}")); }
    fs::symlink_metadata(&ruta).map(|m| huella(&m)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ultra-archivo-{nombre}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src")).unwrap();
        d
    }

    #[test]
    fn leer_guardar_y_conservar_permisos() {
        let d = carpeta("ida");
        fs::write(d.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::set_permissions(d.join("src/main.rs"), fs::Permissions::from_mode(0o640)).unwrap();
        let (t, h) = leer(&d, "src/main.rs").unwrap();
        assert_eq!(t, "fn main() {}\n");
        let h2 = guardar(&d, "src/main.rs", "fn main() { println!(\"ñ\"); }\n", Some(h)).unwrap();
        assert_eq!(fs::read_to_string(d.join("src/main.rs")).unwrap(), "fn main() { println!(\"ñ\"); }\n");
        assert_eq!(fs::metadata(d.join("src/main.rs")).unwrap().permissions().mode() & 0o777, 0o640);
        assert!(fs::read_dir(d.join("src")).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")), "sin temporales");
        // Alguien mas lo cambia: no se pisa.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(d.join("src/main.rs"), "otro editor\n").unwrap();
        let e = guardar(&d, "src/main.rs", "lo mio\n", Some(h2)).unwrap_err();
        assert!(e.contains("cambio en disco"), "{e}");
        assert_eq!(fs::read_to_string(d.join("src/main.rs")).unwrap(), "otro editor\n");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn enlaces_binarios_y_rutas_raras() {
        let d = carpeta("raros");
        fs::write(d.join("bin.dat"), [1u8, 0, 2]).unwrap();
        fs::write(d.join("latin1.txt"), [0xF1u8, b'a']).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", d.join("enlace.rs")).unwrap();
        std::os::unix::fs::symlink(d.join("src"), d.join("atajo")).unwrap();
        fs::write(d.join("src/a.rs"), "x").unwrap();
        assert!(leer(&d, "bin.dat").unwrap_err().contains("binario"));
        assert!(leer(&d, "latin1.txt").unwrap_err().contains("UTF-8"));
        assert!(leer(&d, "enlace.rs").unwrap_err().contains("enlace"));
        assert!(leer(&d, "atajo/a.rs").unwrap_err().contains("enlace"), "un enlace en una carpeta de encima tampoco");
        assert!(guardar(&d, "enlace.rs", "x", None).is_err());
        for r in ["../x", "/etc/passwd", "src/./a.rs", "", "src"] { assert!(leer(&d, r).is_err(), "{r}"); }
        let _ = fs::remove_dir_all(&d);
    }
}
