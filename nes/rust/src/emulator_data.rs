//! This is the main implementation of the nes emulator. It provides most of the functionality of the emulator.

use crate::{
    apu::NesApu,
    cartridge::NesCartridge,
    cpu::{NesCpu, NesCpuPeripherals},
    motherboard::NesMotherboard,
    ppu::NesPpu,
    romlist::RomList,
};

#[cfg(feature = "eframe")]
use eframe::egui;

/// The main struct for the nes emulator.
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesEmulatorData {
    /// The 6502 cpu
    pub cpu: NesCpu,
    /// The peripherals of the cpu for the emulator
    pub cpu_peripherals: NesCpuPeripherals,
    /// The motherboard for the emualtor
    pub mb: NesMotherboard,
    /// Used for operating the cpu clock divider
    pub cpu_clock_counter: u8,
    /// Used for operating the ppu clock divider
    ppu_clock_counter: u8,
    /// Indicates that the emulator is paused.
    #[cfg(debug_assertions)]
    pub paused: bool,
    /// Indicates that the cpu should be single stepped, used for debugging
    #[cfg(debug_assertions)]
    pub single_step: bool,
    /// Used for debugging, to indicate to run to the end of the current frame, then pause.
    #[cfg(debug_assertions)]
    pub wait_for_frame_end: bool,
    /// Used for frame timing
    pub last_frame_time: u128,
    /// Used for emulating the proper behavior of the cpu for the nmi interrupt
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    nmi: [bool; 3],
    /// Used for triggering the cpu irq line
    prev_irq: bool,
    /// The list of roms for the emulator
    pub roms: RomList,
}

impl NesEmulatorData {
    /// Create a new nes emulator
    pub fn new() -> Self {
        let mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();
        let apu = NesApu::new();

        Self {
            cpu: NesCpu::new(),
            cpu_peripherals: NesCpuPeripherals::new(ppu, apu),
            mb,
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
            nmi: [false; 3],
            prev_irq: false,
            roms: RomList::load_list(),
        }
    }

    /// serialize the structure, returning the raw data
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    /// deserialize the structure from the given data
    pub fn deserialize(&mut self, data: Vec<u8>) -> Result<(), Box<bincode::ErrorKind>> {
        match bincode::deserialize::<Self>(&data) {
            Ok(r) => {
                *self = r;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Reset the cpu, ppu, and apu.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu_peripherals.ppu_reset();
        self.cpu_peripherals.apu.reset();
    }

    /// Remove a cartridge from the motherboard, throwing it away.
    pub fn remove_cartridge(&mut self) {
        self.mb.remove_cartridge();
    }

    /// Insert a cartridge into the motherboard.
    pub fn insert_cartridge(&mut self, cart: NesCartridge) {
        self.mb.insert_cartridge(cart);
    }

    /// Run a cycle for the ppu
    pub fn ppu_step(&mut self) {
        self.cpu_peripherals.ppu_cycle(&mut self.mb);
    }

    /// Run a single cycle of the cpu and ppu system, dividing the input as necessary
    pub fn cycle_step(
        &mut self,
        sound: &mut Option<rb::Producer<f32>>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.cpu_clock_counter += 1;
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            let nmi = self.nmi[0] && self.nmi[1] && self.nmi[2];
            self.cpu_peripherals.apu.clock_slow_pre();
            let irq = self.cpu_peripherals.apu.irq();
            self.cpu.set_dma_input(self.cpu_peripherals.apu.dma());
            self.cpu
                .cycle(&mut self.mb, &mut self.cpu_peripherals, nmi, self.prev_irq);
            self.prev_irq = irq;
            self.cpu_peripherals.apu.clock_slow(sound, filter);
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
