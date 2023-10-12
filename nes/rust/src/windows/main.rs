//! The main window of the emulator
//!
use std::io::Write;

use crate::{controller::NesControllerTrait, ppu::NesPpu, NesEmulatorData};

use egui_multiwin::{
    egui,
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The struct for the main window of the emulator.
pub struct MainNesWindow {
    /// The time of the last emulated frame for the emulator. Even if the emulator is paused, the screen will still run at the proper frame rate.
    last_frame_time: std::time::SystemTime,
    #[cfg(feature = "eframe")]
    c: NesEmulatorData,
    /// The calculated frames per second performance of the emulator.
    fps: f64,
    /// The number of samples per second of the audio output.
    sound_rate: u32,
    /// The producing half of the ring buffer used for audio.
    sound: Option<
        ringbuf::Producer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    >,
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
        nes_data.power_cycle();
        Self {
            last_frame_time: std::time::SystemTime::now(),
            c: nes_data,
            fps: 0.0,
        }
    }

    /// Create a new request for a main window of the emulator.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request(
        rate: u32,
        producer: Option<
            ringbuf::Producer<
                f32,
                std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
            >,
        >,
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
                    width: 640.0,
                    height: 600.0,
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
                    let button = egui::Button::new("Open rom");
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

    fn can_quit(&mut self, _c: &mut NesEmulatorData) -> bool {
        self.sound_stream.take();
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
            println!("Initializing with sample rate {}", self.sound_rate);
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
                if let Some(controller) = &mut c.mb.controllers[0] {
                    for (index, contr) in controller.get_buttons_iter_mut().enumerate() {
                        let cnum = index << 1;
                        let button_config = &c.configuration.controller_config[cnum];
                        contr.update_egui_buttons(i, button_config);
                        //unimplemented!();
                    }
                }
                if let Some(controller) = &mut c.mb.controllers[1] {
                    for (index, contr) in controller.get_buttons_iter_mut().enumerate() {
                        let cnum = 1 + index << 1;
                        let button_config = c.configuration.controller_config[cnum];
                        //unimplemented!();
                    }
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
                        println!("End of frame for debugger");
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
                        windows_to_create.push(super::rom_finder::RomFinder::new_request());
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
                            windows_to_create.push(super::debug_window::DebugNesWindow::new_request());
                        }
                        if ui.button("Dump CPU Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::cpu_memory_dump_window::CpuMemoryDumpWindow::new_request());
                        }
                        if ui.button("Dump PPU Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::ppu_memory_dump_window::PpuMemoryDumpWindow::new_request());
                        }
                        if ui.button("Dump Cartridge Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::cartridge_dump::CartridgeMemoryDumpWindow::new_request());
                        }
                        if ui.button("Dump Cartridge RAM").clicked() {
                            ui.close_menu();
                            windows_to_create.push(
                                super::cartridge_prg_ram_dump::CartridgeMemoryDumpWindow::new_request(),
                            );
                        }
                        if ui.button("Dump ppu pattern table").clicked() {
                            ui.close_menu();
                            windows_to_create
                                .push(super::pattern_table_dump_window::DumpWindow::new_request());
                        }
                        if ui.button("Dump ppu name tables").clicked() {
                            ui.close_menu();
                            windows_to_create
                                .push(super::name_table_dump_window::DumpWindow::new_request());
                        }
                        if ui.button("Dump ppu sprites").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::sprite_dump_window::DumpWindow::new_request());
                        }
                        if ui.button("Reset").clicked() {
                            ui.close_menu();
                            c.reset();
                        }
                        if ui.button("Power cycle").clicked() {
                            ui.close_menu();
                            c.power_cycle();
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

        let name = if let Some(cart) = c.mb.cartridge() {
            cart.save_name()
        } else {
            "state.bin".to_string()
        };
        let name = format!("./saves/{}", name);
        if save_state {
            let mut path = std::path::PathBuf::from(&name);
            path.pop();
            let _ = std::fs::create_dir_all(path);
            let state = Box::new(c.serialize());
            let _e = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&name)
                .unwrap()
                .write_all(&state);
        }

        if load_state {
            if let Ok(a) = std::fs::read(&name) {
                let _e = c.deserialize(a);
            }
        }

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if let Some(t) = &self.texture {
                let zoom = 2.0;
                let r = ui.image(
                    t,
                    egui_multiwin::egui::Vec2 {
                        x: 256.0 * zoom,
                        y: 240.0 * zoom,
                    },
                );
                if r.hovered() {
                    if let Some(pos) = r.hover_pos() {
                        let coord = pos - r.rect.left_top();
                        c.cpu_peripherals.ppu.bg_debug =
                            Some(((coord.x / zoom) as u8, (coord.y / zoom) as u8));
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
