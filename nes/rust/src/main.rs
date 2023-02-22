#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod apu;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod emulator_data;
pub mod motherboard;
pub mod ppu;
pub mod utility;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use cartridge::CartridgeError;
use egui_multiwin::egui::Sense;
use emulator_data::NesEmulatorData;

#[cfg(test)]
mod tests;

use crate::cartridge::NesCartridge;
use crate::ppu::NesPpu;

#[cfg(feature = "eframe")]
use eframe::egui;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui_glow::EguiGlow;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
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

struct MainNesWindow {
    last_frame_time: std::time::SystemTime,
    #[cfg(feature = "eframe")]
    c: NesEmulatorData,
    fps: f64,
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
    #[cfg(feature = "egui-multiwin")]
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(MainNesWindow {
                last_frame_time: std::time::SystemTime::now(),
                fps: 0.0,
            }),
            builder: egui_multiwin::glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::glutin::dpi::LogicalSize {
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

        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(&egui.egui_ctx);
        }

        #[cfg(feature = "puffin")]
        puffin::profile_scope!("frame rendering");

        let mut quit = false;
        let mut windows_to_create = vec![];

        {
            let input = egui.egui_ctx.input();
            if let Some(c) = &mut c.mb.controllers[0] {
                c.provide_egui_ref(&input);
            }
            if let Some(c) = &mut c.mb.controllers[1] {
                c.provide_egui_ref(&input);
            }
        }

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
                egui_multiwin::egui::TextureOptions::NEAREST,
            ));
        } else if let Some(t) = &mut c.texture {
            t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
        }

        egui_multiwin::egui::TopBottomPanel::top("menu_bar").show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let button = egui_multiwin::egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(RomFinder::new());
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

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if let Some(t) = &c.texture {
                ui.image(t, egui_multiwin::egui::Vec2 { x: 256.0, y: 240.0 });
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
            quit: quit,
            new_windows: windows_to_create,
        }
    }
}

#[cfg(feature = "egui-multiwin")]
struct DebugNesWindow {}

#[cfg(feature = "egui-multiwin")]
impl DebugNesWindow {
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DebugNesWindow {}),
            builder: egui_multiwin::glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::glutin::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
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

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
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

#[derive(Serialize, Deserialize)]
struct RomListEntry {
    result: Option<Result<(), CartridgeError>>,
    modified: Option<std::time::SystemTime>,
}

//TODO Create benchmark to determine if the caching scheme is actually beneficial
#[derive(Serialize, Deserialize)]
pub struct RomList {
    elements: std::collections::BTreeMap<PathBuf, RomListEntry>,
}

impl RomList {
    fn new() -> Self {
        Self {
            elements: std::collections::BTreeMap::new(),
        }
    }

    pub fn load_list() -> Self {
        let contents = std::fs::read("./roms.bin");
        if let Err(e) = contents {
            return RomList::new();
        }
        let contents = contents.unwrap();
        let config = bincode::deserialize(&contents[..]);
        config.ok().unwrap_or(RomList::new())
    }

    fn save_list(&self) -> std::io::Result<()> {
        let encoded = bincode::serialize(&self).unwrap();
        std::fs::write("./roms.bin", encoded)
    }
}

#[cfg(feature = "egui-multiwin")]
struct RomFinder {
    scan_complete: bool,
    update_complete: bool,
}

#[cfg(feature = "egui-multiwin")]
impl RomFinder {
    fn find_roms(&mut self, rom_list: &mut RomList) {
        if !self.scan_complete {
            for entry in walkdir::WalkDir::new("./roms/")
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| !e.file_type().is_dir())
            {
                let meta = entry.metadata();
                if let Ok(meta) = meta {
                    let modified = meta.modified();

                    let m = entry.clone().into_path();
                    let name = m.clone().into_os_string().into_string().unwrap();
                    if NesCartridge::load_cartridge(name).is_ok() {
                        if !rom_list.elements.contains_key(&m) {
                            let new_entry = RomListEntry {
                                result: None,
                                modified: None,
                            };
                            rom_list.elements.insert(m, new_entry);
                        }
                    }
                }
            }
            rom_list.save_list();
            self.scan_complete = true;
        }
    }

    fn process_roms(&mut self, rom_list: &mut RomList) {
        if !self.update_complete {
            for (p, entry) in rom_list.elements.iter_mut() {
                let metadata = p.metadata();
                if let Ok(metadata) = metadata {
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                    let last_modified = entry.modified.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if modified > last_modified {
                        let romcheck = NesCartridge::load_cartridge(
                            p.as_os_str().to_str().unwrap().to_string(),
                        );
                        entry.result = Some(romcheck.map(|_i| ()));
                        entry.modified = Some(modified);
                    }
                }
            }
            rom_list.save_list();
            self.update_complete = true;
        }
    }

    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(RomFinder {
                scan_complete: false,
                update_complete: false,
            }),
            builder: egui_multiwin::glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::glutin::dpi::LogicalSize {
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
impl TrackedWindow for RomFinder {
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
        let mut quit = false;
        let mut windows_to_create = vec![];

        //scan for roms if needed
        self.find_roms(&mut c.roms);
        //process to see if any new roms need to be checked
        self.process_roms(&mut c.roms);

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                let mut new_rom = None;
                for (p, entry) in c.roms.elements.iter() {
                    if let Some(r) = &entry.result {
                        if r.is_ok() {
                            if ui
                                .add(
                                    egui_multiwin::egui::Label::new(format!("{}", p.display()))
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
                if let Some(nc) = new_rom {
                    c.remove_cartridge();
                    c.insert_cartridge(nc);
                    c.reset();
                }
            });
        });

        RedrawResponse {
            quit: quit,
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
    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!
    let event_loop = egui_multiwin::glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let mut multi_window = MultiWindow::new();
    let root_window = MainNesWindow::new();
    let mut nes_data = NesEmulatorData::new();
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());
    let nc = NesCartridge::load_cartridge("./nes/test_roms/read_joy3/test_buttons.nes".to_string())
        .unwrap();
    nes_data.mb.controllers[0] = Some(Box::new(controller::StandardController::new()));
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
