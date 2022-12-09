mod cartridge;
mod cpu;

use std::io::BufRead;

use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesMemoryBus;

struct NesMotherboard {
    cart: Option<NesCartridge>,
}

impl NesMotherboard {
    fn new() -> Self {
        Self { cart: None }
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
        if let Some(cart) = &mut self.cart {
            let resp = cart.memory_read(addr);
            if let Some(v) = resp {
                response = v;
            }
        }
        response
    }
    fn memory_cycle_write(&mut self, addr: u16, data: u8, out: [bool; 3], controllers: [bool; 2]) {}
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

#[test]
fn basic_cpu_test() {
    let mut cpu: NesCpu = NesCpu::new();
    //board ram is random on startup
    let mut main_ram: [u8; 2048] = [0; 2048];
    for i in main_ram.iter_mut() {
        *i = rand::random();
    }
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nestest.nes".to_string());
    let goldenlog = std::fs::File::open("./nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String = "".to_string();
    for i in 0..26554 {
        if cpu.instruction_start() {
            t = goldenlog.next().unwrap().unwrap();
        }
        cpu.cycle(&mut mb);
        if cpu.instruction_start() {
            println!("Instruction end at cycle {}", i + 1);
            println!("NESTEST LOG LINE: {}", t);
        }
    }
}

fn main() {
    println!("I am groot!");

    let mut cpu: NesCpu = NesCpu::new();
    //board ram is random on startup
    let mut main_ram: [u8; 2048] = [0; 2048];
    for i in main_ram.iter_mut() {
        *i = rand::random();
    }
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nes/rust/nestest.nes".to_string());
    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    loop {
        cpu.cycle(&mut mb);
    }
}
