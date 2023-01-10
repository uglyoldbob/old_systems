#![allow(dead_code)]

pub mod cartridge;
pub mod cpu;
pub mod emulator_data;
pub mod motherboard;
pub mod ppu;
pub mod utility;

use emulator_data::NesEmulatorData;
use crate::cartridge::NesCartridge;
use crate::cpu::{NesCpu, NesCpuPeripherals};
use crate::motherboard::NesMotherboard;
use crate::ppu::NesPpu;
use crate::utility::convert_hex_to_decimal;

use criterion::{criterion_group, criterion_main, Criterion};
use std::io::BufRead;

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
        b.iter(
            || {
                'emulator_loop: loop {
                    nes_data.ppu_step();
                    if nes_data.cpu_peripherals.ppu_frame_end() {
                        let data = nes_data.cpu_peripherals.ppu_get_frame();
                        break 'emulator_loop;
                    }
                }
            },
        );
    });

    group.bench_function("nestest", |b| {
        b.iter_batched(|| {
            let cpu: NesCpu = NesCpu::new();
            let ppu: NesPpu = NesPpu::new();
            let cpu_peripherals: NesCpuPeripherals = NesCpuPeripherals::new(ppu);
            let mut mb: NesMotherboard = NesMotherboard::new();
            let nc = NesCartridge::load_cartridge("./nestest.nes".to_string());
            let goldenlog = std::fs::File::open("./nestest.log").unwrap();
            let goldenlog = std::io::BufReader::new(goldenlog).lines();

            let mut nc = nc.unwrap();
            nc.rom_byte_hack(0xfffc, 0x00);
            mb.insert_cartridge(nc);
            let data = CpuBench1{
                cpu: cpu,
                cpu_peripherals: cpu_peripherals,
                goldenlog: goldenlog,
                mb: mb,
            };
            Box::new(data)
        }, |data| {
            let mut data = data;
            let mut t: String;
            let mut b;
            let mut log_line = 0;
            for i in 0..26554 {
                data.cpu.cycle(&mut data.mb, &mut data.cpu_peripherals);
                if data.cpu.instruction_start() {
                    log_line += 1;
                    t = data.goldenlog.next().unwrap().unwrap();
        //            println!("Instruction end at cycle {}", i + 1);
        //            println!("NESTEST LOG LINE {}: {}", log_line, t);
                    b = t.as_bytes();
                    let d = convert_hex_to_decimal(b[0] as char) as u16;
                    let d2 = convert_hex_to_decimal(b[1] as char) as u16;
                    let d3 = convert_hex_to_decimal(b[2] as char) as u16;
                    let d4 = convert_hex_to_decimal(b[3] as char) as u16;
                    let address = d << 12 | d2 << 8 | d3 << 4 | d4;

                    let reg_a: u8 = (convert_hex_to_decimal(b[50] as char) as u8) << 4
                        | convert_hex_to_decimal(b[51] as char) as u8;
                    assert_eq!(data.cpu.get_a(), reg_a);

                    let reg_x: u8 = (convert_hex_to_decimal(b[55] as char) as u8) << 4
                        | convert_hex_to_decimal(b[56] as char) as u8;
                    assert_eq!(data.cpu.get_x(), reg_x);

                    let reg_y: u8 = (convert_hex_to_decimal(b[60] as char) as u8) << 4
                        | convert_hex_to_decimal(b[61] as char) as u8;
                    assert_eq!(data.cpu.get_y(), reg_y);

                    let reg_p: u8 = (convert_hex_to_decimal(b[65] as char) as u8) << 4
                        | convert_hex_to_decimal(b[66] as char) as u8;
                    assert_eq!(data.cpu.get_p(), reg_p);

                    let reg_sp: u8 = (convert_hex_to_decimal(b[71] as char) as u8) << 4
                        | convert_hex_to_decimal(b[72] as char) as u8;
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
        }, criterion::BatchSize::PerIteration);
    });
    
}

pub fn bench1(c: &mut Criterion) {
    let wdir = std::env::current_dir().unwrap();
    println!("Current dir is {}", wdir.display());

    let mut group = c.benchmark_group("basic ppu rendering");
    let mut nes_data = NesEmulatorData::new();
    group.bench_function("basic 1", |b| {
        b.iter(
            || {
                'emulator_loop: loop {
                    nes_data.ppu_step();
                    if nes_data.cpu_peripherals.ppu_frame_end() {
                        let data = nes_data.cpu_peripherals.ppu_get_frame();
                        break 'emulator_loop;
                    }
                }
            },
        );
    });
}

criterion_group!(benches, bench1, cpu_bench);
criterion_main!(benches);
