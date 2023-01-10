#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod cartridge;
pub mod cpu;
pub mod emulator_data;
pub mod motherboard;
pub mod ppu;
pub mod utility;
use emulator_data::NesEmulatorData;
use motherboard::NesMotherboard;

#[cfg(test)]
use std::io::BufRead;

use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesCpuPeripherals;
use crate::cpu::NesMemoryBus;
use crate::ppu::NesPpu;
use crate::utility::convert_hex_to_decimal;

#[test]
fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn check_nes_roms() {
    let mut roms = Vec::new();
    let pb = std::path::PathBuf::from("./");
    let entries = std::fs::read_dir(&pb).unwrap();
    for e in entries.into_iter() {
        if let Ok(e) = e {
            let path = e.path();
            let meta = std::fs::metadata(&path).unwrap();
            if meta.is_file() {
                println!("Element {}", path.display());
                if path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".nes")
                {
                    roms.push(path);
                }
            }
        }
    }
    println!("Checking roms in {}", pb.display());
    for r in roms {
        println!("Testing rom {}", r.display());
        let nc = NesCartridge::load_cartridge(r.into_os_string().into_string().unwrap());
        assert!(
            nc.is_ok(),
            "Unable to load rom because {:?}",
            nc.err().unwrap()
        );
    }
}

#[test]
fn basic_cpu_test() {
    let mut cpu: NesCpu = NesCpu::new();
    let ppu: NesPpu = NesPpu::new();
    let mut cpu_peripherals: NesCpuPeripherals = NesCpuPeripherals::new(ppu);
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nestest.nes".to_string());
    let goldenlog = std::fs::File::open("./nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();
    let mut log_line = 0;

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String;
    let mut b;
    for i in 0..26554 {
        cpu.cycle(&mut mb, &mut cpu_peripherals);
        if cpu.instruction_start() {
            log_line += 1;
            t = goldenlog.next().unwrap().unwrap();
            println!("Instruction end at cycle {}", i + 1);
            println!("NESTEST LOG LINE {}: {}", log_line, t);
            b = t.as_bytes();
            let d = convert_hex_to_decimal(b[0] as char) as u16;
            let d2 = convert_hex_to_decimal(b[1] as char) as u16;
            let d3 = convert_hex_to_decimal(b[2] as char) as u16;
            let d4 = convert_hex_to_decimal(b[3] as char) as u16;
            let address = d << 12 | d2 << 8 | d3 << 4 | d4;

            let reg_a: u8 = (convert_hex_to_decimal(b[50] as char) as u8) << 4
                | convert_hex_to_decimal(b[51] as char) as u8;
            assert_eq!(cpu.get_a(), reg_a);

            let reg_x: u8 = (convert_hex_to_decimal(b[55] as char) as u8) << 4
                | convert_hex_to_decimal(b[56] as char) as u8;
            assert_eq!(cpu.get_x(), reg_x);

            let reg_y: u8 = (convert_hex_to_decimal(b[60] as char) as u8) << 4
                | convert_hex_to_decimal(b[61] as char) as u8;
            assert_eq!(cpu.get_y(), reg_y);

            let reg_p: u8 = (convert_hex_to_decimal(b[65] as char) as u8) << 4
                | convert_hex_to_decimal(b[66] as char) as u8;
            assert_eq!(cpu.get_p(), reg_p);

            let reg_sp: u8 = (convert_hex_to_decimal(b[71] as char) as u8) << 4
                | convert_hex_to_decimal(b[72] as char) as u8;
            assert_eq!(cpu.get_sp(), reg_sp);

            println!("Address is {:x} {:x}", address, cpu.get_pc());
            assert_eq!(cpu.get_pc(), address);
            println!("");

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
    assert_eq!(cpu.get_pc(), 0xc66e);
}

fn make_dummy_texture<'a, T>(tc: &'a sdl2::render::TextureCreator<T>) -> sdl2::render::Texture<'a> {
    let mut data: Vec<u8> = vec![0; (4 * 4 * 2) as usize];
    let mut surf = sdl2::surface::Surface::from_data(
        data.as_mut_slice(),
        4,
        4,
        (2 * 4) as u32,
        sdl2::pixels::PixelFormatEnum::RGB555,
    )
    .unwrap();
    let _e = surf.set_color_key(true, sdl2::pixels::Color::BLACK);
    sdl2::render::Texture::from_surface(&surf, tc).unwrap()
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut vid_win = video_subsystem.window("UglyOldBob NES Emulator", 256 * 1, 240 * 1);
    let mut windowb = vid_win.position_centered();
    let window = windowb.opengl().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    canvas.set_scale(1.0, 1.0).unwrap();
    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let dummy_texture = make_dummy_texture(&texture_creator);

    let mut texture = texture_creator
        .create_texture_target(sdl2::pixels::PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let mut nes_data = NesEmulatorData::new();

    let nc = NesCartridge::load_cartridge("./nes/rust/cpu.nes".to_string());

    let nc = nc.unwrap();
    nes_data.insert_cartridge(nc);

    let mut prev_time = std::time::SystemTime::now();

    'app_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::KeyDown { timestamp, window_id, keycode, scancode, keymod, repeat } => {
                    if let Some(k) = keycode {
                        match k {
                            sdl2::keyboard::Keycode::Escape => {
                                break 'app_loop;
                            }
                            _ => {

                            }
                        }
                    }
                }
                sdl2::event::Event::Quit { .. } => {
                    break 'app_loop;
                }
                _ => {}
            }
        }
      
        'emulator_loop: loop {
            #[cfg(debug_assertions)]
            {
                if !nes_data.paused {
                    nes_data.cycle_step();
                    if nes_data.single_step
                        && nes_data.cpu_clock_counter == 0
                        && nes_data.cpu.breakpoint_option()
                    {
                        nes_data.paused = true;
                        break 'emulator_loop;
                    }
                } else {
                    break 'emulator_loop;
                }
                if nes_data.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
            #[cfg(not(debug_assertions))]
            {
                nes_data.cycle_step();
                if nes_data.cpu_peripherals.ppu_frame_end() {
                    break 'emulator_loop;
                }
            }
        }

        let frame_data = nes_data.cpu_peripherals.ppu_get_frame();
        texture.update(None, frame_data, 768).unwrap();

        canvas.clear();
        canvas.copy(&dummy_texture, None, None).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        let framerate = 60;

        let elapsed_time = std::time::SystemTime::now()
            .duration_since(prev_time)
            .unwrap()
            .as_nanos();
        prev_time = std::time::SystemTime::now();
        let wait = if elapsed_time < 1_000_000_000u128 / framerate {
            1_000_000_000u32 / framerate as u32 - (elapsed_time as u32)
        } else {
            0
        };
        ::std::thread::sleep(std::time::Duration::new(0, wait));
    }
}
