use crate::{
    apu::NesApu,
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
    prev_irq: bool,
}

impl NesEmulatorData {
    pub fn new() -> Self {
        let mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();
        let apu = NesApu::new();

        Self {
            cpu: NesCpu::new(),
            cpu_peripherals: NesCpuPeripherals::new(ppu, apu),
            mb: mb,
            #[cfg(debug_assertions)]
            paused: true,
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
            prev_irq: false,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu_peripherals.ppu_reset();
        self.cpu_peripherals.apu.reset();
    }

    pub fn insert_cartridge(&mut self, cart: NesCartridge) {
        self.mb.insert_cartridge(cart);
    }

    pub fn ppu_step(&mut self) {
        self.cpu_peripherals.ppu_cycle(&mut self.mb);
    }

    pub fn cycle_step(&mut self) {
        self.cpu_clock_counter += 1;
        self.cpu_peripherals.apu.clock_fast();
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            let nmi = self.nmi[0] && self.nmi[1] && self.nmi[2];
            let irq = self.cpu_peripherals.apu.irq();
            self.cpu
                .cycle(&mut self.mb, &mut self.cpu_peripherals, nmi, self.prev_irq);
            self.prev_irq = irq;
            self.cpu_peripherals.apu.clock_slow();
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
