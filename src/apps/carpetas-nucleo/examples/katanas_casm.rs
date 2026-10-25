// SPDX-License-Identifier: GPL-2.0-only
//! Lista las katanas de CASM y si tienen caso aislado. Uso: cargo run --example katanas_casm -- <carpeta CASM>
fn main() {
    let raiz = std::env::args().nth(1).expect("carpeta CASM");
    let nombres = ["src/main.c", "src/frontend.c", "src/emitter.c", "verify.py", "test.py"];
    let a: Vec<(String, String)> = nombres.iter().map(|n| (format!("CASM/{n}"), std::fs::read_to_string(format!("{raiz}/{n}")).unwrap())).collect();
    let r: Vec<(&str, &str)> = a.iter().map(|(x, y)| (x.as_str(), y.as_str())).collect();
    let k = carpetas_nucleo::casm::katanas(&r);
    let con = k.iter().filter(|x| !x.casos.is_empty()).count();
    println!("{} katanas, {con} con caso", k.len());
    for x in &k { println!("{:<12} {:<20} {:<40} {}", format!("{:?}", x.etapa), x.regla, x.sitios.iter().map(|(f, l)| format!("{}:{l}", f.rsplit('/').next().unwrap())).collect::<Vec<_>>().join(","), if x.casos.is_empty() { "SIN CASO".into() } else { x.casos.join(",") }); }
}
