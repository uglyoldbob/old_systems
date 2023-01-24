#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod apu;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod emulator_data;
pub mod motherboard;
pub mod ppu;
pub mod utility;
use emulator_data::NesEmulatorData;

#[cfg(test)]
mod tests;

use crate::cartridge::NesCartridge;
use crate::ppu::NesPpu;

use egui_glow::EguiGlow;
use egui_multiwin::{
    multi_window::{MultiWindow, NewWindowRequest},
    tracked_window::{RedrawResponse, TrackedWindow},
};
struct MainNesWindow {}

impl MainNesWindow {
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(MainNesWindow {}),
            builder: glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(glutin::dpi::LogicalSize {
                    width: 320.0,
                    height: 300.0,
                })
                .with_title("UglyOldBob NES Emulator"),
        }
    }
}

impl TrackedWindow for MainNesWindow {
    type Data = NesEmulatorData;

    fn is_root(&self) -> bool {
        true
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
    ) -> RedrawResponse<Self::Data> {
        egui.egui_ctx.request_repaint();
        let mut quit = false;
        let mut windows_to_create = vec![];

        'emulator_loop: loop {
            #[cfg(debug_assertions)]
            {
                if !c.paused {
                    c.cycle_step();
                    if c.cpu_clock_counter == 0 && c.cpu.breakpoint_option() {
                        if c.single_step {
                            c.paused = true;
                            c.single_step = false;
                            break 'emulator_loop;
                        }
                    }
                } else {
                    break 'emulator_loop;
                }
                if c.cpu_peripherals.ppu_frame_end() {
                    if c.wait_for_frame_end {
                        c.paused = true;
                        c.wait_for_frame_end = false;
                    }
                    break 'emulator_loop;
                }
            }
            #[cfg(not(debug_assertions))]
            {
                c.cycle_step();
                if c.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
        }

        let image = NesPpu::convert_to_egui(c.cpu_peripherals.ppu_get_frame());

        if let None = c.texture {
            c.texture = Some(egui.egui_ctx.load_texture(
                "NES_PPU",
                image,
                egui::TextureFilter::Nearest,
            ));
        } else if let Some(t) = &mut c.texture {
            t.set_partial([0, 0], image, egui::TextureFilter::Nearest);
        }

        egui::TopBottomPanel::top("menu_bar").show(&egui.egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let button = egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {
                        ui.close_menu();
                    }
                });
                #[cfg(debug_assertions)]
                {
                    ui.menu_button("Debug", |ui| {
                        if ui.button("Debugger").clicked() {
                            ui.close_menu();
                            windows_to_create.push(DebugNesWindow::new());
                        }
                        if ui.button("Reset").clicked() {
                            ui.close_menu();
                            c.reset();
                        }
                    });
                }
            });
        });

        egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if let Some(t) = &c.texture {
                ui.image(t, egui::Vec2 { x: 256.0, y: 240.0 });
            }
        });
        RedrawResponse {
            quit: quit,
            new_windows: windows_to_create,
        }
    }
}

struct DebugNesWindow {}

impl DebugNesWindow {
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DebugNesWindow {}),
            builder: glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(glutin::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Debug"),
        }
    }
}

impl TrackedWindow for DebugNesWindow {
    type Data = NesEmulatorData;

    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
    ) -> RedrawResponse<Self::Data> {
        egui.egui_ctx.request_repaint();
        let mut quit = false;
        let mut windows_to_create = vec![];

        egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("Debug window");
            #[cfg(debug_assertions)]
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
                } else {
                    if ui.button("Pause").clicked() {
                        c.single_step = true;
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(format!("Address: 0x{:x}", c.cpu.get_pc()));
                    if let Some(t) = c.cpu.disassemble() {
                        ui.label(t);
                    }
                });
                ui.label(format!(
                    "A: {:x}, X: {:x}, Y: {:x}, P: {:x}, SP: {:x}",
                    c.cpu.get_a(),
                    c.cpu.get_x(),
                    c.cpu.get_y(),
                    c.cpu.get_p(),
                    c.cpu.get_sp(),
                ));
                ui.label(format!(
                    "Frame number {}",
                    c.cpu_peripherals.ppu_frame_number()
                ));
            }
        });
        RedrawResponse {
            quit: quit,
            new_windows: windows_to_create,
        }
    }
}

fn main() {
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let mut multi_window = MultiWindow::new();
    let root_window = MainNesWindow::new();
    let mut nes_data = NesEmulatorData::new();
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());
    let nc = NesCartridge::load_cartridge(
        "./nes/test_roms/apu_test/rom_singles/8-dmc_rates.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    let _e = multi_window.add(root_window, &event_loop);
    #[cfg(debug_assertions)]
    {
        if nes_data.paused {
            let debug_win = DebugNesWindow::new();
            let _e = multi_window.add(debug_win, &event_loop);
        }
    }
    multi_window.run(event_loop, nes_data);
}
