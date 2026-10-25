// Mide juntar + construir la ciudad con una lista real de rutas (una por linea).
use carpetas_nucleo::ciudad::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let texto = std::fs::read_to_string(&a[1]).unwrap();
    let v: Vec<ArchivoCiudad> = texto.lines().map(|l| ArchivoCiudad { ruta: l.into(), tam: 1000, lineas: Some(30), archivos: 1, bloque: false }).collect();
    let t = std::time::Instant::now();
    let j = juntar(v, "", MAX_EDIFICIOS);
    let tj = t.elapsed();
    let t = std::time::Instant::now();
    let c = construir("", &j, Huella::Bytes, Altura::Lineas, &|_| 0);
    println!("juntar: {tj:?} -> {} edificios; construir: {:?} -> {} distritos", j.len(), t.elapsed(), c.distritos.len());
}
