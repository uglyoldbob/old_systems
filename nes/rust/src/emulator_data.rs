use crate::{
    cartridge::NesCartridge,
    cpu::{NesCpu, NesCpuPeripherals},
    motherboard::NesMotherboard,
    ppu::NesPpu,
};

pub struct NesEmulatorData {
    pub cpu: NesCpu,
    pub cpu_peripherals: NesCpuPeripherals,
    pub mb: NesMotherboard,
    pub cpu_clock_counter: u8,
    ppu_clock_counter: u8,
    #[cfg(debug_assertions)]
    pub paused: bool,
    #[cfg(debug_assertions)]
    pub single_step: bool,
    #[cfg(debug_assertions)]
    pub wait_for_frame_end: bool,
    pub last_frame_time: u128,
    pub texture: Option<egui::TextureHandle>,
    nmi: [bool; 3],
}

impl NesEmulatorData {
    pub fn new() -> Self {
        let mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();

        Self {
            cpu: NesCpu::new(),
            cpu_peripherals: NesCpuPeripherals::new(ppu),
            mb: mb,
            #[cfg(debug_assertions)]
            paused: false,
            #[cfg(debug_assertions)]
            single_step: false,
            #[cfg(debug_assertions)]
            wait_for_frame_end: false,
            cpu_clock_counter: rand::random::<u8>() % 16,
            ppu_clock_counter: rand::random::<u8>() % 4,
            last_frame_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            texture: None,
            nmi: [false; 3],
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu_peripherals.ppu_reset();
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
            let nmi = self.nmi[0] && self.nmi[1] && self.nmi[2];
            self.cpu.cycle(&mut self.mb, &mut self.cpu_peripherals, nmi);
        }

        self.ppu_clock_counter += 1;
        if self.ppu_clock_counter >= 4 {
            self.ppu_clock_counter = 0;
            self.cpu_peripherals.ppu_cycle(&mut self.mb);
            self.nmi[0] = self.nmi[1];
            self.nmi[1] = self.nmi[2];
            self.nmi[2] = self.cpu_peripherals.ppu_irq();
        }
    }
}
