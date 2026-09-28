//! The module for dumping all of the ppu address space

use crate::NesEmulatorData;

use eframe::egui;

/// The window for dumping cpu data
pub struct PpuMemoryDumpWindow {}

impl PpuMemoryDumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        PpuMemoryDumpWindow {}
    }
}

impl PpuMemoryDumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("PPU_MEMORY_DUMP_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("PPU memory")
                .with_inner_size([400.0, 300.0]),
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("PPU Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        #[cfg(feature = "debugger")]
                        {
                            for i in (0..=0x3FFF).step_by(8) {
                                let d: [u8; 8] = [
                                    c.mb.ppu_peek(i),
                                    c.mb.ppu_peek(i + 1),
                                    c.mb.ppu_peek(i + 2),
                                    c.mb.ppu_peek(i + 3),
                                    c.mb.ppu_peek(i + 4),
                                    c.mb.ppu_peek(i + 5),
                                    c.mb.ppu_peek(i + 6),
                                    c.mb.ppu_peek(i + 7),
                                ];
                                let a: [String; 8] = d.map(|d| format!("{:02X}", d));
                                let b = String::from_utf8_lossy(&d);
                                let display = format!(
                                    "{:04X}: {} {} {} {}\t{} {} {} {}\t{}",
                                    i, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], b,
                                );
                                ui.label(
                                    egui::RichText::new(display)
                                        .font(egui::FontId::monospace(12.0)),
                                );
                            }
                        }
                    });
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            },
        );
    }
}
