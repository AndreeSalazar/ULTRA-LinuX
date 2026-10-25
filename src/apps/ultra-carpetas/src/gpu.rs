// SPDX-License-Identifier: GPL-2.0-only
//! Dibujo retenido del grafo en la GPU (wgpu: Vulkan en Linux).
//!
//! - La CPU arma la geometria (rectangulos de nodos, cabeceras, cables, conectores,
//!   barras y rejilla) en coordenadas del mundo, SOLO cuando cambia el contenido.
//! - Se sube a un bufer de la GPU y ahi se queda.
//! - Mover la camara o hacer zoom solo escribe 16 bytes (escala y desplazamiento);
//!   el shader `grafo.wgsl` recoloca todo. La CPU no vuelve a dibujar.
//! - Textos, iconos y botones siguen en egui por encima (sus texturas ya estan en la GPU).

use bytemuck::{Pod, Zeroable};
use carpetas_nucleo::{Camara, Cable, Direccion, NodoGrafo, Tema};
use eframe::egui_wgpu::{self, wgpu, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu::util::DeviceExt;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub struct Vertice { pub pos: [f32; 2], pub color: [f32; 4] }

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub struct Uniforme { pub escala: [f32; 2], pub desplazamiento: [f32; 2] }

/// Mundo -> coordenadas de la GPU dentro del rectangulo del lienzo (el viewport del callback).
pub fn uniforme(cam: Camara, ancho: f32, alto: f32) -> Uniforme {
    let escala = [2.0 * cam.zoom / ancho.max(1.0), -2.0 * cam.zoom / alto.max(1.0)];
    Uniforme { escala, desplazamiento: [-cam.x * escala[0] - 1.0, -cam.y * escala[1] + 1.0] }
}

// ---------- Geometria (CPU, solo cuando cambia el contenido) ----------
fn rgba(c: [u8; 3], a: f32) -> [f32; 4] { [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, a] }
fn mezcla(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8] }

struct Malla(Vec<Vertice>);

impl Malla {
    fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        self.0.extend_from_slice(&[Vertice { pos: a, color }, Vertice { pos: b, color }, Vertice { pos: c, color }]);
    }
    fn rect(&mut self, min: [f32; 2], max: [f32; 2], color: [f32; 4]) {
        self.tri(min, [max[0], min[1]], max, color);
        self.tri(min, max, [min[0], max[1]], color);
    }
    fn borde(&mut self, min: [f32; 2], max: [f32; 2], g: f32, color: [f32; 4]) {
        self.rect(min, [max[0], min[1] + g], color);
        self.rect([min[0], max[1] - g], max, color);
        self.rect(min, [min[0] + g, max[1]], color);
        self.rect([max[0] - g, min[1]], max, color);
    }
    fn linea(&mut self, a: [f32; 2], b: [f32; 2], g: f32, color: [f32; 4]) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let largo = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / largo * g / 2.0, dx / largo * g / 2.0);
        let (p1, p2, p3, p4) = ([a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny], [b[0] - nx, b[1] - ny], [a[0] - nx, a[1] - ny]);
        self.tri(p1, p2, p3, color);
        self.tri(p1, p3, p4, color);
    }
    fn circulo(&mut self, c: [f32; 2], r: f32, color: [f32; 4]) {
        const N: usize = 14;
        for i in 0..N {
            let (a0, a1) = (i as f32 / N as f32 * std::f32::consts::TAU, (i + 1) as f32 / N as f32 * std::f32::consts::TAU);
            self.tri(c, [c[0] + r * a0.cos(), c[1] + r * a0.sin()], [c[0] + r * a1.cos(), c[1] + r * a1.sin()], color);
        }
    }
}

/// Toda la geometria del grafo. Pura: se prueba sin GPU.
pub fn geometria(nodos: &[NodoGrafo], cables: &[Cable], t: &Tema, direccion: Direccion) -> Vec<Vertice> {
    let mut m = Malla(Vec::with_capacity(nodos.len() * 120 + cables.len() * 160 + 2400));
    // Rejilla alrededor de los nodos (lineas cada 40, marcadas cada 200).
    let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
    for n in nodos { min = [min[0].min(n.pos[0]), min[1].min(n.pos[1])]; max = [max[0].max(n.pos[0] + n.tam[0]), max[1].max(n.pos[1] + n.tam[1])]; }
    if nodos.is_empty() { min = [0.0; 2]; max = [0.0; 2]; }
    let margen = 2000.0;
    let (x0, y0) = (((min[0] - margen) / 200.0).floor() * 200.0, ((min[1] - margen) / 200.0).floor() * 200.0);
    let (x1, y1) = (max[0] + margen, max[1] + margen);
    let mut x = x0;
    while x <= x1 { let fuerte = ((x / 40.0).round() as i64) % 5 == 0; m.rect([x, y0], [x + 1.0, y1], rgba(t.linea, if fuerte { 0.55 } else { 0.22 })); x += 40.0; }
    let mut y = y0;
    while y <= y1 { let fuerte = ((y / 40.0).round() as i64) % 5 == 0; m.rect([x0, y], [x1, y + 1.0], rgba(t.linea, if fuerte { 0.55 } else { 0.22 })); y += 40.0; }
    // Cables: Bezier cubicas en 24 tramos (los "noodles" de Blender).
    for c in cables {
        let (a, b) = (c.desde, c.hasta);
        let (c1, c2) = match direccion {
            Direccion::Blender => { let d = ((b[0] - a[0]).abs() * 0.5).max(40.0); ([a[0] + d, a[1]], [b[0] - d, b[1]]) }
            Direccion::Houdini => { let d = ((b[1] - a[1]).abs() * 0.5).max(30.0); ([a[0], a[1] + d], [b[0], b[1] - d]) }
        };
        let punto = |t: f32| {
            let u = 1.0 - t;
            let (k0, k1, k2, k3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            [k0 * a[0] + k1 * c1[0] + k2 * c2[0] + k3 * b[0], k0 * a[1] + k1 * c1[1] + k2 * c2[1] + k3 * b[1]]
        };
        let color = rgba(c.color, 0.85);
        let mut previo = punto(0.0);
        for i in 1..=24 { let p = punto(i as f32 / 24.0); m.linea(previo, p, 2.0, color); previo = p; }
    }
    // Nodos: cuerpo, borde, cabecera del color de su tipo, conectores y barra de peso.
    for n in nodos {
        let (p, s) = (n.pos, n.tam);
        let max = [p[0] + s[0], p[1] + s[1]];
        // Un grupo del organizador se ve como una pila de tarjetas (varios archivos en uno).
        if n.grupo.is_some() {
            for k in [8.0, 4.0] {
                m.rect([p[0] + k, p[1] + k], [max[0] + k, max[1] + k], rgba(t.superficie, 0.85));
                m.borde([p[0] + k, p[1] + k], [max[0] + k, max[1] + k], 1.0, rgba(n.color, 0.55));
            }
        }
        m.rect(p, max, rgba(t.superficie, 1.0));
        let cab = 22.0;
        m.rect(p, [max[0], p[1] + cab], rgba(mezcla(n.color, if n.raiz { 0.75 } else { 0.5 }), 1.0));
        m.borde(p, max, if n.seleccionado { 2.0 } else { 1.0 }, rgba(if n.seleccionado { t.acento } else { t.linea }, 1.0));
        if n.seleccionado && t.brillo {
            m.borde([p[0] - 3.0, p[1] - 3.0], [max[0] + 3.0, max[1] + 3.0], 2.0, rgba(t.acento, 0.3));
            m.borde([p[0] - 6.0, p[1] - 6.0], [max[0] + 6.0, max[1] + 6.0], 2.0, rgba(t.acento, 0.12));
        }
        // Error atrapado: anillo rojo (avisos: amarillo). Encima de la seleccion para que se vea.
        if n.marcas.errores > 0 {
            m.borde(p, max, 2.0, rgba(t.error, 1.0));
            m.borde([p[0] - 4.0, p[1] - 4.0], [max[0] + 4.0, max[1] + 4.0], 3.0, rgba(t.error, 0.35));
            m.borde([p[0] - 8.0, p[1] - 8.0], [max[0] + 8.0, max[1] + 8.0], 2.0, rgba(t.error, 0.14));
        } else if n.marcas.avisos > 0 {
            m.borde(p, max, 2.0, rgba(t.tipos[2], 1.0));
        }
        let (entrada, salida) = match direccion {
            Direccion::Blender => ([p[0], p[1] + s[1] / 2.0], [max[0], p[1] + s[1] / 2.0]),
            Direccion::Houdini => ([p[0] + s[0] / 2.0, p[1]], [p[0] + s[0] / 2.0, max[1]]),
        };
        if !n.raiz { m.circulo(entrada, 5.0, rgba(n.color, 1.0)); m.circulo(entrada, 3.5, rgba(t.superficie, 1.0)); }
        if n.expandida == Some(true) { m.circulo(salida, 5.0, rgba(t.fondo, 1.0)); m.circulo(salida, 4.0, rgba(n.color, 1.0)); }
        let barra_min = [p[0] + 10.0, max[1] - 12.0];
        let barra_max = [max[0] - 10.0, max[1] - 8.0];
        m.rect(barra_min, barra_max, rgba(t.linea, 1.0));
        if n.peso > 0.0 {
            let lleno = barra_min[0] + (barra_max[0] - barra_min[0]) * n.peso.clamp(0.0, 1.0);
            m.rect(barra_min, [lleno, barra_max[1]], rgba(if n.peso > 0.6 { t.error } else { n.color }, 1.0));
        }
    }
    m.0
}

// ---------- GPU ----------
struct Recursos {
    pipeline: wgpu::RenderPipeline,
    uniforme: wgpu::Buffer,
    grupo: wgpu::BindGroup,
    vertices: Option<wgpu::Buffer>,
    cantidad: u32,
    clave: u64,
    subidas: u64,
}

/// Crea el pipeline una vez. Devuelve false si no hay wgpu (entonces se dibuja en CPU).
pub fn instalar(estado: Option<&egui_wgpu::RenderState>, profundidad: Option<wgpu::TextureFormat>) -> bool {
    let Some(rs) = estado else { return false };
    let d = &rs.device;
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("ultra-carpetas grafo"), source: wgpu::ShaderSource::Wgsl(include_str!("grafo.wgsl").into()) });
    let uniforme = d.create_buffer(&wgpu::BufferDescriptor { label: Some("camara"), size: std::mem::size_of::<Uniforme>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let disposicion = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("camara"), entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0, visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });
    let grupo = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("camara"), layout: &disposicion,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforme.as_entire_binding() }] });
    let capas = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("grafo"), bind_group_layouts: &[Some(&disposicion)], immediate_size: 0 });
    let atributos = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
    let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grafo"), layout: Some(&capas),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertice>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &atributos })] },
        primitive: wgpu::PrimitiveState::default(),
        // El cuadro tiene bufer de profundidad (por la ciudad): el grafo es 2D, ni lo mira ni lo escribe.
        depth_stencil: profundidad.map(|format| wgpu::DepthStencilState { format, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always),
                                                                         stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: rs.target_format, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    });
    rs.renderer.write().callback_resources.insert(Recursos { pipeline, uniforme, grupo, vertices: None, cantidad: 0, clave: u64::MAX, subidas: 0 });
    true
}

/// El callback de cada cuadro: lleva la camara y, por si hace falta, la geometria y su clave.
pub struct Lienzo { pub clave: u64, pub geometria: Arc<Vec<Vertice>>, pub uniforme: Uniforme, pub subidas: Arc<std::sync::atomic::AtomicU64> }

impl CallbackTrait for Lienzo {
    fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue, _s: &ScreenDescriptor, _e: &mut wgpu::CommandEncoder, recursos: &mut CallbackResources) -> Vec<wgpu::CommandBuffer> {
        let Some(r) = recursos.get_mut::<Recursos>() else { return Vec::new() };
        if r.clave != self.clave {
            // Solo aqui viaja la geometria a la GPU: cuando el grafo cambio de verdad.
            r.vertices = (!self.geometria.is_empty()).then(|| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grafo"), contents: bytemuck::cast_slice(&self.geometria), usage: wgpu::BufferUsages::VERTEX }));
            r.cantidad = self.geometria.len() as u32;
            r.clave = self.clave;
            r.subidas += 1;
            self.subidas.store(r.subidas, std::sync::atomic::Ordering::Relaxed);
        }
        queue.write_buffer(&r.uniforme, 0, bytemuck::bytes_of(&self.uniforme)); // 16 bytes por cuadro
        Vec::new()
    }

    fn paint(&self, _info: egui::PaintCallbackInfo, pase: &mut wgpu::RenderPass<'static>, recursos: &CallbackResources) {
        let Some(r) = recursos.get::<Recursos>() else { return };
        let Some(vb) = &r.vertices else { return };
        pase.set_pipeline(&r.pipeline);
        pase.set_bind_group(0, &r.grupo, &[]);
        pase.set_vertex_buffer(0, vb.slice(..));
        pase.draw(0..r.cantidad, 0..1);
    }
}

use eframe::egui;

#[cfg(test)]
mod pruebas {
    use super::*;
    use carpetas_nucleo::{Tipo, TEMAS};

    fn nodo(id: &str, pos: [f32; 2]) -> NodoGrafo {
        NodoGrafo { id: id.into(), nombre: id.into(), tipo: Tipo::Carpeta, pos, tam: [210.0, 74.0], color: [255, 0, 0], icono: "", detalle: String::new(), marcas: Default::default(), raiz: id.is_empty(), grupo: None, rol: None, atenuado: false,
                    peso: 0.5, expandida: Some(true), seleccionado: false, play: None }
    }

    #[test]
    fn camara_lleva_el_mundo_a_la_gpu() {
        let u = uniforme(Camara { x: 100.0, y: 50.0, zoom: 2.0 }, 800.0, 600.0);
        let a_gpu = |p: [f32; 2]| [p[0] * u.escala[0] + u.desplazamiento[0], p[1] * u.escala[1] + u.desplazamiento[1]];
        let cerca = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5;
        assert!(cerca(a_gpu([100.0, 50.0]), [-1.0, 1.0]), "la esquina de la camara es arriba a la izquierda");
        assert!(cerca(a_gpu([500.0, 350.0]), [1.0, -1.0]), "800x600 px con zoom 2 son 400x300 de mundo");
        assert!(cerca(a_gpu([300.0, 200.0]), [0.0, 0.0]), "el centro del lienzo");
    }

    #[test]
    fn un_grupo_es_una_pila_de_tarjetas() {
        let t = TEMAS[0];
        let mut g = nodo("u", [0.0, 0.0]);
        let solo = geometria(&[g.clone()], &[], &t, Direccion::Blender);
        g.grupo = Some("h · cc".into());
        let pila = geometria(&[g], &[], &t, Direccion::Blender);
        assert!(pila.len() > solo.len());
        assert!(pila.iter().any(|v| v.pos == [218.0, 82.0]), "tarjeta de atras desplazada 8 px");
    }

    #[test]
    fn un_error_pone_anillo_rojo_en_la_gpu() {
        let t = TEMAS[0];
        let limpio = geometria(&[nodo("a", [0.0, 0.0])], &[], &t, Direccion::Blender);
        let mut n = nodo("a", [0.0, 0.0]);
        n.marcas.errores = 2;
        let con_error = geometria(&[n.clone()], &[], &t, Direccion::Blender);
        let rojo = rgba(t.error, 1.0);
        assert!(!limpio.iter().any(|v| v.color == rojo && v.pos[0] < 0.0));
        assert!(con_error.iter().any(|v| v.color == rojo), "borde rojo");
        assert!(con_error.iter().any(|v| v.pos[0] <= -7.0), "halo por fuera del nodo");
        n.marcas.errores = 0;
        n.marcas.avisos = 1;
        assert!(geometria(&[n], &[], &t, Direccion::Blender).iter().any(|v| v.color == rgba(t.tipos[2], 1.0)), "aviso amarillo");
    }

    #[test]
    fn geometria_completa_y_solo_triangulos() {
        let t = TEMAS[0];
        let nodos = [nodo("", [0.0, 0.0]), nodo("a", [300.0, 0.0])];
        let cables = [Cable { desde: [210.0, 37.0], hasta: [300.0, 37.0], color: [0, 255, 0], clase: Default::default() }];
        let g = geometria(&nodos, &cables, &t, Direccion::Blender);
        assert_eq!(g.len() % 3, 0);
        assert!(g.len() > 1000 && g.len() < 40_000, "{} vertices", g.len());
        assert!(g.iter().all(|v| v.pos.iter().all(|x| x.is_finite()) && v.color.iter().all(|c| (0.0..=1.0).contains(c))));
        // Cada nodo deja su cuerpo exacto como primer rectangulo tras la rejilla y los cables.
        let cuerpo = g.iter().any(|v| v.pos == [300.0, 0.0]) && g.iter().any(|v| v.pos == [510.0, 74.0]);
        assert!(cuerpo);
        assert!(geometria(&[], &[], &t, Direccion::Houdini).len() > 0, "sin nodos queda la rejilla");
    }
}
