// SPDX-License-Identifier: GPL-2.0-only
//! A escala de Chromium: cargo test --release --test escala -- --ignored --nocapture
use carpetas_nucleo::dependencias::{conectar, Dependencias};
use std::time::Instant;

/// 500 000 archivos; cada uno incluye 8 de los anteriores, casi siempre de "base" (los
/// primeros), como las cabeceras comunes de Chromium. Determinista (LCG).
fn grafo(n: u32) -> Dependencias {
    let rutas: Vec<String> = (0..n).map(|i| format!("d{}/f{i}.h", i % 997)).collect();
    let mut s: u64 = 42;
    let mut azar = |m: u32| { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 33) % m.max(1) as u64) as u32 };
    let mut aristas = Vec::new();
    for i in 1..n {
        for _ in 0..8 {
            let tope = if azar(4) == 0 { i } else { i.min(2000) };
            aristas.push((i, azar(tope)));
        }
    }
    Dependencias::nuevo(rutas, &aristas)
}

#[test]
#[ignore]
fn alcance_y_fallo_conectado_a_escala() {
    let t = Instant::now();
    let d = grafo(500_000);
    eprintln!("grafo: {} archivos, {} aristas en {:?}", d.rutas.len(), d.aristas, t.elapsed());
    for max in [20_000usize, 1_000_000] {
        let t = Instant::now();
        let (v, recortado) = d.alcance(&[0], true, max);
        eprintln!("alcance desde la base (tope {max}): {} archivos, recortado {recortado}, {:?}", v.len(), t.elapsed());
    }
    let fallos: std::collections::BTreeSet<String> = (0..1500u32).map(|i| d.rutas[(i * 331 % 500_000) as usize].clone()).collect();
    let t = Instant::now();
    let c = conectar(&d, &fallos, &Default::default());
    eprintln!("fallo conectado con 1500 fallos: {} causas, {} papeles, truncada {}, {:?}", c.causas.len(), c.roles.len(), c.truncada, t.elapsed());
    // El peor caso: 300 cabeceras de la base fallan a la vez (cada una arrastra a casi todos).
    let base: std::collections::BTreeSet<String> = (1500..1800u32).map(|i| d.rutas[i as usize].clone()).collect();
    let t = Instant::now();
    let c = conectar(&d, &base, &Default::default());
    eprintln!("fallo conectado con 300 cabeceras de base: {} causas, {} papeles, la primera arrastra {}, {:?}", c.causas.len(), c.roles.len(), c.causas[0].arrastra, t.elapsed());
}
