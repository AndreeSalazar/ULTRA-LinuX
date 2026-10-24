// SPDX-License-Identifier: GPL-2.0-only
//! Donde guarda Linux el orden de cada carpeta. El nucleo da y recibe el texto JSON.

use carpetas_nucleo::disposicion::{huella, MAX_JSON};
use carpetas_nucleo::Disposicion;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn base() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(x).join("ultra/carpetas");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/home/ultra".into())).join(".config/ultra/carpetas")
}

pub fn archivo_para(base: &Path, carpeta: &Path) -> PathBuf {
    base.join("disposicion").join(format!("{:016x}.json", huella(carpeta.as_os_str().as_encoded_bytes())))
}

/// Enlaces, archivos que no son normales o enormes: disposicion vacia.
pub fn cargar(ruta: &Path) -> Disposicion {
    let Ok(meta) = fs::symlink_metadata(ruta) else { return Disposicion::default() };
    if !meta.file_type().is_file() || meta.len() > MAX_JSON as u64 {
        return Disposicion::default();
    }
    fs::read(ruta).map(|b| Disposicion::de_json(&b)).unwrap_or_default()
}

pub fn archivo_ajustes(base: &Path) -> PathBuf { base.join("ajustes.json") }

/// Ajustes de la app (tema, brillo); con las mismas reglas que el orden.
pub fn cargar_ajustes(ruta: &Path) -> carpetas_nucleo::Ajustes {
    let Ok(meta) = fs::symlink_metadata(ruta) else { return Default::default() };
    if !meta.file_type().is_file() || meta.len() > 64 * 1024 {
        return Default::default();
    }
    fs::read(ruta).map(|b| carpetas_nucleo::Ajustes::de_json(&b)).unwrap_or_default()
}

/// Escritura atomica: temporal 0600 y rename. La carpeta no puede ser un enlace.
pub fn guardar(ruta: &Path, json: &str) -> std::io::Result<()> {
    let carpeta = ruta.parent().ok_or_else(|| std::io::Error::other("ruta sin carpeta"))?;
    fs::create_dir_all(carpeta)?;
    if fs::symlink_metadata(carpeta)?.file_type().is_symlink() {
        return Err(std::io::Error::other("carpeta de disposicion enlazada"));
    }
    let temporal = carpeta.join(format!(".{}.tmp", std::process::id()));
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporal)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&temporal, ruta)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn guardar_y_cargar_en_disco() {
        let base = std::env::temp_dir().join(format!("ultra-carpetas-almacen-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let ruta = archivo_para(&base, Path::new("/home/ultra/Imágenes"));
        let mut d = Disposicion::default();
        d.sincronizar(&["a.png".to_string(), "b.txt".to_string()], 800.0);
        d.rejilla = true;
        guardar(&ruta, &d.a_json()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&ruta).unwrap().permissions().mode() & 0o777, 0o600);
        let leida = cargar(&ruta);
        assert_eq!(leida.posiciones, d.posiciones);
        assert!(leida.rejilla);
        let enlace = base.join("enlace.json");
        std::os::unix::fs::symlink(&ruta, &enlace).unwrap();
        assert_eq!(cargar(&enlace), Disposicion::default());
        assert_ne!(archivo_para(&base, Path::new("/a")), archivo_para(&base, Path::new("/b")));
        let _ = fs::remove_dir_all(&base);
    }
}
