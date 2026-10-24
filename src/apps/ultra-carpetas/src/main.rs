// SPDX-License-Identifier: GPL-2.0-only
//! ULTRA Carpetas para Linux: la capa fina sobre `carpetas-nucleo` (no_std). Diseno: Eddi Salazar.
//!
//! Como React + Redux: aqui solo se traducen eventos a `Accion`, se ejecutan los `Efecto`
//! y se dibuja la `Vista` con egui sobre wgpu (Vulkan). Tres modos que se combinan:
//! explorador (arbol lateral), iconos libres y grafo de nodos (Blender o Houdini).
//!
//! Carriles aislados (F12 los muestra con semaforo): el disco va en su propio hilo, la
//! vista se recalcula solo si cambio el estado, el grafo solo pinta lo que se ve y los
//! iconos SVG son solo los incrustados. Solo ordena la vista: nunca toca tus archivos.

mod almacen;
mod archivo;
mod disco;
mod editor_ui;
mod entradas;
mod gpu;
mod pintar_grafo;
mod proceso;
mod svg;
mod vivos;

use carpetas_nucleo::carriles::{luz, presupuesto, Carril, Luz, MAX_DESPLEGADAS, MAX_DIAGNOSTICOS, MAX_ENTRADAS, MAX_LINEAS_POR_CUADRO, MAX_NODOS, MAX_PROCESOS};
use carpetas_nucleo::{atajo, reducir, vista, Accion, AccionPaleta, Comando, Dibujo, Direccion, Efecto, EstiloIconos, Estado, Modo, Nodo, Orden, Rgb,
                      Tecla, Tema, Tipo, Vista};
use std::sync::atomic::{AtomicU64, Ordering};
use disco::{ruta_segura, Disco, Pedido};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use entradas::legible;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn color(c: Rgb) -> Color32 { Color32::from_rgb(c[0], c[1], c[2]) }

fn acento() -> Rgb {
    // El mismo acento que el resto del escritorio (Personalizar); hielo si no hay.
    let salida = std::process::Command::new("ultra-personalizar").arg("acento").stdin(std::process::Stdio::null())
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    if salida.len() == 8 && salida.chars().all(|c| c.is_ascii_hexdigit()) {
        let v = |i: usize| u8::from_str_radix(&salida[i..i + 2], 16).unwrap_or(0);
        return [v(0), v(2), v(4)];
    }
    carpetas_nucleo::tipos::HIELO
}

/// Ultimo tiempo y maximo por carril, en microsegundos.
#[derive(Default)]
struct Medidas { ultimo: [u64; 9], maximo: [u64; 9] }

impl Medidas {
    fn anotar(&mut self, c: Carril, us: u64) { let i = c as usize; self.ultimo[i] = us; self.maximo[i] = self.maximo[i].max(us); }
}

struct App {
    estado: Estado,
    carpeta: PathBuf,
    atras: Vec<PathBuf>,
    error: Option<String>,
    texto_paleta: String,
    tema_aplicado: Option<Tema>,
    contexto: egui::Context,
    disco: Disco,
    generacion: u64,
    /// Carpeta pedida y aun sin respuesta (y si hay que recordarla para "Atras").
    pendiente: Option<(PathBuf, bool)>,
    vigiladas: Vec<String>,
    cache: Option<(u64, Arc<Vista>)>,
    medidas: Medidas,
    iconos: svg::Iconos,
    tiempo_iconos: Cell<u64>,
    /// Hay wgpu: el grafo se dibuja con geometria retenida en la GPU.
    gpu_ok: bool,
    geometria: Option<Geometria>,
    subidas: Arc<AtomicU64>,
    cuadros: u64,
    proceso: Option<proceso::Proceso>,
    texto_terminal: String,
    vivos: vivos::Muestreador,
    editor: editor_ui::PanelEditor,
    /// Huella del archivo abierto en el editor (para no pisar cambios de fuera).
    huella: Option<archivo::Huella>,
}

/// Geometria del grafo ya en la GPU y de que Vista salio. Si la Vista cambia pero el grafo
/// no (una linea de la terminal, un aviso), no se rehace ni se vuelve a subir.
struct Geometria { version: u64, clave: u64, fuente: Arc<Vista>, vertices: Arc<Vec<gpu::Vertice>> }

impl App {
    fn new(cc: &eframe::CreationContext<'_>, carpeta: PathBuf) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let gpu_ok = gpu::instalar(cc.wgpu_render_state.as_ref());
        fuentes(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let ctx_vivos = cc.egui_ctx.clone();
        let mut app = App {
            estado: Estado { acento: acento(), ..Estado::default() }, carpeta: PathBuf::new(), atras: Vec::new(), error: None, contexto: cc.egui_ctx.clone(),
            texto_paleta: String::new(), tema_aplicado: None, disco: Disco::nuevo(move || ctx.request_repaint()), generacion: 0,
            pendiente: None, vigiladas: Vec::new(), cache: None, medidas: Medidas::default(), iconos: svg::Iconos::nuevo(), tiempo_iconos: Cell::new(0),
            gpu_ok, geometria: None, subidas: Arc::new(AtomicU64::new(0)), cuadros: 0, proceso: None, texto_terminal: String::new(),
            vivos: vivos::Muestreador::nuevo(move || ctx_vivos.request_repaint()), editor: editor_ui::PanelEditor::default(), huella: None,
        };
        let ajustes = almacen::cargar_ajustes(&almacen::archivo_ajustes(&almacen::base()));
        app.despachar(Accion::CargarAjustes(ajustes));
        app.abrir(carpeta, false);
        app
    }

    /// El unico camino para cambiar el estado: accion -> reducer -> efectos.
    fn despachar(&mut self, accion: Accion) {
        for efecto in reducir(&mut self.estado, accion) { self.ejecutar(efecto); }
        self.vigilar();
    }

    fn ejecutar(&mut self, efecto: Efecto) {
        match efecto {
            Efecto::GuardarAjustes(json) => {
                if let Err(e) = almacen::guardar(&almacen::archivo_ajustes(&almacen::base()), &json) { self.error = Some(format!("No se pudo guardar los ajustes: {e}")); }
            }
            Efecto::Guardar(json) => {
                if let Err(e) = almacen::guardar(&almacen::archivo_para(&almacen::base(), &self.carpeta), &json) { self.error = Some(format!("No se pudo guardar el orden: {e}")); }
            }
            Efecto::AbrirArchivo(rel) => {
                // El nucleo solo pide rutas que leyo; aun asi, nada de "..", absolutas ni raras.
                if !disco::ruta_limpia(&rel) { return; }
                if let Err(e) = std::process::Command::new("xdg-open").arg(self.carpeta.join(&rel)).stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
                    self.error = Some(format!("No se pudo abrir {rel}: {e}"));
                }
            }
            Efecto::EntrarCarpeta(rel) => match ruta_segura(&self.carpeta, &rel) {
                Some(p) => self.abrir(p, true),
                None => self.error = Some(format!("No se entra en {rel}: enlace o ruta rara")),
            },
            Efecto::LeerCarpeta(rel) => self.disco.pedir(Pedido::Listar { generacion: self.generacion, base: self.carpeta.clone(), ruta: rel, ocultos: self.estado.ocultos }),
            Efecto::Subir => { if let Some(p) = self.carpeta.parent().map(Path::to_path_buf) { self.abrir(p, true); } }
            Efecto::Atras => { if let Some(p) = self.atras.pop() { self.abrir(p, false); } }
            Efecto::Ejecutar { titulo: _, carpeta, programa, args } => {
                let Some(dir) = ruta_segura(&self.carpeta, &carpeta) else {
                    self.despachar(Accion::Salida { linea: format!("carpeta rechazada: {carpeta}"), error: true });
                    self.despachar(Accion::Termino(None));
                    return;
                };
                let ctx = self.contexto.clone();
                match proceso::Proceso::lanzar(&dir, &programa, &args, move || ctx.request_repaint()) {
                    Ok(p) => { self.vivos.seguir(Some(p.grupo())); self.proceso = Some(p); }
                    Err(e) => { self.despachar(Accion::Salida { linea: e, error: true }); self.despachar(Accion::Termino(None)); }
                }
            }
            Efecto::Detener => { if let Some(p) = &self.proceso { p.detener(); } }
            Efecto::LeerArchivo(rel) => {
                let t = Instant::now();
                let resultado = archivo::leer(&self.carpeta, &rel).map(|(texto, h)| { self.huella = Some(h); texto });
                self.medidas.anotar(Carril::Disco, t.elapsed().as_micros() as u64);
                self.despachar(Accion::ArchivoLeido { id: rel, resultado });
            }
            Efecto::GuardarArchivo { id, texto } => {
                let resultado = archivo::guardar(&self.carpeta, &id, &texto, self.huella).map(|h| { self.huella = Some(h); });
                self.despachar(Accion::Guardado { id, resultado });
            }
            Efecto::Releer { .. } => {
                for ruta in std::iter::once(String::new()).chain(self.estado.expandidas.iter().cloned()) {
                    self.disco.pedir(Pedido::Listar { generacion: self.generacion, base: self.carpeta.clone(), ruta, ocultos: self.estado.ocultos });
                }
            }
        }
    }

    /// Pide la carpeta al hilo del disco; se cambia cuando llega la respuesta.
    fn abrir(&mut self, carpeta: PathBuf, recordar: bool) {
        let carpeta = carpeta.canonicalize().unwrap_or(carpeta);
        self.generacion += 1;
        self.pendiente = Some((carpeta.clone(), recordar));
        self.disco.pedir(Pedido::Listar { generacion: self.generacion, base: carpeta, ruta: String::new(), ocultos: self.estado.ocultos });
    }

    /// Lo que llego del disco desde el ultimo cuadro.
    fn recoger(&mut self) {
        for l in self.disco.recoger() {
            self.medidas.anotar(Carril::Disco, l.micros);
            if l.generacion != self.generacion { continue; } // respuesta de una carpeta anterior
            if let Some((pedida, recordar)) = self.pendiente.clone() {
                if l.base == pedida && l.ruta.is_empty() {
                    self.pendiente = None;
                    match l.resultado {
                        Ok(entradas) => {
                            if recordar && !self.carpeta.as_os_str().is_empty() { self.atras.push(self.carpeta.clone()); }
                            self.carpeta = pedida;
                            self.error = None;
                            self.vigiladas.clear();
                            let disposicion = almacen::cargar(&almacen::archivo_para(&almacen::base(), &self.carpeta));
                            let nombre = self.carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
                            self.despachar(Accion::Cargar { nombre, entradas, disposicion });
                            self.despachar(Accion::RutaRaiz(self.carpeta.to_string_lossy().into_owned()));
                        }
                        Err(e) => { self.error = Some(format!("No se pudo abrir {}: {e}", pedida.display())); self.vigiladas.clear(); self.vigilar(); }
                    }
                    continue;
                }
            }
            if l.base != self.carpeta { continue; }
            match l.resultado {
                Ok(entradas) => self.despachar(Accion::Contenido { ruta: l.ruta, entradas }),
                Err(e) if !l.ruta.is_empty() => self.error = Some(format!("No se pudo leer {}: {e}", l.ruta)),
                Err(e) => self.error = Some(format!("No se pudo leer la carpeta: {e}")),
            }
        }
    }

    /// Dice al hilo del disco que carpetas vigilar: la abierta y las desplegadas.
    fn vigilar(&mut self) {
        if self.carpeta.as_os_str().is_empty() { return; }
        let rutas: Vec<String> = std::iter::once(String::new()).chain(self.estado.expandidas.iter().cloned()).collect();
        if rutas != self.vigiladas {
            self.vigiladas = rutas.clone();
            self.disco.pedir(Pedido::Vigilar { generacion: self.generacion, base: self.carpeta.clone(), rutas, ocultos: self.estado.ocultos });
        }
    }

    /// La Vista en cache: solo se recalcula si cambio el contenido (mover la camara no cuenta).
    fn vista(&mut self) -> Arc<Vista> {
        if let Some((version, v)) = &self.cache { if *version == self.estado.version_contenido { return v.clone(); } }
        if self.estado.ajustes.modo == Modo::Grafo {
            let t = Instant::now();
            let _ = carpetas_nucleo::grafo::acomodar(&self.estado);
            self.medidas.anotar(Carril::Acomodo, t.elapsed().as_micros() as u64);
        }
        let t = Instant::now();
        let v = Arc::new(vista(&self.estado));
        self.medidas.anotar(Carril::Vista, t.elapsed().as_micros() as u64);
        self.cache = Some((self.estado.version_contenido, v.clone()));
        v
    }

    /// Geometria del grafo para la GPU: la CPU la arma solo si cambio el grafo de verdad.
    fn retenido(&mut self, v: &Arc<Vista>, tema: &Tema) -> Option<pintar_grafo::Retenido> {
        if !self.gpu_ok || self.estado.ajustes.dibujo != Dibujo::Gpu { return None; }
        let (nodos, cables) = v.grafo.as_ref()?;
        let version = self.estado.version_contenido;
        let mismo = |g: &Geometria| g.version == version || (g.fuente.grafo == v.grafo && g.fuente.tema == v.tema);
        match &mut self.geometria {
            Some(g) if mismo(g) => { g.version = version; g.fuente = v.clone(); }
            _ => {
                let t = Instant::now();
                let vertices = Arc::new(gpu::geometria(nodos, cables, tema, self.estado.ajustes.direccion));
                self.medidas.anotar(Carril::Geometria, t.elapsed().as_micros() as u64);
                let clave = self.geometria.as_ref().map(|g| g.clave + 1).unwrap_or(1);
                self.geometria = Some(Geometria { version, clave, fuente: v.clone(), vertices });
            }
        }
        let g = self.geometria.as_ref()?;
        Some((g.clave, g.vertices.clone(), self.subidas.clone()))
    }

    /// Lo que va midiendo /proc del proceso en marcha (su hilo manda una muestra cada 500 ms).
    fn recoger_vivos(&mut self) {
        if let Some(m) = self.vivos.recoger() {
            self.medidas.anotar(Carril::Procesos, m.micros);
            self.despachar(Accion::Procesos(m.procesos));
        }
    }

    /// Lo que salio del proceso desde el ultimo cuadro (como mucho 512 lineas).
    fn recoger_proceso(&mut self) {
        let Some(p) = &self.proceso else { return };
        let t = Instant::now();
        let mensajes = p.recoger(MAX_LINEAS_POR_CUADRO);
        let quedan = mensajes.len() == MAX_LINEAS_POR_CUADRO;
        for m in mensajes {
            match m {
                proceso::Mensaje::Linea(linea, error) => self.despachar(Accion::Salida { linea, error }),
                proceso::Mensaje::Fin(codigo) => { self.proceso = None; self.vivos.seguir(None); self.despachar(Accion::Termino(codigo)); }
            }
        }
        self.medidas.anotar(Carril::Terminal, t.elapsed().as_micros() as u64);
        if quedan { self.contexto.request_repaint(); } // el resto, en el siguiente cuadro
    }

    fn pintar_icono(&self, ui: &egui::Ui, clave: &str, rect: Rect, col: Color32) -> bool {
        if clave.is_empty() { return false; }
        let t = Instant::now();
        let hecho = self.iconos.pintar(ui, clave, rect, col);
        self.tiempo_iconos.set(self.tiempo_iconos.get() + t.elapsed().as_micros() as u64);
        hecho
    }

    /// Teclas de egui -> teclas del nucleo -> accion (el "keybindings.json").
    fn teclado(&mut self, ctx: &egui::Context) {
        const MAPA: [(Key, Tecla); 24] = [(Key::S, Tecla::S), (Key::F8, Tecla::F8), (Key::T, Tecla::T), (Key::Comma, Tecla::Coma), (Key::G, Tecla::G), (Key::L, Tecla::L), (Key::H, Tecla::H),
            (Key::Z, Tecla::Z), (Key::A, Tecla::A), (Key::P, Tecla::P), (Key::B, Tecla::B), (Key::D, Tecla::D), (Key::I, Tecla::I), (Key::Tab, Tecla::Tab),
            (Key::Home, Tecla::Inicio), (Key::F12, Tecla::F12), (Key::Backspace, Tecla::Retroceso), (Key::Enter, Tecla::Enter),
            (Key::Escape, Tecla::Escape), (Key::F5, Tecla::F5), (Key::ArrowUp, Tecla::Arriba), (Key::ArrowDown, Tecla::Abajo),
            (Key::ArrowLeft, Tecla::Izquierda), (Key::Backtick, Tecla::Acento)];
        // Cada tecla con los modificadores de SU evento: si Ctrl se suelta en el mismo cuadro
        // (tecleo rapido, macros), Ctrl+S sigue siendo Ctrl+S.
        let pulsadas: Vec<(Tecla, egui::Modifiers)> = ctx.input(|i| i.events.iter().filter_map(|e| match e {
            egui::Event::Key { key, pressed: true, modifiers, .. } => MAPA.iter().find(|(k, _)| k == key).map(|(_, t)| (*t, *modifiers)),
            _ => None,
        }).collect());
        // Escribiendo en la terminal: las letras son letras, no atajos (solo Ctrl, F y Esc).
        let escribiendo = ctx.egui_wants_keyboard_input() && self.estado.paleta.is_none();
        for (t, m) in pulsadas {
            // En el editor, Esc cierra el editor (con aviso si hay cambios sin guardar).
            if t == Tecla::Escape && self.editor.foco && self.estado.paleta.is_none() { self.despachar(Accion::Comando(Comando::CerrarEditor)); continue; }
            // Escribiendo (editor, terminal): Ctrl+Z, Ctrl+A y las letras son del texto. Solo pasan
            // las teclas F, Esc y los Ctrl que no tienen sentido dentro de un texto.
            let global = matches!(t, Tecla::F5 | Tecla::F8 | Tecla::F12 | Tecla::Escape)
                || (m.command && matches!(t, Tecla::S | Tecla::P | Tecla::Acento | Tecla::B | Tecla::Coma));
            if escribiendo && !global { continue; }
            if let Some(a) = atajo(t, m.command, m.shift, m.alt, self.estado.paleta.is_some()) { self.despachar(a); }
        }
    }

    fn barra(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("BMO-X // CARPETAS").color(c.acento).strong().size(16.0));
            ui.add_space(10.0);
            if ui.button("Atrás").clicked() { acciones.push(Accion::Comando(Comando::Atras)); }
            if ui.button("Arriba").on_hover_text("Retroceso").clicked() { acciones.push(Accion::Comando(Comando::Subir)); }
            ui.add_space(6.0);
            let mut acumulada = PathBuf::new();
            let mut destino = None;
            for parte in self.carpeta.components() {
                acumulada.push(parte);
                let texto = parte.as_os_str().to_string_lossy();
                let texto = if texto == "/" { "/".to_string() } else { format!("{texto} /") };
                if ui.add(egui::Button::new(egui::RichText::new(texto).color(c.texto)).frame(false)).clicked() { destino = Some(acumulada.clone()); }
            }
            if let Some(d) = destino { self.abrir(d, true); }
            if self.pendiente.is_some() { ui.label(egui::RichText::new("leyendo…").color(c.tenue)); }
        });
        let play_raiz = self.vista().play_raiz.clone();
        ui.horizontal(|ui| {
            let a = &self.estado.ajustes;
            let grafo = a.modo == Modo::Grafo;
            if ui.selectable_label(!grafo, "Iconos").clicked() && grafo { acciones.push(Accion::Comando(Comando::Modo)); }
            if ui.selectable_label(grafo, "Grafo [Tab]").clicked() && !grafo { acciones.push(Accion::Comando(Comando::Modo)); }
            if ui.selectable_label(a.explorador, "Explorador [Ctrl+B]").clicked() { acciones.push(Accion::Comando(Comando::Explorador)); }
            ui.separator();
            let d = &self.estado.disp;
            if grafo {
                let texto = match a.direccion { Direccion::Blender => "Blender: izq-der [D]", Direccion::Houdini => "Houdini: arriba-abajo [D]" };
                if ui.button(texto).on_hover_text("Blender: de izquierda a derecha. Houdini: de arriba abajo.").clicked() { acciones.push(Accion::Comando(Comando::Direccion)); }
                if ui.button("Encuadrar [Inicio]").clicked() { acciones.push(Accion::Comando(Comando::Encuadrar)); }
                if ui.add_enabled(!d.bloqueada, egui::Button::new("Reacomodar")).clicked() { acciones.push(Accion::Comando(Comando::Reacomodar)); }
            } else {
                if ui.selectable_label(d.rejilla, "Rejilla [G]").clicked() { acciones.push(Accion::Comando(Comando::Rejilla)); }
                ui.add_enabled_ui(!d.bloqueada, |ui| {
                    ui.menu_button("Ordenar", |ui| {
                        for (texto, o) in [("Por nombre", Orden::Nombre), ("Por tipo", Orden::Tipo), ("Por tamano", Orden::Tamano)] {
                            if ui.button(texto).clicked() { acciones.push(Accion::Comando(Comando::Ordenar(o))); ui.close(); }
                        }
                    });
                });
            }
            if ui.selectable_label(d.bloqueada, if d.bloqueada { "Candado puesto [L]" } else { "Candado [L]" }).clicked() { acciones.push(Accion::Comando(Comando::Candado)); }
            if ui.button("Deshacer [Ctrl+Z]").clicked() { acciones.push(Accion::Comando(Comando::Deshacer)); }
            ui.separator();
            if ui.button(format!("Iconos: {} [I]", a.iconos.nombre())).clicked() { acciones.push(Accion::Comando(Comando::Iconos)); }
            if let Some(titulo) = &play_raiz {
                if ui.add(egui::Button::new(egui::RichText::new(format!("Ejecutar: {titulo} [F5]")).color(c.verde))).clicked() { acciones.push(Accion::Ejecutar(String::new())); }
            }
            let (errores, avisos) = self.estado.errores.conteo("");
            if errores + avisos > 0 {
                use carpetas_nucleo::diagnosticos::contar;
                let texto = match (errores, avisos) {
                    (0, a) => format!("{} [F8]", contar(a, "aviso", "avisos")),
                    (e, 0) => format!("{} [F8]", contar(e, "error", "errores")),
                    (e, a) => format!("{} · {} [F8]", contar(e, "error", "errores"), contar(a, "aviso", "avisos")),
                };
                if ui.add(egui::Button::new(egui::RichText::new(texto).color(if errores > 0 { c.error } else { c.aviso }).strong()))
                    .on_hover_text("Ir al siguiente error: abre el editor en la linea").clicked() { acciones.push(Accion::Comando(Comando::SiguienteError)); }
            }
            if ui.selectable_label(self.estado.terminal.abierta, "Terminal [Ctrl+`]").clicked() { acciones.push(Accion::Comando(Comando::Terminal)); }
            if ui.button("Comandos [Ctrl+Shift+P]").clicked() { acciones.push(Accion::Comando(Comando::Paleta)); }
            if ui.selectable_label(self.estado.panel_ajustes, "Ajustes [Ctrl+,]").clicked() { acciones.push(Accion::Comando(Comando::Ajustes)); }
        });
        for a in acciones { self.despachar(a); }
    }

    /// Explorador como el de VS Code y el de tu TITAN: arbol sincronizado con el grafo.
    fn explorador(&mut self, ui: &mut egui::Ui, v: &Vista, c: &Colores) {
        let Some(filas) = &v.arbol else { return };
        let mut acciones = Vec::new();
        ui.label(egui::RichText::new("EXPLORADOR").color(c.acento).strong());
        ui.label(egui::RichText::new(self.carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into())).color(c.tenue));
        ui.add_space(4.0);
        let ctrl = ui.input(|i| i.modifiers.command);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for f in filas {
                let (fila, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
                let p = ui.painter_at(fila);
                if f.seleccionado { p.rect_filled(fila, 0.0, c.acento.gamma_multiply(0.18)); p.rect_filled(Rect::from_min_size(fila.min, Vec2::new(2.0, fila.height())), 0.0, c.acento); }
                else if r.hovered() { p.rect_filled(fila, 0.0, c.superficie); }
                let x = fila.min.x + 6.0 + f.nivel as f32 * 14.0;
                // Triangulo de desplegar (dibujado: no depende de la fuente).
                if let Some(abierta) = f.expandida {
                    let t = Rect::from_center_size(Pos2::new(x + 5.0, fila.center().y), Vec2::splat(12.0));
                    let rt = ui.interact(t, ui.make_persistent_id(("arbol-flecha", &f.id)), Sense::click());
                    let m = t.center();
                    let puntos = if abierta { vec![m + Vec2::new(-4.0, -2.0), m + Vec2::new(4.0, -2.0), m + Vec2::new(0.0, 3.0)] }
                                 else { vec![m + Vec2::new(-2.0, -4.0), m + Vec2::new(3.0, 0.0), m + Vec2::new(-2.0, 4.0)] };
                    p.add(egui::Shape::convex_polygon(puntos, if rt.hovered() { c.acento } else { c.tenue }, Stroke::NONE));
                    if rt.clicked() { acciones.push(Accion::Expandir(f.id.clone())); continue; }
                }
                let ic = Rect::from_min_size(Pos2::new(x + 14.0, fila.center().y - 8.0), Vec2::splat(16.0));
                if !self.pintar_icono(ui, f.icono, ic, color(f.color)) {
                    p.rect(ic.shrink(2.0), 1, if f.tipo == Tipo::Carpeta { color(f.color).gamma_multiply(0.35) } else { Color32::TRANSPARENT }, Stroke::new(1.2, color(f.color)), StrokeKind::Inside);
                }
                let col_nombre = if f.marcas.errores > 0 { c.error } else if f.marcas.avisos > 0 { c.aviso } else if f.seleccionado { c.texto } else { c.tenue };
                p.text(Pos2::new(ic.max.x + 6.0, fila.center().y), Align2::LEFT_CENTER, &f.nombre, FontId::proportional(13.0), col_nombre);
                let derecha = Pos2::new(fila.max.x - if f.play.is_some() { 26.0 } else { 4.0 }, fila.center().y - 8.5);
                pintar_grafo::insignia(&p, c, derecha, &f.marcas);
                if r.secondary_clicked() { acciones.push(Accion::AbrirEditor { id: f.id.clone(), linea: None }); }
                if let Some(primero) = &f.marcas.primero { if r.hovered() { r.clone().on_hover_text(format!("{primero}\nclic derecho: abrir en el editor")); } }
                if let Some(titulo) = &f.play {
                    let caja = Rect::from_center_size(Pos2::new(fila.max.x - 14.0, fila.center().y), Vec2::splat(16.0));
                    let rp = ui.interact(caja, ui.make_persistent_id(("arbol-play", &f.id)), Sense::click());
                    pintar_grafo::triangulo_play(&p, caja, if rp.hovered() { c.texto } else { c.verde });
                    if rp.clicked() { acciones.push(Accion::Ejecutar(f.id.clone())); continue; }
                    rp.on_hover_text(format!("Ejecutar: {titulo}"));
                }
                if r.clicked() { acciones.push(Accion::Pulsar { nombre: f.id.clone(), sumar: ctrl }); }
                if r.double_clicked() { acciones.push(Accion::Abrir(f.id.clone())); }
            }
        });
        for a in acciones { self.despachar(a); }
    }

    fn iconos_libres(&mut self, ui: &mut egui::Ui, v: &Vista, c: &Colores) {
        let ancho = ui.available_width().max(1.0);
        let mut acciones = Vec::new();
        if (ancho - self.estado.ancho).abs() > 0.5 { acciones.push(Accion::Ancho(ancho)); }
        let tam = Vec2::new(v.tam[0].max(ui.available_width()), (v.tam[1] + 200.0).max(ui.available_height()));
        let (lienzo, fondo) = ui.allocate_exact_size(tam, Sense::click_and_drag());
        let pintor = ui.painter_at(lienzo);
        let origen = lienzo.min + Vec2::splat(16.0);
        let local = |p: Pos2| { let d = p - origen; [d.x, d.y] };
        let ctrl = ui.input(|i| i.modifiers.command);
        let visible = ui.clip_rect().expand(40.0);
        for nodo in &v.nodos {
            match nodo {
                Nodo::Rejilla { paso } => {
                    let mut y = 0.0;
                    while y < tam.y { let mut x = 0.0; while x < tam.x { let p = origen + Vec2::new(x, y); if visible.contains(p) { pintor.circle_filled(p, 1.2, c.linea); } x += paso[0]; } y += paso[1]; }
                }
                Nodo::Icono { nombre, pos, tam, tipo, color: col, seleccionado, etiqueta, icono, play, marcas } => {
                    let rect = Rect::from_min_size(origen + Vec2::new(pos[0], pos[1]), Vec2::new(tam[0], tam[1]));
                    if !visible.intersects(rect) { continue; } // carril de pintado: solo lo que se ve
                    let r = ui.interact(rect, ui.make_persistent_id(("icono", &self.carpeta, nombre)), Sense::click_and_drag());
                    if r.clicked() { acciones.push(Accion::Pulsar { nombre: nombre.clone(), sumar: ctrl }); }
                    if r.double_clicked() { acciones.push(Accion::Abrir(nombre.clone())); }
                    if r.drag_started() { acciones.push(Accion::EmpezarArrastre(nombre.clone())); }
                    if r.dragged() { let d = r.drag_delta(); acciones.push(Accion::Arrastrar([d.x, d.y])); }
                    if r.drag_stopped() { acciones.push(Accion::Soltar); }
                    let caja = Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 30.0), Vec2::splat(48.0));
                    if r.hovered() || *seleccionado { marco(&pintor, c, rect, *seleccionado); }
                    if !self.pintar_icono(ui, icono, caja, color(*col)) { icono_neon(&pintor, rect, *tipo, color(*col), etiqueta); }
                    etiqueta_nombre(&pintor, c, rect, nombre, *seleccionado);
                    if marcas.errores > 0 { pintor.rect_stroke(rect.expand(2.0), c.radio(), Stroke::new(2.0, c.error), StrokeKind::Outside); }
                    pintar_grafo::insignia(&pintor, c, rect.right_top() + Vec2::new(4.0, -8.0), marcas);
                    if r.secondary_clicked() { acciones.push(Accion::AbrirEditor { id: nombre.clone(), linea: None }); }
                    if let Some(titulo) = play {
                        let caja = Rect::from_min_size(rect.right_top() + Vec2::new(-22.0, 4.0), Vec2::splat(18.0));
                        let rp = ui.interact(caja, ui.make_persistent_id(("icono-play", nombre)), Sense::click());
                        pintor.circle_filled(caja.center(), 9.0, c.panel);
                        pintar_grafo::triangulo_play(&pintor, caja, if rp.hovered() { c.texto } else { c.verde });
                        if rp.clicked() { acciones.push(Accion::Ejecutar(nombre.clone())); }
                        rp.on_hover_text(format!("Ejecutar: {titulo}"));
                    }
                    let tam_txt = self.estado.entradas.iter().find(|e| &e.nombre == nombre).map(|e| e.tam).unwrap_or(0);
                    let error = marcas.primero.as_ref().map(|p| format!("\n\n{p}")).unwrap_or_default();
                    r.on_hover_text(format!("{nombre}\n{}{error}\nclic derecho: abrir en el editor", if *tipo == Tipo::Carpeta { "Carpeta".into() } else { legible(tam_txt) }));
                }
                Nodo::Banda { desde, hasta, color: col } => {
                    let banda = Rect::from_two_pos(origen + Vec2::new(desde[0], desde[1]), origen + Vec2::new(hasta[0], hasta[1]));
                    pintor.rect(banda, c.radio(), color(*col).gamma_multiply(0.08), Stroke::new(1.0, color(*col)), StrokeKind::Inside);
                }
            }
        }
        if fondo.drag_started() { if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::EmpezarBanda { punto: local(p), sumar: ctrl }); } }
        if fondo.dragged() { if let Some(p) = fondo.interact_pointer_pos() { acciones.push(Accion::MoverBanda(local(p))); } }
        if fondo.drag_stopped() { acciones.push(Accion::TerminarBanda); }
        if fondo.clicked() && !ctrl { acciones.push(Accion::Comando(Comando::LimpiarSeleccion)); }
        for a in acciones { self.despachar(a); }
    }

    /// Paleta de comandos como en VS Code: escribe, flechas y Enter.
    fn paleta(&mut self, ctx: &egui::Context, v: &Vista, c: &Colores) {
        let Some(vp) = &v.paleta else { self.texto_paleta.clear(); return };
        let mut acciones = Vec::new();
        egui::Area::new(egui::Id::unique("ultra-carpetas-paleta")).anchor(Align2::CENTER_TOP, [0.0, 70.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.acento)).corner_radius(c.radio()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(560.0);
                ui.label(egui::RichText::new("COMANDOS //").color(c.acento).strong());
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto_paleta).hint_text("escribe para buscar (ej. grafo, hou, tema drac, icon)").desired_width(540.0));
                r.request_focus();
                if r.changed() { acciones.push(Accion::Paleta(AccionPaleta::Escribir(self.texto_paleta.clone()))); }
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (i, (titulo, tecla, elegido)) in vp.filas.iter().enumerate() {
                        let fila = ui.horizontal(|ui| {
                            let texto = egui::RichText::new(*titulo).color(if *elegido { c.sobre_acento } else { c.texto });
                            let r = ui.add(egui::Button::new(texto).fill(if *elegido { c.acento } else { Color32::TRANSPARENT }).min_size(Vec2::new(420.0, 22.0)));
                            ui.label(egui::RichText::new(*tecla).color(c.tenue).monospace());
                            r
                        }).inner;
                        if *elegido { fila.scroll_to_me(None); }
                        if fila.clicked() { acciones.push(Accion::Paleta(AccionPaleta::Elegir(i))); }
                    }
                });
                if vp.filas.is_empty() { ui.label(egui::RichText::new("Ningun comando coincide").color(c.tenue)); }
            });
        });
        for a in acciones { self.despachar(a); }
    }

    /// Ajustes como en VS Code (Ctrl+,): temas con vista previa, iconos y direccion del grafo.
    fn ajustes(&mut self, ctx: &egui::Context, v: &Vista, c: &Colores) {
        let Some(va) = &v.ajustes else { return };
        let mut acciones = Vec::new();
        let (estilo_actual, dir_actual) = (self.estado.ajustes.iconos, self.estado.ajustes.direccion);
        egui::Area::new(egui::Id::unique("ultra-carpetas-ajustes")).anchor(Align2::CENTER_TOP, [0.0, 70.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.acento)).corner_radius(c.radio()).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(760.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("AJUSTES").color(c.acento).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cerrar [Esc]").clicked() { acciones.push(Accion::Comando(Comando::Ajustes)); }
                    });
                });
                ui.label(egui::RichText::new("TEMA — clic, T (siguiente) o «tema …» en la paleta. Se guarda solo.").color(c.tenue));
                ui.add_space(6.0);
                egui::Grid::new("temas").num_columns(3).spacing([10.0, 10.0]).show(ui, |ui| {
                    for (i, (_, nombre, descripcion, muestras, elegido)) in va.temas.iter().enumerate() {
                        let (rect, r) = ui.allocate_exact_size(Vec2::new(240.0, 80.0), Sense::click());
                        let p = ui.painter_at(rect);
                        // La tarjeta se pinta con los colores del propio tema: vista previa real.
                        p.rect(rect, c.radio(), color(muestras[1]), Stroke::new(if *elegido { 2.0 } else { 1.0 }, if *elegido { color(muestras[2]) } else if r.hovered() { c.acento } else { c.linea }), StrokeKind::Inside);
                        for (k, m) in muestras.iter().enumerate() {
                            let caja = Rect::from_min_size(rect.min + Vec2::new(10.0 + k as f32 * 22.0, 10.0), Vec2::new(18.0, 18.0));
                            p.rect(caja, CornerRadius::same(2), color(*m), Stroke::new(1.0, Color32::from_white_alpha(30)), StrokeKind::Inside);
                        }
                        p.text(rect.min + Vec2::new(10.0, 36.0), Align2::LEFT_TOP, *nombre, FontId::proportional(14.0), color(muestras[2]));
                        let desc = p.layout(descripcion.to_string(), FontId::proportional(10.5), c.tenue, 220.0);
                        p.galley(rect.min + Vec2::new(10.0, 54.0), desc, c.tenue);
                        if *elegido { p.text(rect.right_top() + Vec2::new(-10.0, 10.0), Align2::RIGHT_TOP, "EN USO", FontId::monospace(10.0), color(muestras[2])); }
                        if r.clicked() { acciones.push(Accion::Comando(Comando::Tema(i as u8))); }
                        if (i + 1) % 3 == 0 { ui.end_row(); }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ICONOS").color(c.acento).strong());
                    for estilo in [EstiloIconos::Neon, EstiloIconos::Material, EstiloIconos::Lucide] {
                        if ui.selectable_label(estilo_actual == estilo, estilo.nombre()).clicked() {
                            let mut e = estilo_actual;
                            while e != estilo { e = e.siguiente(); acciones.push(Accion::Comando(Comando::Iconos)); }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("GRAFO").color(c.acento).strong());
                    for (d, texto) in [(Direccion::Blender, "Blender: de izquierda a derecha"), (Direccion::Houdini, "Houdini: de arriba abajo")] {
                        if ui.selectable_label(dir_actual == d, texto).clicked() && dir_actual != d { acciones.push(Accion::Comando(Comando::Direccion)); }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("DIBUJO").color(c.acento).strong());
                    let dibujo = self.estado.ajustes.dibujo;
                    let texto_gpu = if self.gpu_ok { "GPU retenido (la CPU arma la geometria una vez)" } else { "GPU retenido (no hay wgpu aqui)" };
                    if ui.add_enabled(self.gpu_ok, egui::Button::selectable(dibujo == Dibujo::Gpu, texto_gpu)).clicked() && dibujo != Dibujo::Gpu { acciones.push(Accion::Comando(Comando::Dibujo)); }
                    if ui.selectable_label(dibujo == Dibujo::Cpu, "CPU cada cuadro").clicked() && dibujo != Dibujo::Cpu { acciones.push(Accion::Comando(Comando::Dibujo)); }
                });
                ui.horizontal(|ui| {
                    let mut brillo = va.brillo;
                    if ui.checkbox(&mut brillo, "Brillo neon en lo seleccionado").changed() { acciones.push(Accion::Comando(Comando::Brillo)); }
                    let mut propio = va.acento_escritorio;
                    if ui.checkbox(&mut propio, "Usar el acento del escritorio en cualquier tema").changed() { acciones.push(Accion::Comando(Comando::AcentoEscritorio)); }
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ERRORES").color(c.acento).strong());
                    let mut parar = self.estado.ajustes.parar_en_error;
                    if ui.checkbox(&mut parar, "Parar el proceso en el primer error atrapado").changed() { acciones.push(Accion::Comando(Comando::PararEnError)); }
                });
            });
        });
        for a in acciones { self.despachar(a); }
    }

    /// Terminal de abajo, como la de VS Code: salida en vivo, parar, limpiar y una linea
    /// para escribir comandos (sin shell: sin tuberias ni comodines). Lee el Estado en vivo
    /// (la salida no rehace la Vista) y solo dibuja las filas que se ven. Las lineas que
    /// apuntan a un archivo (--> src/main.rs:3:5) son enlaces: clic = editor en esa linea.
    fn terminal(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        let t = &self.estado.terminal;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("TERMINAL").color(c.acento).strong());
            match &t.en_curso {
                Some(titulo) => {
                    ui.label(egui::RichText::new(format!("en marcha: {titulo}")).color(c.verde));
                    if !self.estado.procesos.is_empty() {
                        let (cpu, mem) = carpetas_nucleo::procesos::totales(&self.estado.procesos);
                        ui.label(egui::RichText::new(format!("· {} procesos · CPU {cpu:.0}% · {}", self.estado.procesos.len(), pintar_grafo::memoria(mem)))
                            .color(c.tenue).monospace().size(11.5));
                    }
                    if ui.button("Detener").clicked() { acciones.push(Accion::Comando(Comando::Detener)); }
                }
                None => { ui.label(egui::RichText::new("sin procesos").color(c.tenue)); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cerrar").clicked() { acciones.push(Accion::Comando(Comando::Terminal)); }
                if ui.button("Limpiar").clicked() { acciones.push(Accion::Comando(Comando::LimpiarTerminal)); }
            });
        });
        let alto = (ui.available_height() - 30.0).max(40.0);
        let fila = ui.fonts_mut(|f| f.row_height(&FontId::monospace(12.0))) + ui.spacing().item_spacing.y;
        let (base, raiz) = (self.estado.errores.base.clone(), self.estado.raiz_absoluta.clone());
        egui::ScrollArea::vertical().max_height(alto).auto_shrink([false, false]).stick_to_bottom(true)
            .show_rows(ui, fila, t.lineas.len(), |ui, rango| {
            for (linea, error) in t.lineas.range(rango) {
                // stderr no siempre es un error (cargo escribe ahi su progreso): rojo solo si lo parece.
                let minus = linea.to_lowercase();
                let grave = minus.contains("error") || minus.contains("panicked") || minus.contains("fatal") || minus.starts_with("[termino con");
                let col = if *error && grave { c.error } else if *error { c.aviso } else if linea.starts_with("$ ") { c.acento } else { c.texto };
                let destino = carpetas_nucleo::diagnosticos::ubicacion(linea)
                    .and_then(|(ruta, l, _)| carpetas_nucleo::diagnosticos::resolver(&ruta, &base, &raiz).map(|id| (id, l)));
                match destino {
                    Some((id, l)) => {
                        let texto = egui::RichText::new(linea).monospace().size(12.0).color(c.acento).underline();
                        if ui.add(egui::Label::new(texto).sense(Sense::click())).on_hover_text(format!("abrir {id} en la linea {l}")).clicked() {
                            acciones.push(Accion::AbrirEditor { id, linea: Some(l) });
                        }
                    }
                    None => { ui.label(egui::RichText::new(linea).monospace().size(12.0).color(col)); }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("$").monospace().color(c.acento));
            let r = ui.add(egui::TextEdit::singleline(&mut self.texto_terminal).font(egui::TextStyle::Monospace)
                .hint_text("comando (ej. cargo build, ls -la, git status) · sin shell").desired_width(f32::INFINITY));
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && !self.texto_terminal.trim().is_empty() {
                acciones.push(Accion::LineaComando(std::mem::take(&mut self.texto_terminal)));
                r.request_focus();
            }
        });
        for a in acciones { self.despachar(a); }
    }

    /// F12: carriles con semaforo, como las herramientas de desarrollo.
    fn carriles(&self, ctx: &egui::Context, v: &Vista, c: &Colores) {
        if !self.estado.carriles { return; }
        egui::Area::new(egui::Id::unique("ultra-carpetas-carriles")).anchor(Align2::RIGHT_BOTTOM, [-12.0, -40.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.linea)).corner_radius(c.radio()).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(430.0);
                ui.label(egui::RichText::new("CARRILES // SEMAFORO  (F12)").color(c.acento).strong());
                for carril in [Carril::Disco, Carril::Acomodo, Carril::Vista, Carril::Geometria, Carril::Pintado, Carril::Iconos, Carril::Terminal,
                               Carril::Procesos, Carril::Editor] {
                    let p = presupuesto(carril);
                    let (u, m) = (self.medidas.ultimo[carril as usize], self.medidas.maximo[carril as usize]);
                    let foco = match luz(carril, u) { Luz::Verde => Color32::from_rgb(61, 255, 154), Luz::Amarillo => Color32::from_rgb(252, 238, 10), Luz::Rojo => Color32::from_rgb(255, 42, 79) };
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 5.0, foco);
                        ui.label(egui::RichText::new(format!("{:<10} {:>7} µs  máx {:>7}  verde ≤ {}", p.nombre, u, m, p.verde_us)).monospace().size(11.0).color(c.texto));
                    }).response.on_hover_text(p.que_consume);
                }
                let nodos = v.grafo.as_ref().map(|g| g.0.len()).unwrap_or(0);
                ui.label(egui::RichText::new(format!("nodos {nodos}/{MAX_NODOS} · desplegadas {}/{MAX_DESPLEGADAS} · entradas {}/{MAX_ENTRADAS} · iconos SVG {}",
                    self.estado.expandidas.len(), self.estado.entradas.len(), self.iconos.cantidad())).color(c.tenue).size(11.0));
                ui.label(egui::RichText::new(format!("procesos {}/{MAX_PROCESOS} · errores atrapados {}/{MAX_DIAGNOSTICOS} · lineas terminal {}",
                    self.estado.procesos.len(), self.estado.errores.total, self.estado.terminal.lineas.len())).color(c.tenue).size(11.0));
                let modo = match (self.gpu_ok, self.estado.ajustes.dibujo) { (true, Dibujo::Gpu) => "GPU retenido", (false, _) => "CPU (sin wgpu)", _ => "CPU" };
                let vertices = self.geometria.as_ref().map(|g| g.vertices.len()).unwrap_or(0);
                ui.label(egui::RichText::new(format!("grafo: {modo} · {vertices} vertices · subidas a la GPU {} en {} cuadros",
                    self.subidas.load(Ordering::Relaxed), self.cuadros)).color(c.tenue).size(11.0))
                    .on_hover_text("Cada subida es cuando el grafo cambio de verdad. Mover o hacer zoom solo manda 16 bytes de camara.");
            });
        });
    }
}

/// Los colores del tema ya convertidos para egui.
pub(crate) struct Colores { fondo: Color32, panel: Color32, superficie: Color32, linea: Color32, texto: Color32, tenue: Color32,
                            acento: Color32, sobre_acento: Color32, error: Color32, verde: Color32, aviso: Color32, esquinas: u8, brillo: bool }

impl Colores {
    fn de(t: &Tema) -> Colores {
        Colores { fondo: color(t.fondo), panel: color(t.panel), superficie: color(t.superficie), linea: color(t.linea), texto: color(t.texto),
                  tenue: color(t.tenue), acento: color(t.acento), sobre_acento: color(t.sobre_acento), error: color(t.error),
                  verde: color(t.tipos[5]), aviso: color(t.tipos[2]), esquinas: t.esquinas, brillo: t.brillo }
    }
    fn radio(&self) -> CornerRadius { CornerRadius::same(self.esquinas) }
}

fn marco(pintor: &egui::Painter, c: &Colores, rect: Rect, seleccionado: bool) {
    if c.brillo && seleccionado {
        for (k, alfa) in [(2.0, 0.35), (4.0, 0.18), (7.0, 0.08)] { pintor.rect_stroke(rect.expand(k), c.radio(), Stroke::new(2.0, c.acento.gamma_multiply(alfa)), StrokeKind::Outside); }
    }
    pintor.rect(rect, c.radio(), if seleccionado { c.acento.gamma_multiply(0.16) } else { c.superficie.gamma_multiply(0.6) },
                Stroke::new(1.0, if seleccionado { c.acento } else { c.linea }), StrokeKind::Inside);
}

fn icono_neon(pintor: &egui::Painter, rect: Rect, tipo: Tipo, col: Color32, etiqueta: &str) {
    let centro = rect.center_top() + Vec2::new(0.0, 30.0);
    let trazo = Stroke::new(2.0, col);
    if tipo == Tipo::Carpeta {
        let pts = vec![centro + Vec2::new(-26.0, -16.0), centro + Vec2::new(-8.0, -16.0), centro + Vec2::new(-3.0, -10.0), centro + Vec2::new(26.0, -10.0),
                       centro + Vec2::new(26.0, 18.0), centro + Vec2::new(-26.0, 18.0)];
        pintor.add(egui::Shape::convex_polygon(pts, col.gamma_multiply(0.18), trazo));
        pintor.line_segment([centro + Vec2::new(-26.0, -4.0), centro + Vec2::new(26.0, -4.0)], Stroke::new(1.0, col.gamma_multiply(0.6)));
    } else {
        let pts = vec![centro + Vec2::new(-18.0, -22.0), centro + Vec2::new(8.0, -22.0), centro + Vec2::new(18.0, -12.0), centro + Vec2::new(18.0, 22.0), centro + Vec2::new(-18.0, 22.0)];
        pintor.add(egui::Shape::convex_polygon(pts, col.gamma_multiply(0.12), trazo));
        pintor.line_segment([centro + Vec2::new(8.0, -22.0), centro + Vec2::new(8.0, -12.0)], trazo);
        pintor.line_segment([centro + Vec2::new(8.0, -12.0), centro + Vec2::new(18.0, -12.0)], trazo);
        if !etiqueta.is_empty() { pintor.text(centro + Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, etiqueta, FontId::monospace(10.0), col); }
        if tipo == Tipo::Enlace { pintor.text(centro + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, "enlace", FontId::monospace(8.0), col); }
    }
}

fn etiqueta_nombre(pintor: &egui::Painter, c: &Colores, rect: Rect, nombre: &str, seleccionado: bool) {
    let mut texto = nombre.to_string();
    if texto.chars().count() > 26 { texto = texto.chars().take(25).collect::<String>() + "…"; }
    let galeria = pintor.layout(texto, FontId::proportional(12.5), if seleccionado { c.texto } else { c.tenue }, rect.width() - 6.0);
    pintor.galley(rect.center_top() + Vec2::new(-galeria.size().x / 2.0, 58.0), galeria, c.texto);
}

/// Aplica el tema a egui (botones, menus, campos de texto).
fn estilo(ctx: &egui::Context, t: &Tema) {
    let c = Colores::de(t);
    let mut v = egui::Visuals::dark();
    v.panel_fill = c.fondo;
    v.window_fill = c.panel;
    v.extreme_bg_color = c.fondo;
    v.faint_bg_color = c.superficie;
    v.override_text_color = Some(c.texto);
    v.selection.bg_fill = c.acento.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, c.acento);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.linea);
    v.widgets.inactive.weak_bg_fill = c.panel;
    v.widgets.inactive.bg_fill = c.superficie;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.linea);
    v.widgets.hovered.weak_bg_fill = c.acento.gamma_multiply(0.15);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.acento);
    v.widgets.active.weak_bg_fill = c.acento.gamma_multiply(0.3);
    v.widgets.active.bg_stroke = Stroke::new(1.0, c.acento);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = c.radio();
    }
    v.window_corner_radius = c.radio();
    v.menu_corner_radius = c.radio();
    ctx.set_visuals(v);
}

fn fuentes(ctx: &egui::Context) {
    for ruta in ["/usr/share/fonts/ultra-neon/ChakraPetch-Medium.ttf", "/usr/share/fonts/ultra-neon/ChakraPetch-Regular.ttf"] {
        if let Ok(datos) = std::fs::read(ruta) {
            let mut f = egui::FontDefinitions::default();
            f.font_data.insert("chakra".into(), std::sync::Arc::new(egui::FontData::from_owned(datos)));
            f.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "chakra".into());
            ctx.set_fonts(f);
            break;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let inicio = Instant::now();
        self.tiempo_iconos.set(0);
        let ctx = ui.ctx().clone();
        self.cuadros += 1;
        self.recoger();
        self.recoger_proceso();
        self.recoger_vivos();
        self.teclado(&ctx);
        if self.estado.panel_ajustes && ctx.input(|i| i.key_pressed(Key::Escape)) { self.despachar(Accion::Comando(Comando::Ajustes)); }
        let tema = self.estado.ajustes.tema(self.estado.acento);
        if self.tema_aplicado != Some(tema) { estilo(&ctx, &tema); self.tema_aplicado = Some(tema); }
        let c = Colores::de(&tema);
        egui::Panel::top("barra").frame(egui::Frame::new().fill(c.panel).inner_margin(8.0).stroke(Stroke::new(1.0, c.linea))).show(ui, |ui| self.barra(ui, &c));
        let v = self.vista();
        let estado = format!("{} · {} · tema {}", v.estado, if v.modo == Modo::Grafo { "grafo" } else { "iconos" }, tema.nombre);
        egui::Panel::bottom("estado").frame(egui::Frame::new().fill(c.panel).inner_margin(6.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(estado).color(c.tenue));
                if let Some(e) = &self.error { ui.label(egui::RichText::new(format!("· {e}")).color(c.error)); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Solo ordena la vista: tus archivos no se mueven · F12 carriles").color(c.tenue).italics());
                });
            });
        });
        if self.estado.terminal.abierta {
            egui::Panel::bottom("terminal").resizable(true).default_size(220.0)
                .frame(egui::Frame::new().fill(c.fondo).inner_margin(8.0).stroke(Stroke::new(1.0, c.linea))).show(ui, |ui| self.terminal(ui, &c));
        }
        if self.estado.editor.is_some() {
            let mut acciones = Vec::new();
            let mut micros = 0;
            egui::Panel::right("editor").resizable(true).default_size(620.0).min_size(320.0)
                .frame(egui::Frame::new().fill(c.panel).inner_margin(8.0).stroke(Stroke::new(1.0, c.linea)))
                .show(ui, |ui| micros = self.editor.pintar(ui, &self.estado, &c, &mut acciones));
            self.medidas.anotar(Carril::Editor, micros);
            for a in acciones { self.despachar(a); }
        } else {
            self.editor.foco = false;
        }
        if v.arbol.is_some() {
            egui::Panel::left("explorador").resizable(true).default_size(250.0)
                .frame(egui::Frame::new().fill(c.panel).inner_margin(8.0).stroke(Stroke::new(1.0, c.linea))).show(ui, |ui| self.explorador(ui, &v, &c));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(c.fondo)).show(ui, |ui| {
            if v.modo == Modo::Grafo {
                let mut acciones = Vec::new();
                let lienzo = self.estado.lienzo;
                let retenido = self.retenido(&v, &tema);
                let t = &self.estado.terminal;
                let vivos = t.en_curso.as_deref().map(|titulo| pintar_grafo::Vivos { id: &t.carpeta, titulo, procesos: &self.estado.procesos });
                pintar_grafo::grafo(ui, &v, self.estado.camara, retenido, &self.iconos, &c, self.estado.ajustes.direccion, lienzo, vivos, &mut acciones);
                for a in acciones { self.despachar(a); }
            } else {
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.iconos_libres(ui, &v, &c));
            }
        });
        let v = self.vista();
        self.paleta(&ctx, &v, &c);
        self.ajustes(&ctx, &v, &c);
        self.carriles(&ctx, &v, &c);
        self.medidas.anotar(Carril::Iconos, self.tiempo_iconos.get());
        self.medidas.anotar(Carril::Pintado, inicio.elapsed().as_micros() as u64);
        if self.estado.carriles { ctx.request_repaint_after(Duration::from_millis(500)); }
    }
}

fn main() -> eframe::Result {
    let carpeta = std::env::args_os().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into())));
    let opciones = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title("ULTRA Carpetas").with_app_id("ultra-carpetas").with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("ultra-carpetas", opciones, Box::new(move |cc| Ok(Box::new(App::new(cc, carpeta)))))
}
