//! The main window of the emulator
//!
use std::io::Write;

use crate::{controller::SnesControllerTrait, network::NodeRole, SnesEmulatorData};

use common_emulator::audio::AudioProducerWithRate;
use common_emulator::recording::Recording;

#[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
use cpal::traits::StreamTrait;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The struct for the main window of the emulator.
pub struct MainSnesWindow {
    /// The last time a rewind point was saved.
    rewind_point: Option<std::time::Instant>,
    /// The rewind points
    rewinds: [Vec<u8>; 3],
    /// The time of the last drawn frame for the emulator.
    last_frame_time: std::time::Instant,
    /// The time of the last emulated frame for the emulator.
    last_emulated_frame: std::time::Instant,
    /// Used to synchronize the emulator to the right frame rate
    emulator_time: std::time::Duration,
    #[cfg(feature = "eframe")]
    c: SnesEmulatorData,
    /// The calculated frames per second performance of the program. Will be higher than the fps of the emulator.
    fps: f64,
    /// The calculated frames per second performance of the emulator.
    emulator_fps: f64,
    /// The producing half of the ring buffer used for audio.
    sound: Option<AudioProducerWithRate>,
    /// The texture used for rendering the ppu image.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub texture: Option<egui::TextureHandle>,
    /// The filter used for audio playback, filtering out high frequency noise, increasing the quality of audio playback.
    filter: Option<biquad::DirectForm1<f32>>,
    /// The stream used for audio playback during emulation
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    sound_stream: Option<cpal::Stream>,
    /// Indicates the last know state of the sound stream
    paused: bool,
    /// Used for the zapper
    mouse: bool,
    /// Used for the zapper
    mouse_vision: bool,
    /// The delay required for the zapper
    mouse_delay: u8,
    /// The zapper was fired "off-screen"
    mouse_miss: bool,
    /// The result of opening gstreamer
    have_gstreamer: Result<(), gstreamer::glib::Error>,
    /// The recording object
    recording: Recording,
    /// The audio objects for a streaming server
    audio_streaming: Vec<std::sync::Weak<std::sync::Mutex<AudioProducerWithRate>>>,
}

impl MainSnesWindow {
    #[cfg(feature = "eframe")]
    pub fn new_request(
        c: SnesEmulatorData,
        rate: u32,
        producer: Option<crate::AudioProducer>,
        stream: Option<cpal::Stream>,
    ) -> Self {
        Self {
            last_frame_time: std::time::SystemTime::now(),
            c,
            fps: 0.0,
            sound_rate: rate,
            sound: producer,
            texture: None,
            filter: None,
            sound_stream: stream,
            paused: false,
        }
    }

    /// Create a new request for a main window of the emulator.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request(
        producer: Option<AudioProducerWithRate>,
        stream: Option<cpal::Stream>,
    ) -> NewWindowRequest {
        use std::time::Duration;

        let have_gstreamer = gstreamer::init();
        gstreamer::debug_add_log_function(|a, b, c, d, e, f, g| {
            println!("GSTREAMER: {:?} {} {} {} {} {:?} {:?}", a, b, c, d, e, f, g);
        });
        gstreamer::debug_set_active(true);
        if let Err(e) = &have_gstreamer {
            println!("Failed to open gstreamer: {:?}", e);
        }

        NewWindowRequest {
            window_state: super::Windows::Main(MainSnesWindow {
                have_gstreamer,
                rewind_point: None,
                rewinds: [Vec::new(), Vec::new(), Vec::new()],
                last_frame_time: std::time::Instant::now(),
                last_emulated_frame: std::time::Instant::now(),
                emulator_time: Duration::from_millis(0),
                fps: 0.0,
                emulator_fps: 0.0,
                sound: producer,
                texture: None,
                filter: None,
                sound_stream: stream,
                paused: false,
                mouse: false,
                mouse_vision: false,
                mouse_delay: 0,
                mouse_miss: false,
                recording: Recording::new(),
                audio_streaming: Vec::new(),
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
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "eframe")]
impl eframe::App for MainSnesWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(ctx);
        }

        if self.filter.is_none() && self.sound_stream.is_some() {
            println!("Initializing with sample rate {}", self.sound_rate);
            let rf = self.sound_rate as f32;
            let sampling_frequency = c.cpu_frequency();
            let filter_coeff = biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                biquad::Hertz::<f32>::from_hz(sampling_frequency).unwrap(),
                biquad::Hertz::<f32>::from_hz(rf / 2.2).unwrap(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap();
            self.filter = Some(biquad::DirectForm1::<f32>::new(filter_coeff));
            self.c
                .cpu_peripherals
                .apu
                .set_audio_interval(self.sound_sample_interval);
        }

        {
            ctx.input(|i| {
                if let Some(controller) = &mut self.c.mb.controllers[0] {
                    for (index, contr) in controller.get_buttons_iter_mut().enumerate() {
                        let cnum = index << 1;
                        let button_config = &self.c.configuration.controller_config[cnum];
                        contr.update_egui_buttons(i, button_config);
                        //unimplemented!();
                    }
                }
                if let Some(controller) = &mut self.c.mb.controllers[1] {
                    for (index, contr) in controller.get_buttons_iter_mut().enumerate() {
                        let cnum = 1 + (index << 1);
                        let button_config = &self.c.configuration.controller_config[cnum];
                        contr.update_egui_buttons(i, button_config);
                    }
                }
            });
        }

        {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("nes frame render");
            'emulator_loop: loop {
                self.c.cycle_step(&mut self.sound, &mut self.filter);
                if self.c.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
        }
        {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("nes frame convert");
            let image = SnesPpu::convert_to_egui(self.c.cpu_peripherals.ppu_get_frame());

            if let None = self.texture {
                self.texture =
                    Some(ctx.load_texture("NES_PPU", image, egui::TextureOptions::LINEAR));
            } else if let Some(t) = &mut self.texture {
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

        let mut save_state = false;
        let mut load_state = false;

        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            save_state = true;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::F6)) {
            load_state = true;
        }

        let name = if let Some(cart) = self.c.mb.cartridge() {
            cart.save_name()
        } else {
            "state.bin".to_string()
        };
        let name = format!("./saves/{}", name);
        if save_state {
            let mut path = std::path::PathBuf::from(&name);
            path.pop();
            let _ = std::fs::create_dir_all(path);
            let state = Box::new(self.c.serialize());
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
                let _e = self.c.deserialize(a);
            }
        }

        egui::CentralPanel::default().show(&ctx, |ui| {
            if let Some(t) = &self.texture {
                let size = ui.available_size();
                let zoom = (size.x / 256.0).min(size.y / 240.0);
                let r = ui.add(egui::Image::from_texture(egui::load::SizedTexture {
                    id: t.id(),
                    size: egui::Vec2 {
                        x: 256.0 * zoom,
                        y: 240.0 * zoom,
                    },
                }));
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
impl TrackedWindow for MainSnesWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn can_quit(&mut self, _c: &mut SnesEmulatorData) -> bool {
        self.sound_stream.take();
        loop {
            if self.recording.stop().is_ok() {
                break;
            }
        }
        true
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut SnesEmulatorData,
        egui: &mut EguiGlow,
        window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        egui.egui_ctx.request_repaint();

        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(&egui.egui_ctx);
        }

        let time_now = std::time::Instant::now();
        let frame_time = time_now.duration_since(self.last_frame_time);
        self.last_frame_time = time_now;

        if self.rewind_point.is_none() {
            self.rewind_point = Some(time_now);
            let p = c.serialize();
            self.rewinds[0] = p.clone();
            self.rewinds[1] = p.clone();
            self.rewinds[2] = p.clone();
        } else if let Some(t) = self.rewind_point {
            if let Some(rew) = c.local.configuration.rewind_interval {
                if time_now.duration_since(t) > rew {
                    self.rewinds[2] = self.rewinds[1].clone();
                    self.rewinds[1] = self.rewinds[0].clone();
                    self.rewinds[0] = c.serialize();
                    self.rewind_point = Some(time_now);
                }
            }
        }

        let new_fps = 1_000_000_000.0 / frame_time.as_nanos() as f64;
        self.fps = (self.fps * 0.95) + (0.05 * new_fps);

        c.mb.get_controller_mut(0).rapid_fire(frame_time);
        c.mb.get_controller_mut(1).rapid_fire(frame_time);

        let nanos = 1_000_000_000.0 / (c.ppu_frame_rate() * c.mb.speed_ratio);
        let emulator_frame = std::time::Duration::from_nanos(nanos as u64);
        let mut render = false;
        self.emulator_time += frame_time;
        while self.emulator_time > emulator_frame {
            let new_time = std::time::Instant::now();
            let new_emulated_fps = 1_000_000_000.0
                / new_time.duration_since(self.last_emulated_frame).as_nanos() as f64;
            self.emulator_fps = (self.emulator_fps * 0.95) + (0.05 * new_emulated_fps);
            self.emulator_time -= emulator_frame;
            if self.emulator_time < emulator_frame {
                self.last_emulated_frame = new_time;
            }
            render = true;
        }

        #[cfg(feature = "puffin")]
        puffin::profile_scope!("frame rendering");

        if self.filter.is_none() && self.sound_stream.is_some() {
            println!("Initializing with sample rate {}", c.local.get_sound_rate());
            let rf = c.local.get_sound_rate() as f32;
            let sampling_frequency = c.cpu_frequency();
            let filter_coeff = biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                biquad::Hertz::<f32>::from_hz(sampling_frequency).unwrap(),
                biquad::Hertz::<f32>::from_hz(rf / 2.2).unwrap(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap();
            self.filter = Some(biquad::DirectForm1::<f32>::new(filter_coeff));
            if let Some(sound) = &mut self.sound {
                sound.set_audio_interval(sampling_frequency / rf);
            }
        }

        let quit = false;
        let mut windows_to_create = vec![];

        {
            egui.egui_ctx.input(|i| {
                for index in 0..2 {
                    let controller = c.mb.get_controller_mut(index);
                    for contr in controller.get_buttons_iter_mut() {
                        let cnum = index;
                        let button_config = &c.local.configuration.controller_config[cnum as usize];
                        contr.update_egui_buttons(i, button_config);
                    }
                }
            });
            if let Some(olocal) = &mut c.olocal {
                while let Some(_e) = olocal.gilrs.next_event() {}
            }
            if let Some(olocal) = &mut c.olocal {
                let gilrs = &mut olocal.gilrs;
                for (id, gamepad) in gilrs.gamepads() {
                    let gs = gamepad.state();
                    for (code, button) in gs.buttons() {
                        for index in 0..4 {
                            let controller = c.mb.get_controller_mut(index);
                            for contr in controller.get_buttons_iter_mut() {
                                let cnum = index;
                                let button_config =
                                    &c.local.configuration.controller_config[cnum as usize];
                                contr.update_gilrs_buttons(id, code, button, button_config);
                            }
                        }
                    }
                    for (code, axis) in gs.axes() {
                        for index in 0..4 {
                            let controller = c.mb.get_controller_mut(index);
                            for contr in controller.get_buttons_iter_mut() {
                                let cnum = index;
                                let button_config =
                                    &c.local.configuration.controller_config[cnum as usize];
                                contr.update_gilrs_axes(id, code, axis, button_config);
                            }
                        }
                    }
                }
            }

            if let Some(olocal) = &mut c.olocal {
                if let Some(network) = &mut olocal.network {
                    match network.role() {
                        NodeRole::Player => {
                            let controller = c.mb.get_controller_ref(0);
                            for i in 0..2 {
                                let _e = network.send_controller_data(i, controller.button_data());
                            }
                        }
                        NodeRole::PlayerHost => {
                            for i in 0..4 {
                                if let Some(bc) = network.get_button_data(i) {
                                    let controller = c.mb.get_controller_mut(i);
                                    if let Some(con) = controller.get_buttons_iter_mut().next() {
                                        *con = bc;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(olocal) = &mut c.olocal {
            if let Some(network) = &mut olocal.network {
                match network.role() {
                    NodeRole::Observer | NodeRole::Player => {
                        if render {
                            network.get_video_data(&mut c.local.image);
                            if let Some(sound) = &mut self.sound {
                                network.push_audio(sound);
                            }
                        }
                        render = false;
                    }
                    NodeRole::PlayerHost => {
                        if let Some(a) = network.get_sound_stream() {
                            self.audio_streaming.push(a);
                        }
                    }
                    _ => {}
                }
            }
        }

        {
            let mut tvec = Vec::with_capacity(self.audio_streaming.len());
            let quantity = self.audio_streaming.len();
            for _i in 0..quantity {
                let e = self.audio_streaming.pop().unwrap();
                if let Some(_a) = e.upgrade() {
                    tvec.push(e);
                } else {
                    println!("Dropping a weak audio producer");
                }
            }
            self.audio_streaming = tvec;
        }

        if render {
            let mut sound = Vec::new();
            if let Some(s) = &mut self.sound {
                sound.push(s);
            }
            if let Some(s) = self.recording.get_sound() {
                sound.push(s);
            }
            'emulator_loop: loop {
                #[cfg(feature = "debugger")]
                {
                    if !c.paused {
                        c.cycle_step(&mut sound, &mut self.audio_streaming, &mut self.filter);
                        if c.cpu.breakpoint_option() && (c.cpu.breakpoint() || c.single_step) {
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
                        if !self.paused {
                            let image = c
                                .cpu_peripherals
                                .ppu_get_frame()
                                .to_pixels_egui()
                                .resize(c.local.configuration.scaler);
                            c.local.image = image;
                        }
                        self.recording.send_frame(&c.local.image);
                        if let Some(olocal) = &mut c.olocal {
                            if let Some(network) = &mut olocal.network {
                                if network.role() == NodeRole::PlayerHost {
                                    let _e = network.video_data(&c.local.image);
                                }
                            }
                        }

                        if self.mouse_delay > 0 {
                            self.mouse_delay -= 1;
                            if self.mouse_delay == 0 {
                                self.mouse = false;
                                self.mouse_miss = false;
                            }
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
        }

        if self.paused {
            let image = c
                .cpu_peripherals
                .ppu_get_frame()
                .to_pixels_egui()
                .resize(c.local.configuration.scaler);
            c.local.image = image;
        }
        let image = c.local.image.clone().to_egui();

        if self.texture.is_none() {
            self.texture = Some(egui.egui_ctx.load_texture(
                "NES_PPU",
                image,
                egui_multiwin::egui::TextureOptions::NEAREST,
            ));
        } else if let Some(t) = &mut self.texture {
            if t.size()[0] != image.width() || t.size()[1] != image.height() {
                self.texture = Some(egui.egui_ctx.load_texture(
                    "NES_PPU",
                    image,
                    egui_multiwin::egui::TextureOptions::NEAREST,
                ));
            } else {
                t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
            }
        }

        let mut save_state = false;
        let mut load_state = false;
        let mut rewind_state = false;
        //Some(true) means start recording, Some(false) means stop recording
        let mut start_stop_recording: Option<bool> = None;

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
                        load_state = true;
                        ui.close_menu();
                    }

                    if !self.recording.is_recording() {
                        let button = egui_multiwin::egui::Button::new("Begin recording");
                        if ui.add_enabled(true, button).clicked()
                            || egui
                                .egui_ctx
                                .input(|i| i.key_pressed(egui_multiwin::egui::Key::F6))
                        {
                            start_stop_recording = Some(true);
                            ui.close_menu();
                        }
                    }
                    else {
                        let button = egui_multiwin::egui::Button::new("Stop recording");
                        if ui.add_enabled(true, button).clicked()
                            || egui
                                .egui_ctx
                                .input(|i| i.key_pressed(egui_multiwin::egui::Key::F6))
                        {
                            start_stop_recording = Some(false);
                            ui.close_menu();
                        }
                    }

                    let button = egui_multiwin::egui::Button::new("Open data path");
                    if ui.add_enabled(true, button).clicked()
                    {
                        open::that_in_background(c.local.get_save_other());
                        ui.close_menu();
                    }

                    let button = egui_multiwin::egui::Button::new("Networking");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(super::network::Window::new_request());
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let button = egui_multiwin::egui::Button::new("Configuration");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(super::configuration::Window::new_request());
                        ui.close_menu();
                    }
                    let button = egui_multiwin::egui::Button::new("Controllers");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(super::controllers::Window::new_request());
                        ui.close_menu();
                    }
                    let button = egui_multiwin::egui::Button::new("Game genie");
                    if ui.add_enabled(true, button).clicked() {
                        windows_to_create.push(super::genie::Window::new_request());
                        ui.close_menu();
                    }
                });
                #[cfg(feature = "debugger")]
                {
                    ui.menu_button("Debug", |ui| {
                        if ui.button("Debugger").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::debug_window::DebugSnesWindow::new_request());
                        }
                        if ui.button("Dump CPU Data").clicked() {
                            ui.close_menu();
                            windows_to_create.push(super::cpu_memory_dump_window::CpuMemoryDumpWindow::new_request());
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

        if egui
            .egui_ctx
            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F7))
        {
            rewind_state = true;
        }

        if egui
            .egui_ctx
            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F11))
        {
            if c.mb.speed_ratio < 1.0 {
                c.mb.speed_ratio = 1.0;
            } else {
                c.mb.speed_ratio = 0.5;
            }
        }

        if egui
            .egui_ctx
            .input(|i| i.key_pressed(egui_multiwin::egui::Key::F12))
        {
            match window.fullscreen() {
                Some(_a) => window.set_fullscreen(None),
                None => {
                    window.set_fullscreen(Some(
                        egui_multiwin::winit::window::Fullscreen::Borderless(None),
                    ));
                }
            }
        }

        let record_path = c.local.record_path();
        if let Some(rec) = start_stop_recording {
            if rec {
                c.local.resolution_locked = true;
                let sampling_frequency = c.cpu_frequency();
                let tn = chrono::Local::now();
                let mut recpath = record_path.clone();
                recpath.push(format!("{}.avi", tn.format("%Y-%m-%d %H%M%S")));
                self.recording.start(
                    &self.have_gstreamer,
                    &c.local.image,
                    c.ppu_frame_rate() as u8,
                    recpath,
                    sampling_frequency,
                );
            } else {
                c.local.resolution_locked = false;
                loop {
                    if self.recording.stop().is_ok() {
                        break;
                    }
                }
            }
        }

        let name = if let Some(cart) = c.mb.cartridge() {
            cart.save_name()
        } else {
            "state.bin".to_string()
        };
        let ppp = <std::path::PathBuf as std::str::FromStr>::from_str(&name).unwrap();
        let mut save_path = c.local.save_path();
        save_path.push(ppp.file_name().unwrap());
        if save_state {
            let mut path = save_path.clone();
            path.pop();
            let _ = std::fs::create_dir_all(path);
            let state = Box::new(c.serialize());
            let _e = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(save_path.clone())
                .unwrap()
                .write_all(&state);
        }

        if load_state {
            if let Ok(a) = std::fs::read(save_path) {
                let e = c.deserialize(a);
                if e.is_err() {
                    println!("Error loading state {:?}", e);
                }
            }
        }

        if rewind_state {
            let e = c.deserialize(self.rewinds[1].clone());
            if e.is_err() {
                println!("Error loading rewind state {:?}", e);
            }
        }

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.vertical_centered(|ui| {
                let size = ui.available_size();
                ui.horizontal_centered(|ui| {
                    if let Some(olocal) = &mut c.olocal {
                        if let Some(network) = &mut olocal.network {
                            let myc = network.get_controller_id();
                            if network.role() == NodeRole::Observer
                                || network.role() == NodeRole::Player
                            {
                                ui.vertical(|ui| {
                                    for i in 0..4 {
                                        if ui
                                            .add(egui::SelectableLabel::new(
                                                myc == Some(i),
                                                format!("Controller {}", i),
                                            ))
                                            .clicked()
                                        {
                                            let _e = network.request_controller(i);
                                        }
                                    }
                                    if ui
                                        .add(egui::SelectableLabel::new(
                                            myc.is_none(),
                                            "No controller",
                                        ))
                                        .clicked()
                                    {
                                        let _e = network.release_controller();
                                    }
                                });
                            }
                        }
                    }

                    if let Some(t) = &self.texture {
                        let zoom = (size.x / t.size()[0] as f32).min(size.y / t.size()[1] as f32);
                        let r = ui.add(
                            egui::Image::from_texture(egui::load::SizedTexture {
                                id: t.id(),
                                size: egui_multiwin::egui::Vec2 {
                                    x: t.size()[0] as f32 * zoom,
                                    y: t.size()[1] as f32 * zoom,
                                },
                            })
                            .sense(egui::Sense::click_and_drag()),
                        );
                        if r.clicked() || r.dragged() {
                            self.mouse = true;
                            self.mouse_miss = false;
                            self.mouse_delay = 10;
                        } else if r.clicked_by(egui::PointerButton::Secondary)
                            || r.dragged_by(egui::PointerButton::Secondary)
                        {
                            self.mouse = true;
                            self.mouse_miss = true;
                            self.mouse_delay = 10;
                        }
                        if r.hovered() {
                            if let Some(pos) = r.hover_pos() {
                                let coord = pos - r.rect.left_top();
                                c.cpu_peripherals.ppu.bg_debug =
                                    Some(((coord.x / zoom) as u8, (coord.y / zoom) as u8));

                                let pixel = c.local.image.get_pixel(coord / zoom);
                                self.mouse_vision = !self.mouse_miss
                                    && pixel.r() > 10
                                    && pixel.g() > 10
                                    && pixel.b() > 10;

                                //println!("Hover at {:?}", pos - r.rect.left_top());
                            } else {
                                self.mouse_vision = false;
                            }
                        } else {
                            self.mouse_vision = false;
                        }
                    }
                });
            });
            window.set_title(&format!(
                "UglyOldBob NES Emulator - {:.0}/{:.0} FPS",
                self.emulator_fps, self.fps
            ));
            if c.mb
                .get_controller_ref(0)
                .button_data()
                .pressed(crate::controller::BUTTON_COMBO_LEFT)
            {
                ui.label("LEFT");
            }
        });

        if let Some(s) = &mut self.sound_stream {
            if c.paused && !self.paused {
                self.paused = s.pause().is_ok();
            }
            if !c.paused && self.paused {
                self.paused = s.play().is_err();
            }
        }

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
