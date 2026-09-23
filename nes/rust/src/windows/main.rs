//! The main window of the emulator
//!
use std::io::Write;

use crate::{
    controller::{ButtonCombination, NesControllerTrait},
    NesEmulatorData,
};

#[cfg(not(target_os = "android"))]
use crate::emulator_data::BluetoothControllerOwner;

#[cfg(not(target_os = "android"))]
use common_emulator::network::NodeRole;

use common_emulator::audio::AudioProducerWithRate;
#[cfg(not(target_os = "android"))]
use common_emulator::recording::Recording;

use eframe::egui;
use std::collections::HashMap;

#[cfg(target_os = "android")]
struct OnscreenButton {
    button_opacity: f32,
}

#[cfg(target_os = "android")]
impl OnscreenButton {
    fn new(opacity: f32) -> Self {
        Self {
            button_opacity: opacity,
        }
    }

    fn btn_colors(&self, pressed: bool) -> (egui::Color32, egui::Stroke) {
        let alpha = (self.button_opacity * 255.0) as u8;
        if pressed {
            (
                egui::Color32::from_rgba_unmultiplied(220, 80, 80, alpha),
                egui::Stroke::new(
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                ),
            )
        } else {
            (
                egui::Color32::from_rgba_unmultiplied(60, 60, 70, alpha),
                egui::Stroke::new(
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(100, 100, 120, alpha),
                ),
            )
        }
    }

    fn paint_btn(&self, ui: &egui::Ui, rect: egui::Rect, label: &str, pressed: bool, round: f32) {
        let (fill, stroke) = self.btn_colors(pressed);
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(round as u8),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        if !label.is_empty() {
            let alpha = (self.button_opacity * 255.0) as u8;
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(rect.height() * 0.38),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
            );
        }
    }
}

/// The struct for the main window of the emulator.
pub struct MainNesWindow {
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
    pub c: NesEmulatorData,
    /// The calculated frames per second performance of the program. Will be higher than the fps of the emulator.
    fps: f64,
    /// The calculated frames per second performance of the emulator.
    emulator_fps: f64,
    /// The producing half of the ring buffer used for audio.
    sound: Option<AudioProducerWithRate>,
    /// The texture used for rendering the ppu image.
    pub texture: Option<egui::TextureHandle>,
    /// The filter used for audio playback, filtering out high frequency noise, increasing the quality of audio playback.
    filter: Option<biquad::DirectForm1<f32>>,
    /// The stream used for audio playback during emulation
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
    #[cfg(not(target_os = "android"))]
    /// The result of opening gstreamer
    have_gstreamer: Result<(), gstreamer::glib::Error>,
    #[cfg(not(target_os = "android"))]
    /// The recording object
    recording: Recording,
    /// The audio objects for a streaming server
    audio_streaming: Vec<std::sync::Weak<std::sync::Mutex<AudioProducerWithRate>>>,
    /// The percentage of time taken for rendering
    render_percent: f32,
    /// The open rom window
    open_rom_window: Option<crate::windows::rom_finder::RomFinder>,
    #[cfg(not(target_os = "android"))]
    /// The networking window
    networking_window: Option<crate::windows::network::Window>,
    /// The configuration window
    configuration_window: Option<crate::windows::configuration::Window>,
    /// The controllers window
    controllers_window: Option<crate::windows::controllers::Window>,
    /// The game genie window
    game_genie_window: Option<crate::windows::genie::Window>,
    #[cfg(feature = "debugger")]
    /// The cartridge dump window
    cartridge_dump_window: Option<crate::windows::cartridge_dump::CartridgeMemoryDumpWindow>,
    #[cfg(feature = "debugger")]
    /// The prg ram dump window
    cartridge_prm_ram_dump_window:
        Option<crate::windows::cartridge_prg_ram_dump::CartridgeMemoryDumpWindow>,
    #[cfg(feature = "debugger")]
    /// The cpu memory dump window
    cpu_memory_dump_window: Option<crate::windows::cpu_memory_dump_window::CpuMemoryDumpWindow>,
    #[cfg(feature = "debugger")]
    /// The debug window
    debug_window: Option<crate::windows::debug_window::DebugNesWindow>,
    #[cfg(feature = "debugger")]
    /// The nametable dump window
    nametable_dump_window: Option<crate::windows::name_table_dump_window::DumpWindow>,
    #[cfg(feature = "debugger")]
    /// The pattern table dump window
    pattern_table_dump_window: Option<crate::windows::pattern_table_dump_window::DumpWindow>,
    #[cfg(feature = "debugger")]
    /// The ppu memory dump window
    ppu_memory_dump_window: Option<crate::windows::ppu_memory_dump_window::PpuMemoryDumpWindow>,
    #[cfg(feature = "rom_status")]
    /// The rom checker window
    pub rom_checker_window: Option<crate::windows::rom_checker::Window>,
    #[cfg(feature = "debugger")]
    /// the sprite dump window
    sprite_dump_window: Option<crate::windows::sprite_dump_window::DumpWindow>,
    #[cfg(target_os = "android")]
    android_menubar: AndroidMenuBar,
    #[cfg(target_os = "android")]
    on_screen_buttons: [OnscreenButton; 11],
    #[cfg(target_os = "android")]
    on_screen_arrows: [OnscreenButton; 8],
    active_touches: HashMap<u64, egui::Pos2>,
}

#[cfg(target_os = "android")]
struct AndroidMenuBar {
    show_menubar: bool,
}

#[cfg(target_os = "android")]
impl Default for AndroidMenuBar {
    fn default() -> Self {
        Self {
            show_menubar: false,
        }
    }
}
#[cfg(target_os = "android")]
impl AndroidMenuBar {
    const MENU_HEIGHT: f32 = 60.0;
    const SNAP_SPEED: f32 = 0.30;

    fn menu_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
        ui.add_sized([100.0, 52.0], egui::Button::new(text))
    }

    /// This function handles the android menu system, returning true if it showed the menu system
    /// This indicates that the emulator should be paused
    fn show(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) -> bool {
        // --------------------------------------------------------
        // Normal TopBottomPanel
        // --------------------------------------------------------

        if self.show_menubar {
            egui::TopBottomPanel::top("android_menu_bar")
                .exact_height(Self::MENU_HEIGHT)
                .show_inside(ui, |menu_ui| {
                    egui::MenuBar::new().ui(menu_ui, |menu_ui| {
                        let rect = menu_ui.max_rect();

                        // Normal menu.
                        menu_ui.horizontal_centered(|ui| {
                            if ui.add_sized([56.0, 52.0], egui::Button::new("☰")).clicked() {
                                self.show_menubar = false;
                            }

                            ui.separator();

                            if Self::menu_button(ui, "Load").clicked() {
                                // ...
                            }

                            if Self::menu_button(ui, "Save").clicked() {
                                // ...
                            }

                            if Self::menu_button(ui, "Settings").clicked() {
                                // ...
                            }
                        });
                    });
                });
        }

        // --------------------------------------------------------
        // Menu button when closed
        // --------------------------------------------------------

        if !self.show_menubar {
            egui::TopBottomPanel::top("android_menu_bar")
                .exact_height(Self::MENU_HEIGHT)
                .show_inside(ui, |ui| {
                    let response = ui.add_sized([56.0, 56.0], egui::Button::new("☰"));

                    if response.clicked() {
                        self.show_menubar = true;
                    }
                });
        }
        self.show_menubar
    }
}

impl MainNesWindow {
    pub fn new(
        c: NesEmulatorData,
        _rate: u32,
        producer: Option<AudioProducerWithRate>,
        stream: Option<cpal::Stream>,
    ) -> Self {
        use std::time::Duration;
        #[cfg(not(target_os = "android"))]
        let have_gstreamer = gstreamer::init();
        #[cfg(not(target_os = "android"))]
        gstreamer::log::set_threshold_from_string("appsink:WARN", false);
        #[cfg(not(target_os = "android"))]
        if let Err(e) = &have_gstreamer {
            log::error!("Failed to open gstreamer: {:?}", e);
        }
        #[cfg(feature = "rom_status")]
        let rom_checker_window = crate::windows::rom_checker::Window::new(&c);

        #[cfg(target_os = "android")]
        let on_screen_buttons = [
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
        ];

        #[cfg(target_os = "android")]
        let on_screen_arrows = [
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
            OnscreenButton::new(1.0),
        ];

        Self {
            c,
            #[cfg(not(target_os = "android"))]
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
            #[cfg(not(target_os = "android"))]
            recording: Recording::new(),
            audio_streaming: Vec::new(),
            render_percent: 0.0,
            configuration_window: None,
            controllers_window: None,
            game_genie_window: None,
            #[cfg(not(target_os = "android"))]
            networking_window: None,
            open_rom_window: None,
            #[cfg(feature = "debugger")]
            cartridge_dump_window: None,
            #[cfg(feature = "debugger")]
            cartridge_prm_ram_dump_window: None,
            #[cfg(feature = "debugger")]
            cpu_memory_dump_window: None,
            #[cfg(feature = "debugger")]
            debug_window: Some(crate::windows::debug_window::DebugNesWindow::new(&c)),
            #[cfg(feature = "debugger")]
            nametable_dump_window: None,
            #[cfg(feature = "debugger")]
            pattern_table_dump_window: None,
            #[cfg(feature = "debugger")]
            ppu_memory_dump_window: None,
            #[cfg(feature = "rom_status")]
            rom_checker_window: Some(rom_checker_window),
            #[cfg(feature = "debugger")]
            sprite_dump_window: None,
            #[cfg(target_os = "android")]
            android_menubar: Default::default(),
            #[cfg(target_os = "android")]
            on_screen_buttons,
            #[cfg(target_os = "android")]
            on_screen_arrows,
            active_touches: HashMap::new(),
        }
    }

    #[cfg(target_os = "android")]
    fn touch_hits(&self, rect: Option<egui::Rect>) -> bool {
        rect.map_or(false, |r| {
            self.active_touches.values().any(|&pos| r.contains(pos))
        })
    }
}

impl eframe::App for MainNesWindow {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            puffin_egui::profiler_window(&egui.egui_ctx);
        }

        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Touch { id, phase, pos, .. } = event {
                    match phase {
                        egui::TouchPhase::Start | egui::TouchPhase::Move => {
                            self.active_touches.insert(id.0, *pos);
                        }
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            self.active_touches.remove(&id.0);
                        }
                    }
                }
            }
        });

        self.c.check_network();

        let time_now = std::time::Instant::now();
        let frame_time = time_now.duration_since(self.last_frame_time);
        self.last_frame_time = time_now;

        if self.rewind_point.is_none() {
            self.rewind_point = Some(time_now);
            let p = self.c.serialize();
            self.rewinds[0] = p.clone();
            self.rewinds[1] = p.clone();
            self.rewinds[2] = p.clone();
        } else if let Some(t) = self.rewind_point {
            if let Some(rew) = self.c.local.configuration.rewind_interval {
                if time_now.duration_since(t) > rew {
                    self.rewinds[2] = self.rewinds[1].clone();
                    self.rewinds[1] = self.rewinds[0].clone();
                    self.rewinds[0] = self.c.serialize();
                    self.rewind_point = Some(time_now);
                }
            }
        }

        let new_fps = 1_000_000_000.0 / frame_time.as_nanos() as f64;
        self.fps = (self.fps * 0.95) + (0.05 * new_fps);

        self.c.mb.get_controller_mut(0).rapid_fire(frame_time);
        self.c.mb.get_controller_mut(1).rapid_fire(frame_time);
        self.c.mb.get_controller_mut(2).rapid_fire(frame_time);
        self.c.mb.get_controller_mut(3).rapid_fire(frame_time);

        let nanos = 1_000_000_000.0 / (self.c.ppu_frame_rate() * self.c.mb.speed_ratio);
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

        #[cfg(target_os = "android")]
        if self.android_menubar.show(ui, frame) {
            render = false;
        }

        #[cfg(feature = "puffin")]
        puffin::profile_scope!("frame rendering");

        // The audio filter gates ALL audio sample generation (see
        // NesApu::build_audio_sample), including the samples fed to the streaming
        // pipeline. Previously this was only initialized when a local audio output
        // device was available. When hosting a stream on a machine without working
        // local audio output, the filter stayed None, so no audio was produced and
        // mpegtsmux stalled waiting for an audio stream, breaking streaming
        // entirely. Initialize it whenever we have local audio OR we are hosting a
        // stream, falling back to the streaming sample rate when no device exists.
        if self.filter.is_none()
            && (self.sound_stream.is_some() || !self.audio_streaming.is_empty())
        {
            let local_rate = self.c.local.get_sound_rate();
            let rf = if local_rate == 0 {
                // Matches the audio rate used by the streaming pipeline.
                44100.0
            } else {
                local_rate as f32
            };
            log::info!("Initializing audio filter with sample rate {}", rf);
            let sampling_frequency = self.c.cpu_frequency();
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

        {
            ui.ctx().input(|i| {
                for index in 0..4 {
                    let controller = self.c.mb.get_controller_mut(index);
                    if !controller.should_ignore_local_inputs() {
                        if let crate::controller::NesController::Zapper(z) = controller {
                            z.provide_zapper_data(self.mouse, self.mouse_vision);
                        } else {
                            for contr in controller.get_buttons_iter_mut() {
                                let cnum = index;
                                let button_config =
                                    &self.c.local.configuration.controller_config[cnum as usize];
                                contr.update_egui_buttons(i, button_config);
                            }
                        }
                    }
                }
            });
            #[cfg(not(target_os = "android"))]
            if let Some(olocal) = &mut self.c.olocal {
                while let Some(_e) = olocal.gilrs.next_event() {}
            }
            #[cfg(not(target_os = "android"))]
            if let Some(olocal) = &mut self.c.olocal {
                let gilrs = &mut olocal.gilrs;
                for (id, gamepad) in gilrs.gamepads() {
                    let gs = gamepad.state();
                    for (code, button) in gs.buttons() {
                        for index in 0..4 {
                            let controller = self.c.mb.get_controller_mut(index);
                            if !controller.should_ignore_local_inputs() {
                                if let crate::controller::NesController::Zapper(_z) = controller {
                                } else {
                                    for contr in controller.get_buttons_iter_mut() {
                                        let cnum = index;
                                        let button_config =
                                            &self.c.local.configuration.controller_config
                                                [cnum as usize];
                                        contr.update_gilrs_buttons(id, code, button, button_config);
                                    }
                                }
                            }
                        }
                    }
                    for (code, axis) in gs.axes() {
                        for index in 0..4 {
                            let controller = self.c.mb.get_controller_mut(index);
                            if !controller.should_ignore_local_inputs() {
                                if let crate::controller::NesController::Zapper(_z) = controller {
                                } else {
                                    for contr in controller.get_buttons_iter_mut() {
                                        let cnum = index;
                                        let button_config =
                                            &self.c.local.configuration.controller_config
                                                [cnum as usize];
                                        contr.update_gilrs_axes(id, code, axis, button_config);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            #[cfg(not(target_os = "android"))]
            self.c.check_bluetooth_controllers();

            #[cfg(not(target_os = "android"))]
            if let Some(olocal) = &mut self.c.olocal {
                if let Some(network) = &mut olocal.network {
                    match network.role() {
                        NodeRole::Player => {
                            let controller = self.c.mb.get_controller_ref(0);
                            for i in 0..4 {
                                let _e = network.send_controller_data(
                                    i,
                                    bincode::serialize(&controller.button_data()).unwrap(),
                                );
                            }
                        }
                        NodeRole::PlayerHost => {
                            for i in 0..4 {
                                if let Some(bc) = network.get_button_data(i) {
                                    if let Ok(bc) = bincode::deserialize::<ButtonCombination>(bc) {
                                        let controller = self.c.mb.get_controller_mut(i);
                                        if let Some(con) = controller.get_buttons_iter_mut().next()
                                        {
                                            *con = bc;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        #[cfg(not(target_os = "android"))]
        if let Some(olocal) = &mut self.c.olocal {
            if let Some(network) = &mut olocal.network {
                match network.role() {
                    NodeRole::Observer | NodeRole::Player => {
                        if render {
                            network.get_video_data(&mut self.c.local.image);
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
                    log::info!("Dropping a weak audio producer");
                }
            }
            self.audio_streaming = tvec;
        }

        if render {
            let mut sound = Vec::new();
            if let Some(s) = &mut self.sound {
                sound.push(s);
            }
            #[cfg(not(target_os = "android"))]
            if let Some(s) = self.recording.get_sound() {
                sound.push(s);
            }
            'emulator_loop: loop {
                #[cfg(feature = "debugger")]
                {
                    if !self.c.paused {
                        self.c
                            .cycle_step(&mut sound, &mut self.audio_streaming, &mut self.filter);
                        if self.c.cpu_clock_counter == 0
                            && self.c.cpu.breakpoint_option()
                            && (self.c.cpu.breakpoint() || self.c.single_step)
                        {
                            self.c.paused = true;
                            self.c.single_step = false;
                            break 'emulator_loop;
                        }
                    } else {
                        break 'emulator_loop;
                    }
                    if self.c.cpu_peripherals.ppu_frame_end() {
                        if self.c.wait_for_frame_end {
                            log::debug!("End of frame for debugger");
                            self.c.paused = true;
                            self.c.wait_for_frame_end = false;
                        }
                        if !self.paused {
                            let image = self
                                .c
                                .cpu_peripherals
                                .ppu_get_frame()
                                .to_pixels_egui()
                                .resize(self.c.local.configuration.scaler);
                            self.c.local.image = image;
                        }
                        self.recording.send_frame(&self.c.local.image);
                        if let Some(olocal) = &mut self.c.olocal {
                            if let Some(network) = &mut olocal.network {
                                if network.role() == NodeRole::PlayerHost {
                                    let _e = network.video_data(&self.c.local.image);
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
                    {
                        self.c
                            .cycle_step(&mut sound, &mut self.audio_streaming, &mut self.filter);
                    }
                    if self.c.cpu_peripherals.ppu_frame_end() {
                        if !self.paused {
                            let image = self
                                .c
                                .cpu_peripherals
                                .ppu_get_frame()
                                .to_pixels_egui()
                                .resize(self.c.local.configuration.scaler);
                            self.c.local.image = image;
                        }
                        #[cfg(not(target_os = "android"))]
                        self.recording.send_frame(&self.c.local.image);
                        #[cfg(not(target_os = "android"))]
                        if let Some(olocal) = &mut self.c.olocal {
                            if let Some(network) = &mut olocal.network {
                                if network.role() == NodeRole::PlayerHost {
                                    let _e = network.video_data(&self.c.local.image);
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
            }
            let time_after_render = std::time::Instant::now();
            let render_time = time_after_render.duration_since(time_now).as_nanos();
            let render_percent = render_time as f32 / nanos;
            self.render_percent = (self.render_percent * 0.95) + (0.05 * render_percent);
        }

        if self.paused {
            let image = self
                .c
                .cpu_peripherals
                .ppu_get_frame()
                .to_pixels_egui()
                .resize(self.c.local.configuration.scaler);
            self.c.local.image = image;
        }
        let image = self.c.local.image.clone().to_egui();

        if self.texture.is_none() {
            self.texture = Some(ui.ctx().load_texture(
                "NES_PPU",
                image,
                egui::TextureOptions::NEAREST,
            ));
        } else if let Some(t) = &mut self.texture {
            if t.size()[0] != image.width() || t.size()[1] != image.height() {
                self.texture = Some(ui.ctx().load_texture(
                    "NES_PPU",
                    image,
                    egui::TextureOptions::NEAREST,
                ));
            } else {
                t.set_partial([0, 0], image, egui::TextureOptions::NEAREST);
            }
        }

        let mut save_state = false;
        let mut load_state = false;
        let mut rewind_state = false;
        //Some(true) means start recording, Some(false) means stop recording
        let mut start_stop_recording: Option<bool> = None;

        #[cfg(not(target_os = "android"))]
        if let Some(olocal) = &mut self.c.olocal {
            let mut pop_front = false;
            let mut pending_player = None;
            if let Some(pending) = olocal.pending_bluetooth_controllers.front() {
                ui.ctx().show_viewport_immediate(
                    egui::ViewportId::from_hash_of("CONTROLLERS_WINDOW"),
                    egui::ViewportBuilder::default()
                        .with_title("Controller Config")
                        .with_inner_size([400.0, 300.0]),
                    |ui, _class| {
                        egui::CentralPanel::default().show_inside(ui, |ui| {
                            ui.label(format!("Select the player number for {:x?}", pending.addr));
                            ui.horizontal(|ui| {
                                for i in 0..4 {
                                    let btn = egui::Button::new(format!("{}", i + 1))
                                        .min_size([50.0, 50.0].into());
                                    if ui.add(btn).clicked() {
                                        let _ = pending.response.blocking_send(
                                            crate::BluetoothControllerResponse::SetPlayerNumber(i),
                                        );
                                        pop_front = true;
                                        pending_player = Some(i);
                                    }
                                }
                            });
                        });
                    },
                );
            }
            if pop_front {
                if let Some(a) = olocal.pending_bluetooth_controllers.pop_front() {
                    let addr = a.addr;
                    if let Some(player) = pending_player {
                        let nes_controller = self.c.mb.get_controller_mut(player);
                        nes_controller.ignore_local_inputs(true);
                        let c = &mut olocal.bluetooth_controllers[player as usize];
                        let mut cc = crate::controller::ControllerConfig::new();
                        cc.set_keys_bluetooth(addr);
                        let d = BluetoothControllerOwner {
                            address: addr,
                            controller_config: cc,
                        };
                        *c = Some(std::sync::Arc::new(std::sync::Mutex::new(d)));
                    }
                }
            }
        }
        {
            let mut quit_rom_window = false;
            if let Some(win) = &mut self.open_rom_window {
                win.show(ui, &mut quit_rom_window, &mut self.c);
            }
            if quit_rom_window {
                self.open_rom_window.take();
            }
        }
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.configuration_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.configuration_window.take();
            }
        }
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.controllers_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.controllers_window.take();
            }
        }
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.game_genie_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.game_genie_window.take();
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.networking_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.networking_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.cartridge_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.cartridge_dump_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.cartridge_prm_ram_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.cartridge_prm_ram_dump_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.cpu_memory_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.cpu_memory_dump_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.debug_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.debug_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.nametable_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.nametable_dump_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.pattern_table_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.pattern_table_dump_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.ppu_memory_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.ppu_memory_dump_window.take();
            }
        }
        #[cfg(feature = "rom_status")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.rom_checker_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.rom_checker_window.take();
            }
        }
        #[cfg(feature = "debugger")]
        {
            let mut quit_window = false;
            if let Some(win) = &mut self.sprite_dump_window {
                win.show(ui, &mut quit_window, &mut self.c);
            }
            if quit_window {
                self.sprite_dump_window.take();
            }
        }

        #[cfg(not(target_os = "android"))]
        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let is_fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.menu_button("File", |ui| {
                    let button = egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {
                        self.open_rom_window = Some(crate::windows::rom_finder::RomFinder::new());
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    let button = egui::Button::new("Save state - F5");
                    if ui.add_enabled(true, button).clicked()
                        || ui.ctx().input(|i| i.key_pressed(egui::Key::F5))
                    {
                        save_state = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    let button = egui::Button::new("Load state - F6");
                    if ui.add_enabled(true, button).clicked()
                        || ui.ctx().input(|i| i.key_pressed(egui::Key::F6))
                    {
                        load_state = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    #[cfg(not(target_os = "android"))]
                    if !self.recording.is_recording() {
                        let button = egui::Button::new("Begin recording - F7");
                        if ui.add_enabled(true, button).clicked()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::F7))
                        {
                            start_stop_recording = Some(true);
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    } else {
                        let button = egui::Button::new("Stop recording - F7");
                        if ui.add_enabled(true, button).clicked()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::F7))
                        {
                            start_stop_recording = Some(false);
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    }

                    let button = egui::Button::new("Rewind - F8");
                    if ui.add_enabled(true, button).clicked()
                        || ui.ctx().input(|i| i.key_pressed(egui::Key::F8))
                    {
                        rewind_state = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    if self.c.mb.speed_ratio < 1.0 {
                        let button = egui::Button::new("Disable slow mode - F11");
                        if ui.add_enabled(true, button).clicked()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::F11))
                        {
                            self.c.mb.speed_ratio = 1.0;
                        }
                    } else {
                        let button = egui::Button::new("Enable slow mode - F11");
                        if ui.add_enabled(true, button).clicked()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::F11))
                        {
                            self.c.mb.speed_ratio = 0.5;
                        }
                    }

                    let button = egui::Button::new("Toggle fullscreen - F12");
                    if ui.add_enabled(true, button).clicked()
                        || ui.ctx().input(|i| i.key_pressed(egui::Key::F12))
                    {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    let button = egui::Button::new("Open data path");
                    if ui.add_enabled(true, button).clicked() {
                        open::that_in_background(self.c.local.get_save_other());
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    #[cfg(not(target_os = "android"))]
                    {
                        let button = egui::Button::new("Networking");
                        if ui.add_enabled(true, button).clicked() {
                            self.networking_window = Some(crate::windows::network::Window::new());
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    }

                    let button = egui::Button::new("Exit");
                    if ui.add_enabled(true, button).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_kind(egui::UiKind::Menu);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let button = egui::Button::new("Configuration");
                    if ui.add_enabled(true, button).clicked() {
                        self.configuration_window =
                            Some(crate::windows::configuration::Window::new());
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    let button = egui::Button::new("Controllers");
                    if ui.add_enabled(true, button).clicked() {
                        self.controllers_window = Some(crate::windows::controllers::Window::new());
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    let button = egui::Button::new("Game genie");
                    if ui.add_enabled(true, button).clicked() {
                        self.game_genie_window = Some(crate::windows::genie::Window::new());
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    if ui.button("Reset").clicked() {
                        ui.close_kind(egui::UiKind::Menu);
                        self.c.reset();
                    }
                    if ui.button("Power cycle").clicked() {
                        ui.close_kind(egui::UiKind::Menu);
                        self.c.power_cycle();
                    }
                });
                #[cfg(feature = "debugger")]
                {
                    ui.menu_button("Debug", |ui| {
                        if ui.button("Debugger").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.debug_window = Some(super::debug_window::DebugNesWindow::new());
                        }
                        if ui.button("Dump CPU Data").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.cpu_memory_dump_window =
                                Some(super::cpu_memory_dump_window::CpuMemoryDumpWindow::new());
                        }
                        if ui.button("Dump PPU Data").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.ppu_memory_dump_window =
                                Some(super::ppu_memory_dump_window::PpuMemoryDumpWindow::new());
                        }
                        if ui.button("Dump Cartridge Data").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.cartridge_dump_window =
                                Some(super::cartridge_dump::CartridgeMemoryDumpWindow::new());
                        }
                        if ui.button("Dump Cartridge RAM").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.cartridge_prm_ram_dump_window = Some(
                                super::cartridge_prg_ram_dump::CartridgeMemoryDumpWindow::new(),
                            );
                        }
                        if ui.button("Dump ppu pattern table").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.pattern_table_dump_window =
                                Some(super::pattern_table_dump_window::DumpWindow::new());
                        }
                        if ui.button("Dump ppu name tables").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.nametable_dump_window =
                                Some(super::name_table_dump_window::DumpWindow::new());
                        }
                        if ui.button("Dump ppu sprites").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                            self.sprite_dump_window =
                                Some(super::sprite_dump_window::DumpWindow::new());
                        }
                    });
                }
                if is_fullscreen {
                    let text = format!(
                        "UglyOldBob NES Emulator {} - {:.0} FPS {:.1} percent",
                        env!("CARGO_PKG_VERSION"),
                        self.emulator_fps,
                        self.render_percent * 100.0
                    );
                    ui.label(text);
                }
            });
        });

        if ui.ctx().input(|i| i.key_pressed(egui::Key::F5)) {
            save_state = true;
        }

        if ui.ctx().input(|i| i.key_pressed(egui::Key::F6)) {
            load_state = true;
        }

        #[cfg(not(target_os = "android"))]
        if !self.recording.is_recording() {
            if ui.ctx().input(|i| i.key_pressed(egui::Key::F7)) {
                start_stop_recording = Some(true);
            }
        } else {
            if ui.ctx().input(|i| i.key_pressed(egui::Key::F7)) {
                start_stop_recording = Some(false);
            }
        }

        if ui.ctx().input(|i| i.key_pressed(egui::Key::F8)) {
            rewind_state = true;
        }

        if ui.ctx().input(|i| i.key_pressed(egui::Key::F11)) {
            if self.c.mb.speed_ratio < 1.0 {
                self.c.mb.speed_ratio = 1.0;
            } else {
                self.c.mb.speed_ratio = 0.5;
            }
        }

        if ui.ctx().input(|i| i.key_pressed(egui::Key::F12)) {
            let f = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!f));
        }

        #[cfg(not(target_os = "android"))]
        {
            let record_path = self.c.local.record_path();
            if let Some(rec) = start_stop_recording {
                if rec {
                    self.c.local.resolution_locked = true;
                    let sampling_frequency = self.c.cpu_frequency();
                    let tn = chrono::Local::now();
                    let mut recpath = record_path.clone();
                    recpath.push(format!("{}.avi", tn.format("%Y-%m-%d %H%M%S")));
                    self.recording.start(
                        &self.have_gstreamer,
                        &self.c.local.image,
                        self.c.ppu_frame_rate() as u8,
                        recpath,
                        sampling_frequency,
                    );
                } else {
                    self.c.local.resolution_locked = false;
                    loop {
                        if self.recording.stop().is_ok() {
                            break;
                        }
                    }
                }
            }
        }

        let name = if let Some(cart) = self.c.mb.cartridge() {
            cart.save_name()
        } else {
            "state.bin".to_string()
        };
        let ppp = <std::path::PathBuf as std::str::FromStr>::from_str(&name).unwrap();
        let mut save_path = self.c.local.save_path();
        save_path.push(ppp.file_name().unwrap());
        if save_state {
            let mut path = save_path.clone();
            path.pop();
            let _ = std::fs::create_dir_all(path);
            let state = Box::new(self.c.serialize());
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
                let e = self.c.deserialize(a);
                if e.is_err() {
                    log::error!("Error loading state {:?}", e);
                }
            }
        }

        if rewind_state {
            let e = self.c.deserialize(self.rewinds[1].clone());
            if e.is_err() {
                log::error!("Error loading rewind state {:?}", e);
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let size = ui.available_size();

            // Controller buttons — draw outside the centering logic
            #[cfg(not(target_os = "android"))]
            if let Some(olocal) = &mut self.c.olocal {
                if let Some(network) = &mut olocal.network {
                    let myc = network.get_controller_id();
                    if network.role() == NodeRole::Observer || network.role() == NodeRole::Player {
                        ui.vertical(|ui| {
                            for i in 0..4 {
                                if ui
                                    .add(egui::Button::selectable(
                                        myc == Some(i),
                                        format!("Controller {}", i),
                                    ))
                                    .clicked()
                                {
                                    let _e = network.request_controller(i);
                                }
                            }
                            if ui
                                .add(egui::Button::selectable(myc.is_none(), "No controller"))
                                .clicked()
                            {
                                let _e = network.release_controller();
                            }
                        });
                    }
                }
            }

            // Center the image manually using add_sized + centering offset
            if let Some(t) = &self.texture {
                let zoom = (size.x / t.size()[0] as f32).min(size.y / t.size()[1] as f32);

                let img_size = egui::vec2(t.size()[0] as f32 * zoom, t.size()[1] as f32 * zoom);

                let available = ui.available_size();

                let offset = if available.x < available.y {
                    // Portrait: top-align
                    egui::vec2(((available.x - img_size.x) * 0.5).max(0.0), 0.0)
                } else {
                    // Landscape: center
                    ((available - img_size) * 0.5).max(egui::Vec2::ZERO)
                };

                let rect = egui::Rect::from_min_size(ui.cursor().min + offset, img_size);

                let r = ui.put(
                    rect,
                    egui::Image::from_texture(egui::load::SizedTexture {
                        id: t.id(),
                        size: img_size,
                    })
                    .sense(egui::Sense::click_and_drag()),
                );

                #[cfg(target_os = "android")]
                if self.c.local.configuration.use_screen_controller {
                    let mut buttons_pressed = [false; 11];
                    let mut arrows_pressed = [false; 8];
                    let arrow_configs: Vec<(egui::Rect, &str, f32)> = if size.x > size.y {
                        //landscape
                        vec![
                            (
                                //up
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 2.0,
                                        rect.min.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //down
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 2.0,
                                        rect.min.y + 51.0 + 55.0 * 3.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 3.0,
                                        rect.min.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // up-left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 3.0,
                                        rect.min.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // up-right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // down-left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 3.0,
                                        rect.min.y + 51.0 + 55.0 * 3.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // down-right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x - 51.0 - 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 3.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                        ]
                    } else {
                        //portrait
                        vec![
                            (
                                //up
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 1.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //down
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 1.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 0.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                //right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 2.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // up-left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 0.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // up-right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 2.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // down-left
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 0.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                            (
                                // down-right
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        55.0 * 2.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                " ",
                                1.0,
                            ),
                        ]
                    };
                    let button_configs: Vec<(egui::Rect, &str, f32, usize)> = if size.x > size.y {
                        //landscape
                        vec![
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 2.0,
                                        rect.min.y + 51.0 + 55.0 * 3.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "A",
                                1.0,
                                crate::controller::BUTTON_COMBO_A,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 2.0,
                                        rect.min.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "AA",
                                1.0,
                                crate::controller::BUTTON_COMBO_TURBOA,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 2.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "BB",
                                1.0,
                                crate::controller::BUTTON_COMBO_TURBOB,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 3.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "B",
                                1.0,
                                crate::controller::BUTTON_COMBO_B,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 1.0,
                                        rect.min.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "ST",
                                1.0,
                                crate::controller::BUTTON_COMBO_START,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + img_size.x + 51.0 + 55.0 * 2.0,
                                        rect.min.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "SE",
                                1.0,
                                crate::controller::BUTTON_COMBO_SELECT,
                            ),
                        ]
                    } else {
                        //portrait
                        vec![
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 5.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "A",
                                1.0,
                                crate::controller::BUTTON_COMBO_A,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 5.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "AA",
                                1.0,
                                crate::controller::BUTTON_COMBO_TURBOA,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 4.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "BB",
                                1.0,
                                crate::controller::BUTTON_COMBO_TURBOB,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 4.0,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "B",
                                1.0,
                                crate::controller::BUTTON_COMBO_B,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 2.5,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 1.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "ST",
                                1.0,
                                crate::controller::BUTTON_COMBO_START,
                            ),
                            (
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        51.0 + 55.0 * 2.5,
                                        rect.min.y + img_size.y + 51.0 + 55.0 * 0.0,
                                    ),
                                    egui::vec2(51.0, 51.0),
                                ),
                                "SE",
                                1.0,
                                crate::controller::BUTTON_COMBO_SELECT,
                            ),
                        ]
                    };
                    for (i, button) in &mut self.on_screen_buttons.iter().enumerate() {
                        if let Some(c) = button_configs.get(i) {
                            let pressed = self.touch_hits(Some(c.0));
                            if pressed {
                                buttons_pressed[i] = true;
                            }
                            button.paint_btn(ui, c.0, c.1, pressed, c.2);
                        }
                    }
                    for (i, arrow) in &mut self.on_screen_arrows.iter().enumerate() {
                        if let Some(c) = arrow_configs.get(i) {
                            let pressed = self.touch_hits(Some(c.0));
                            if pressed {
                                arrows_pressed[i] = true;
                            }
                            arrow.paint_btn(ui, c.0, c.1, pressed, c.2);
                        }
                    }
                    //up
                    if arrows_pressed[0] {
                        buttons_pressed[6] = true;
                    }
                    //down
                    if arrows_pressed[1] {
                        buttons_pressed[7] = true;
                    }
                    //left
                    if arrows_pressed[2] {
                        buttons_pressed[8] = true;
                    }
                    //right
                    if arrows_pressed[3] {
                        buttons_pressed[9] = true;
                    }
                    //up-left
                    if arrows_pressed[4] {
                        buttons_pressed[6] = true;
                        buttons_pressed[8] = true;
                    }
                    //up-right
                    if arrows_pressed[5] {
                        buttons_pressed[6] = true;
                        buttons_pressed[9] = true;
                    }
                    //down-left
                    if arrows_pressed[6] {
                        buttons_pressed[7] = true;
                        buttons_pressed[8] = true;
                    }
                    //down-right
                    if arrows_pressed[7] {
                        buttons_pressed[7] = true;
                        buttons_pressed[9] = true;
                    }
                    if (buttons_pressed[6]) {
                        buttons_pressed[7] = false;
                    }
                    if (buttons_pressed[8]) {
                        buttons_pressed[9] = false;
                    }

                    let controller = self.c.mb.get_controller_mut(0);
                    if !controller.should_ignore_local_inputs() {
                        if let crate::controller::NesController::Zapper(_z) = controller {
                        } else {
                            for (i, pressed) in buttons_pressed.iter().enumerate() {
                                if let Some(bc) = button_configs.get(i) {
                                    for contr in controller.get_buttons_iter_mut() {
                                        contr.update_raw_button_data(*pressed, bc.3 as u8);
                                    }
                                }
                            }
                            for contr in controller.get_buttons_iter_mut() {
                                contr.update_raw_button_data(
                                    buttons_pressed[6],
                                    crate::controller::BUTTON_COMBO_UP as u8,
                                );
                                contr.update_raw_button_data(
                                    buttons_pressed[7],
                                    crate::controller::BUTTON_COMBO_DOWN as u8,
                                );
                                contr.update_raw_button_data(
                                    buttons_pressed[8],
                                    crate::controller::BUTTON_COMBO_LEFT as u8,
                                );
                                contr.update_raw_button_data(
                                    buttons_pressed[9],
                                    crate::controller::BUTTON_COMBO_RIGHT as u8,
                                );
                            }
                        }
                    }
                }

                if (r.clicked_by(egui::PointerButton::Secondary)
                    || r.dragged_by(egui::PointerButton::Secondary))
                    && !self.mouse
                {
                    self.mouse = true;
                    self.mouse_miss = true;
                    self.mouse_delay = 15;
                } else if (r.clicked() || r.dragged()) && !self.mouse {
                    self.mouse = true;
                    self.mouse_miss = false;
                    self.mouse_delay = 15;
                }

                if r.hovered() {
                    if let Some(pos) = r.hover_pos() {
                        let coord = pos - r.rect.left_top();

                        #[cfg(feature = "debugger")]
                        {
                            self.c.cpu_peripherals.ppu.bg_debug =
                                Some(((coord.x / zoom) as u8, (coord.y / zoom) as u8));
                        }

                        let scale_factor = self
                            .c
                            .local
                            .configuration
                            .scaler
                            .map(|s| s.scale_factor())
                            .unwrap_or(1.0);

                        let zcoord = coord / (zoom * scale_factor);

                        self.c
                            .mb
                            .set_zapper_coords(zcoord.x as u16, zcoord.y as u16);

                        let pixel = self.c.local.image.get_pixel(coord / zoom);

                        self.mouse_vision = !self.mouse_miss
                            && pixel.r() > 100
                            && pixel.g() > 100
                            && pixel.b() > 100;
                    }
                }
            }

            #[cfg(not(target_os = "android"))]
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "UglyOldBob NES Emulator {} - {:.0} FPS {:.1} percent",
                    env!("CARGO_PKG_VERSION"),
                    self.emulator_fps,
                    self.render_percent * 100.0
                )));
        });

        #[cfg(feature = "debugger")]
        {
            use cpal::traits::StreamTrait;
            if let Some(s) = &mut self.sound_stream {
                if self.c.paused && !self.paused {
                    self.paused = s.pause().is_ok();
                }
                if !self.c.paused && self.paused {
                    self.paused = s.play().is_err();
                }
            }
        }
        ui.ctx().request_repaint();
    }
}
