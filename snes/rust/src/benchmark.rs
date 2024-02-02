#![allow(dead_code)]

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod genie;
mod motherboard;
mod ppu;
pub mod windows;

#[cfg(feature = "egui-multiwin")]
/// Dynamically generated code for the egui-multiwin module allows for use of enum_dispatch for speed gains.
pub mod egui_multiwin_dynamic {
    egui_multiwin::tracked_window!(
        crate::emulator_data::SnesEmulatorData,
        common_emulator::event::Event,
        crate::windows::Windows
    );
    egui_multiwin::multi_window!(
        crate::emulator_data::SnesEmulatorData,
        common_emulator::event::Event,
        crate::windows::Windows
    );
}

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

use crate::apu::SnesApu;
use crate::cartridge::SnesCartridge;
use crate::cpu::{SnesCpu, SnesCpuPeripherals};
use crate::motherboard::SnesMotherboard;
use crate::ppu::SnesPpu;
use common_emulator::convert_hex_to_decimal;
use emulator_data::SnesEmulatorData;

use criterion::Criterion;
use std::io::BufRead;

/// An audio producer of several different kinds of data
pub enum AudioProducer {
    U8(
        ringbuf::Producer<
            u8,
            std::sync::Arc<ringbuf::SharedRb<u8, Vec<std::mem::MaybeUninit<u8>>>>,
        >,
    ),
    U16(
        ringbuf::Producer<
            u16,
            std::sync::Arc<ringbuf::SharedRb<u16, Vec<std::mem::MaybeUninit<u16>>>>,
        >,
    ),
    U32(
        ringbuf::Producer<
            u32,
            std::sync::Arc<ringbuf::SharedRb<u32, Vec<std::mem::MaybeUninit<u32>>>>,
        >,
    ),
    F32(
        ringbuf::Producer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    ),
}

/// An audio consumer of several different kinds of data
pub enum AudioConsumer {
    U8(
        ringbuf::Consumer<
            u8,
            std::sync::Arc<ringbuf::SharedRb<u8, Vec<std::mem::MaybeUninit<u8>>>>,
        >,
    ),
    U16(
        ringbuf::Consumer<
            u16,
            std::sync::Arc<ringbuf::SharedRb<u16, Vec<std::mem::MaybeUninit<u16>>>>,
        >,
    ),
    U32(
        ringbuf::Consumer<
            u32,
            std::sync::Arc<ringbuf::SharedRb<u32, Vec<std::mem::MaybeUninit<u32>>>>,
        >,
    ),
    F32(
        ringbuf::Consumer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    ),
}

struct CpuBench1 {
    cpu: SnesCpu,
    cpu_peripherals: SnesCpuPeripherals,
    goldenlog: std::io::Lines<std::io::BufReader<std::fs::File>>,
    mb: SnesMotherboard,
}

pub fn cpu_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic cpu test");
    println!("Checking current directory");
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut nes_data = SnesEmulatorData::new(None);
    group.bench_function("basic 2", |b| {
        b.iter(|| 'emulator_loop: loop {
            nes_data.cycle_step(&mut Vec::new(), &mut Vec::new(), &mut None);
            if nes_data.cpu_peripherals.ppu_frame_end() {
                nes_data.cpu_peripherals.ppu_get_frame();
                break 'emulator_loop;
            }
        });
    });
}

pub fn bench1(c: &mut Criterion) {
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut group = c.benchmark_group("basic ppu rendering");
    let mut nes_data = SnesEmulatorData::new(None);
    group.bench_function("basic 1", |b| {
        b.iter(|| 'emulator_loop: loop {
            nes_data.cycle_step(&mut Vec::new(), &mut Vec::new(), &mut None);
            if nes_data.cpu_peripherals.ppu_frame_end() {
                let _data = nes_data.cpu_peripherals.ppu_get_frame();
                break 'emulator_loop;
            }
        });
    });
}

pub fn romlist_bench(c: &mut Criterion) {
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut group = c.benchmark_group("romlist parse bench");
    group.bench_function("first run", |b| {
        b.iter(|| {
            let _e = std::fs::remove_file("./roms.bin");
            let mut list = common_emulator::romlist::RomListParser::new(std::path::PathBuf::new());
            list.find_roms(
                "../test_roms",
                std::path::PathBuf::new(),
                std::path::PathBuf::new(),
                |n, p| SnesCartridge::load_cartridge(n, p),
            );
            list.process_roms(std::path::PathBuf::new(), |n, p| {
                SnesCartridge::load_cartridge(n, p)
            });
        });
    });

    group.bench_function("second run", |b| {
        b.iter(|| {
            let mut list = common_emulator::romlist::RomListParser::new(std::path::PathBuf::new());
            list.find_roms(
                "../test_roms",
                std::path::PathBuf::new(),
                std::path::PathBuf::new(),
                |n, p| SnesCartridge::load_cartridge(n, p),
            );
            list.process_roms(std::path::PathBuf::new(), |n, p| {
                SnesCartridge::load_cartridge(n, p)
            });
        });
    });
}

pub fn image_scaling_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Image scaling");
    let base_image = common_emulator::video::RgbImage::new(256, 240);
    group.bench_function("to_pixels conversion", |b| {
        b.iter_batched(
            || Box::new(common_emulator::video::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels();
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.bench_function("to_pixels nul resizing conversion", |b| {
        b.iter_batched(
            || Box::new(common_emulator::video::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels().resize(None);
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.bench_function("to_egui_pixels nul resizing conversion", |b| {
        b.iter_batched(
            || Box::new(common_emulator::video::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels_egui().resize(None).to_egui();
            },
            criterion::BatchSize::PerIteration,
        );
    });

    for alg in <common_emulator::video::ScalingAlgorithm as strum::IntoEnumIterator>::iter() {
        let text = format!("to_pixels {} resizing conversion", alg.to_string());
        group.bench_function(text, |b| {
            b.iter_batched(
                || Box::new(common_emulator::video::RgbImage::new(256, 240)),
                |data| {
                    let _e = data.to_pixels().resize(Some(alg));
                },
                criterion::BatchSize::PerIteration,
            );
        });
        let text = format!("to_egui_pixels {} resizing conversion", alg.to_string());
        group.bench_function(text, |b| {
            b.iter_batched(
                || Box::new(common_emulator::video::RgbImage::new(256, 240)),
                |data| {
                    let _e = data.to_pixels_egui().resize(Some(alg)).to_egui();
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
}

fn benches() {
    let mut criterion = crate::Criterion::default().configure_from_args();
    bench1(&mut criterion);
    cpu_bench(&mut criterion);
    romlist_bench(&mut criterion);
    image_scaling_bench(&mut criterion);
}

fn main() {
    benches();
    crate::Criterion::default()
        .configure_from_args()
        .final_summary();
}
