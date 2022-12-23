mod cartridge;
mod cpu;

use std::io::BufRead;

use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesMemoryBus;

struct NesMotherboard {
    cart: Option<NesCartridge>,
    ram: [u8; 2048],
}

impl NesMotherboard {
    fn new() -> Self {
        //board ram is random on startup
        let mut main_ram: [u8; 2048] = [0; 2048];
        for i in main_ram.iter_mut() {
            *i = rand::random();
        }
        Self {
            cart: None,
            ram: main_ram,
        }
    }

    fn insert_cartridge(&mut self, c: NesCartridge) {
        if let None = self.cart {
            self.cart = Some(c);
        }
    }
}

impl NesMemoryBus for NesMotherboard {
    fn memory_cycle_read(&mut self, addr: u16, out: [bool; 3], controllers: [bool; 2]) -> u8 {
        let mut response: u8 = 0;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = self.ram[addr as usize];
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                //ppu registers
            }
            0x4000..=0x4017 => {
                //apu and io
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    let resp = cart.memory_read(addr);
                    if let Some(v) = resp {
                        response = v;
                    }
                }
            }
        }
        response
    }
    fn memory_cycle_write(&mut self, addr: u16, data: u8, out: [bool; 3], controllers: [bool; 2]) {
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                self.ram[addr as usize] = data;
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                //ppu registers
            }
            0x4000..=0x4017 => {
                //apu and io
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    //TODO?
                }
            }
        }
    }
}

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

fn convert_hex_to_decimal(d: char) -> u8 {
    match d {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' | 'a' => 10,
        'B' | 'b' => 11,
        'C' | 'c' => 12,
        'D' | 'd' => 13,
        'E' | 'e' => 14,
        'F' | 'f' => 15,
        _ => 0,
    }
}

#[test]
fn basic_cpu_test() {
    let mut cpu: NesCpu = NesCpu::new();

    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nestest.nes".to_string());
    let goldenlog = std::fs::File::open("./nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();
    let mut log_line = 0;

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String = "".to_string();
    let mut b = t.as_bytes();
    for i in 0..26554 {
        cpu.cycle(&mut mb);
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
}

fn main() {
    println!("I am groot!");

    let mut cpu: NesCpu = NesCpu::new();
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nes/rust/nestest.nes".to_string());
    let goldenlog = std::fs::File::open("./nes/rust/nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();
    let mut log_line = 0;

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String = "".to_string();
    let mut b = t.as_bytes();
    for i in 0..26554 {
        cpu.cycle(&mut mb);
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
}
