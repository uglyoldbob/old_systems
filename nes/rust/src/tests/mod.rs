use std::io::BufRead;

use crate::apu::NesApu;
use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesCpuPeripherals;
use crate::cpu::NesMemoryBus;
use crate::motherboard::NesMotherboard;
use crate::ppu::NesPpu;
use crate::utility::convert_hex_to_decimal;
use crate::NesEmulatorData;

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
    let apu: NesApu = NesApu::new();
    let mut cpu_peripherals: NesCpuPeripherals = NesCpuPeripherals::new(ppu, apu);
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("../test_roms/other/nestest.nes".to_string());
    let goldenlog = std::fs::File::open("../test_roms/other/nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();
    let mut log_line = 0;

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String;
    let mut b;
    for i in 0..26554 {
        cpu.cycle(&mut mb, &mut cpu_peripherals, false, false);
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
    let nc =
        NesCartridge::load_cartridge("../test_roms/vbl_nmi_timing/1.frame_basics.nes".to_string())
            .unwrap();
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
    let nc =
        NesCartridge::load_cartridge("../test_roms/vbl_nmi_timing/2.vbl_timing.nes".to_string())
            .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/vbl_nmi_timing/3.even_odd_frames.nes".to_string(),
    )
    .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/vbl_nmi_timing/4.vbl_clear_timing.nes".to_string(),
    )
    .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/vbl_nmi_timing/5.nmi_suppression.nes".to_string(),
    )
    .unwrap();
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
    let nc =
        NesCartridge::load_cartridge("../test_roms/vbl_nmi_timing/6.nmi_disable.nes".to_string())
            .unwrap();
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
fn vbl_nmi_test7() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/vbl_nmi_timing/7.nmi_timing.nes".to_string())
            .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/branch_timing_tests/1.Branch_Basics.nes".to_string(),
    )
    .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/branch_timing_tests/2.Backward_Branch.nes".to_string(),
    )
    .unwrap();
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
    let nc = NesCartridge::load_cartridge(
        "../test_roms/branch_timing_tests/3.Forward_Branch.nes".to_string(),
    )
    .unwrap();
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
fn ppu_open_bus() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("../test_roms/ppu_open_bus/ppu_open_bus.nes".to_string())
        .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 251 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn ppu_test1() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_ppu_tests_2005.09.15b/palette_ram.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01".to_string().as_bytes()));
}

#[test]
fn ppu_test2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_ppu_tests_2005.09.15b/sprite_ram.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 40 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01".to_string().as_bytes()));
}

#[test]
fn ppu_test3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_ppu_tests_2005.09.15b/vbl_clear_time.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 40 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01".to_string().as_bytes()));
}

#[test]
fn ppu_test4() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_ppu_tests_2005.09.15b/vram_access.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 40 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01".to_string().as_bytes()));
}

#[test]
fn cpu_test_dummy_writes_oam() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/cpu_dummy_writes/cpu_dummy_writes_oam.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 350 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(833, "0ASSED".to_string().as_bytes()));
}

#[test]
fn cpu_test_dummy_writes_ppu() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/cpu_dummy_writes/cpu_dummy_writes_ppumem.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 250 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(769, "0ASSED".to_string().as_bytes()));
}

#[test]
fn cpu_test_exec_space_ppu() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/cpu_exec_space/test_cpu_exec_space_ppuio.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 48 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(641, "0ASSED".to_string().as_bytes()));
}

#[test]
fn cpu_timing_test() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/cpu_timing_test6/cpu_timing_test.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 645 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(258, "PASSED".to_string().as_bytes()));
}

#[test]
fn cpu_dma_test2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/dmc_dma_during_read4/dma_2007_write.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(289, "Passed".to_string().as_bytes()));
}

#[test]
fn oam_test() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/oam_read/oam_read.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 33 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(673, "Passed".to_string().as_bytes()));
}

#[test]
fn oam_stress() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/oam_stress/oam_stress.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 1793 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(673, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test1() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/01.len_ctr.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/02.len_table.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/03.irq_flag.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test4() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/04.clock_jitter.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test5() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/05.len_timing_mode0.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test6() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/06.len_timing_mode1.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test7() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/07.irq_flag_timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test8() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/08.irq_timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test9() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/blargg_apu_2005.07.30/09.reset_timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(162, "$01 ".to_string().as_bytes()));
}

#[test]
fn apu_test10() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("../test_roms/apu_reset/4015_cleared.nes".to_string())
        .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test11() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_reset/4017_timing.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(193, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(225, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test12() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("../test_roms/apu_reset/4017_written.nes".to_string())
        .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET again".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test13() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_reset/irq_flag_cleared.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test14() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_reset/len_ctrs_enabled.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test15() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_reset/works_immediately.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press RESET".to_string().as_bytes()));
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_1() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_test/rom_singles/1-len_ctr.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/2-len_table.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/3-irq_flag.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_4() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/apu_test/rom_singles/4-jitter.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_5() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/5-len_timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 130 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_6() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/6-irq_flag_timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 25 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_7() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/7-dmc_basics.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn apu_test16_8() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/apu_test/rom_singles/8-dmc_rates.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn cpu_test_exec_space_apu() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/cpu_exec_space/test_cpu_exec_space_apu.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 315 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(513, "0ASSED".to_string().as_bytes()));
}

#[test]
fn cpu_reset() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("../test_roms/cpu_reset/ram_after_reset.nes".to_string())
        .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(129, "Press reset AFTER".to_string().as_bytes()));
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 145 {
                break;
            }
        }
    }
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(161, "Passed".to_string().as_bytes()));
}

#[test]
fn cpu_reset2() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/cpu_reset/registers.nes".to_string()).unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 20 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(193, "Press reset AFTER".to_string().as_bytes()));
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 150 {
                break;
            }
        }
    }
    nes_data.reset();
    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 15 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(225, "Passed".to_string().as_bytes()));
}

#[test]
fn cpu_timing_test2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge("../test_roms/instr_timing/instr_timing.nes".to_string())
        .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 300 {
                break;
            }
        }
    }
    assert!(nes_data
        .mb
        .check_vram(65, "All 2 tests passed".to_string().as_bytes()));
}

#[test]
fn ppu_sprite_test_1() {
    let mut nes_data = NesEmulatorData::new();
    let nc =
        NesCartridge::load_cartridge("../test_roms/sprite_overflow_tests/1.Basics.nes".to_string())
            .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn ppu_sprite_test_2() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/sprite_overflow_tests/2.Details.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}

#[test]
fn ppu_sprite_test_3() {
    let mut nes_data = NesEmulatorData::new();
    let nc = NesCartridge::load_cartridge(
        "../test_roms/sprite_overflow_tests/3.Timing.nes".to_string(),
    )
    .unwrap();
    nes_data.insert_cartridge(nc);

    loop {
        nes_data.cycle_step();
        if nes_data.cpu_peripherals.ppu_frame_end() {
            if nes_data.cpu_peripherals.ppu_frame_number() == 30 {
                break;
            }
        }
    }
    assert!(nes_data.mb.check_vram(194, "PASSED".to_string().as_bytes()));
}
