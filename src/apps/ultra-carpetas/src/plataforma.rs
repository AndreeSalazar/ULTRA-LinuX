// SPDX-License-Identifier: GPL-2.0-only
//! Lo que cambia entre Linux y Windows al tocar archivos, en un solo sitio.
//!
//! - **Identidad de un archivo:** en Unix, (dispositivo, inodo). En Windows, la biblioteca
//!   estable de Rust no da el índice del archivo, así que vale 0 y la huella se queda con la
//!   fecha y el tamaño.
//! - **Permisos al crear:** en Unix, el modo (`0o600` o el del archivo original). En Windows
//!   no hay modo: solo se copia «solo lectura».

use std::fs;

/// (dispositivo, inodo) en Unix; (0, 0) en Windows.
pub fn identidad(m: &fs::Metadata) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (m.dev(), m.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        (0, 0)
    }
}

/// Crea el archivo con el modo dado en Unix (en Windows, sin modo).
pub fn con_modo(o: &mut fs::OpenOptions, modo: u32) -> &mut fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(modo)
    }
    #[cfg(not(unix))]
    {
        let _ = modo;
        o
    }
}

/// El modo del archivo (los 12 bits bajos) en Unix; 0o644 en Windows.
pub fn modo(m: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o7777
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        0o644
    }
}

/// Los permisos que tendrá la copia: el modo en Unix; «solo lectura» del original en Windows.
pub fn permisos_como(original: &fs::Metadata) -> fs::Permissions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::Permissions::from_mode(modo(original))
    }
    #[cfg(not(unix))]
    {
        original.permissions()
    }
}
