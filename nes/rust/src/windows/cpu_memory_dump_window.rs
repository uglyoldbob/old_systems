//! The module for dumping all of the cpu address space

use crate::NesEmulatorData;

use eframe::egui;

/// The window for dumping cpu data
pub struct CpuMemoryDumpWindow {}

impl CpuMemoryDumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        CpuMemoryDumpWindow {}
    }
}

impl CpuMemoryDumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("CPU_MEMORY_DUMP_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Cpu memory dump")
                .with_inner_size([400.0, 300.0]),
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("CPU Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        #[cfg(feature = "debugger")]
                        {
                            for i in (0..=0xFFFF).step_by(8) {
                                let a1 = if let Some(a) = c.mb.memory_dump(i, &c.cpu_peripherals) {
                                    format!("{:02X}", a)
                                } else {
                                    "**".to_string()
                                };
                                let a2 =
                                    if let Some(a) = c.mb.memory_dump(i + 1, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a3 =
                                    if let Some(a) = c.mb.memory_dump(i + 2, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a4 =
                                    if let Some(a) = c.mb.memory_dump(i + 3, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a5 =
                                    if let Some(a) = c.mb.memory_dump(i + 4, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a6 =
                                    if let Some(a) = c.mb.memory_dump(i + 5, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a7 =
                                    if let Some(a) = c.mb.memory_dump(i + 6, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                let a8 =
                                    if let Some(a) = c.mb.memory_dump(i + 7, &c.cpu_peripherals) {
                                        format!("{:02X}", a)
                                    } else {
                                        "**".to_string()
                                    };
                                ui.label(format!(
                                    "{:04X}: {} {} {} {}\t{} {} {} {}",
                                    i, a1, a2, a3, a4, a5, a6, a7, a8,
                                ));
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
