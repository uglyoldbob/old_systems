use std::io::BufRead;

use crate::NesEmulatorData;
use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesCpuPeripherals;
use crate::cpu::NesMemoryBus;
use crate::motherboard::NesMotherboard;
use crate::ppu::NesPpu;
use crate::utility::convert_hex_to_decimal;

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
        cpu.cycle(&mut mb, &mut cpu_peripherals, false);
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

#[test]
fn vbl_nmi_test1() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./1.frame_basics.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 176 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./2.vbl_timing.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 156 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./3.even_odd_frames.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 101 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test4() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./4.vbl_clear_timing.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 119 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test5() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./5.nmi_suppression.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 168 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test6() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./6.nmi_disable.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 110 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn vbl_nmi_test7() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./7.nmi_timing.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 111 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn cpu_branch_timing1() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./1.Branch_Basics.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 14 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}


#[test]
fn cpu_branch_timing2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./2.Backward_Branch.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 16 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn cpu_branch_timing3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("./3.Forward_Branch.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 16 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}