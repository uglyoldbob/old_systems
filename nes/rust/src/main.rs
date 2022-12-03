mod cpu;

use crate::cpu::NesCpu;
use crate::cpu::NesMemoryBus;

struct NesMotherboard {}

impl NesMotherboard {
    fn new() -> Self {
        Self {}
    }
}

impl NesMemoryBus for NesMotherboard {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        out: [bool; 3],
        controllers: [bool; 2],
    ) -> Option<u8> {
        None
    }
    fn memory_cycle_write(&mut self, addr: u16, data: u8, out: [bool; 3], controllers: [bool; 2]) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
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

    loop {
        cpu.cycle(&mut mb);
    }
}
