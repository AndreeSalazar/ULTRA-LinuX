// Mide Ctrl+P sobre una lista real de rutas (una por linea). Uso:
// cargo run --release --example medir_ir_a -- rutas.txt consulta
use carpetas_nucleo::proyecto::{buscar_rutas, Indice};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let texto = std::fs::read_to_string(&a[1]).unwrap();
    let mut i = Indice::default();
    for l in texto.lines() { i.agregar(l, false); }
    for q in a[2..].iter() {
        let t = std::time::Instant::now();
        let r = buscar_rutas(&i, q, 50);
        println!("{q:>22}: {:>6.1} ms  -> {}", t.elapsed().as_secs_f64() * 1000.0, r.iter().take(3).map(|k| i.ruta(*k)).collect::<Vec<_>>().join(" | "));
    }
}
