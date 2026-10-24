// SPDX-License-Identifier: GPL-2.0-only
//! Ajustes de la app (no por carpeta): tema, brillo y si usar el acento del escritorio.
//! Como settings.json de VS Code, pero validado: lo desconocido vuelve al defecto.

use crate::grafo::{Direccion, Modo};
use crate::iconos::EstiloIconos;
use crate::tema::{por_id, Tema, TEMAS};
use crate::tipos::Rgb;
use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ajustes {
    pub tema: String,
    /// None = lo que diga el tema; Some = forzado.
    pub brillo: Option<bool>,
    /// Usar el acento de Personalizar en cualquier tema (en Neon BMO-X siempre).
    pub acento_escritorio: bool,
    pub iconos: EstiloIconos,
    pub direccion: Direccion,
    pub modo: Modo,
    /// Arbol lateral como el explorador de VS Code (Ctrl+B).
    pub explorador: bool,
}

impl Default for Ajustes {
    fn default() -> Self {
        Ajustes { tema: TEMAS[0].id.to_string(), brillo: None, acento_escritorio: false, iconos: EstiloIconos::Neon,
                  direccion: Direccion::Blender, modo: Modo::Iconos, explorador: true }
    }
}

impl Ajustes {
    pub fn de_json(bytes: &[u8]) -> Ajustes {
        if bytes.len() > 64 * 1024 { return Ajustes::default(); }
        let mut a: Ajustes = serde_json::from_slice(bytes).unwrap_or_default();
        if por_id(&a.tema).is_none() { a.tema = TEMAS[0].id.to_string(); }
        a
    }

    pub fn a_json(&self) -> String { serde_json::to_string_pretty(self).unwrap_or_default() }

    pub fn indice(&self) -> usize { por_id(&self.tema).unwrap_or(0) }

    /// El tema con lo que el usuario cambio encima.
    pub fn tema(&self, acento_escritorio: Rgb) -> Tema {
        let i = self.indice();
        let mut t = TEMAS[i];
        if i == 0 || self.acento_escritorio { t.acento = acento_escritorio; }
        if let Some(b) = self.brillo { t.brillo = b; }
        t
    }
}
