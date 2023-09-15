#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! This is the nes emulator written in rust. It is compatible with windows, linux, and osx.

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod motherboard;
mod ppu;
mod romlist;
#[cfg(test)]
mod utility;

use std::io::Write;

use controller::NesControllerTrait;
use egui_multiwin::egui::Sense;
use emulator_data::NesEmulatorData;

#[cfg(test)]
mod tests;

use crate::cartridge::NesCartridge;
use crate::ppu::NesPpu;

/// The initial rom that the emulator will load. Only for developmment of the beta version (0.1.x)
const INITIAL_ROM: Option<&str> = Some("./roms/nes/Legend of Zelda, The (U) (PRG 0).nes");
//const INITIAL_ROM: Option<&str> = Some("./nes/roms/USA/Spelunker (U) [!].nes");

#[cfg(feature = "egui-multiwin")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "eframe")]
use eframe::egui;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui_glow::EguiGlow;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
    egui,
    multi_window::{MultiWindow, NewWindowRequest},
    tracked_window::{RedrawResponse, TrackedWindow},
};

#[cfg(feature = "sdl2")]
use sdl2::event::Event;
#[cfg(feature = "sdl2")]
use sdl2::keyboard::Keycode;
#[cfg(feature = "sdl2")]
use sdl2::mouse::MouseButton;
#[cfg(feature = "sdl2")]
use sdl2::pixels::Color;
#[cfg(feature = "sdl2")]
use sdl2::pixels::PixelFormatEnum;
#[cfg(feature = "sdl2")]
use sdl2::render::Canvas;
#[cfg(feature = "sdl2")]
use sdl2::render::Texture;
#[cfg(feature = "sdl2")]
use sdl2::render::TextureCreator;

#[cfg(feature = "sdl2")]
pub const EMBEDDED_FONT: &[u8] = include_bytes!("cmsltt10.ttf");

/// The struct for the main window of the emulator.
struct MainNesWindow {
    /// The time of the last emulated frame for the emulator. Even if the emulator is paused, the screen will still run at the proper frame rate.
    last_frame_time: std::time::SystemTime,
    #[cfg(feature = "eframe")]
    c: NesEmulatorData,
    /// The calculated frames per second performance of the emulator.
    fps: f64,
    /// The number of samples per second of the audio output.
    sound_rate: u32,
    /// The producing half of the ring buffer used for audio.
    sound: Option<rb::Producer<f32>>,
    /// The texture used for rendering the ppu image.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub texture: Option<egui::TextureHandle>,
    /// The filter used for audio playback, filtering out high frequency noise, increasing the quality of audio playback.
    filter: Option<biquad::DirectForm1<f32>>,
    /// The interval between sound samples based on the sample rate used in the stream
    sound_sample_interval: f32,
    /// The stream used for audio playback during emulation
    sound_stream: Option<cpal::Stream>,
}

impl MainNesWindow {
    #[cfg(feature = "eframe")]
    fn new() -> Self {
        let mut nes_data = NesEmulatorData::new();
        let nc = NesCartridge::load_cartridge(
            "./nes/test_roms/cpu_exec_space/test_cpu_exec_space_apu.nes".to_string(),
        )
        .unwrap();
        nes_data.insert_cartridge(nc);
        Self {
            last_frame_time: std::time::SystemTime::now(),
            c: nes_data,
            fps: 0.0,
        }
    }

    /// Create a new request for a main window of the emulator.
    #[cfg(feature = "egui-multiwin")]
    fn new_request(
        rate: u32,
        producer: Option<rb::Producer<f32>>,
        stream: Option<cpal::Stream>,
    ) -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(MainNesWindow {
                last_frame_time: std::time::SystemTime::now(),
                fps: 0.0,
                sound_rate: rate,
                sound: producer,
                texture: None,
                filter: None,
                sound_sample_interval: 0.0,
                sound_stream: stream,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 300.0,
                })
                .with_title("UglyOldBob NES Emulator"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "eframe")]
impl eframe::App for MainNesWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(ctx);
        }

        {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("nes frame render");
            'emulator_loop: loop {
                self.c.cycle_step();
                if self.c.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
        }
        {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("nes frame convert");
            let image = NesPpu::convert_to_egui(self.c.cpu_peripherals.ppu_get_frame());

            if let None = self.c.texture {
                self.c.texture =
                    Some(ctx.load_texture("NES_PPU", image, egui::TextureOptions::LINEAR));
            } else if let Some(t) = &mut self.c.texture {
                t.set_partial([0, 0], image, egui::TextureOptions::LINEAR);
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let button = egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(&ctx, |ui| {
            if let Some(t) = &self.c.texture {
                ui.image(t, egui::Vec2 { x: 256.0, y: 240.0 });
            }
            ui.label(format!("{:.0} FPS", self.fps));
        });

        {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("sleep time");
            let time_now = std::time::SystemTime::now();
            let frame_time = time_now.duration_since(self.last_frame_time).unwrap();
            let desired_frame_length = std::time::Duration::from_nanos(1_000_000_000u64 / 60);
            if frame_time < desired_frame_length {
                let st = (desired_frame_length - frame_time);
                spin_sleep::sleep(st);
            }

            let new_frame_time = std::time::SystemTime::now();
            let new_fps = 1_000_000_000.0
                / new_frame_time
                    .duration_since(self.last_frame_time)
                    .unwrap()
                    .as_nanos() as f64;
            self.fps = (self.fps * 0.95) + (0.05 * new_fps);
            self.last_frame_time = new_frame_time;
        }
        ctx.request_repaint();
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for MainNesWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<NesEmulatorData> {
        egui.egui_ctx.request_repaint();

        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(&egui.egui_ctx);
        }

        #[cfg(feature = "puffin")]
        puffin::profile_scope!("frame rendering");

        if self.filter.is_none() && self.sound_stream.is_some() {
            let rf = self.sound_rate as f32;
            let sampling_frequency = 21.47727e6 / 12.0;
            let filter_coeff = biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                biquad::Hertz::<f32>::from_hz(sampling_frequency).unwrap(),
                biquad::Hertz::<f32>::from_hz(rf / 2.2).unwrap(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap();
            self.filter = Some(biquad::DirectForm1::<f32>::new(filter_coeff));
            self.sound_sample_interval = sampling_frequency / rf;
            c.cpu_peripherals
                .apu
                .set_audio_interval(self.sound_sample_interval);
        }

        let quit = false;
        let mut windows_to_create = vec![];

        {
            egui.egui_ctx.input(|i| {
                if let Some(c) = &mut c.mb.controllers[0] {
                    c.provide_egui_ref(i);
                }
                if let Some(c) = &mut c.mb.controllers[1] {
                    c.provide_egui_ref(i);
                }
            });
        }

        'emulator_loop: loop {
            #[cfg(feature = "debugger")]
            {
                if !c.paused {
                    c.cycle_step(&mut self.sound, &mut self.filter);
                    if c.cpu_clock_counter == 0
                        && c.cpu.breakpoint_option()
                        && (c.cpu.breakpoint() || c.single_step)
                    {
                        c.paused = true;
                        c.single_step = false;
                        break 'emulator_loop;
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
            #[cfg(not(feature = "debugger"))]
            {
                c.cycle_step(&mut self.sound, &mut self.filter);
                if c.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
        }

        let image = NesPpu::convert_to_egui(c.cpu_peripherals.ppu_get_frame());

        if self.texture.is_none() {
            self.texture = Some(egui.egui_ctx.load_texture(
                "NES_PPU",
                image,
                egui_multiwin::egui::TextureOptions::NEAREST,
            ));
        } else if let Some(t) = &mut self.texture {
            t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
        }

        let mut save_state = false;
        let mut load_state = false;

        egui_multiwin::egui::TopBottomPanel::top("menu_bar").show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let button = egui_multiwin::egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(RomFinder::new_request());
                        ui.close_menu();
                    }

                    let button = egui_multiwin::egui::Button::new("Save state");
                    if ui.add_enabled(true, button).clicked()
                        || egui
                            .egui_ctx
                            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F5))
                    {
                        save_state = true;
                        ui.close_menu();
                    }

                    let button = egui_multiwin::egui::Button::new("Load state");
                    if ui.add_enabled(true, button).clicked()
                        || egui
                            .egui_ctx
                            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F6))
                    {
                        println!("Loading state");
                        load_state = true;
                        ui.close_menu();
                    }
                });
                #[cfg(feature = "debugger")]
                {
                    ui.menu_button("Debug", |ui| {
                        if ui.button("Debugger").clicked() {
                            ui.close_menu();
                            windows_to_create.push(DebugNesWindow::new_request());
                        }
                        if ui.button("Dump CPU Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(CpuMemoryDumpWindow::new_request());
                        }
                        if ui.button("Dump Cartridge Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(CartridgeMemoryDumpWindow::new_request());
                        }
                        if ui.button("Dump Cartridge RAM").clicked() {
                            ui.close_menu();
                            windows_to_create.push(
                                cartridge_prg_ram_dump::CartridgeMemoryDumpWindow::new_request(),
                            );
                        }
                        if ui.button("Dump ppu pattern table").clicked() {
                            ui.close_menu();
                            windows_to_create
                                .push(pattern_table_dump_window::DumpWindow::new_request());
                        }
                        if ui.button("Dump ppu name tables").clicked() {
                            ui.close_menu();
                            windows_to_create
                                .push(name_table_dump_window::DumpWindow::new_request());
                        }
                        if ui.button("Reset").clicked() {
                            ui.close_menu();
                            c.reset();
                        }
                    });
                }
            });
        });

        if egui
            .egui_ctx
            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F5))
        {
            save_state = true;
        }

        if egui
            .egui_ctx
            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F6))
        {
            load_state = true;
        }

        if save_state {
            let state = Box::new(c.serialize());
            let _e = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open("./state.bin")
                .unwrap()
                .write_all(&state);
        }

        if load_state {
            if let Ok(a) = std::fs::read("./state.bin") {
                let _e = c.deserialize(a);
            }
        }

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if let Some(t) = &self.texture {
                let r = ui.image(t, egui_multiwin::egui::Vec2 { x: 256.0, y: 240.0 });
                if r.hovered() {
                    if let Some(pos) = r.hover_pos() {
                        //println!("Hover at {:?}", pos - r.rect.left_top());
                    }
                }
            }
            ui.label(format!("{:.0} FPS", self.fps));
        });

        let time_now = std::time::SystemTime::now();
        let frame_time = time_now.duration_since(self.last_frame_time).unwrap();
        let desired_frame_length = std::time::Duration::from_nanos(1_000_000_000u64 / 60);
        if frame_time < desired_frame_length {
            let st = desired_frame_length - frame_time;
            spin_sleep::sleep(st);
        }

        let new_frame_time = std::time::SystemTime::now();
        let new_fps = 1_000_000_000.0
            / new_frame_time
                .duration_since(self.last_frame_time)
                .unwrap()
                .as_nanos() as f64;
        self.fps = (self.fps * 0.95) + (0.05 * new_fps);
        self.last_frame_time = new_frame_time;

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}

mod cartridge_prg_ram_dump;
mod name_table_dump_window;
mod pattern_table_dump_window;

/// The window for dumping cartridge program data
#[cfg(feature = "egui-multiwin")]
struct CartridgeMemoryDumpWindow {}

impl CartridgeMemoryDumpWindow {
    fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(CartridgeMemoryDumpWindow {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Cartridge ROM Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for CartridgeMemoryDumpWindow {
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
            ui.label("Cartridge Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    if let Some(cart) = c.mb.cartridge() {
                        for (i, chunk) in cart.cartridge().prg_rom.chunks(8).enumerate() {
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
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}

/// The window for dumping cpu data
#[cfg(feature = "egui-multiwin")]
struct CpuMemoryDumpWindow {}

impl CpuMemoryDumpWindow {
    fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(CpuMemoryDumpWindow {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES CPU Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for CpuMemoryDumpWindow {
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
            ui.label("CPU Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    for i in (0..=0xFFFF).step_by(8) {
                        let a1 = if let Some(a) = c.mb.memory_dump(i, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a2 = if let Some(a) = c.mb.memory_dump(i + 1, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a3 = if let Some(a) = c.mb.memory_dump(i + 2, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a4 = if let Some(a) = c.mb.memory_dump(i + 3, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a5 = if let Some(a) = c.mb.memory_dump(i + 4, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a6 = if let Some(a) = c.mb.memory_dump(i + 5, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a7 = if let Some(a) = c.mb.memory_dump(i + 6, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        let a8 = if let Some(a) = c.mb.memory_dump(i + 7, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            format!("**")
                        };
                        ui.label(format!(
                            "{:04X}: {} {} {} {}\t{} {} {} {}",
                            i, a1, a2, a3, a4, a5, a6, a7, a8,
                        ));
                    }
                }
            });
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}

/// The structure for a debug window of the emulator.
#[cfg(feature = "egui-multiwin")]
struct DebugNesWindow {
    breakpoint: String,
}

#[cfg(feature = "egui-multiwin")]
impl DebugNesWindow {
    /// Create a new request for a Debug window.
    fn new_request() -> NewWindowRequest<NesEmulatorData> {
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
                    if ui.button("Reset").clicked() {
                        c.reset();
                    }
                } else if ui.button("Pause").clicked() {
                    c.single_step = true;
                    c.paused = true;
                }
                if let Some(cart) = c.mb.cartridge() {
                    ui.label(format!("ROM format: {:?}", cart.rom_format));
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
                }
            }
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}

/// The structure for a window that helps a user select a rom to load.
#[cfg(feature = "egui-multiwin")]
struct RomFinder {
    /// The element responsible for parsing the list of roms known by the emulator.
    parser: romlist::RomListParser,
}

#[cfg(feature = "egui-multiwin")]
impl RomFinder {
    /// Create a new request to make a RomFinder window.
    fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(RomFinder {
                parser: romlist::RomListParser::new(),
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Rom Select"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for RomFinder {
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
        let mut quit = false;
        let windows_to_create = vec![];

        //scan for roms if needed
        self.parser.find_roms("./roms");
        //process to see if any new roms need to be checked
        self.parser.process_roms();

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                let mut new_rom = None;
                for (p, entry) in self.parser.list().elements.iter() {
                    if let Some(r) = &entry.result {
                        if let Ok(r) = r {
                            if ui
                                .add(
                                    egui_multiwin::egui::Label::new(format!(
                                        "{:x}: {}",
                                        r.mapper,
                                        p.display()
                                    ))
                                    .sense(Sense::click()),
                                )
                                .double_clicked()
                            {
                                new_rom = Some(
                                    NesCartridge::load_cartridge(p.to_str().unwrap().into())
                                        .unwrap(),
                                );
                                quit = true;
                            }
                        }
                    }
                }
                ui.label("Unsupported roms below here");
                for (p, entry) in self.parser.list().elements.iter() {
                    if let Some(Err(r)) = &entry.result {
                        ui.label(format!("Rom: {}: {:?}", p.display(), r));
                    }
                }
                if let Some(nc) = new_rom {
                    c.remove_cartridge();
                    c.insert_cartridge(nc);
                    c.reset();
                }
            });
        });

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}

#[cfg(feature = "sdl2")]
fn make_dummy_texture<'a, T>(tc: &'a TextureCreator<T>) -> Texture<'a> {
    let mut data: Vec<u8> = vec![0; (4 * 4 * 2) as usize];
    let mut surf = sdl2::surface::Surface::from_data(
        data.as_mut_slice(),
        4,
        4,
        (2 * 4) as u32,
        PixelFormatEnum::RGB555,
    )
    .unwrap();
    let _e = surf.set_color_key(true, sdl2::pixels::Color::BLACK);
    Texture::from_surface(&surf, tc).unwrap()
}

#[cfg(feature = "sdl2")]
struct Text<'a> {
    t: Texture<'a>,
    color: sdl2::pixels::Color,
}

#[cfg(feature = "sdl2")]
impl<'a> Text<'a> {
    fn new<T>(
        t: String,
        color: sdl2::pixels::Color,
        font: &sdl2::ttf::Font,
        tc: &'a TextureCreator<T>,
    ) -> Self {
        let pr = font.render(t.as_str());
        let ft = pr.solid(color).unwrap();
        Self {
            color: color,
            t: Texture::from_surface(&ft, tc).unwrap(),
        }
    }
    fn set_text<T>(
        &mut self,
        tc: &'a TextureCreator<T>,
        t: String,
        font: &sdl2::ttf::Font,
        color: sdl2::pixels::Color,
    ) {
        self.color = color;
        let pr = font.render(t.as_str());
        let ft = pr.solid(self.color).unwrap();
        self.t = Texture::from_surface(&ft, tc).unwrap();
    }
    fn draw(&self, canvas: &mut Canvas<sdl2::video::Window>) {
        let q = self.t.query();
        let _e = canvas.copy(
            &self.t,
            None,
            sdl2::rect::Rect::new(0, 0, q.width.into(), q.height.into()),
        );
    }
}

#[cfg(feature = "sdl2")]
fn main() {
    let ttf_context = sdl2::ttf::init().unwrap();
    let efont = sdl2::rwops::RWops::from_bytes(EMBEDDED_FONT).unwrap();
    let font = ttf_context.load_font_from_rwops(efont, 14).unwrap();

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(egui_sdl2_gl::sdl2::video::GLProfile::Core);
    gl_attr.set_double_buffer(true);
    gl_attr.set_multisample_samples(4);

    video_subsystem.text_input().start();

    let mut vid_win = video_subsystem.window("UglyOldBob NES Emulator", 1024, 768);
    let mut windowb = vid_win.position_centered();

    let window = windowb.opengl().build().unwrap();

    let _ctx = window.gl_create_context().unwrap();

    let shader_ver = egui_sdl2_gl::ShaderVersion::Default;
    let (mut painter, mut egui_state) =
        egui_sdl2_gl::with_sdl2(&window, shader_ver, egui_sdl2_gl::DpiScaling::Custom(2.0));
    let mut egui_ctx = egui_sdl2_gl::egui::CtxRef::default();

    let i = sdl2::mixer::InitFlag::MP3;
    let _sdl2mixer = sdl2::mixer::init(i).unwrap();
    let audio = sdl2::mixer::open_audio(44100, 16, 2, 1024);

    let flags = sdl2::image::InitFlag::all();
    let _sdl2_image = sdl2::image::init(flags).unwrap();

    let mut last_frame_time: std::time::SystemTime = std::time::SystemTime::now();

    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "./nes/test_roms/cpu_exec_space/test_cpu_exec_space_apu.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    let mut frame: Vec<egui_sdl2_gl::egui::Color32> = Vec::with_capacity(256 * 240);
    for _i in 0..(256 * 240) {
        frame.push(egui_sdl2_gl::egui::Color32::BLACK);
    }
    let nes_frame_texture_id = painter.new_user_texture((256, 240), &frame, false);

    let mut quit = false;

    let mut last_framerate = 60.0;
    let mut fps = 0.0;

    let start_time = std::time::Instant::now();
    'main_loop: loop {
        let frame_start = std::time::SystemTime::now();

        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_frame(egui_state.input.take());

        'emulator_loop: loop {
            nes_data.cycle_step();
            if nes_data.cpu_peripherals.ppu_frame_end() {
                break 'emulator_loop;
            }
        }

        let frame_data = nes_data.cpu_peripherals.ppu_get_frame();
        NesPpu::convert_for_sdl2(frame_data, &mut frame);

        //todo update buffer
        painter.update_user_texture_data(nes_frame_texture_id, &frame);

        egui_sdl2_gl::egui::Window::new("Egui with SDL2 and GL")
            .title_bar(false)
            .fixed_rect(egui_sdl2_gl::egui::Rect {
                min: egui_sdl2_gl::egui::Pos2 { x: 0.0, y: 0.0 },
                max: egui_sdl2_gl::egui::Pos2 { x: 256.0, y: 240.0 },
            })
            .show(&egui_ctx, |ui| {
                ui.label(format!("FPS: {:.0}", fps));
                ui.separator();
                ui.add(egui_sdl2_gl::egui::Image::new(
                    nes_frame_texture_id,
                    egui_sdl2_gl::egui::vec2(256.0, 240.0),
                ));
            });

        let (egui_output, paint_cmds) = egui_ctx.end_frame();

        egui_state.process_output(&window, &egui_output);

        let paint_jobs = egui_ctx.tessellate(paint_cmds);

        painter.paint_jobs(None, paint_jobs, &egui_ctx.font_image());

        window.gl_swap_window();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                _ => {
                    // Process input event
                    egui_state.process_input(&window, event, &mut painter);
                }
            }
        }

        if quit {
            break;
        }

        let time_now = std::time::SystemTime::now();
        let frame_time = time_now.duration_since(last_frame_time).unwrap();
        let desired_frame_length = std::time::Duration::from_nanos(1_000_000_000u64 / 60);
        if frame_time < desired_frame_length {
            let st = (desired_frame_length - frame_time);
            spin_sleep::sleep(st);
        }

        let new_frame_time = std::time::SystemTime::now();
        let new_fps = 1_000_000_000.0
            / new_frame_time
                .duration_since(last_frame_time)
                .unwrap()
                .as_nanos() as f64;
        fps = (fps * 0.95) + (0.05 * new_fps);
        last_frame_time = new_frame_time;
    }
}

#[cfg(feature = "eframe")]
fn main() {
    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!
    let mut options = eframe::NativeOptions::default();
    //TODO only disable vsync when required
    options.vsync = false;
    eframe::run_native(
        "UglyOldBob NES Emulator",
        options,
        Box::new(|_cc| Box::new(MainNesWindow::new())),
    );
}

#[cfg(feature = "egui-multiwin")]
fn main() {
    use rb::RB;

    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!
    let event_loop = egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event().build();
    let mut nes_data = NesEmulatorData::new();
    nes_data.paused = true;
    let mut multi_window = MultiWindow::new();

    let host = cpal::default_host();
    let device = host.default_output_device();
    let mut sound_rate = 0;
    let mut sound_producer = None;
    let sound_stream = if let Some(d) = &device {
        let ranges = d.supported_output_configs();
        if let Ok(mut r) = ranges {
            let config = r.next().unwrap().with_max_sample_rate();
            let format = config.sample_format();
            println!("output format is {:?}", format);
            let config = config.config();

            let rb = rb::SpscRb::new((config.sample_rate.0 as f32 * 0.1) as usize);
            let (producer, consumer) = (rb.producer(), rb.consumer());
            let mut stream = d
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        let _e = rb::RbConsumer::read(&consumer, data);
                    },
                    move |_err| {},
                    None,
                )
                .ok();
            if let Some(s) = &mut stream {
                s.play().unwrap();
                sound_rate = config.sample_rate.0;
                sound_producer = Some(producer);
            }
            stream
        } else {
            None
        }
    } else {
        None
    };

    let root_window = MainNesWindow::new_request(sound_rate, sound_producer, sound_stream);

    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());
    nes_data.mb.controllers[0] = Some(controller::StandardController::new());

    if let Some(c) = INITIAL_ROM {
        let nc = NesCartridge::load_cartridge(c.to_string()).unwrap();
        nes_data.insert_cartridge(nc);
    }

    let _e = multi_window.add(root_window, &event_loop);
    #[cfg(feature = "debugger")]
    {
        if nes_data.paused {
            let debug_win = DebugNesWindow::new_request();
            let _e = multi_window.add(debug_win, &event_loop);
        }
    }
    multi_window.run(event_loop, nes_data);
}
