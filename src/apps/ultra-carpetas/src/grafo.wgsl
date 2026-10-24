// SPDX-License-Identifier: GPL-2.0-only
// El "libreto" de la GPU para el grafo de ULTRA Carpetas.
// La CPU sube la geometria UNA vez (en coordenadas del mundo). Cada cuadro solo llega
// la camara (escala y desplazamiento, 16 bytes) y la GPU recoloca todo ella sola.

struct Camara {
    escala: vec2<f32>,
    desplazamiento: vec2<f32>,
};

@group(0) @binding(0) var<uniform> camara: Camara;

struct Salida {
    @builtin(position) posicion: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(@location(0) mundo: vec2<f32>, @location(1) color: vec4<f32>) -> Salida {
    var s: Salida;
    s.posicion = vec4<f32>(mundo * camara.escala + camara.desplazamiento, 0.0, 1.0);
    s.color = color;
    return s;
}

@fragment
fn fs(entrada: Salida) -> @location(0) vec4<f32> {
    // Alfa premultiplicado, como pinta egui.
    return vec4<f32>(entrada.color.rgb * entrada.color.a, entrada.color.a);
}
