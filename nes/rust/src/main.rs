mod cartridge;
mod cpu;

use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesMemoryBus;

struct NesMotherboard {}

impl NesMotherboard {
    fn new() -> Self {
        Self {}
    }
}

impl NesMemoryBus for NesMotherboard {
    fn memory_cycle_read(&mut self, addr: u16, out: [bool; 3], controllers: [bool; 2]) -> u8 {
        0xff
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
        assert!(nc.is_ok(), "Unable to load rom because {:?}", nc.err().unwrap());
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

    loop {
        cpu.cycle(&mut mb);
    }
}
