//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! This is the nes emulator written in rust. It is compatible with windows, linux, and osx.

#[cfg(all(feature = "eframe", feature = "egui-multiwin"))]
compile_error!(
    "feature \"eframe\" and feature \"egui-multiwin\" cannot be enabled at the same time"
);
#[cfg(all(feature = "eframe", feature = "sdl2"))]
compile_error!("feature \"eframe\" and feature \"sdl2\" cannot be enabled at the same time");
#[cfg(all(feature = "sdl2", feature = "egui-multiwin"))]
compile_error!("feature \"sdl2\" and feature \"egui-multiwin\" cannot be enabled at the same time");

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod event;
mod genie;
mod motherboard;
mod network;
mod ppu;
mod recording;
mod romlist;
#[cfg(test)]
mod utility;

use emulator_data::SnesEmulatorData;

#[cfg(not(target_arch = "wasm32"))]
///Run an asynchronous object on a new thread. Maybe not the best way of accomplishing this, but it does work.
pub fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
///Run an asynchronous object on a new thread. Maybe not the best way of accomplishing this, but it does work.
pub fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(test)]
mod tests;

use crate::cartridge::SnesCartridge;

#[cfg(feature = "rom_status")]
pub mod rom_status;

#[cfg(any(feature = "egui-multiwin", feature = "eframe"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
/// Dynamically generated code for the egui-multiwin module allows for use of enum_dispatch for speed gains.
pub mod egui_multiwin_dynamic {
    egui_multiwin::tracked_window!(
        crate::emulator_data::SnesEmulatorData,
        crate::event::Event,
        crate::windows::Windows
    );
    egui_multiwin::multi_window!(
        crate::emulator_data::SnesEmulatorData,
        crate::event::Event,
        crate::windows::Windows
    );
}

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

/// The primary font to use for rendering the gui
#[cfg(feature = "sdl2")]
pub const EMBEDDED_FONT: &[u8] = include_bytes!("cmsltt10.ttf");

mod windows;

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
    let mut egui_ctx = egui_sdl2_gl::egui::Context::default();

    let i = sdl2::mixer::InitFlag::MP3;
    let _sdl2mixer = sdl2::mixer::init(i).unwrap();
    let audio = sdl2::mixer::open_audio(44100, 16, 2, 1024);

    let flags = sdl2::image::InitFlag::all();
    let _sdl2_image = sdl2::image::init(flags).unwrap();

    let mut last_frame_time: std::time::Instant = std::time::Instant::now();

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
        let frame_start = std::time::Instant::now();

        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_frame(egui_state.input.take());

        'emulator_loop: loop {
            nes_data.cycle_step(&mut None, &mut None);
            if nes_data.cpu_peripherals.ppu_frame_end() {
                break 'emulator_loop;
            }
        }

        let frame_data = nes_data.cpu_peripherals.ppu_get_frame();
        crate::ppu::NesPpu::convert_for_sdl2(frame_data, &mut frame);

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

        let egui_sdl2_gl::egui::FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        } = egui_ctx.end_frame();

        egui_state.process_output(&window, &platform_output);

        let paint_jobs = egui_ctx.tessellate(shapes);

        painter.paint_jobs(None, textures_delta, paint_jobs);

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

        let time_now = std::time::Instant::now();
        let frame_time = time_now.duration_since(last_frame_time);
        let desired_frame_length = std::time::Duration::from_nanos(1_000_000_000u64 / 60);
        if frame_time < desired_frame_length {
            let st = (desired_frame_length - frame_time);
            spin_sleep::sleep(st);
        }

        let new_frame_time = std::time::Instant::now();
        let new_fps =
            1_000_000_000.0 / new_frame_time.duration_since(last_frame_time).as_nanos() as f64;
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

    let mut nes_data = NesEmulatorData::new();
    nes_data
        .parser
        .find_roms(nes_data.configuration.get_rom_path());

    let host = cpal::default_host();
    let device = host.default_output_device();
    let mut sound_rate = 0;
    let mut sound_producer = None;
    let sound_stream = if let Some(d) = &device {
        let ranges = d.supported_output_configs();
        if let Ok(mut r) = ranges {
            let supportedconfig = r.next().unwrap().with_max_sample_rate();
            let format = supportedconfig.sample_format();
            println!("output format is {:?}", format);
            let mut config = supportedconfig.config();
            let mut num_samples = (config.sample_rate.0 as f32 * 0.1) as usize;
            let sbs = supportedconfig.buffer_size();
            let num_samples_buffer = if let cpal::SupportedBufferSize::Range { min, max } = sbs {
                if num_samples > *max as usize {
                    num_samples = *max as usize;
                    cpal::BufferSize::Fixed(*max as cpal::FrameCount)
                } else if num_samples < *min as usize {
                    num_samples = *min as usize;
                    cpal::BufferSize::Fixed(*min as cpal::FrameCount)
                } else {
                    cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
                }
            } else {
                //TODO maybe do somethind else when buffer size is unknown
                cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
            };
            config.buffer_size = num_samples_buffer;
            config.channels = 2;
            println!("SBS IS {:?}", sbs);

            println!("audio config is {:?}", config);

            nes_data.cpu_peripherals.apu.set_audio_buffer(num_samples);
            println!(
                "Audio buffer size is {} elements, sample rate is {}",
                num_samples, config.sample_rate.0
            );
            let rb = ringbuf::HeapRb::new(num_samples * 2);
            let (producer, mut consumer) = rb.split();
            let mut stream = d
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _cb: &cpal::OutputCallbackInfo| {
                        let mut index = 0;
                        while index < data.len() {
                            let c = consumer.pop_slice(&mut data[index..]);
                            if c == 0 {
                                break;
                            }
                            index += c;
                        }
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

    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());
    nes_data.mb.controllers[0] = Some(controller::StandardController::new());

    if let Some(c) = nes_data.configuration.start_rom() {
        let nc = NesCartridge::load_cartridge(c.to_string()).unwrap();
        nes_data.insert_cartridge(nc);
    }

    eframe::run_native(
        "UglyOldBob NES Emulator",
        options,
        Box::new(move |_cc| {
            Box::new(crate::windows::main::MainNesWindow::new_request(
                nes_data,
                sound_rate,
                sound_producer,
                sound_stream,
            ))
        }),
    );
}

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::multi_window::MultiWindow;

#[cfg(feature = "egui-multiwin")]
fn main() {
    use crate::apu::{AudioProducer, AudioProducerWithRate};

    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!
    let mut event_loop = egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event();
    #[cfg(target_os = "linux")]
    egui_multiwin::winit::platform::x11::EventLoopBuilderExtX11::with_x11(&mut event_loop);
    let event_loop = event_loop.build();

    let proxy: egui_multiwin::winit::event_loop::EventLoopProxy<event::Event> =
        event_loop.create_proxy();

    let mut nes_data = SnesEmulatorData::new(Some(proxy));
    println!(
        "There are {} roms in the romlist",
        nes_data.local.parser.list().elements.len()
    );
    nes_data.local.parser.find_roms(
        nes_data.local.configuration.get_rom_path(),
        nes_data.local.save_path(),
        nes_data.local.get_save_other(),
    );
    let mut multi_window = MultiWindow::new();

    let host = cpal::default_host();
    let device = host.default_output_device();
    let mut sound_producer = None;
    let sound_stream = if let Some(d) = &device {
        let ranges = d.supported_output_configs();
        if let Ok(r) = ranges {
            let mut configs: Vec<cpal::SupportedStreamConfigRange> = r.collect();
            for c in &configs {
                println!(
                    "Audio: {:?} {:?}-{:?}",
                    c.sample_format(),
                    c.min_sample_rate(),
                    c.max_sample_rate()
                );
            }
            configs.sort_by(|c, d| {
                let index = |sf| match sf {
                    cpal::SampleFormat::I8 => 10,
                    cpal::SampleFormat::I16 => 10,
                    cpal::SampleFormat::I32 => 10,
                    cpal::SampleFormat::I64 => 10,
                    cpal::SampleFormat::U8 => 3,
                    cpal::SampleFormat::U16 => 1,
                    cpal::SampleFormat::U32 => 0,
                    cpal::SampleFormat::U64 => 10,
                    cpal::SampleFormat::F32 => 2,
                    cpal::SampleFormat::F64 => 10,
                    _ => 10,
                };
                let ic = index(c.sample_format());
                let id = index(d.sample_format());
                ic.partial_cmp(&id).unwrap()
            });
            configs.sort_by(|c, d| {
                c.max_sample_rate()
                    .partial_cmp(&d.max_sample_rate())
                    .unwrap()
            });

            let supportedconfig = configs[0].clone().with_max_sample_rate();
            let format = supportedconfig.sample_format();
            println!("output format is {:?}", format);
            let mut config = supportedconfig.config();
            let mut num_samples = (config.sample_rate.0 as f32 * 0.1) as usize;
            let sbs = supportedconfig.buffer_size();
            let num_samples_buffer = if let cpal::SupportedBufferSize::Range { min, max } = sbs {
                if num_samples > *max as usize {
                    num_samples = *max as usize;
                    cpal::BufferSize::Fixed(*max as cpal::FrameCount)
                } else if num_samples < *min as usize {
                    num_samples = *min as usize;
                    cpal::BufferSize::Fixed(*min as cpal::FrameCount)
                } else {
                    cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
                }
            } else {
                //TODO maybe do somethind else when buffer size is unknown
                cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
            };
            config.buffer_size = num_samples_buffer;
            config.channels = 2;
            println!("SBS IS {:?}", sbs);

            println!("audio config is {:?}", config);

            println!(
                "Audio buffer size is {} elements, sample rate is {}",
                num_samples, config.sample_rate.0
            );

            let (mut stream, user_audio) = match format {
                cpal::SampleFormat::U8 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U8(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u8], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::U16 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U16(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u16], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::U32 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U32(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u32], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::F32 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::F32(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [f32], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                _ => todo!(),
            };

            if let Some(s) = &mut stream {
                s.play().unwrap();
                nes_data.local.set_sound_rate(config.sample_rate.0);
                sound_producer = Some(user_audio);
            }
            stream
        } else {
            None
        }
    } else {
        None
    };

    let root_window = windows::main::MainNesWindow::new_request(sound_producer, sound_stream);

    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());
    nes_data.mb.set_controller(
        0,
        nes_data.local.configuration.controller_type[0].make_controller(),
    );
    nes_data.mb.set_controller(
        1,
        nes_data.local.configuration.controller_type[1].make_controller(),
    );
    nes_data.mb.set_controller(
        2,
        nes_data.local.configuration.controller_type[2].make_controller(),
    );
    nes_data.mb.set_controller(
        3,
        nes_data.local.configuration.controller_type[3].make_controller(),
    );

    if nes_data.local.configuration.sticky_rom {
        if let Some(c) = nes_data.local.configuration.start_rom() {
            let nc =
                SnesCartridge::load_cartridge(c.to_string(), &nes_data.local.save_path()).unwrap();
            nes_data.insert_cartridge(nc);
        }
    }

    let _e = multi_window.add(root_window, &mut nes_data, &event_loop);
    #[cfg(feature = "debugger")]
    {
        if nes_data.paused {
            let debug_win = windows::debug_window::DebugNesWindow::new_request();
            let _e = multi_window.add(debug_win, &mut nes_data, &event_loop);
        }
    }

    #[cfg(feature = "rom_status")]
    {
        let _e = multi_window.add(
            windows::rom_checker::Window::new_request(&nes_data),
            &mut nes_data,
            &event_loop,
        );
    }

    multi_window.run(event_loop, nes_data);
}
