//! The module for the main debug window

use crate::NesEmulatorData;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The structure for a debug window of the emulator.
pub struct DebugNesWindow {
    /// The string for a new breakpoint, in hexadecimal characters.
    breakpoint: String,
}

#[cfg(feature = "egui-multiwin")]
impl DebugNesWindow {
    /// Create a new request for a Debug window.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DebugNesWindow {
                breakpoint: "".to_string(),
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 480.0,
                })
                .with_title("UglyOldBob NES Debug"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for DebugNesWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<NesEmulatorData> {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("Debug window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
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
                            c.cartridge().chr_rom.len()
                        ));
                        ui.label(format!("Prg rom size: {:X}", c.cartridge().prg_rom.len()));
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
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
