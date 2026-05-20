//! This modules contains the window for editing game genie codes for the current game

use crate::NesEmulatorData;

use eframe::egui;

/// The window for dumping ppu nametable data
pub struct Window {
    /// The string for a new game genie code
    code: String,
}

impl Window {
    pub fn new() -> Self {
        Self {
            code: Default::default(),
        }
    }
}

impl Window {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("GENIE_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Game genie")
                .with_inner_size([400.0, 300.0]),
            |ui, class| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    if let Some(cart) = c.mb.cartridge_mut() {
                        let mut delete = None;
                        for code in &cart.cartridge().volatile.genie {
                            ui.horizontal(|ui| {
                                ui.label(format!("Code: {}", code.code()));
                                if ui.button("Delete").clicked() {
                                    delete = Some(code.to_owned());
                                }
                            });
                        }
                        if let Some(code) = delete {
                            cart.cartridge_volatile_mut().remove_code(&code);
                        }
                        if cart.cartridge().volatile.genie.len() > 0 {
                            ui.separator();
                        }
                        ui.label("Enter game genie code");
                        ui.text_edit_singleline(&mut self.code);
                        if let Ok(v) = crate::genie::GameGenieCode::from_str(&self.code) {
                            if ui.button("Add game genie code").clicked() {
                                cart.cartridge_volatile_mut().genie.push(v);
                            }
                        }
                    }
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            },
        );
    }
}
