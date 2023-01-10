use crate::{
    cartridge::NesCartridge,
    cpu::{NesCpu, NesCpuPeripherals},
    motherboard::NesMotherboard,
    ppu::NesPpu,
};

pub struct NesEmulatorData {
    pub cpu: NesCpu,
    pub cpu_peripherals: NesCpuPeripherals,
    mb: NesMotherboard,
    pub cpu_clock_counter: u8,
    ppu_clock_counter: u8,
    #[cfg(debug_assertions)]
    pub paused: bool,
    #[cfg(debug_assertions)]
    pub single_step: bool,
    pub last_frame_time: u128,
    pub texture: Option<egui::TextureHandle>,
}

impl NesEmulatorData {
    pub fn new() -> Self {
        let mut mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();

        Self {
            cpu: NesCpu::new(),
            cpu_peripherals: NesCpuPeripherals::new(ppu),
            mb: mb,
            #[cfg(debug_assertions)]
            paused: false,
            #[cfg(debug_assertions)]
            single_step: false,
            cpu_clock_counter: rand::random::<u8>() % 16,
            ppu_clock_counter: rand::random::<u8>() % 4,
            last_frame_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            texture: None,
        }
    }

    pub fn insert_cartridge(&mut self, cart: NesCartridge) {
        self.mb.insert_cartridge(cart);
    }

    pub fn ppu_step(&mut self) {
        self.cpu_peripherals.ppu_cycle(&mut self.mb);
    }

    pub fn cycle_step(&mut self) {
        self.cpu_clock_counter += 1;
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            self.cpu.cycle(&mut self.mb, &mut self.cpu_peripherals);
        }

        self.ppu_clock_counter += 1;
        if self.ppu_clock_counter >= 4 {
            self.ppu_clock_counter = 0;
            self.cpu_peripherals.ppu_cycle(&mut self.mb);
        }
    }
}
