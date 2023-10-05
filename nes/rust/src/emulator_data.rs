//! This is the main implementation of the nes emulator. It provides most of the functionality of the emulator.

use std::io::Write;

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
use egui_multiwin::multi_window::CommonEventHandler;

/// Persistent configuration for the emulator
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmulatorConfiguration {
    /// Should a rom be persistent from one run to another?
    sticky_rom: bool,
    /// What is the startup rom for the emulator?
    start_rom: Option<String>,
    #[serde(skip)]
    /// The path for saving and loading
    path: String,
    /// The root path for all roms
    rom_path: String,
}

impl Default for EmulatorConfiguration {
    fn default() -> Self {
        Self {
            sticky_rom: true,
            start_rom: None,
            path: "".to_string(),
            rom_path: "./roms".to_string(),
        }
    }
}

impl EmulatorConfiguration {
    /// Update startup rom if necessary
    pub fn set_startup(&mut self, name: String) {
        if self.sticky_rom {
            self.start_rom = Some(name);
            self.save();
        }
    }

    /// Retrieve the root path for roms.
    pub fn get_rom_path(&self) -> &str {
        &self.rom_path
    }

    ///Load a configuration file
    pub fn load(name: String) -> Self {
        let mut result = EmulatorConfiguration {
            path: name.to_owned(),
            ..Default::default()
        };
        if let Ok(a) = std::fs::read(&name) {
            if let Ok(buf) = std::str::from_utf8(&a) {
                match toml::from_str(buf) {
                    Ok(p) => {
                        result = p;
                        result.path = name.to_owned();
                    }
                    Err(e) => {
                        println!("Failed to load config file: {}", e);
                    }
                }
            }
        } else {
            result.save();
        }
        result
    }

    /// Save results to disk
    pub fn save(&self) {
        let data = toml::to_string(self).unwrap();

        let mut path = std::path::PathBuf::from(&self.path);
        path.pop();
        let _ = std::fs::create_dir_all(path);
        let mut options = std::fs::OpenOptions::new();
        let mut f = if std::path::Path::new(&self.path).exists() {
            options
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.path)
                .unwrap()
        } else {
            options
                .write(true)
                .create_new(true)
                .open(&self.path)
                .unwrap()
        };
        let _e = f.write_all(data.as_bytes());
    }

    ///Retrieve the start rom
    pub fn start_rom(&self) -> Option<String> {
        self.start_rom.to_owned()
    }
}

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
    #[cfg(feature = "debugger")]
    pub paused: bool,
    /// Indicates that the cpu should be single stepped, used for debugging
    #[cfg(feature = "debugger")]
    pub single_step: bool,
    /// Used for debugging, to indicate to run to the end of the current frame, then pause.
    #[cfg(feature = "debugger")]
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
    /// The parser for known roms
    #[serde(skip)]
    pub parser: crate::romlist::RomListParser,
    /// This variable is for keeping track of which roms have been manually tested
    #[cfg(feature = "rom_status")]
    #[serde(skip)]
    pub rom_test: crate::rom_status::RomListTestParser,
    /// This contains the non-volatile configuration of the emulator
    #[serde(skip)]
    pub configuration: EmulatorConfiguration,
}

impl CommonEventHandler<NesEmulatorData, u32> for NesEmulatorData {
    fn process_event(
        &mut self,
        _event: u32,
    ) -> Vec<egui_multiwin::multi_window::NewWindowRequest<NesEmulatorData>> {
        vec![]
    }
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
            #[cfg(feature = "debugger")]
            paused: false,
            #[cfg(feature = "debugger")]
            single_step: false,
            #[cfg(feature = "debugger")]
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
            parser: crate::romlist::RomListParser::new(),
            #[cfg(feature = "rom_status")]
            rom_test: crate::rom_status::RomListTestParser::new(),
            configuration: EmulatorConfiguration::load("./config.toml".to_string()),
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
                let config_path = self.configuration.path.to_owned();
                *self = r;
                self.configuration.path = config_path;
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

    /// Effectively power cycles the emulator. Technically throws away the current system and builds a new one.
    pub fn power_cycle(&mut self) {
        let cart = self.remove_cartridge();
        let controller1 = self.mb.controllers[0].take();
        let controller2 = self.mb.controllers[1].take();
        let mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();
        let mut apu = NesApu::new();

        let audio_interval = self.cpu_peripherals.apu.get_audio_interval();
        let buffer_len = self.cpu_peripherals.apu.get_audio_buffer_length();
        apu.set_audio_interval(audio_interval);
        apu.set_audio_buffer(buffer_len);

        let breakpoints = self.cpu.breakpoints.clone();
        self.cpu = NesCpu::new();
        self.cpu.breakpoints = breakpoints;

        self.cpu_peripherals = NesCpuPeripherals::new(ppu, apu);
        self.mb = mb;

        self.mb.controllers[0] = controller1;
        self.mb.controllers[1] = controller2;

        self.cpu_clock_counter = rand::random::<u8>() % 16;
        self.ppu_clock_counter = rand::random::<u8>() % 4;
        self.last_frame_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        self.nmi = [false; 3];
        self.prev_irq = false;
        if let Some(cart) = cart {
            let name = cart.rom_name();
            let cart = NesCartridge::load_cartridge(name);
            if let Ok(cart) = cart {
                self.insert_cartridge(cart);
            }
        }
    }

    /// Remove a cartridge from the motherboard, returning it to the caller.
    pub fn remove_cartridge(&mut self) -> Option<NesCartridge> {
        self.mb.remove_cartridge()
    }

    /// Insert a cartridge into the motherboard.
    pub fn insert_cartridge(&mut self, cart: NesCartridge) {
        self.configuration.set_startup(cart.rom_name().to_owned());
        self.mb.insert_cartridge(cart);
    }

    /// Run a single cycle of the cpu and ppu system, dividing the input as necessary
    pub fn cycle_step(
        &mut self,
        sound: &mut Option<ringbuf::Producer<f32, std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>>>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.cpu_clock_counter += 1;
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            let nmi = self.nmi[0] && self.nmi[1] && self.nmi[2];
            self.cpu_peripherals.apu.clock_slow(sound, filter);
            let irq = self.cpu_peripherals.apu.irq();
            self.cpu.set_dma_input(self.cpu_peripherals.apu.dma());
            self.cpu
                .cycle(&mut self.mb, &mut self.cpu_peripherals, nmi, self.prev_irq);
            self.prev_irq = irq;
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
