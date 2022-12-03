mod cpu;

struct NesMotherboard {}

impl NesMemoryBus for NesMotherboard {
    pub fn new() -> Self {
        Self {

        }
    }
}

fn main() {
    println!("I am groot!");

    let cpu: NesCpu = NesCpu::new();
    //board ram is random on startup
    let mut main_ram: [u8; 2048] = [0; 2048];
    for i in r.iter_mut() {
        *i = rand::random();
    }
    let mb: NesMotherboard = NesMotherboard::new();

    while (true) {
        cpu.cycle(&mut mb);
    }
}
