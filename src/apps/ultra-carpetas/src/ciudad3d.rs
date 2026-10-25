// SPDX-License-Identifier: GPL-2.0-only
//! Vista Ciudad en 3D (idea de CodeCity): distritos = carpetas, edificios = archivos.
//! El nucleo arma la ciudad (treemap); aqui se convierte en instancias para la GPU, se
//! mueve la camara, se elige con el raton (rayo contra cajas) y se ponen etiquetas.
//!
//! Carril Ciudad: las instancias se rehacen solo si cambian los datos, la medida o el
//! color; girar, acercar o mover la camara solo escribe 80 bytes.

use crate::{color, Colores};
use bytemuck::{Pod, Zeroable};
use carpetas_nucleo::ciudad::{color_calor, color_lenguaje, leyenda_lenguajes, lineas_de, Altura, ArchivoCiudad, Ciudad, Color as ColorCiudad, Edificio, TERRAZA};
use carpetas_nucleo::{legible, Accion, Comando, Estado, Rol};
use carpetas_nucleo::museo::Eje;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use eframe::egui_wgpu::{self, wgpu, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu::util::DeviceExt;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub struct Instancia { pub suelo: [f32; 4], pub alto: [f32; 2], pub color: [f32; 4] }

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub struct UniformeCiudad { pub vp: [[f32; 4]; 4], pub luz: [f32; 4] }

// ---------- Matematicas (sin dependencias) ----------
type V3 = [f32; 3];
fn resta(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn punto(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cruz(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn unitario(a: V3) -> V3 { let l = punto(a, a).sqrt().max(1e-9); [a[0] / l, a[1] / l, a[2] / l] }
fn mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0.0; 4]; 4];
    for c in 0..4 { for f in 0..4 { r[c][f] = (0..4).map(|k| a[k][f] * b[c][k]).sum(); } }
    r
}

const FOV: f32 = 0.75; // ~43 grados

/// Camara en orbita alrededor de un punto del suelo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camara3 { pub objetivo: V3, pub giro: f32, pub inclinacion: f32, pub distancia: f32 }

impl Default for Camara3 {
    fn default() -> Self { Camara3 { objetivo: [500.0, 0.0, 500.0], giro: 0.7, inclinacion: 0.78, distancia: 1450.0 } }
}

impl Camara3 {
    pub fn ojo(&self) -> V3 {
        let (cp, sp) = (self.inclinacion.cos(), self.inclinacion.sin());
        [self.objetivo[0] + self.distancia * cp * self.giro.sin(), self.objetivo[1] + self.distancia * sp, self.objetivo[2] + self.distancia * cp * self.giro.cos()]
    }
    fn ejes(&self) -> (V3, V3, V3) {
        let f = unitario(resta(self.objetivo, self.ojo()));
        let s = unitario(cruz(f, [0.0, 1.0, 0.0]));
        (f, s, cruz(s, f))
    }
    /// Vista y proyeccion (columnas, profundidad 0..1 como wgpu).
    pub fn vp(&self, aspecto: f32) -> [[f32; 4]; 4] {
        let ojo = self.ojo();
        let (f, s, u) = self.ejes();
        let vista = [[s[0], u[0], -f[0], 0.0], [s[1], u[1], -f[1], 0.0], [s[2], u[2], -f[2], 0.0], [-punto(s, ojo), -punto(u, ojo), punto(f, ojo), 1.0]];
        let (cerca, lejos) = (self.distancia * 0.02, self.distancia * 6.0 + 2000.0);
        let g = 1.0 / (FOV / 2.0).tan();
        let proy = [[g / aspecto.max(0.01), 0.0, 0.0, 0.0], [0.0, g, 0.0, 0.0], [0.0, 0.0, lejos / (cerca - lejos), -1.0], [0.0, 0.0, cerca * lejos / (cerca - lejos), 0.0]];
        mul(&proy, &vista)
    }
    /// Rayo (origen, direccion) por un punto del lienzo en -1..1.
    pub fn rayo(&self, ndc: [f32; 2], aspecto: f32) -> (V3, V3) {
        let (f, s, u) = self.ejes();
        let t = (FOV / 2.0).tan();
        let d = [f[0] + s[0] * ndc[0] * t * aspecto + u[0] * ndc[1] * t, f[1] + s[1] * ndc[0] * t * aspecto + u[1] * ndc[1] * t,
                 f[2] + s[2] * ndc[0] * t * aspecto + u[2] * ndc[1] * t];
        (self.ojo(), unitario(d))
    }
}

/// Mundo -> pantalla (None si queda detras de la camara).
pub fn proyectar(vp: &[[f32; 4]; 4], p: V3, lienzo: Rect) -> Option<Pos2> {
    let c: Vec<f32> = (0..4).map(|f| vp[0][f] * p[0] + vp[1][f] * p[1] + vp[2][f] * p[2] + vp[3][f]).collect();
    if c[3] <= 1e-3 { return None; }
    let (x, y) = (c[0] / c[3], c[1] / c[3]);
    Some(Pos2::new(lienzo.min.x + (x + 1.0) * 0.5 * lienzo.width(), lienzo.min.y + (1.0 - y) * 0.5 * lienzo.height()))
}

/// Distancia del rayo a una caja (x0,y0,z0)-(x1,y1,z1), si la toca.
pub fn rayo_caja(o: V3, d: V3, min: V3, max: V3) -> Option<f32> {
    let (mut t0, mut t1) = (0.0f32, f32::MAX);
    for k in 0..3 {
        if d[k].abs() < 1e-9 { if o[k] < min[k] || o[k] > max[k] { return None; } continue; }
        let (a, b) = ((min[k] - o[k]) / d[k], (max[k] - o[k]) / d[k]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
        if t0 > t1 { return None; }
    }
    Some(t0)
}

fn rgba(c: [u8; 3], a: f32) -> [f32; 4] { [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, a] }

/// Color de un edificio segun el modo.
fn color_edificio(e: &Edificio, modo: ColorCiudad, estado: &Estado, c: &Colores) -> [u8; 3] {
    let nombre = e.ruta.rsplit('/').next().unwrap_or(&e.ruta);
    match modo {
        ColorCiudad::Lenguaje => color_lenguaje(nombre, e.bloque),
        ColorCiudad::Calor => {
            let h = &estado.historia.datos;
            let v = h.de(&e.ruta).veces;
            // Raiz: que los pocos cambios no queden todos en el mismo azul.
            color_calor((v > 0).then(|| carpetas_nucleo::ciudad::raiz2(v as f32 / h.max_veces().max(1) as f32)))
        }
        ColorCiudad::Edad => {
            let h = &estado.historia.datos;
            let u = h.de(&e.ruta).ultimo;
            if u == 0 { return [44, 48, 60]; }
            // Reciente = claro y del acento; viejo = apagado. Escala: lo leido de la historia.
            let t = (u.saturating_sub(h.desde)) as f32 / (h.hasta.saturating_sub(h.desde)).max(1) as f32;
            let a = c.acento;
            let mezcla = |x: u8, viejo: f32| (viejo + (x as f32 - viejo) * t) as u8;
            [mezcla(a.r(), 70.0), mezcla(a.g(), 76.0), mezcla(a.b(), 98.0)]
        }
        ColorCiudad::Salud => {
            // Errores atrapados y sospechas del museo (exacto = rojo, revisar = amarillo).
            let (err, av) = estado.errores.conteo(&e.ruta);
            let (exactas, revisar) = estado.analisis.conteo(&e.ruta);
            if err + exactas > 0 { [235, 70, 80] } else if av + revisar > 0 { [230, 175, 60] } else { [70, 185, 110] }
        }
        ColorCiudad::Triangulo => {
            // El eje que mas pesa en el edificio (a igualdad: Seguridad, luego Rendimiento).
            let n = estado.analisis.conteo_ejes(&e.ruta);
            let ejes = Eje::TODOS.iter().filter(|x| estado.filtro_eje.is_none_or(|f| f == **x));
            match ejes.filter(|x| n[x.indice()] > 0).max_by_key(|x| (n[x.indice()], core::cmp::Reverse(x.indice()))) {
                Some(x) => { let k = crate::color_eje(c, *x); [k.r(), k.g(), k.b()] }
                None => [52, 56, 68],
            }
        }
        ColorCiudad::Busqueda => {
            if estado.proyecto.conteo(&e.ruta) > 0 { let a = c.acento; [a.r(), a.g(), a.b()] } else { [58, 62, 74] }
        }
    }
}

/// Lo que se recuerda entre cuadros: la ciudad armada, sus instancias y la camara.
#[derive(Default)]
pub struct Vista3 {
    clave: Option<(u64, Altura, ColorCiudad, carpetas_nucleo::ciudad::Huella, usize, u64, usize, [u8; 3], u64, (u64, u64, Option<String>), u64)>,
    pub ciudad: Arc<Ciudad>,
    instancias: Arc<Vec<Instancia>>,
    version_gpu: u64,
    pub camara: Camara3,
    revelado_visto: u64,
    raiz_vista: String,
    pub micros: u64,
    /// Shift+arrastrar: donde empezo la banda de la seleccion grupal.
    banda: Option<Pos2>,
}

/// Cajas resaltadas como mucho: lo elegido, lo de debajo del raton y el grupo con sus puentes.
const MAX_RESALTES: usize = 128;

impl Vista3 {
    /// Rehace la ciudad y sus instancias si cambio algo que se ve (no la camara).
    pub fn preparar(&mut self, e: &Estado, c: &Colores) {
        let a = c.acento;
        let clave = (e.ciudad.version, e.ajustes.altura_ciudad, e.ajustes.color_ciudad, e.ajustes.huella_ciudad, e.errores.total, e.proyecto.generacion, e.proyecto.total, [a.r(), a.g(), a.b()], e.historia.version,
                     e.conectado.clave.clone(), e.analisis.version);
        if self.clave.as_ref() == Some(&clave) { return; }
        let t = std::time::Instant::now();
        let medida = |r: &str| match e.ajustes.altura_ciudad {
            Altura::Errores => { let (x, y) = e.errores.conteo(r); x * 3 + y }
            Altura::Coincidencias => e.proyecto.conteo(r),
            Altura::Cambios => e.historia.datos.de(r).veces,
            _ => 0,
        };
        let ciudad = carpetas_nucleo::ciudad::construir(&e.ciudad.raiz, &e.ciudad.archivos, e.ajustes.huella_ciudad, e.ajustes.altura_ciudad, &medida);
        let mut v = Vec::with_capacity(ciudad.distritos.len() + ciudad.edificios.len());
        for d in &ciudad.distritos {
            // Terrazas: cada nivel algo mas claro; el suelo de la raiz casi negro.
            let k = (0.10 + d.nivel as f32 * 0.05).min(0.35);
            let base = [c.superficie.r(), c.superficie.g(), c.superficie.b()];
            let col = [(base[0] as f32 * (1.0 + k)) as u8, (base[1] as f32 * (1.0 + k)) as u8, ((base[2] as f32 * (1.0 + k)) as u8).saturating_add(6)];
            v.push(Instancia { suelo: d.rect, alto: [d.base - TERRAZA, TERRAZA], color: rgba(col, 1.0) });
        }
        for b in &ciudad.edificios {
            let col = match (e.aislar.is_some(), e.conectado.conexion.roles.get(&b.ruta)) {
                // SUPER GRAFO aislado: blanco la causa, rojo neon lo que falla, rojo oscuro lo que arrastra.
                (true, Some(Rol::Causa)) => [255, 255, 255],
                (true, Some(Rol::Falla)) => [255, 42, 79],
                (true, Some(Rol::Arrastra)) => [150, 32, 52],
                (true, Some(Rol::Base)) => [190, 196, 212],
                (true, None) => [34, 36, 44],
                _ => color_edificio(b, e.ajustes.color_ciudad, e, c),
            };
            v.push(Instancia { suelo: b.rect, alto: [b.base, b.altura], color: rgba(col, 1.0) });
        }
        if self.raiz_vista != ciudad.raiz { self.camara = Camara3::default(); self.raiz_vista = ciudad.raiz.clone(); }
        self.ciudad = Arc::new(ciudad);
        self.instancias = Arc::new(v);
        self.version_gpu += 1;
        self.clave = Some(clave);
        self.micros = t.elapsed().as_micros() as u64;
    }

    /// Edificio (o distrito) bajo el raton: el mas cercano que toca el rayo.
    fn elegir(&self, o: V3, d: V3) -> (Option<usize>, Option<usize>) {
        let mut mejor = (f32::MAX, None);
        for (i, b) in self.ciudad.edificios.iter().enumerate() {
            let min = [b.rect[0], b.base, b.rect[1]];
            let max = [b.rect[0] + b.rect[2], b.base + b.altura, b.rect[1] + b.rect[3]];
            if let Some(t) = rayo_caja(o, d, min, max) { if t < mejor.0 { mejor = (t, Some(i)); } }
        }
        let mut distrito = (f32::MAX, None, 0u32);
        for (i, x) in self.ciudad.distritos.iter().enumerate() {
            let (min, max) = ([x.rect[0], x.base - TERRAZA, x.rect[1]], [x.rect[0] + x.rect[2], x.base, x.rect[1] + x.rect[3]]);
            if let Some(t) = rayo_caja(o, d, min, max) {
                // El mas hondo (el mas alto) que toca: el distrito mas concreto.
                if x.nivel > distrito.2 || distrito.1.is_none() || (x.nivel == distrito.2 && t < distrito.0) { distrito = (t, Some(i), x.nivel); }
            }
        }
        (mejor.1, distrito.1)
    }
}

// ---------- GPU ----------
struct Recursos { pipeline: wgpu::RenderPipeline, uniforme: wgpu::Buffer, grupo: wgpu::BindGroup, instancias: Option<wgpu::Buffer>, cantidad: u32,
                  version: u64, resaltes: wgpu::Buffer, n_resaltes: u32 }

pub fn instalar(rs: &egui_wgpu::RenderState, profundidad: Option<wgpu::TextureFormat>) -> bool {
    let Some(formato) = profundidad else { return false };
    let d = &rs.device;
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("ultra-carpetas ciudad"), source: wgpu::ShaderSource::Wgsl(include_str!("ciudad.wgsl").into()) });
    let uniforme = d.create_buffer(&wgpu::BufferDescriptor { label: Some("ciudad camara"), size: std::mem::size_of::<UniformeCiudad>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let disposicion = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("ciudad"), entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });
    let grupo = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("ciudad"), layout: &disposicion,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforme.as_entire_binding() }] });
    let capas = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("ciudad"), bind_group_layouts: &[Some(&disposicion)], immediate_size: 0 });
    let atributos = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2, 2 => Float32x4];
    let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ciudad"), layout: Some(&capas),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instancia>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &atributos })] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState { format: formato, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less),
                                                      stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: rs.target_format, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    });
    let resaltes = d.create_buffer(&wgpu::BufferDescriptor { label: Some("ciudad resaltes"), size: (std::mem::size_of::<Instancia>() * MAX_RESALTES) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    rs.renderer.write().callback_resources.insert(Recursos { pipeline, uniforme, grupo, instancias: None, cantidad: 0, version: u64::MAX, resaltes, n_resaltes: 0 });
    true
}

struct Pintor { version: u64, instancias: Arc<Vec<Instancia>>, uniforme: UniformeCiudad, resaltes: Vec<Instancia> }

impl CallbackTrait for Pintor {
    fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue, _s: &ScreenDescriptor, _e: &mut wgpu::CommandEncoder, recursos: &mut CallbackResources) -> Vec<wgpu::CommandBuffer> {
        let Some(r) = recursos.get_mut::<Recursos>() else { return Vec::new() };
        if r.version != self.version {
            r.instancias = (!self.instancias.is_empty()).then(|| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ciudad"), contents: bytemuck::cast_slice(&self.instancias), usage: wgpu::BufferUsages::VERTEX }));
            r.cantidad = self.instancias.len() as u32;
            r.version = self.version;
        }
        queue.write_buffer(&r.uniforme, 0, bytemuck::bytes_of(&self.uniforme));
        r.n_resaltes = self.resaltes.len().min(MAX_RESALTES) as u32;
        if r.n_resaltes > 0 { queue.write_buffer(&r.resaltes, 0, bytemuck::cast_slice(&self.resaltes[..r.n_resaltes as usize])); }
        Vec::new()
    }

    fn paint(&self, _info: egui::PaintCallbackInfo, pase: &mut wgpu::RenderPass<'static>, recursos: &CallbackResources) {
        let Some(r) = recursos.get::<Recursos>() else { return };
        let Some(b) = &r.instancias else { return };
        pase.set_pipeline(&r.pipeline);
        pase.set_bind_group(0, &r.grupo, &[]);
        pase.set_vertex_buffer(0, b.slice(..));
        pase.draw(0..36, 0..r.cantidad);
        if r.n_resaltes > 0 {
            pase.set_vertex_buffer(0, r.resaltes.slice(..));
            pase.draw(0..36, 0..r.n_resaltes);
        }
    }
}

fn por(x: f32) -> String { if x >= 10.0 { format!("{x:.0} %") } else if x >= 0.1 { format!("{x:.1} %") } else { "<0.1 %".into() } }

/// Pinta la ciudad, su leyenda y etiquetas; traduce el raton a acciones.
pub fn pintar(ui: &mut egui::Ui, v3: &mut Vista3, e: &Estado, k: &crate::combinar_ui::Cache, c: &Colores, gpu: bool, acciones: &mut Vec<Accion>) {
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let pintor = ui.painter_at(rect);
    pintor.rect_filled(rect, 0.0, c.fondo);
    if !gpu {
        pintor.text(rect.center(), Align2::CENTER_CENTER, "La vista Ciudad necesita wgpu con bufer de profundidad (aqui no hay).", FontId::proportional(14.0), c.tenue);
        return;
    }
    if e.ciudad.archivos.is_empty() {
        let t = if !e.indice.listo { "indexando el proyecto…" } else if e.ciudad.cargando { "levantando la ciudad…" } else { "esta carpeta no tiene archivos" };
        pintor.text(rect.center(), Align2::CENTER_CENTER, t, FontId::proportional(15.0), c.tenue);
        if e.ciudad.archivos.is_empty() { return; }
    }
    v3.preparar(e, c);
    let aspecto = rect.width() / rect.height().max(1.0);
    // Camara: arrastrar = girar; central o derecho = mover; rueda = acercar.
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.drag_started_by(egui::PointerButton::Primary) && shift { v3.banda = ui.input(|i| i.pointer.press_origin()); }
    let cam = &mut v3.camara;
    if resp.dragged_by(egui::PointerButton::Primary) && v3.banda.is_none() {
        let d = resp.drag_delta();
        cam.giro -= d.x * 0.008;
        cam.inclinacion = (cam.inclinacion + d.y * 0.006).clamp(0.12, 1.5);
    }
    if resp.dragged_by(egui::PointerButton::Middle) || resp.dragged_by(egui::PointerButton::Secondary) {
        let d = resp.drag_delta();
        let k = cam.distancia * 0.0016;
        let (s, cg) = (cam.giro.sin(), cam.giro.cos());
        cam.objetivo[0] -= (d.x * cg - d.y * s) * k;
        cam.objetivo[2] -= (-d.x * s - d.y * cg) * k;
    }
    if resp.hovered() {
        let rueda = ui.input(|i| i.smooth_scroll_delta().y);
        if rueda != 0.0 { cam.distancia = (cam.distancia * 1.0015f32.powf(-rueda)).clamp(25.0, 5000.0); }
    }
    // Ctrl+P (u otra vista) revelo algo: la camara va a su edificio (o a su bloque).
    if e.revelado.1 != v3.revelado_visto {
        v3.revelado_visto = e.revelado.1;
        let id = &e.revelado.0;
        if let Some(b) = v3.ciudad.edificios.iter().find(|b| &b.ruta == id || id.starts_with(&(b.ruta.clone() + "/"))) {
            v3.camara.objetivo = [b.rect[0] + b.rect[2] / 2.0, b.base, b.rect[1] + b.rect[3] / 2.0];
            v3.camara.distancia = v3.camara.distancia.min(420.0);
        }
    }
    let vp = v3.camara.vp(aspecto);
    // Raton: que hay debajo.
    let (bajo, distrito) = match resp.hover_pos() {
        Some(p) => {
            let ndc = [(p.x - rect.min.x) / rect.width() * 2.0 - 1.0, 1.0 - (p.y - rect.min.y) / rect.height() * 2.0];
            let (o, d) = v3.camara.rayo(ndc, aspecto);
            v3.elegir(o, d)
        }
        None => (None, None),
    };
    // Resaltes: lo seleccionado y lo que esta bajo el raton (cajas un poco mas grandes y claras).
    let mut resaltes = Vec::new();
    let mut resaltar = |b: &Edificio, col: [u8; 3]| {
        let g = 0.4;
        resaltes.push(Instancia { suelo: [b.rect[0] - g, b.rect[1] - g, b.rect[2] + 2.0 * g, b.rect[3] + 2.0 * g], alto: [b.base, b.altura + g], color: rgba(col, 1.0) });
    };
    let sel = e.seleccion.iter().next();
    if let Some(b) = sel.and_then(|s| v3.ciudad.edificios.iter().find(|b| &b.ruta == s)) { resaltar(b, [255, 255, 255]); }
    // Seleccion grupal: los del grupo en blanco, lo que comparten en el acento y quien los
    // junta en verde. Los cables se dibujan despues, sobre la ciudad.
    let grupo = !e.grupo.is_empty() && !k.del_grafo;
    let anclas = if grupo { anclas(&v3.ciudad, k) } else { Anclas::default() };
    if grupo {
        let (a, v) = (c.acento, c.verde);
        for (i, col) in anclas.miembros.iter().map(|x| (x, [255, 255, 255]))
            .chain(anclas.comparten.iter().map(|x| (x, [a.r(), a.g(), a.b()])))
            .chain(anclas.juntan.iter().map(|x| (x, [v.r(), v.g(), v.b()]))) {
            if let Some(Ancla::Edificio(j)) = i { resaltar(&v3.ciudad.edificios[*j], col); }
        }
    }
    if let Some(i) = bajo { let a = c.acento; resaltar(&v3.ciudad.edificios[i], [a.r(), a.g(), a.b()]); }
    let luz = [0.45, 0.85, 0.30, (v3.camara.distancia * 0.0012).clamp(0.08, 1.2)];
    pintor.add(egui_wgpu::Callback::new_paint_callback(rect, Pintor { version: v3.version_gpu, instancias: v3.instancias.clone(), uniforme: UniformeCiudad { vp, luz }, resaltes }));

    // Etiquetas de distritos: los de arriba siempre que quepan; mas hondos si se ven grandes.
    let mut ocupadas: Vec<Rect> = Vec::new();
    let mut orden: Vec<&carpetas_nucleo::ciudad::Distrito> = v3.ciudad.distritos.iter().filter(|d| d.nivel >= 1 && d.nivel <= 4).collect();
    orden.sort_by(|a, b| a.nivel.cmp(&b.nivel).then(b.tam.cmp(&a.tam)));
    for d in orden.into_iter().take(400) {
        let centro = [d.rect[0] + d.rect[2] / 2.0, d.base + 2.0, d.rect[1] + d.rect[3] / 2.0];
        let (Some(p), Some(q)) = (proyectar(&vp, centro, rect), proyectar(&vp, [d.rect[0] + d.rect[2], d.base, d.rect[1]], rect)) else { continue };
        let ancho_pantalla = (q - p).length() * 2.0;
        if d.nivel > 1 && ancho_pantalla < 150.0 { continue; }
        if !rect.contains(p) { continue; }
        let texto = format!("{} ({}) {}", d.nombre, d.archivos, por(d.porcentaje));
        let g = pintor.layout_no_wrap(texto, FontId::proportional(if d.nivel == 1 { 12.5 } else { 11.0 }), Color32::WHITE);
        let caja = Rect::from_center_size(p, g.size() + Vec2::new(14.0, 6.0));
        if ocupadas.iter().any(|o| o.intersects(caja)) { continue; }
        ocupadas.push(caja.expand(2.0));
        pintor.rect(caja, 5, Color32::from_black_alpha(190), Stroke::new(1.0, Color32::from_white_alpha(40)), StrokeKind::Inside);
        pintor.galley(caja.center() - g.size() / 2.0, g, Color32::WHITE);
    }

    if grupo { cables(&pintor, &vp, rect, &v3.ciudad, k, &anclas, c); }
    leyenda(ui, rect, v3, e, c, acciones);

    // Ficha del edificio (o del distrito) bajo el raton.
    if let (Some(p), Some(i)) = (resp.hover_pos(), bajo) {
        let b = &v3.ciudad.edificios[i];
        let distrito = b.ruta.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let (l, estimadas) = lineas_de(&ArchivoCiudad { ruta: b.ruta.clone(), tam: b.tam, lineas: b.lineas, archivos: b.archivos, bloque: b.bloque });
        let lineas = if estimadas { format!("~{l} lineas (estimadas por tamano, aun sin contar)") } else if l == 0 && b.lineas.is_none() { "sin lineas (no es texto)".to_string() } else { format!("{l} lineas") };
        let mut t = if b.bloque { format!("{}/ (bloque de {} archivos)\n", b.ruta, b.archivos) } else { format!("{}\n", b.ruta) };
        t += &format!("{} · {}\n{} de {} · {} de toda la ciudad", legible(b.tam), lineas, por(b.del_distrito), if distrito.is_empty() { "la raiz" } else { distrito }, por(b.del_total));
        let (err, av) = e.errores.conteo(&b.ruta);
        if err + av > 0 { t += &format!("\n{err} errores · {av} avisos"); }
        let n = e.proyecto.conteo(&b.ruta);
        if n > 0 { t += &format!("\n{n} resultados de la busqueda"); }
        if e.historia.lista && e.historia.sin.is_none() {
            let h = e.historia.datos.de(&b.ruta);
            t += &if h.veces == 0 { "\nsin cambios en la historia leida".to_string() }
                  else { format!("\n{} en git · ultimo {} · {}", carpetas_nucleo::diagnosticos::contar(h.veces, "cambio", "cambios"),
                                 carpetas_nucleo::historia::hace(h.ultimo, e.historia.ahora), carpetas_nucleo::diagnosticos::contar(h.autores as u32, "autor", "autores")) };
        }
        if let Some(rol) = e.conectado.conexion.roles.get(&b.ruta) { t += "\n"; t += crate::pintar_grafo::texto_rol(*rol); }
        if let Some((_, s)) = e.analisis.primera(&b.ruta) {
            let titulo = carpetas_nucleo::museo::ficha(s.ficha).map(|f| f.titulo).unwrap_or("");
            t += &format!("\nMuseo {} · {titulo} (linea {}): {}", s.ficha, s.linea, s.motivo);
        }
        t += if b.bloque { "\nclic: enfocar este bloque" } else { "\nclic: mostrar en el explorador · clic derecho: editor" };
        ficha(&pintor, rect, p, &t, c);
    } else if let (Some(p), Some(i)) = (resp.hover_pos(), distrito) {
        let d = &v3.ciudad.distritos[i];
        let t = format!("{}/\n{} archivos · {} · {} del total\ndoble clic: enfocar", if d.ruta.is_empty() { "(raiz)" } else { &d.ruta }, d.archivos, legible(d.tam), por(d.porcentaje));
        ficha(&pintor, rect, p, &t, c);
    }
    // Shift+arrastrar: banda; al soltar, al grupo los edificios cuya azotea cae dentro.
    if let Some(inicio) = v3.banda {
        let fin = ui.input(|i| i.pointer.interact_pos()).unwrap_or(inicio);
        let caja = Rect::from_two_pos(inicio, fin);
        pintor.rect(caja, 2, c.acento.gamma_multiply(0.10), Stroke::new(1.0, c.acento), StrokeKind::Inside);
        if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
            v3.banda = None;
            let dentro: Vec<String> = v3.ciudad.edificios.iter()
                .filter(|b| proyectar(&vp, [b.rect[0] + b.rect[2] / 2.0, b.base + b.altura, b.rect[1] + b.rect[3] / 2.0], rect).is_some_and(|q| caja.contains(q)))
                .take(carpetas_nucleo::combinar::MAX_GRUPO).map(|b| b.ruta.clone()).collect();
            if !dentro.is_empty() { acciones.push(Accion::Grupo(carpetas_nucleo::AccionGrupo::Sumar(dentro))); }
        }
    }
    // Clics. Ctrl+clic: meter o sacar del grupo (archivos y bloques).
    if resp.clicked() && ui.input(|i| i.modifiers.command) {
        if let Some(b) = bajo.map(|i| &v3.ciudad.edificios[i]) { acciones.push(Accion::Grupo(carpetas_nucleo::AccionGrupo::Alternar(b.ruta.clone()))); }
    } else if resp.clicked() {
        match bajo.map(|i| &v3.ciudad.edificios[i]) {
            Some(b) if b.bloque => acciones.push(Accion::Enfocar(b.ruta.clone())),
            Some(b) => acciones.push(Accion::Revelar { id: b.ruta.clone(), editor: false }),
            None => {}
        }
    }
    if resp.secondary_clicked() {
        if let Some(b) = bajo.map(|i| &v3.ciudad.edificios[i]).filter(|b| !b.bloque) { acciones.push(Accion::Revelar { id: b.ruta.clone(), editor: true }); }
    }
    if resp.double_clicked() {
        if let Some(d) = distrito.map(|i| &v3.ciudad.distritos[i]) { if !d.ruta.is_empty() && bajo.is_none() { acciones.push(Accion::Enfocar(d.ruta.clone())); } }
    }
    pintor.text(rect.left_bottom() + Vec2::new(10.0, -8.0), Align2::LEFT_BOTTOM,
                "arrastrar: girar · central/derecho: mover · rueda: acercar · clic: mostrar · Ctrl+clic o Shift+arrastrar: grupo (panel Combinar) · clic derecho: editor · doble clic en un distrito: enfocar · U: subir",
                FontId::proportional(11.0), c.tenue);
}

fn ficha(pintor: &egui::Painter, lienzo: Rect, p: Pos2, t: &str, c: &Colores) {
    let g = pintor.layout(t.to_string(), FontId::proportional(12.0), c.texto, 380.0);
    let mut caja = Rect::from_min_size(p + Vec2::new(16.0, 16.0), g.size() + Vec2::new(16.0, 12.0));
    if caja.max.x > lienzo.max.x { caja = caja.translate(Vec2::new(-(caja.width() + 32.0), 0.0)); }
    if caja.max.y > lienzo.max.y { caja = caja.translate(Vec2::new(0.0, -(caja.height() + 32.0))); }
    pintor.rect(caja, 5, c.panel, Stroke::new(1.0, c.acento), StrokeKind::Inside);
    pintor.galley(caja.min + Vec2::new(8.0, 6.0), g, c.texto);
}

/// Leyenda como la de CodeCity: que significa cada cosa, con los cambios a mano.
fn leyenda(ui: &mut egui::Ui, lienzo: Rect, v3: &mut Vista3, e: &Estado, c: &Colores, acciones: &mut Vec<Accion>) {
    let ciudad = v3.ciudad.clone();
    let pos = lienzo.min + Vec2::new(12.0, 12.0);
    egui::Area::new(egui::Id::unique("ultra-carpetas-leyenda-ciudad")).fixed_pos(pos).order(egui::Order::Middle).show(ui.ctx(), |ui| {
        egui::Frame::new().fill(c.panel.gamma_multiply(0.94)).stroke(Stroke::new(1.0, c.linea)).corner_radius(8).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(300.0);
            let titulo = if ciudad.raiz.is_empty() { e.nombre_raiz.clone() } else { ciudad.raiz.clone() };
            ui.label(egui::RichText::new(format!("CIUDAD // {titulo}")).color(c.acento).strong().size(15.0));
            ui.label(egui::RichText::new(format!("{} archivos · {} distritos · {} lineas · {}", crate::paneles::miles(ciudad.archivos),
                                                 crate::paneles::miles(ciudad.distritos.len().saturating_sub(1) as u64), crate::paneles::miles(ciudad.lineas), legible(ciudad.tam)))
                     .color(c.tenue).size(11.0));
            ui.label(egui::RichText::new("Edificios = archivos. Distritos = carpetas.").color(c.texto).size(12.0));
            if ciudad.con_bloques { ui.label(egui::RichText::new("Bloques grises = carpetas hondas juntadas (clic: entrar).").color(c.tenue).size(11.0)); }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("HUELLA:").color(c.tenue).size(11.0));
                if ui.small_button(e.ajustes.huella_ciudad.nombre()).on_hover_text("Cambiar: bytes (todo) o lineas (solo texto, como CodeCity)").clicked() { acciones.push(Accion::Comando(Comando::HuellaCiudad)); }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ALTURA:").color(c.tenue).size(11.0));
                if ui.small_button(e.ajustes.altura_ciudad.nombre()).on_hover_text("Cambiar: lineas, errores, busqueda o plana").clicked() { acciones.push(Accion::Comando(Comando::AlturaCiudad)); }
            });
            if e.ajustes.altura_ciudad == Altura::Lineas {
                let t = if e.indice.lineas_listas { format!("{} lineas contadas (texto de hasta 2 MiB)", crate::paneles::miles(e.indice.lineas)) } else { "contando lineas… (mientras, se estiman por tamano)".to_string() };
                ui.label(egui::RichText::new(t).color(c.tenue).size(10.5));
            }
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("COLOR:").color(c.tenue).size(11.0));
                if ui.small_button(e.ajustes.color_ciudad.nombre()).on_hover_text("Cambiar: lenguaje, salud o busqueda").clicked() { acciones.push(Accion::Comando(Comando::ColorCiudad)); }
            });
            let muestra = |ui: &mut egui::Ui, col: [u8; 3], t: String| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().rect_filled(r, 2, color(col));
                    ui.label(egui::RichText::new(t).size(11.5).color(c.texto));
                });
            };
            match e.ajustes.color_ciudad {
                ColorCiudad::Lenguaje => {
                    let total = ciudad.archivos.max(1) as f32;
                    for (n, col, k) in leyenda_lenguajes(&ciudad).into_iter().take(8) { muestra(ui, col, format!("{n}  {k} ({})", por(k as f32 / total * 100.0))); }
                }
                ColorCiudad::Calor | ColorCiudad::Edad => {
                    let h = &e.historia;
                    if !h.lista { ui.label(egui::RichText::new("leyendo la historia de git…").color(c.tenue).size(10.5)); }
                    else if let Some(m) = &h.sin { ui.label(egui::RichText::new(format!("Sin historia: {m}")).color(c.aviso).size(11.0)); }
                    else {
                        let d = &h.datos;
                        ui.label(egui::RichText::new(format!("{} commits · {} · de {} a {}{}", d.commits, carpetas_nucleo::diagnosticos::contar(d.autores, "autor", "autores"),
                            carpetas_nucleo::historia::hace(d.desde, h.ahora), carpetas_nucleo::historia::hace(d.hasta, h.ahora),
                            if d.truncada { " (solo los ultimos)" } else { "" })).color(c.tenue).size(10.5));
                        // Barra de la escala.
                        let (barra, _) = ui.allocate_exact_size(Vec2::new(280.0, 10.0), Sense::hover());
                        for k in 0..56 {
                            let t = k as f32 / 55.0;
                            let col = if e.ajustes.color_ciudad == ColorCiudad::Calor { color_calor(Some(t)) } else {
                                let a = c.acento; [(70.0 + (a.r() as f32 - 70.0) * t) as u8, (76.0 + (a.g() as f32 - 76.0) * t) as u8, (98.0 + (a.b() as f32 - 98.0) * t) as u8] };
                            ui.painter().rect_filled(Rect::from_min_size(barra.min + Vec2::new(k as f32 * 5.0, 0.0), Vec2::new(5.0, 10.0)), 0, color(col));
                        }
                        let (izq, der) = if e.ajustes.color_ciudad == ColorCiudad::Calor { ("casi no cambia", "cambia mucho") } else { ("hace mas", "reciente") };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(izq).color(c.tenue).size(10.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(egui::RichText::new(der).color(c.tenue).size(10.5)));
                        });
                        if e.ajustes.color_ciudad == ColorCiudad::Calor {
                            ui.label(egui::RichText::new("PUNTOS CALIENTES (cambios x tamano)").color(c.acento).size(11.0));
                            let lineas = ciudad.edificios.iter().filter(|b| !b.bloque).map(|b| {
                                let (l, _) = lineas_de(&ArchivoCiudad { ruta: b.ruta.clone(), tam: b.tam, lineas: b.lineas, archivos: b.archivos, bloque: false });
                                (b.ruta.as_str(), l)
                            });
                            for (ruta, veces, l, _) in carpetas_nucleo::historia::puntos_calientes(d, lineas, 8) {
                                let nombre = ruta.rsplit('/').next().unwrap_or(&ruta).to_string();
                                let r = ui.add(egui::Button::new(egui::RichText::new(format!("{nombre}  {veces} cambios · {l} lineas")).size(11.0)).frame(false))
                                    .on_hover_text(format!("{ruta}\nclic: mostrar y llevar la camara"));
                                if r.clicked() { acciones.push(Accion::Revelar { id: ruta.clone(), editor: false }); }
                            }
                        }
                    }
                }
                ColorCiudad::Salud => {
                    muestra(ui, [235, 70, 80], "con errores (ultima ejecucion)".into());
                    muestra(ui, [230, 175, 60], "con avisos".into());
                    muestra(ui, [70, 185, 110], "sin errores conocidos".into());
                    if e.errores.total == 0 { ui.label(egui::RichText::new("Aun no se ejecuto nada (F5): todo en verde.").color(c.tenue).size(10.5)); }
                }
                ColorCiudad::Triangulo => {
                    for x in Eje::TODOS {
                        let k = crate::color_eje(c, x);
                        let n = e.analisis.sospechas.values().flatten().filter(|s| carpetas_nucleo::museo::eje(s.ficha) == x).count();
                        muestra(ui, [k.r(), k.g(), k.b()], format!("{} · {n}", x.nombre()));
                    }
                    muestra(ui, [52, 56, 68], "nada del museo".into());
                    ui.label(egui::RichText::new("Seguridad es una regla; Rendimiento y Elegancia, guias. Solo Seguridad cuenta como fallo.").color(c.tenue).size(10.5));
                }
                ColorCiudad::Busqueda => {
                    let a = c.acento;
                    muestra(ui, [a.r(), a.g(), a.b()], format!("con resultados de «{}»", e.proyecto.patron));
                    muestra(ui, [58, 62, 74], "sin resultados".into());
                    if e.proyecto.patron.is_empty() { ui.label(egui::RichText::new("Busca con Ctrl+Shift+F y los edificios se encienden.").color(c.tenue).size(10.5)); }
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.small_button("Encuadrar").clicked() { v3.camara = Camara3::default(); }
                if !ciudad.raiz.is_empty() && ui.small_button("Subir [U]").clicked() { acciones.push(Accion::Comando(Comando::SubirFoco)); }
            });
        });
    });
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn camara_mira_al_objetivo_y_el_rayo_central_lo_toca() {
        let cam = Camara3::default();
        let vp = cam.vp(1.6);
        let lienzo = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0));
        let p = proyectar(&vp, cam.objetivo, lienzo).unwrap();
        assert!((p.x - 800.0).abs() < 1.0 && (p.y - 500.0).abs() < 1.0, "{p:?}");
        let (o, d) = cam.rayo([0.0, 0.0], 1.6);
        let t = rayo_caja(o, d, [490.0, -1.0, 490.0], [510.0, 1.0, 510.0]).expect("el rayo del centro toca la caja del centro");
        assert!((t - cam.distancia).abs() < 5.0, "{t}");
        assert!(rayo_caja(o, d, [0.0, -1.0, 0.0], [10.0, 1.0, 10.0]).is_none());
        // Un punto alto se ve mas arriba en pantalla.
        let alto = proyectar(&vp, [500.0, 100.0, 500.0], lienzo).unwrap();
        assert!(alto.y < p.y);
    }
}

/// Donde esta en la ciudad algo del grupo: un edificio (archivo o bloque) o un distrito.
#[derive(Clone, Copy, PartialEq)]
enum Ancla { Edificio(usize), Distrito(usize) }

#[derive(Default)]
struct Anclas { miembros: Vec<Option<Ancla>>, comparten: Vec<Option<Ancla>>, juntan: Vec<Option<Ancla>> }

/// Busca cada ruta del grupo (y sus puentes) en la ciudad de una pasada. Lo que esta dentro
/// de un bloque juntado cae en ese bloque.
fn anclas(ciudad: &Ciudad, k: &crate::combinar_ui::Cache) -> Anclas {
    use std::collections::HashMap;
    let cb = &k.c;
    let rutas: Vec<&str> = cb.miembros.iter().map(|m| m.id.as_str()).chain(cb.comparten.iter().map(|x| x.ruta.as_str())).chain(cb.juntan.iter().map(|x| x.ruta.as_str())).collect();
    let mut hallado: HashMap<&str, Ancla> = HashMap::with_capacity(rutas.len());
    let buscados: std::collections::HashSet<&str> = rutas.iter().copied().collect();
    for (i, b) in ciudad.edificios.iter().enumerate() { if buscados.contains(b.ruta.as_str()) { hallado.insert(&ciudad.edificios[i].ruta, Ancla::Edificio(i)); } }
    let faltan: Vec<&str> = rutas.iter().copied().filter(|r| !hallado.contains_key(r)).collect();
    for r in faltan {
        let a = ciudad.distritos.iter().position(|d| d.ruta == r).map(Ancla::Distrito)
            .or_else(|| ciudad.edificios.iter().position(|b| b.bloque && r.len() > b.ruta.len() && r.starts_with(b.ruta.as_str()) && r.as_bytes()[b.ruta.len()] == b'/').map(Ancla::Edificio));
        if let Some(a) = a { hallado.insert(r, a); }
    }
    let de = |r: &str| hallado.get(r).copied();
    Anclas { miembros: cb.miembros.iter().map(|m| de(&m.id)).collect(), comparten: cb.comparten.iter().map(|x| de(&x.ruta)).collect(), juntan: cb.juntan.iter().map(|x| de(&x.ruta)).collect() }
}

/// La azotea de un ancla (el centro de arriba del edificio o del distrito).
fn azotea(ciudad: &Ciudad, a: Ancla) -> V3 {
    match a {
        Ancla::Edificio(i) => { let b = &ciudad.edificios[i]; [b.rect[0] + b.rect[2] / 2.0, b.base + b.altura, b.rect[1] + b.rect[3] / 2.0] }
        Ancla::Distrito(i) => { let d = &ciudad.distritos[i]; [d.rect[0] + d.rect[2] / 2.0, d.base, d.rect[1] + d.rect[3] / 2.0] }
    }
}

/// Cables sobre la ciudad: arcos blancos entre los del grupo que se usan (flecha en el
/// usado) y arcos finos del acento (o verdes) hasta lo que comparten (o quien los junta).
fn cables(pintor: &egui::Painter, vp: &[[f32; 4]; 4], lienzo: Rect, ciudad: &Ciudad, k: &crate::combinar_ui::Cache, an: &Anclas, c: &Colores) {
    use eframe::egui::epaint::QuadraticBezierShape;
    let cb = &k.c;
    let punto = |a: Option<Ancla>| a.and_then(|a| proyectar(vp, azotea(ciudad, a), lienzo));
    let arco = |p: Pos2, q: Pos2, col: Color32, w: f32, flecha: bool| {
        let alto = ((q - p).length() * 0.35).clamp(18.0, 220.0);
        let ctl = p + (q - p) / 2.0 - Vec2::new(0.0, alto);
        if c.brillo { pintor.add(QuadraticBezierShape::from_points_stroke([p, ctl, q], false, Color32::TRANSPARENT, Stroke::new(w + 4.0, col.gamma_multiply(0.22)))); }
        pintor.add(QuadraticBezierShape::from_points_stroke([p, ctl, q], false, Color32::TRANSPARENT, Stroke::new(w, col)));
        if flecha {
            let dir = (q - ctl).normalized();
            let lado = Vec2::new(-dir.y, dir.x);
            pintor.add(egui::Shape::convex_polygon(vec![q, q - dir * 10.0 + lado * 5.0, q - dir * 10.0 - lado * 5.0], col, Stroke::NONE));
        }
    };
    for (i, x) in cb.comparten.iter().enumerate() {
        let Some(q) = punto(an.comparten[i]) else { continue };
        for &m in &x.miembros { if let Some(p) = punto(an.miembros[m]) { arco(p, q, c.acento.gamma_multiply(0.75), 1.3, false); } }
    }
    for (i, x) in cb.juntan.iter().enumerate() {
        let Some(q) = punto(an.juntan[i]) else { continue };
        for &m in &x.miembros { if let Some(p) = punto(an.miembros[m]) { arco(q, p, c.verde.gamma_multiply(0.75), 1.3, false); } }
    }
    for &(a, b, veces) in &cb.directas {
        if let (Some(p), Some(q)) = (punto(an.miembros[a]), punto(an.miembros[b])) { arco(p, q, Color32::WHITE, 1.8 + (veces as f32).ln_1p(), true); }
    }
}
