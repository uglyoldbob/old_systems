//! The module for the main debug window

use crate::NesEmulatorData;

use eframe::egui;

/// The structure for a debug window of the emulator.
pub struct DebugNesWindow {
    /// The string for a new breakpoint, in hexadecimal characters.
    breakpoint: String,
}

impl DebugNesWindow {
    /// Create a new Debug window.
    pub fn new() -> Self {
        Self {
            breakpoint: "".to_string(),
        }
    }
}

impl DebugNesWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("DEBUG_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Debug")
                .with_inner_size([400.0, 300.0]),
            |ui, class| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.label("Debug window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        #[cfg(feature = "debugger")]
                        {
                            if c.paused {
                                if ui.button("Unpause").clicked() {
                                    c.paused = false;
                                    c.single_step = false;
                                }
                                if ui.button("Single step").clicked() {
                                    c.single_step = true;
                                    c.paused = false;
                                }
                                if ui.button("Advance frame").clicked() {
                                    c.wait_for_frame_end = true;
                                    c.paused = false;
                                }
                            } else if ui.button("Pause").clicked() {
                                c.single_step = true;
                                c.paused = true;
                            }
                            if ui.button("Reset").clicked() {
                                c.reset();
                            }
                            if ui.button("Power cycle").clicked() {
                                c.power_cycle();
                            }
                            if let Some(cart) = c.mb.cartridge() {
                                ui.label(format!("ROM format: {:?}", cart.rom_format));
                            }
                            ui.horizontal(|ui| {
                                ui.label(format!("Address: 0x{:x}", c.cpu.debugger.pc));
                                if let Some(t) = c.cpu.disassemble() {
                                    ui.label(t);
                                }
                            });
                            ui.label(format!(
                                "A: {:x}, X: {:x}, Y: {:x}, P: {:x}, SP: {:x}",
                                c.cpu.debugger.a,
                                c.cpu.debugger.x,
                                c.cpu.debugger.y,
                                c.cpu.debugger.p,
                                c.cpu.debugger.s,
                            ));
                            ui.label(format!(
                                "Frame number {}",
                                c.cpu_peripherals.ppu_frame_number()
                            ));
                            ui.label("Breakpoints");
                            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                                let mut found = false;
                                let mut delete = None;
                                for (i, b) in c.cpu.breakpoints.iter().enumerate() {
                                    found = true;
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Breakpoint at {:X}", b));
                                        if ui.button("Delete").clicked() {
                                            delete = Some(i);
                                        }
                                    });
                                }
                                if let Some(i) = delete {
                                    c.cpu.breakpoints.remove(i);
                                }
                                if !found {
                                    ui.label("No breakpoints");
                                }
                            });
                            ui.text_edit_singleline(&mut self.breakpoint);
                            if let Ok(v) = u16::from_str_radix(&self.breakpoint, 16) {
                                if ui.button("Create breakpoint").clicked() {
                                    c.cpu.breakpoints.push(v);
                                }
                            }
                            ui.label("Cartridge registers:");
                            if let Some(c) = c.mb.cartridge() {
                                for (n, v) in c.cartridge_registers() {
                                    ui.label(format!("{}: {:x}", n, v));
                                }
                                ui.label(format!(
                                    "Chr memory size: {:X}",
                                    c.cartridge().nonvolatile.chr_rom.len()
                                ));
                                ui.label(format!(
                                    "Prg rom size: {:X}",
                                    c.cartridge().nonvolatile.prg_rom.len()
                                ));
                                ui.label(format!(
                                    "Prg ram size: {:X}",
                                    c.cartridge().volatile.prg_ram.len()
                                ));
                            }
                            ui.label(format!(
                                "X,Y = {},{} @ {:X}",
                                c.cpu_peripherals.ppu.column(),
                                c.cpu_peripherals.ppu.row(),
                                c.cpu_peripherals.ppu.vram_address()
                            ));
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
