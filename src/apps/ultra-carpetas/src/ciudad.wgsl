// SPDX-License-Identifier: GPL-2.0-only
// La ciudad (vista Ciudad de ULTRA Carpetas): cada edificio y cada distrito es una
// instancia; el cubo lo arma este shader con el numero de vertice (36 por caja), asi la
// CPU solo sube 44 bytes por edificio. Girar o acercar la camara solo cambia 96 bytes.
//
// La cronologia (ciudad fantasma): cada instancia sabe cuando nacio (0 = primer commit,
// 1 = el ultimo). Mover el tiempo solo cambia `tiempo.x`: lo que aun no nacio es un solar
// oscuro y bajito; lo que acaba de nacer brilla.

struct Camara {
    vp: mat4x4<f32>,
    // xyz: hacia donde esta la luz; w: ancho del borde oscuro (en unidades del mundo).
    luz: vec4<f32>,
    // x: el momento que se mira (0..1; negativo = hoy, sin cronologia); y: cuanto dura el
    // brillo de lo recien nacido.
    tiempo: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camara: Camara;

const CUBO = array<vec3<f32>, 36>(
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(0.0, 1.0, 1.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(0.0, 1.0, 1.0),
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 1.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(0.0, 0.0, 1.0)
);

const NORMALES = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, -1.0, 0.0)
);

struct Salida {
    @builtin(position) posicion: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Coordenadas dentro de la caja (0..1) y su tamano: para oscurecer los bordes.
    @location(1) local: vec3<f32>,
    @location(2) tam: vec3<f32>,
    @location(3) @interpolate(flat) cara: u32,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, @location(0) suelo: vec4<f32>, @location(1) alto: vec2<f32>, @location(2) color: vec4<f32>,
      @location(3) nace: f32) -> Salida {
    let p = CUBO[vi];
    var h = alto.y;
    var base = color.rgb;
    let ahora = camara.tiempo.x;
    if (ahora >= 0.0) {
        if (nace > ahora + 0.000001) {
            // Aun no existe: el solar, bajito y oscuro (se ve donde ira).
            h = min(h, 0.6);
            base = vec3<f32>(0.11, 0.12, 0.16);
        } else if (ahora - nace <= camara.tiempo.y) {
            // Recien nacido: brilla (mezcla con un dorado claro).
            base = mix(base, vec3<f32>(1.0, 0.9, 0.5), 0.65);
        }
    }
    let tam = vec3<f32>(suelo.z, h, suelo.w);
    let mundo = vec3<f32>(suelo.x, alto.x, suelo.y) + p * tam;
    let cara = vi / 6u;
    let n = NORMALES[cara];
    // Luz: ambiente mas difusa; el techo un poco mas claro (se lee como en CodeCity).
    let d = max(dot(n, normalize(camara.luz.xyz)), 0.0);
    var k = 0.42 + 0.58 * d;
    if (cara == 0u) { k = k * 1.06; }
    var s: Salida;
    s.posicion = camara.vp * vec4<f32>(mundo, 1.0);
    s.color = vec4<f32>(base * k, color.a);
    s.local = p;
    s.tam = tam;
    s.cara = cara;
    return s;
}

@fragment
fn fs(e: Salida) -> @location(0) vec4<f32> {
    // Borde oscuro fino en cada cara: edificios iguales y pegados se siguen distinguiendo.
    var a = vec2<f32>(0.0, 0.0);
    var t = vec2<f32>(1.0, 1.0);
    if (e.cara == 0u || e.cara == 5u) { a = e.local.xz; t = e.tam.xz; }
    else if (e.cara == 1u || e.cara == 2u) { a = e.local.xy; t = e.tam.xy; }
    else { a = e.local.zy; t = e.tam.zy; }
    let dist = min(min(a.x * t.x, (1.0 - a.x) * t.x), min(a.y * t.y, (1.0 - a.y) * t.y));
    var c = e.color.rgb;
    if (dist < camara.luz.w && min(t.x, t.y) > camara.luz.w * 4.0) { c = c * 0.62; }
    return vec4<f32>(c * e.color.a, e.color.a);
}
