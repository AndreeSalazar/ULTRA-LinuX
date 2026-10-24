// SPDX-License-Identifier: GPL-2.0-only
//! Donde va cada icono en cada carpeta. Solo es la vista: nunca toca los archivos.
//!
//! Se guarda un JSON por carpeta en ~/.config/ultra/carpetas/disposicion/<huella>.json.
//! Un archivo danado, enorme o enlazado vuelve a una disposicion vacia (se recoloca sola).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CELDA: [f32; 2] = [112.0, 104.0];
const MAX_ARCHIVO: u64 = 1 << 20;
const MAX_HISTORIAL: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Disposicion {
    /// Nombre del archivo -> esquina superior izquierda del icono.
    pub posiciones: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    pub rejilla: bool,
    #[serde(default)]
    pub bloqueada: bool,
    #[serde(skip)]
    historial: Vec<BTreeMap<String, [f32; 2]>>,
}

/// Huella FNV-1a de 64 bits de la ruta: nombre de archivo estable y sin caracteres raros.
pub fn huella(ruta: &Path) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in ruta.as_os_str().as_encoded_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn archivo_para(base: &Path, carpeta: &Path) -> PathBuf {
    base.join("disposicion").join(format!("{}.json", huella(carpeta)))
}

/// Encaja una posicion en la rejilla (redondea a la celda mas cercana, nunca negativa).
pub fn ajustar(p: [f32; 2]) -> [f32; 2] {
    [(p[0] / CELDA[0]).round().max(0.0) * CELDA[0], (p[1] / CELDA[1]).round().max(0.0) * CELDA[1]]
}

fn columnas(ancho: f32) -> usize {
    ((ancho / CELDA[0]).floor() as usize).max(1)
}

fn celda_de(p: [f32; 2]) -> (usize, usize) {
    let a = ajustar(p);
    ((a[0] / CELDA[0]) as usize, (a[1] / CELDA[1]) as usize)
}

impl Disposicion {
    pub fn cargar(ruta: &Path) -> Disposicion {
        let Ok(meta) = fs::symlink_metadata(ruta) else { return Disposicion::default() };
        if !meta.file_type().is_file() || meta.len() > MAX_ARCHIVO {
            return Disposicion::default();
        }
        let mut d: Disposicion = fs::read(ruta).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // Valores absurdos (NaN, negativos, gigantes) no entran.
        d.posiciones.retain(|_, p| p.iter().all(|v| v.is_finite() && (0.0..=1.0e6).contains(v)));
        d
    }

    /// Escritura atomica: archivo temporal 0600 y rename. La carpeta no puede ser un enlace.
    pub fn guardar(&self, ruta: &Path) -> std::io::Result<()> {
        let carpeta = ruta.parent().ok_or_else(|| std::io::Error::other("ruta sin carpeta"))?;
        fs::create_dir_all(carpeta)?;
        if fs::symlink_metadata(carpeta)?.file_type().is_symlink() {
            return Err(std::io::Error::other("carpeta de disposicion enlazada"));
        }
        let temporal = carpeta.join(format!(".{}.tmp", std::process::id()));
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporal)?;
            f.write_all(&serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
            f.sync_all()?;
        }
        fs::rename(&temporal, ruta)
    }

    /// Olvida lo que ya no existe y coloca lo nuevo en la primera celda libre (por filas).
    pub fn sincronizar(&mut self, nombres: &[String], ancho: f32) -> bool {
        let presentes: BTreeSet<&String> = nombres.iter().collect();
        let antes = self.posiciones.len();
        self.posiciones.retain(|n, _| presentes.contains(n));
        let mut cambio = self.posiciones.len() != antes;
        let mut ocupadas: BTreeSet<(usize, usize)> = self.posiciones.values().map(|p| celda_de(*p)).collect();
        let cols = columnas(ancho);
        let mut siguiente = 0usize;
        for nombre in nombres {
            if self.posiciones.contains_key(nombre) {
                continue;
            }
            while ocupadas.contains(&(siguiente % cols, siguiente / cols)) {
                siguiente += 1;
            }
            let (c, f) = (siguiente % cols, siguiente / cols);
            ocupadas.insert((c, f));
            self.posiciones.insert(nombre.clone(), [c as f32 * CELDA[0], f as f32 * CELDA[1]]);
            cambio = true;
        }
        cambio
    }

    /// Guarda el estado actual para poder deshacer.
    pub fn recordar(&mut self) {
        self.historial.push(self.posiciones.clone());
        if self.historial.len() > MAX_HISTORIAL {
            self.historial.remove(0);
        }
    }

    pub fn deshacer(&mut self) -> bool {
        match self.historial.pop() {
            Some(previas) => { self.posiciones = previas; true }
            None => false,
        }
    }

    /// Mueve un grupo. Con candado no se mueve nada.
    pub fn mover(&mut self, nombres: &BTreeSet<String>, delta: [f32; 2]) {
        if self.bloqueada {
            return;
        }
        for n in nombres {
            if let Some(p) = self.posiciones.get_mut(n) {
                p[0] = (p[0] + delta[0]).clamp(0.0, 1.0e6);
                p[1] = (p[1] + delta[1]).clamp(0.0, 1.0e6);
            }
        }
    }

    /// Al soltar: con rejilla, cada icono del grupo a su celda; sin rejilla queda donde lo dejaste.
    pub fn soltar(&mut self, nombres: &BTreeSet<String>) {
        if self.rejilla {
            for n in nombres {
                if let Some(p) = self.posiciones.get_mut(n) {
                    *p = ajustar(*p);
                }
            }
        }
    }

    /// Reparte en rejilla siguiendo el orden dado (nombre, tipo o tamano lo decide quien llama).
    pub fn ordenar(&mut self, orden: &[String], ancho: f32) {
        if self.bloqueada {
            return;
        }
        self.recordar();
        let cols = columnas(ancho);
        for (i, n) in orden.iter().enumerate() {
            self.posiciones.insert(n.clone(), [(i % cols) as f32 * CELDA[0], (i / cols) as f32 * CELDA[1]]);
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn n(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }
    fn grupo(v: &[&str]) -> BTreeSet<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn coloca_nuevos_en_celdas_libres_sin_pisar() {
        let mut d = Disposicion::default();
        d.posiciones.insert("b".into(), [0.0, 0.0]); // ocupa la primera celda
        assert!(d.sincronizar(&n(&["a", "b", "c"]), CELDA[0] * 2.0));
        assert_eq!(d.posiciones["b"], [0.0, 0.0], "lo que ya tenia sitio no se mueve");
        assert_eq!(d.posiciones["a"], [CELDA[0], 0.0]);
        assert_eq!(d.posiciones["c"], [0.0, CELDA[1]]);
        assert!(!d.sincronizar(&n(&["a", "b", "c"]), 500.0), "sin cambios no hay que guardar");
        assert!(d.sincronizar(&n(&["a"]), 500.0));
        assert_eq!(d.posiciones.len(), 1, "lo borrado del disco se olvida");
    }

    #[test]
    fn mover_soltar_con_rejilla_candado_y_deshacer() {
        let mut d = Disposicion::default();
        d.sincronizar(&n(&["x", "y"]), 1000.0);
        let origen = d.posiciones.clone();
        d.recordar();
        d.mover(&grupo(&["x", "y"]), [37.0, 250.0]);
        assert_eq!(d.posiciones["x"], [37.0, 250.0], "sin rejilla queda donde lo sueltas");
        d.rejilla = true;
        d.soltar(&grupo(&["x", "y"]));
        assert_eq!(d.posiciones["x"], [0.0, 2.0 * CELDA[1]]);
        assert!(d.deshacer());
        assert_eq!(d.posiciones, origen);
        assert!(!d.deshacer());
        d.bloqueada = true;
        d.mover(&grupo(&["x"]), [500.0, 500.0]);
        d.ordenar(&n(&["y", "x"]), 1000.0);
        assert_eq!(d.posiciones, origen, "con candado nada se mueve");
        d.mover(&grupo(&["x"]), [-9999.0, f32::MAX]);
        d.bloqueada = false;
        d.mover(&grupo(&["x"]), [-9999.0, 2.0e6]);
        assert_eq!(d.posiciones["x"], [0.0, 1.0e6], "nunca negativo ni infinito");
    }

    #[test]
    fn ordenar_reparte_por_filas() {
        let mut d = Disposicion::default();
        d.ordenar(&n(&["c", "a", "b"]), CELDA[0] * 2.0 + 5.0);
        assert_eq!(d.posiciones["c"], [0.0, 0.0]);
        assert_eq!(d.posiciones["a"], [CELDA[0], 0.0]);
        assert_eq!(d.posiciones["b"], [0.0, CELDA[1]]);
    }

    #[test]
    fn guardar_y_cargar_rechaza_lo_raro() {
        let base = std::env::temp_dir().join(format!("ultra-carpetas-prueba-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let ruta = archivo_para(&base, Path::new("/home/ultra/Imágenes"));
        let mut d = Disposicion::default();
        d.sincronizar(&n(&["foto.png", "notas.txt"]), 800.0);
        d.rejilla = true;
        d.guardar(&ruta).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&ruta).unwrap().permissions().mode() & 0o777, 0o600);
        let leida = Disposicion::cargar(&ruta);
        assert_eq!(leida.posiciones, d.posiciones);
        assert!(leida.rejilla);
        // JSON danado, valores absurdos y enlaces: disposicion vacia o filtrada.
        fs::write(&ruta, b"{no es json").unwrap();
        assert_eq!(Disposicion::cargar(&ruta), Disposicion::default());
        fs::write(&ruta, br#"{"posiciones":{"a":[1e30,2],"b":[-5,1],"c":[10,20]}}"#).unwrap();
        assert_eq!(Disposicion::cargar(&ruta).posiciones.keys().collect::<Vec<_>>(), vec!["c"]);
        let enlace = base.join("enlace.json");
        std::os::unix::fs::symlink(&ruta, &enlace).unwrap();
        assert_eq!(Disposicion::cargar(&enlace), Disposicion::default());
        assert_ne!(huella(Path::new("/a")), huella(Path::new("/b")));
        let _ = fs::remove_dir_all(&base);
    }
}
