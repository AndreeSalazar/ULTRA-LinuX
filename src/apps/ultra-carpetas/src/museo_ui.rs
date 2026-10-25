// SPDX-License-Identifier: GPL-2.0-only
//! Panel Museo del SUPER GRAFO: el fallo conectado (causa probable y lo que arrastra) y las
//! fichas del museo que aparecen en el codigo, cada una con su regla, su control valido,
//! su limite y su referencia: para entender por que, no solo donde.

use crate::{App, Colores};
use carpetas_nucleo::analisis::Modo as ModoConexion;
use carpetas_nucleo::museo::{eje, ficha, Eje, Fuerza};
use carpetas_nucleo::{Accion, Comando};
use eframe::egui::{self, Color32, RichText, Stroke};

const MAX_SITIOS: usize = 60;

impl App {
    pub(crate) fn panel_museo(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        let e = &self.estado;
        let an = &e.analisis;
        ui.add_space(6.0);
        ui.label(RichText::new("SUPER GRAFO // MUSEO").color(c.error).strong());
        let estado = if !e.indice.listo { "indexando el proyecto…".to_string() }
                     else if !an.listo { "leyendo C, C++, ASM, Rust y Python (carril Museo)…".to_string() }
                     else { format!("{} archivos leidos · {} dependencias · {} sospechas en {} archivos (+{} a proposito) · {:.1} s{}",
                                    crate::paneles::miles(an.archivos), crate::paneles::miles(an.dep.aristas), an.total, an.sospechas.len(), an.total_estudio,
                                    an.micros as f64 / 1e6, if an.truncado { " · cortado por el presupuesto" } else { "" }) };
        ui.label(RichText::new(estado).color(c.tenue).size(11.0));
        ui.add_space(6.0);

        // ---- Fallo conectado ----
        let k = &e.conectado;
        ui.horizontal(|ui| {
            let aislado = e.aislar.is_some();
            let texto = if aislado { "Quitar el aislado [A]" } else if e.seleccion.is_empty() { "Aislar los fallos [A]" } else { "Aislar lo seleccionado [A]" };
            let b = egui::Button::new(RichText::new(texto).color(if aislado { c.fondo } else { Color32::WHITE }))
                .fill(if aislado { Color32::WHITE } else { c.error }).stroke(Stroke::new(1.0, Color32::WHITE));
            if ui.add(b).on_hover_text("Apaga todo lo que no esta conectado con el fallo (o con lo seleccionado): asi se ve solo lo que importa").clicked() {
                acciones.push(Accion::Comando(Comando::Aislar));
            }
        });
        ui.add_space(4.0);
        match &k.modo {
            ModoConexion::Ninguno => { ui.label(RichText::new("Nada falla: sin errores atrapados ni sospechas del museo.").color(c.verde).size(12.0)); }
            ModoConexion::Fallos => {
                let n_arrastra = k.conexion.roles.values().filter(|r| **r == carpetas_nucleo::Rol::Arrastra).count();
                ui.label(RichText::new(format!("FALLO CONECTADO · {} archivos fallan · {} se pueden romper con ellos{}", k.fallos, n_arrastra,
                    if k.conexion.truncada { " (recorte)" } else { "" })).color(c.texto).strong().size(12.0));
                ui.label(RichText::new("Causa probable: falla y nada de lo que usa falla. Lo que la usa se pinta en rojo suave y se conecta con cables rojos.")
                    .color(c.tenue).size(11.0));
                for causa in k.conexion.causas.iter().take(12) {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 5.5, c.error);
                        ui.painter().circle_filled(r.center(), 2.2, Color32::WHITE);
                        let b = ui.add(egui::Label::new(RichText::new(&causa.ruta).monospace().size(11.5).color(c.texto)).sense(egui::Sense::click()));
                        if b.clicked() { acciones.push(Accion::Revelar { id: causa.ruta.clone(), editor: false }); }
                        if b.secondary_clicked() { acciones.push(Accion::Revelar { id: causa.ruta.clone(), editor: true }); }
                        b.on_hover_text("clic: mostrar en el grafo · clic derecho: abrir en el editor");
                        let detalle = match (causa.explica, causa.arrastra) {
                            (0, 0) => String::from("falla por su cuenta"),
                            (0, a) => format!("lo usan {a}"),
                            (x, a) => format!("explica {x} · lo usan {a}"),
                        };
                        ui.label(RichText::new(detalle).color(c.tenue).size(11.0));
                    });
                }
                if k.conexion.causas.len() > 12 { ui.label(RichText::new(format!("+{} causas mas", k.conexion.causas.len() - 12)).color(c.tenue).size(11.0)); }
            }
            ModoConexion::Alrededor(id) => {
                let usa = k.conexion.roles.values().filter(|r| **r == carpetas_nucleo::Rol::Base).count();
                let lo_usan = k.conexion.roles.values().filter(|r| **r == carpetas_nucleo::Rol::Arrastra).count();
                ui.label(RichText::new(format!("AISLADO · {id}")).color(c.texto).strong().size(12.0));
                ui.label(RichText::new(format!("usa {usa} archivos (blanco) · lo usan {lo_usan} (rojo suave){}", if k.conexion.truncada { " · recorte" } else { "" }))
                    .color(c.tenue).size(11.0));
            }
        }
        ui.add_space(8.0);
        ui.separator();

        // ---- Museo en el codigo ----
        ui.label(RichText::new("MUSEO EN TU CODIGO").color(c.acento).strong());
        ui.label(RichText::new("Analisis lexico: no compila ni ejecuta. «exacto» = lo escrito es el patron de la ficha; «revisar» = depende de tipos o valores.")
            .color(c.tenue).size(11.0));
        // ---- El triangulo: cuanto hay de cada eje; clic = ver solo ese eje ----
        let filtro = e.filtro_eje;
        let mut cuentas = [0usize; 3];
        for s in an.sospechas.values().flatten() { cuentas[eje(s.ficha).indice()] += 1; }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 28.0), egui::Sense::hover());
            crate::pintar_grafo::triangulo(ui.painter(), c, r.center(), 22.0, [cuentas[0] as u32, cuentas[1] as u32, cuentas[2] as u32]);
            for x in Eje::TODOS {
                let activo = filtro == Some(x);
                let col = crate::color_eje(c, x);
                let texto = RichText::new(format!("{} {}", x.nombre(), cuentas[x.indice()])).size(11.5).color(if activo { c.fondo } else { col });
                let b = egui::Button::new(texto).fill(if activo { col } else { c.superficie }).stroke(Stroke::new(1.0, col.gamma_multiply(0.7)));
                let ayuda = match x {
                    Eje::Seguridad => "Una regla: la llave cierra o no. Lo unico que cuenta como fallo y se conecta en rojo.",
                    Eje::Rendimiento => "Un presupuesto que se mide: hacia donde va el motor. Guia para revisar, no un fallo.",
                    Eje::Elegancia => "Lo que deja leer y revisar el codigo. Guia para revisar, no un fallo.",
                };
                if ui.add(b).on_hover_text(format!("{ayuda}\nClic: ver solo {} (en el grafo, el explorador y la Ciudad).", x.nombre())).clicked() {
                    acciones.push(Accion::FiltroEje(if activo { None } else { Some(x) }));
                }
            }
        });
        let por_ficha: PorFicha = an.por_ficha().into_iter().filter(|(id, _)| filtro.is_none_or(|f| eje(id) == f)).collect();
        if an.listo && por_ficha.is_empty() { ui.label(RichText::new("Ningun patron de las fichas en lo leido.").color(c.verde).size(12.0)); }
        let estudio = an.estudio_por_ficha();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Agrupadas por eje: primero lo que es fallo.
            for x in Eje::TODOS {
                let de: PorFicha = por_ficha.iter().filter(|(id, _)| eje(id) == x).map(|(k, v)| (*k, v.clone())).collect();
                if de.is_empty() { continue; }
                ui.add_space(4.0);
                ui.label(RichText::new(x.nombre().to_uppercase()).color(crate::color_eje(c, x)).strong().size(12.0));
                fichas(ui, c, &de, "museo", &mut acciones);
            }
            if !estudio.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new(format!("CASOS DE ESTUDIO A PROPOSITO · {}", an.total_estudio)).color(c.acento).strong());
                ui.label(RichText::new(format!("Carpetas con {}: minas y casos del museo. Se ensenan aqui, pero no cuentan como fallos ni se conectan.",
                    carpetas_nucleo::analisis::MARCA_ESTUDIO)).color(c.tenue).size(11.0));
                fichas(ui, c, &estudio, "estudio", &mut acciones);
            }
        });
        for a in acciones { self.despachar(a); }
    }
}

type PorFicha<'a> = std::collections::BTreeMap<&'static str, Vec<(&'a String, &'a carpetas_nucleo::museo::Sospecha)>>;

/// Cada ficha plegable: regla, control valido, limite, referencia y los sitios (clic: editor).
fn fichas(ui: &mut egui::Ui, c: &Colores, por_ficha: &PorFicha, sal: &str, acciones: &mut Vec<Accion>) {
    for (id, sitios) in por_ficha {
        let Some(f) = ficha(id) else { continue };
        let exactas = sitios.iter().filter(|(_, s)| s.fuerza == Fuerza::Exacto).count();
        let col = match eje(id) { Eje::Seguridad => if exactas > 0 { c.error } else { c.aviso }, x => crate::color_eje(c, x) };
        let titulo = RichText::new(format!("{id} · {} · {}", f.titulo, sitios.len())).color(col).size(12.5);
        egui::CollapsingHeader::new(titulo).id_salt((sal, *id)).default_open(por_ficha.len() <= 3).show(ui, |ui| {
            ui.label(RichText::new(format!("{} · {} · eje {}: {}", f.lenguaje, f.clase, eje(id).nombre(), f.motivo_eje)).color(c.tenue).size(10.5));
            ui.label(RichText::new(f.regla).color(c.texto).size(11.5));
            ui.label(RichText::new("Control valido").color(c.verde).size(11.0).strong());
            ui.label(RichText::new(f.control).monospace().color(c.texto).size(11.0));
            ui.label(RichText::new(format!("Limite: {}", f.limite)).color(c.tenue).size(11.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Referencia:").color(c.tenue).size(11.0));
                ui.hyperlink_to(RichText::new(f.referencia).size(11.0), f.url);
            });
            ui.label(RichText::new(format!("Ficha: {}", f.pagina)).color(c.tenue).size(10.5));
            ui.add_space(4.0);
            for (ruta, s) in sitios.iter().take(MAX_SITIOS) {
                let marca = if s.fuerza == Fuerza::Exacto { RichText::new("exacto").color(Color32::WHITE).background_color(c.error) }
                            else { RichText::new("revisar").color(c.fondo).background_color(c.aviso) };
                ui.horizontal_wrapped(|ui| {
                    ui.label(marca.size(10.0));
                    let b = ui.add(egui::Label::new(RichText::new(format!("{ruta}:{}", s.linea)).monospace().size(11.0).color(c.acento)).sense(egui::Sense::click()));
                    if b.clicked() { acciones.push(Accion::IrALinea { id: (*ruta).clone(), linea: s.linea }); }
                    b.on_hover_text("abrir en el editor en esa linea");
                });
                ui.label(RichText::new(&s.visto).monospace().size(11.0).color(c.texto));
                ui.label(RichText::new(&s.motivo).size(11.0).color(c.tenue));
                ui.add_space(3.0);
            }
            if sitios.len() > MAX_SITIOS { ui.label(RichText::new(format!("+{} sitios mas", sitios.len() - MAX_SITIOS)).color(c.tenue).size(11.0)); }
        });
    }
}
