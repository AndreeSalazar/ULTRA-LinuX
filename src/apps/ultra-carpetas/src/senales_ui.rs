// SPDX-License-Identifier: GPL-2.0-only
//! «Lo que senalan» en el panel Combinar: cada senal del grupo con lo que ensena su ficha
//! (regla y control valido), en el orden para arreglar, y por donde le llega el fallo a
//! quien no falla por si mismo. Clic: el editor se abre en esa linea.

use crate::Colores;
use carpetas_nucleo::museo::{ficha, Eje};
use carpetas_nucleo::senales::Senales;
use carpetas_nucleo::{Accion, Rol};
use eframe::egui::{self, Color32, RichText, Sense};

/// Cuantas senales se ensenan en la lista (el resto, con «Siguiente»).
const VISTAS: usize = 40;

fn corto(ruta: &str, max: usize) -> String {
    let n = ruta.rsplit('/').next().unwrap_or(ruta);
    if n.chars().count() <= max { n.to_string() } else { format!("{}…", n.chars().take(max - 1).collect::<String>()) }
}

pub fn seccion(ui: &mut egui::Ui, c: &Colores, s: &Senales, acciones: &mut Vec<Accion>) {
    if s.total == 0 && s.por_que.is_empty() { return; }
    ui.add_space(6.0);
    let seguridad = s.lista.iter().filter(|x| x.eje == Eje::Seguridad).count();
    let titulo = format!("LO QUE SENALAN · {} ({} de Seguridad o del compilador)", s.total, seguridad);
    egui::CollapsingHeader::new(RichText::new(titulo).color(c.error).strong().size(12.0)).id_salt("senales-grupo").default_open(true).show(ui, |ui| {
        // Por donde le llega el fallo a quien no falla por si mismo.
        for (id, (rol, cadena)) in &s.por_que {
            let (txt, col) = match rol {
                Rol::Causa => ("causa probable: falla por algo escrito en el", c.error),
                Rol::Falla => ("falla, y ademas usa algo que falla", c.error),
                Rol::Arrastra => ("no falla, pero se puede romper con", c.aviso),
                Rol::Base => ("lo usa el aislado", c.tenue),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(corto(id, 30)).monospace().color(Color32::WHITE).size(11.5));
                let causa = cadena.last().map(|x| corto(&x.hasta, 30)).unwrap_or_default();
                ui.label(RichText::new(if causa.is_empty() { txt.to_string() } else { format!("{txt} {causa}") }).color(col).size(11.0));
            });
            if !cadena.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("le llega por").color(c.tenue).size(10.5));
                    for x in cadena {
                        let donde = match &x.motivo { Some(m) => format!("{}:{}", corto(&x.desde, 26), m.linea), None => corto(&x.desde, 26) };
                        let r = ui.add(egui::Label::new(RichText::new(donde).monospace().color(c.aviso).size(11.0)).sense(Sense::click()))
                            .on_hover_text(format!("{} incluye {}\nclic: abrir el editor en esa linea", x.desde, x.hasta));
                        if r.clicked() {
                            acciones.push(match &x.motivo { Some(m) => Accion::IrALinea { id: x.desde.clone(), linea: m.linea }, None => Accion::Revelar { id: x.desde.clone(), editor: true } });
                        }
                        ui.label(RichText::new("›").color(c.tenue).size(11.0));
                    }
                    ui.label(RichText::new(corto(&cadena.last().map(|x| x.hasta.clone()).unwrap_or_default(), 26)).monospace().color(c.error).size(11.0));
                });
            }
        }
        // Las senales, en el orden para arreglar.
        for (i, x) in s.lista.iter().take(VISTAS).enumerate() {
            let (chip, col) = match (x.ficha, x.grave) {
                (None, true) => ("error".to_string(), c.error),
                (None, false) => ("aviso".to_string(), c.aviso),
                (Some(f), _) => (f.to_string(), crate::color_eje(c, x.eje)),
            };
            let f = x.ficha.and_then(ficha);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(chip).color(col).strong().size(11.0));
                let r = ui.add(egui::Label::new(RichText::new(format!("{}:{}", corto(&x.archivo, 30), x.linea)).monospace().color(Color32::WHITE).size(11.5)).sense(Sense::click()))
                    .on_hover_text(format!("{}\nclic: abrir el editor en esa linea", x.archivo));
                if r.clicked() { acciones.push(Accion::IrALinea { id: x.archivo.clone(), linea: x.linea.max(1) }); }
                if let Some(f) = f { ui.label(RichText::new(f.titulo).color(c.texto).size(11.0)); }
                if x.ficha.is_some() { ui.label(RichText::new(if x.exacta { "exacto" } else { "revisar" }).color(c.tenue).size(10.5)); }
            });
            ui.add(egui::Label::new(RichText::new(&x.texto).color(c.tenue).size(11.0)).wrap());
            // Lo que ensena la ficha: la regla y el control valido, a un clic.
            if let Some(f) = f {
                egui::CollapsingHeader::new(RichText::new("que ensena y como se arregla").color(c.acento).size(10.5)).id_salt(("senal", i)).show(ui, |ui| {
                    ui.label(RichText::new(f.regla).color(c.texto).size(11.0));
                    ui.label(RichText::new("Control valido").color(c.verde).size(10.5));
                    ui.add(egui::Label::new(RichText::new(f.control).monospace().color(Color32::WHITE).size(11.0)).wrap());
                    ui.label(RichText::new(format!("Limite: {}", f.limite)).color(c.tenue).size(10.5));
                    ui.hyperlink_to(RichText::new(f.referencia).size(10.5), f.url);
                });
            }
        }
        if s.total as usize > VISTAS { ui.label(RichText::new(format!("+{} mas: «Siguiente senal» las recorre todas", s.total as usize - VISTAS)).color(c.tenue).size(11.0)); }
    });
}
