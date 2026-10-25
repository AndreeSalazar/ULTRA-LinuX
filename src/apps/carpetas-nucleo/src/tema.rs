// SPDX-License-Identifier: GPL-2.0-only
//! Temas como datos (no_std): la misma tabla sirve para ULTRA Carpetas en Linux y para
//! TITAN en BMO-X. Los colores de los temas publicos salen de sus repositorios oficiales
//! (ver docs/TEMAS-APPS.md); Neon BMO-X y TITAN Taller son propios.
//! `tenue` es el texto secundario oficial de cada paleta (fg_dark, subtext0, subtle,
//! oldWhite...), no el color de comentarios: tiene que pasar WCAG AA (4,5:1) sobre el
//! fondo. Dracula no trae un gris medio: su tenue es FG #f8f8f2 al 70 % sobre su fondo.

use crate::tipos::{Rgb, Tipo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tema {
    pub id: &'static str,
    pub nombre: &'static str,
    pub descripcion: &'static str,
    /// Lienzo, paneles, superficie (tarjetas, hover) y lineas.
    pub fondo: Rgb,
    pub panel: Rgb,
    pub superficie: Rgb,
    pub linea: Rgb,
    pub texto: Rgb,
    pub tenue: Rgb,
    pub acento: Rgb,
    /// Color del texto encima del acento (botones elegidos).
    pub sobre_acento: Rgb,
    pub error: Rgb,
    /// Imagen, video, audio, codigo, comprimido, programa, documento.
    pub tipos: [Rgb; 7],
    /// Radio de las esquinas en pixeles (0 = recto, estilo Neon).
    pub esquinas: u8,
    /// Brillo de neon alrededor de lo seleccionado.
    pub brillo: bool,
}

impl Tema {
    pub fn color_tipo(&self, tipo: Tipo) -> Rgb {
        match tipo {
            Tipo::Carpeta => self.acento,
            Tipo::Imagen => self.tipos[0], Tipo::Video => self.tipos[1], Tipo::Audio => self.tipos[2], Tipo::Codigo => self.tipos[3],
            Tipo::Comprimido => self.tipos[4], Tipo::Programa => self.tipos[5], Tipo::Documento => self.tipos[6],
            Tipo::Enlace | Tipo::Otro => self.tenue,
        }
    }

    /// Cinco colores para la vista previa del tema.
    pub fn muestras(&self) -> [Rgb; 5] {
        [self.fondo, self.panel, self.acento, self.tipos[0], self.tipos[3]]
    }
}

const fn h(v: u32) -> Rgb { [(v >> 16) as u8, (v >> 8) as u8, v as u8] }

pub const TEMAS: &[Tema] = &[
    Tema { id: "neon-bmox", nombre: "Neon BMO-X", descripcion: "Tu escritorio: negro, tu acento, esquinas rectas y brillo",
           fondo: h(0x06050a), panel: h(0x0f0e16), superficie: h(0x15141f), linea: h(0x2a2733), texto: h(0xece9dc), tenue: h(0xa19eae),
           acento: h(0x5ad8ff), sobre_acento: h(0x000000), error: h(0xff2a4f),
           tipos: [h(0xc03bff), h(0xff2a4f), h(0xfcee0a), h(0x00f0ff), h(0xff7a90), h(0x3dff9a), h(0xece9dc)], esquinas: 0, brillo: true },
    Tema { id: "titan-taller", nombre: "TITAN Taller", descripcion: "El de tu TITAN++: azul noche, cabeceras violeta y cables cyan",
           fondo: h(0x05071a), panel: h(0x0a0e2a), superficie: h(0x111640), linea: h(0x2b3170), texto: h(0xe6e9ff), tenue: h(0x9aa3d9),
           acento: h(0x7c4dff), sobre_acento: h(0xffffff), error: h(0xff4d6d),
           tipos: [h(0xd38cff), h(0xff6b9a), h(0xffd166), h(0x38d9ff), h(0xff9f6b), h(0x2fd1a0), h(0xe6e9ff)], esquinas: 4, brillo: true },
    Tema { id: "synthwave-84", nombre: "Synthwave '84", descripcion: "Retro ochentero: violeta, rosa y cyan (robb0wen)",
           fondo: h(0x171520), panel: h(0x241b2f), superficie: h(0x262335), linea: h(0x34294f), texto: h(0xffffff), tenue: h(0xb6b1cd),
           acento: h(0xff7edb), sobre_acento: h(0x171520), error: h(0xfe4450),
           tipos: [h(0xff7edb), h(0xfe4450), h(0xfede5d), h(0x36f9f6), h(0xff8b39), h(0x72f1b8), h(0xffffff)], esquinas: 2, brillo: true },
    Tema { id: "tokyo-night", nombre: "Tokyo Night", descripcion: "Azul noche de ciudad, sobrio y legible (folke)",
           fondo: h(0x16161e), panel: h(0x1a1b26), superficie: h(0x292e42), linea: h(0x414868), texto: h(0xc0caf5), tenue: h(0xa9b1d6),
           acento: h(0x7aa2f7), sobre_acento: h(0x16161e), error: h(0xf7768e),
           tipos: [h(0xbb9af7), h(0xf7768e), h(0xe0af68), h(0x7dcfff), h(0xff9e64), h(0x9ece6a), h(0xa9b1d6)], esquinas: 4, brillo: false },
    Tema { id: "catppuccin-mocha", nombre: "Catppuccin Mocha", descripcion: "Pasteles suaves sobre fondo oscuro (catppuccin)",
           fondo: h(0x11111b), panel: h(0x181825), superficie: h(0x313244), linea: h(0x45475a), texto: h(0xcdd6f4), tenue: h(0xa6adc8),
           acento: h(0xcba6f7), sobre_acento: h(0x11111b), error: h(0xf38ba8),
           tipos: [h(0xf5c2e7), h(0xf38ba8), h(0xf9e2af), h(0x74c7ec), h(0xfab387), h(0xa6e3a1), h(0xbac2de)], esquinas: 8, brillo: false },
    Tema { id: "rose-pine", nombre: "Rose Pine", descripcion: "Elegante y calmado: rosa, oro y espuma (rose-pine)",
           fondo: h(0x191724), panel: h(0x1f1d2e), superficie: h(0x26233a), linea: h(0x403d52), texto: h(0xe0def4), tenue: h(0x908caa),
           acento: h(0xebbcba), sobre_acento: h(0x191724), error: h(0xeb6f92),
           tipos: [h(0xc4a7e7), h(0xeb6f92), h(0xf6c177), h(0x9ccfd8), h(0xebbcba), h(0x31748f), h(0x908caa)], esquinas: 6, brillo: false },
    Tema { id: "kanagawa", nombre: "Kanagawa", descripcion: "Tinta japonesa y papel viejo (rebelot)",
           fondo: h(0x16161d), panel: h(0x1f1f28), superficie: h(0x2a2a37), linea: h(0x363646), texto: h(0xdcd7ba), tenue: h(0xc8c093),
           acento: h(0x7e9cd8), sobre_acento: h(0x16161d), error: h(0xe46876),
           tipos: [h(0x957fb8), h(0xe46876), h(0xe6c384), h(0x7fb4ca), h(0xffa066), h(0x98bb6c), h(0xc8c093)], esquinas: 3, brillo: false },
    Tema { id: "dracula", nombre: "Dracula", descripcion: "El clasico: purpura, rosa y verde (dracula)",
           fondo: h(0x191a21), panel: h(0x21222c), superficie: h(0x343746), linea: h(0x44475a), texto: h(0xf8f8f2), tenue: h(0xb5b6b6),
           acento: h(0xbd93f9), sobre_acento: h(0x191a21), error: h(0xff5555),
           tipos: [h(0xff79c6), h(0xff5555), h(0xf1fa8c), h(0x8be9fd), h(0xffb86c), h(0x50fa7b), h(0xf8f8f2)], esquinas: 4, brillo: false },
    Tema { id: "nord", nombre: "Nord", descripcion: "Artico y tranquilo: grises azulados (nordtheme)",
           fondo: h(0x2e3440), panel: h(0x3b4252), superficie: h(0x434c5e), linea: h(0x4c566a), texto: h(0xeceff4), tenue: h(0xd8dee9),
           acento: h(0x88c0d0), sobre_acento: h(0x2e3440), error: h(0xbf616a),
           tipos: [h(0xb48ead), h(0xbf616a), h(0xebcb8b), h(0x8fbcbb), h(0xd08770), h(0xa3be8c), h(0xe5e9f0)], esquinas: 4, brillo: false },
];

pub fn por_id(id: &str) -> Option<usize> {
    TEMAS.iter().position(|t| t.id == id)
}
