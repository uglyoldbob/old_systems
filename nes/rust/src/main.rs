//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! This is the nes emulator written in rust. It is compatible with windows, linux, and osx.

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod genie;
mod motherboard;
mod ppu;

use bluetooth_rust::{BluetoothAdapterTrait, BluetoothRfcommConnectableAsyncTrait, BluetoothRfcommProfileAsyncTrait};
use emulator_data::NesEmulatorData;

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

use crate::cartridge::NesCartridge;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod windows;

async fn handle_bluetooth_controller_client(_stream: bluetooth_rust::BluetoothStream, a: [u8; 6]) {
    println!("Got a bluetooth connection from {:?}", a)
}

async fn run_bluetooth() {
    println!("Running bluetooth");
    let mut bab = bluetooth_rust::BluetoothAdapterBuilder::new();
    let s = tokio::sync::mpsc::channel(100);
    bab.with_sender(s.0);
    let ba = bab.async_build().await;
    match ba {
        Ok(ba) => {
            let settings = bluetooth_rust::BluetoothRfcommProfileSettings {
                uuid: "76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string(),
                name: Some("NES Controller Service".to_string()),
                service_uuid: Some("76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string()),
                channel: None,
                psm: None,
                authenticate: Some(false),
                authorize: Some(false),
                auto_connect: Some(true),
                sdp_record: None,
                sdp_version: None,
                sdp_features: None,
            };
            let mut profile = ba.register_rfcomm_profile(settings).await.expect("Failed to register bluetooth profile");
            loop {
                let c = profile.connectable().await.expect("Failed to build connectable for bluetooth profile");
                if let Ok(a) = c.accept().await {
                    tokio::spawn(async move {
                        handle_bluetooth_controller_client(a.0, a.1).await;
                    });
                }
            }
        }
        Err(e) => eprintln!("Failed to get bluetooth adapter: {}", e),
    }
}

fn main() {
    use common_emulator::audio::{AudioProducer, AudioProducerWithRate};

    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!

    let mut options = eframe::NativeOptions::default();
    //TODO only disable vsync when required
    options.vsync = false;

    let trt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to start async runtime");
    trt.spawn(run_bluetooth());

    let mut nes_data = NesEmulatorData::new();
    println!(
        "There are {} roms in the romlist",
        nes_data.local.parser.list().elements.len()
    );
    nes_data.local.parser.find_roms(
        nes_data.local.configuration.get_rom_path(),
        nes_data.local.save_path(),
        nes_data.local.get_save_other(),
        |n, p| NesCartridge::load_cartridge(n, p),
    );

    let host = cpal::default_host();
    let device = host.default_output_device();
    let mut sound_rate = 0;
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
                sound_rate = config.sample_rate.0;
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
            if let Ok(nc) = NesCartridge::load_cartridge(c.to_string(), &nes_data.local.save_path())
            {
                nes_data.insert_cartridge(nc);
            }
        }
    }

    eframe::run_native(
        "UglyOldBob NES Emulator",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(crate::windows::main::MainNesWindow::new(
                nes_data,
                sound_rate,
                sound_producer,
                sound_stream,
            )))
        }),
    )
    .expect("Failed to run application");
}
