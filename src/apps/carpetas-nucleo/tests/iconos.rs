// SPDX-License-Identifier: GPL-2.0-only
//! Iconos reconocibles: logos de lenguaje (Rust, C, C++...) tambien en el estilo Neon.
use carpetas_nucleo::iconos::{claves, icono};
use carpetas_nucleo::{tipo_por_extension, EstiloIconos, Tipo};

fn neon(n: &str) -> &'static str { icono(n, tipo_por_extension(n), EstiloIconos::Neon) }
fn material(n: &str) -> &'static str { icono(n, tipo_por_extension(n), EstiloIconos::Material) }

#[test]
fn familia_c_y_cpp_completa() {
    for (n, c) in [("main.c", "material/c"), ("vga.h", "material/h"), ("motor.cpp", "material/cpp"), ("a.cc", "material/cpp"),
                   ("b.cxx", "material/cpp"), ("m.cppm", "material/cpp"), ("motor.hpp", "material/hpp"), ("x.hh", "material/hpp"),
                   ("t.inl", "material/hpp"), ("k.cu", "material/cuda"), ("boot.s", "material/assembly"), ("boot.asm", "material/assembly"),
                   ("CMakeLists.txt", "material/cmake"), ("x.cmake", "material/cmake"), ("Makefile", "material/makefile"),
                   ("meson.build", "material/meson"), (".clang-format", "material/clangd"), ("compile_commands.json", "material/clangd"),
                   ("MAIN.CPP", "material/cpp")] {
        assert_eq!(material(n), c, "{n}");
        assert_eq!(neon(n), c, "{n}: en Neon tambien se ve el logo");
        assert_eq!(tipo_por_extension(n), Tipo::Codigo, "{n} es codigo (color y play)");
    }
}

#[test]
fn rust_y_otros_lenguajes() {
    for (n, c) in [("main.rs", "material/rust"), ("app.py", "material/python"), ("m.go", "material/go"), ("A.java", "material/java"),
                   ("z.zig", "material/zig"), ("s.wgsl", "material/shader"), ("f.frag", "material/shader"), ("x.tsx", "material/react_ts"),
                   ("Cargo.toml", "material/toml"), ("r.sh", "material/console")] {
        assert_eq!(neon(n), c, "{n}");
    }
}

#[test]
fn neon_sigue_dibujando_lo_que_no_es_codigo() {
    for n in ["foto.png", "notas.txt", "video.mp4", "LICENSE", "Cargo.lock", "raro.xyz", "sin_extension"] {
        assert_eq!(neon(n), "", "{n}");
    }
    assert_eq!(icono("src", Tipo::Carpeta, EstiloIconos::Neon), "");
    assert_eq!(icono("main.rs", Tipo::Enlace, EstiloIconos::Neon), "", "un enlace no se disfraza de codigo");
    assert_eq!(tipo_por_extension("notas.txt"), Tipo::Documento, "un .txt normal no es codigo");
    assert_eq!(tipo_por_extension(".rs"), Tipo::Otro, "solo extension, sin nombre");
}

#[test]
fn claves_sin_repetir_formato() {
    for c in claves() { assert!(c.starts_with("material/") && !c.contains(".."), "{c}"); }
}
