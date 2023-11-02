#![allow(dead_code)]

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod motherboard;
mod ppu;
mod recording;
mod rom_status;
mod romlist;
mod utility;
pub mod windows;

use windows::Windows;

#[cfg(feature = "egui-multiwin")]
/// Dynamically generated code for the egui-multiwin module allows for use of enum_dispatch for speed gains.
pub mod egui_multiwin_dynamic {
    egui_multiwin::tracked_window!(
        crate::emulator_data::NesEmulatorData,
        egui_multiwin::NoEvent,
        crate::Windows
    );
    egui_multiwin::multi_window!(
        crate::emulator_data::NesEmulatorData,
        egui_multiwin::NoEvent,
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

use crate::apu::NesApu;
use crate::cartridge::NesCartridge;
use crate::cpu::{NesCpu, NesCpuPeripherals};
use crate::motherboard::NesMotherboard;
use crate::ppu::NesPpu;
use crate::utility::convert_hex_to_decimal;
use emulator_data::NesEmulatorData;

use criterion::Criterion;
use std::io::BufRead;

/// A convenience type for the type used in the ringbuf crate.
pub type AudioProducer =
    ringbuf::Producer<f32, std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>>;
/// A convenience type for the type used in the ringbuf crate.
pub type AudioConsumer =
    ringbuf::Consumer<f32, std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>>;

struct CpuBench1 {
    cpu: NesCpu,
    cpu_peripherals: NesCpuPeripherals,
    goldenlog: std::io::Lines<std::io::BufReader<std::fs::File>>,
    mb: NesMotherboard,
}

pub fn cpu_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic cpu test");
    println!("Checking current directory");
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut nes_data = NesEmulatorData::new();
    group.bench_function("basic 2", |b| {
        b.iter(|| 'emulator_loop: loop {
            nes_data.cycle_step(&mut Vec::new(), &mut None);
            if nes_data.cpu_peripherals.ppu_frame_end() {
                nes_data.cpu_peripherals.ppu_get_frame();
                break 'emulator_loop;
            }
        });
    });

    group.bench_function("nestest", |b| {
        b.iter_batched(
            || {
                let cpu: NesCpu = NesCpu::new();
                let ppu: NesPpu = NesPpu::new();
                let apu: NesApu = NesApu::new();
                let cpu_peripherals: NesCpuPeripherals = NesCpuPeripherals::new(ppu, apu);
                let mut mb: NesMotherboard = NesMotherboard::new();
                let nc = NesCartridge::load_cartridge(
                    "../test_roms/other/nestest.nes".to_string(),
                    &std::path::PathBuf::new(),
                );
                let goldenlog = std::fs::File::open("../test_roms/other/nestest.log").unwrap();
                let goldenlog = std::io::BufReader::new(goldenlog).lines();

                let mut nc = nc.unwrap();
                nc.rom_byte_hack(0xfffc, 0x00);
                mb.insert_cartridge(nc);
                let data = CpuBench1 {
                    cpu,
                    cpu_peripherals,
                    goldenlog,
                    mb,
                };
                Box::new(data)
            },
            |data| {
                let mut data = data;
                let mut t: String;
                let mut b;
                for i in 0..26554 {
                    data.cpu
                        .cycle(&mut data.mb, &mut data.cpu_peripherals, false, false);
                    if data.cpu.instruction_start() {
                        t = data.goldenlog.next().unwrap().unwrap();
                        b = t.as_bytes();
                        let d = convert_hex_to_decimal(b[0] as char) as u16;
                        let d2 = convert_hex_to_decimal(b[1] as char) as u16;
                        let d3 = convert_hex_to_decimal(b[2] as char) as u16;
                        let d4 = convert_hex_to_decimal(b[3] as char) as u16;
                        let address = d << 12 | d2 << 8 | d3 << 4 | d4;

                        let reg_a: u8 = convert_hex_to_decimal(b[50] as char) << 4
                            | convert_hex_to_decimal(b[51] as char);
                        assert_eq!(data.cpu.get_a(), reg_a);

                        let reg_x: u8 = convert_hex_to_decimal(b[55] as char) << 4
                            | convert_hex_to_decimal(b[56] as char);
                        assert_eq!(data.cpu.get_x(), reg_x);

                        let reg_y: u8 = convert_hex_to_decimal(b[60] as char) << 4
                            | convert_hex_to_decimal(b[61] as char);
                        assert_eq!(data.cpu.get_y(), reg_y);

                        let reg_p: u8 = convert_hex_to_decimal(b[65] as char) << 4
                            | convert_hex_to_decimal(b[66] as char);
                        assert_eq!(data.cpu.get_p(), reg_p);

                        let reg_sp: u8 = convert_hex_to_decimal(b[71] as char) << 4
                            | convert_hex_to_decimal(b[72] as char);
                        assert_eq!(data.cpu.get_sp(), reg_sp);

                        //            println!("Address is {:x} {:x}", address, cpu.get_pc());
                        assert_eq!(data.cpu.get_pc(), address);
                        //            println!("");

                        let mut logcycle: u32 = 0;
                        for i in 90..95 {
                            if i < b.len() {
                                logcycle *= 10;
                                logcycle += convert_hex_to_decimal(b[i] as char) as u32;
                            }
                        }
                        assert_eq!(i + 1, logcycle);
                    }
                }
                assert_eq!(data.cpu.get_pc(), 0xc66e);
            },
            criterion::BatchSize::PerIteration,
        );
    });
}

pub fn bench1(c: &mut Criterion) {
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut group = c.benchmark_group("basic ppu rendering");
    let mut nes_data = NesEmulatorData::new();
    group.bench_function("basic 1", |b| {
        b.iter(|| 'emulator_loop: loop {
            nes_data.cycle_step(&mut Vec::new(), &mut None);
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
            let mut list = romlist::RomListParser::new(std::path::PathBuf::new());
            list.find_roms(
                "../test_roms",
                std::path::PathBuf::new(),
                std::path::PathBuf::new(),
            );
            list.process_roms(std::path::PathBuf::new());
        });
    });

    group.bench_function("second run", |b| {
        b.iter(|| {
            let mut list = romlist::RomListParser::new(std::path::PathBuf::new());
            list.find_roms(
                "../test_roms",
                std::path::PathBuf::new(),
                std::path::PathBuf::new(),
            );
            list.process_roms(std::path::PathBuf::new());
        });
    });
}

pub fn image_scaling_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Image scaling");
    let base_image = crate::ppu::RgbImage::new(256, 240);
    group.bench_function("to_pixels conversion", |b| {
        b.iter_batched(
            || Box::new(crate::ppu::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels();
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.bench_function("to_pixels nul resizing conversion", |b| {
        b.iter_batched(
            || Box::new(crate::ppu::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels().resize(None);
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.bench_function("to_egui_pixels nul resizing conversion", |b| {
        b.iter_batched(
            || Box::new(crate::ppu::RgbImage::new(256, 240)),
            |data| {
                let _e = data.to_pixels_egui().resize(None).to_egui();
            },
            criterion::BatchSize::PerIteration,
        );
    });

    for alg in <ppu::ScalingAlgorithm as strum::IntoEnumIterator>::iter() {
        let text = format!("to_pixels {} resizing conversion", alg.to_string());
        group.bench_function(text, |b| {
            b.iter_batched(
                || Box::new(crate::ppu::RgbImage::new(256, 240)),
                |data| {
                    let _e = data.to_pixels().resize(Some(alg));
                },
                criterion::BatchSize::PerIteration,
            );
        });
        let text = format!("to_egui_pixels {} resizing conversion", alg.to_string());
        group.bench_function(text, |b| {
            b.iter_batched(
                || Box::new(crate::ppu::RgbImage::new(256, 240)),
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
