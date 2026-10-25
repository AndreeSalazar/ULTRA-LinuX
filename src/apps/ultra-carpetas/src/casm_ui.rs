// SPDX-License-Identifier: GPL-2.0-only
//! Panel Taller CASM del SUPER GRAFO: como se genera el .hex (la cadena y donde corta),
//! las katanas de CASM con el sitio exacto y el caso que las prueba, y las compilaciones.
//! Todo sale de los archivos de CASM y de build/CASM: nada esta escrito a mano aqui.

use crate::{App, Colores};
use carpetas_nucleo::casm::{Compilacion, Etapa, Katana};
use carpetas_nucleo::{lanzador, Accion};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

const PASOS: [&str; 7] = ["fuente .casm", "frontend", "IR", "codificador x86-64", "HEX", "verificador", "INICIO.X86"];

/// En que paso de la cadena corta cada etapa.
fn paso_de(e: Etapa) -> usize { match e { Etapa::Anfitrion => 0, Etapa::Fuente => 1, Etapa::Codificador => 3, Etapa::Verificador => 5 } }

impl App {
    pub(crate) fn panel_casm(&mut self, ui: &mut egui::Ui, c: &Colores) {
        let mut acciones = Vec::new();
        ui.add_space(6.0);
        ui.label(RichText::new("TALLER CASM // SAMURAI").color(c.error).strong());
        ui.label(RichText::new("CASM prepara y exporta .hex exacto para x86-64. Lo que no sabe hacer seguro, lo corta: nada se emite ni se publica.")
            .color(c.tenue).size(11.0));
        let Some(t) = &self.estado.casm.taller else {
            ui.add_space(6.0);
            ui.label(RichText::new("No hay CASM en esta carpeta. Se busca compilar.sh, verify.py y src/ en la carpeta abierta o en CASM/.").color(c.tenue).size(12.0));
            return;
        };
        let ultima = t.compilaciones.first();
        ui.add_space(6.0);

        // ---- La cadena: como se genera el .hex, y donde corto la ultima vez ----
        ui.label(RichText::new("COMO SE GENERA EL .HEX").color(c.acento).strong().size(12.0));
        let corte = ultima.and_then(|u| u.corte.as_ref()).and_then(|(r, _)| t.katanas.iter().find(|k| &k.regla == r)).map(|k| (paso_de(k.etapa), k.regla.clone()));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
            for (i, paso) in PASOS.iter().enumerate() {
                let (fondo, letra) = match (ultima, &corte) {
                    (Some(u), _) if u.verde => (c.verde.gamma_multiply(0.25), c.texto),
                    (Some(_), Some((p, _))) if i < *p => (c.verde.gamma_multiply(0.25), c.texto),
                    (Some(_), Some((p, _))) if i == *p => (c.error, Color32::WHITE),
                    _ => (c.superficie, c.tenue),
                };
                let b = egui::Button::new(RichText::new(*paso).size(11.0).color(letra)).fill(fondo).stroke(Stroke::new(1.0, c.linea));
                let r = ui.add(b);
                if let Some((p, regla)) = &corte { if i == *p { r.on_hover_text(format!("Aqui corto la ultima compilacion: {regla}")); } }
                if i + 1 < PASOS.len() { ui.label(RichText::new(">").color(c.tenue)); }
            }
        });
        match ultima {
            Some(u) => resumen(ui, c, u, &self.carpeta, &mut acciones),
            None => { ui.label(RichText::new("Aun no hay compilaciones (build/CASM/run-*).").color(c.tenue).size(11.5)); }
        }
        // Compilar: el mismo play que el nodo (bash compilar.sh, sin shell).
        let script = if t.carpeta.is_empty() { String::from("compilar.sh") } else { format!("{}/compilar.sh", t.carpeta) };
        ui.horizontal(|ui| {
            if lanzador(&self.estado, &script).is_some() {
                if ui.button("Compilar el ejemplo (compilar.sh)").on_hover_text("bash compilar.sh: examples/hello.casm. Cada .casm tiene su play en el grafo y el explorador").clicked() {
                    acciones.push(Accion::Ejecutar(script.clone()));
                }
            } else if ui.button("Mostrar CASM para compilar").on_hover_text("Despliega la carpeta de CASM: su play compila").clicked() {
                acciones.push(Accion::Revelar { id: script.clone(), editor: false });
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // ---- Las katanas ----
            let con_caso = t.katanas.iter().filter(|k| !k.casos.is_empty()).count();
            ui.label(RichText::new(format!("KATANAS · {} reglas que cortan · {con_caso} con caso de prueba aislado", t.katanas.len())).color(c.acento).strong().size(12.0));
            ui.label(RichText::new("Leidas de las fuentes de CASM: la regla, donde corta (clic: editor) y el caso de test.py que lo demuestra. Sin caso = aun no esta probada aislada.")
                .color(c.tenue).size(11.0));
            for etapa in [Etapa::Anfitrion, Etapa::Fuente, Etapa::Codificador, Etapa::Verificador] {
                let de: Vec<&Katana> = t.katanas.iter().filter(|k| k.etapa == etapa).collect();
                if de.is_empty() { continue; }
                egui::CollapsingHeader::new(RichText::new(format!("{} · {}", etapa.nombre(), de.len())).size(12.5).color(c.texto))
                    .id_salt(("katanas", etapa.nombre())).default_open(etapa == Etapa::Fuente).show(ui, |ui| {
                    for k in de { katana(ui, c, k, &mut acciones); }
                });
            }
            ui.add_space(8.0);
            ui.separator();
            // ---- Compilaciones ----
            ui.label(RichText::new(format!("COMPILACIONES · {}", t.compilaciones.len())).color(c.acento).strong().size(12.0));
            for comp in t.compilaciones.iter().take(15) { fila(ui, c, comp); }
            if !t.toolchain.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new("COMPILADOR DE CASM").color(c.acento).strong().size(12.0));
                for l in t.toolchain.lines().take(2) { ui.label(RichText::new(l).monospace().size(10.5).color(c.tenue)); }
            }
        });
        for a in acciones { self.despachar(a); }
    }
}

fn corto(sha: &str) -> &str { &sha[..sha.len().min(12)] }

fn nombre_origen(o: &str) -> &str { o.rsplit('/').next().unwrap_or(o) }

fn resumen(ui: &mut egui::Ui, c: &Colores, u: &Compilacion, raiz: &std::path::Path, acciones: &mut Vec<Accion>) {
    ui.horizontal_wrapped(|ui| {
        let (txt, col) = if u.verde { (format!("VERDE · {} B de cuerpo · paquete {} B · sha {}…", u.cuerpo, u.paquete, corto(&u.sha_paquete)), c.verde) }
                         else { (u.corte.as_ref().map(|(_, l)| l.clone()).unwrap_or_else(|| String::from("ROJO: sin paquete publicado")), c.error) };
        ui.label(RichText::new(txt).monospace().size(11.0).color(col));
    });
    if !u.origen.is_empty() {
        let rel = std::path::Path::new(&u.origen).strip_prefix(raiz).ok().and_then(|p| p.to_str()).map(str::to_string);
        let r = ui.add(egui::Label::new(RichText::new(format!("{} · {}", u.run, nombre_origen(&u.origen))).monospace().size(10.5).color(c.tenue)).sense(egui::Sense::click()));
        if let Some(rel) = rel { if r.on_hover_text("clic: abrir la fuente en el editor").clicked() { acciones.push(Accion::Revelar { id: rel, editor: true }); } }
    }
    for a in &u.avisos { ui.label(RichText::new(a).monospace().size(10.5).color(c.aviso)); }
}

fn katana(ui: &mut egui::Ui, c: &Colores, k: &Katana, acciones: &mut Vec<Accion>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&k.regla).monospace().size(11.0).color(Color32::WHITE).background_color(c.error));
        if !k.motivo.is_empty() { ui.label(RichText::new(&k.motivo).size(11.0).color(c.texto)); }
    });
    ui.horizontal_wrapped(|ui| {
        for (archivo, linea) in &k.sitios {
            let b = ui.add(egui::Label::new(RichText::new(format!("{}:{linea}", archivo.rsplit('/').next().unwrap_or(archivo))).monospace().size(10.5).color(c.acento))
                .sense(egui::Sense::click()));
            if b.on_hover_text(format!("{archivo}:{linea} · clic: abrir en esa linea")).clicked() { acciones.push(Accion::IrALinea { id: archivo.clone(), linea: *linea }); }
        }
        if k.casos.is_empty() { ui.label(RichText::new("sin caso aislado").size(10.5).color(c.aviso)); }
        else { ui.label(RichText::new(format!("lo demuestra: {}", k.casos.join(", "))).size(10.5).color(c.verde)); }
    });
    ui.add_space(3.0);
}

fn fila(ui: &mut egui::Ui, c: &Colores, u: &Compilacion) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
        ui.painter().circle_filled(r.center(), 4.0, if u.verde { c.verde } else { c.error });
        let fecha = u.run.strip_prefix("run-").and_then(|x| x.get(..15)).unwrap_or(&u.run);
        let detalle = if u.verde { format!("{} B · {}…", u.paquete, corto(&u.sha_paquete)) }
                      else { u.corte.as_ref().map(|(r, _)| r.clone()).unwrap_or_else(|| String::from("sin paquete")) };
        let r = ui.label(RichText::new(format!("{fecha}  {:<16} {detalle}", nombre_origen(&u.origen))).monospace().size(10.5).color(c.texto));
        let mut tip = u.run.clone();
        if let Some((_, l)) = &u.corte { tip.push('\n'); tip.push_str(l); }
        for a in &u.avisos { tip.push('\n'); tip.push_str(a); }
        r.on_hover_text(tip);
    });
}
