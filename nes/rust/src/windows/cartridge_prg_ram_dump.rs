//! This module is for the window that dumps cartridge program ram data.

use crate::NesEmulatorData;

use eframe::egui;

/// The window for dumping cartridge program data
pub struct CartridgeMemoryDumpWindow {}

impl CartridgeMemoryDumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        CartridgeMemoryDumpWindow {}
    }
}

impl CartridgeMemoryDumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("PRG_RAM_DUMP_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("PRG RAM DUMPER")
                .with_inner_size([400.0, 300.0]),
            |ui, class| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.label("Cartridge Ram Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        #[cfg(feature = "debugger")]
                        {
                            if let Some(cart) = c.mb.cartridge() {
                                for (i, chunk) in cart
                                    .cartridge()
                                    .volatile
                                    .prg_ram
                                    .chunks_exact(8)
                                    .enumerate()
                                {
                                    ui.label(format!(
                                        "{:04X}: {:02X} {:02X} {:02X} {:02X}\t{:02X} {:02X} {:02X} {:02X}",
                                        i * 8,
                                        chunk[0],
                                        chunk[1],
                                        chunk[2],
                                        chunk[3],
                                        chunk[4],
                                        chunk[5],
                                        chunk[6],
                                        chunk[7],
                                    ));
                                }
                            }
                        }
                    });
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            });
    }
}
