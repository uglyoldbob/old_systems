//! This is the main implementation of the nes emulator. It provides most of the functionality of the emulator.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    apu::{AudioProducerWithRate, NesApu},
    cartridge::NesCartridge,
    cpu::{NesCpu, NesCpuPeripherals},
    motherboard::NesMotherboard,
    ppu::NesPpu,
    romlist::RomList,
};

#[cfg(feature = "eframe")]
use eframe::egui;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::multi_window::NewWindowRequest;

/// Persistent configuration for the emulator
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EmulatorConfiguration {
    /// Should a rom be persistent from one run to another?
    pub sticky_rom: bool,
    /// What is the startup rom for the emulator?
    start_rom: Option<String>,
    /// What is the duration between save points fo rewinding?
    pub rewind_interval: Option<std::time::Duration>,
    #[serde(skip)]
    /// The path for saving and loading
    path: String,
    /// The root path for all roms
    rom_path: String,
    /// The controller types
    pub controller_type: [crate::controller::NesControllerType; 4],
    /// The controller configuration for all 4 possible controllers.
    pub controller_config: [crate::controller::ControllerConfig; 4],
    /// The scaler to use for the emulator
    pub scaler: Option<crate::ppu::ScalingAlgorithm>,
}

impl Default for EmulatorConfiguration {
    fn default() -> Self {
        let mut controller = [crate::controller::ControllerConfig::new(); 4];
        #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
        {
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_A, egui::Key::F);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_B, egui::Key::D);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_TURBOA, egui::Key::R);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_TURBOB, egui::Key::E);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_SLOW, egui::Key::W);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_START, egui::Key::S);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_SELECT, egui::Key::A);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_UP, egui::Key::ArrowUp);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_DOWN, egui::Key::ArrowDown);
            controller[0].set_key_egui(crate::controller::BUTTON_COMBO_LEFT, egui::Key::ArrowLeft);
            controller[0]
                .set_key_egui(crate::controller::BUTTON_COMBO_RIGHT, egui::Key::ArrowRight);
        }

        Self {
            sticky_rom: true,
            start_rom: None,
            rewind_interval: Some(std::time::Duration::from_millis(5000)),
            path: "".to_string(),
            rom_path: "./roms".to_string(),
            controller_type: [
                crate::controller::NesControllerType::StandardController,
                crate::controller::NesControllerType::None,
                crate::controller::NesControllerType::None,
                crate::controller::NesControllerType::None,
            ],
            controller_config: controller,
            scaler: None,
        }
    }
}

impl EmulatorConfiguration {
    /// Update startup rom if necessary
    pub fn set_startup(&mut self, name: String) {
        if self.sticky_rom {
            self.start_rom = Some(name);
            self.save();
        } else {
            self.start_rom = None;
            self.save();
        }
    }

    /// Retrieve the root path for roms.
    pub fn get_rom_path(&self) -> &str {
        &self.rom_path
    }

    /// Set the new path for roms
    pub fn set_rom_path(&mut self, pb: PathBuf) {
        self.rom_path = pb.into_os_string().into_string().unwrap();
        self.save();
    }

    ///Load a configuration file
    pub fn load(name: std::path::PathBuf) -> Self {
        let mut result = EmulatorConfiguration {
            path: name.clone().into_os_string().into_string().unwrap(),
            ..Default::default()
        };
        if let Ok(a) = std::fs::read(&name) {
            if let Ok(buf) = std::str::from_utf8(&a) {
                match toml::from_str(buf) {
                    Ok(p) => {
                        result = p;
                        result.path = name.into_os_string().into_string().unwrap();
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

/// Just like LocalEmulatorDataClone, but the members must do not implement Clone
pub struct LocalEmulatorData {
    /// The object for interfacing with joysticks.
    pub gilrs: gilrs::Gilrs,
    /// The network object for interacting with other emulators
    pub network: Option<crate::network::Network>,
}

impl Default for LocalEmulatorData {
    fn default() -> Self {
        Self {
            gilrs: gilrs::GilrsBuilder::new().build().unwrap(),
            network: None,
        }
    }
}

/// Stores data local to this particular emulator, not saved into save states. Elements must be cloneable.
#[derive(Clone)]
pub struct LocalEmulatorDataClone {
    /// This contains the non-volatile configuration of the emulator
    pub configuration: EmulatorConfiguration,
    /// The parser for known roms
    pub parser: crate::romlist::RomListParser,
    /// The list of roms for the emulator
    pub roms: RomList,
    /// This variable is for keeping track of which roms have been manually tested
    #[cfg(feature = "rom_status")]
    pub rom_test: crate::rom_status::RomListTestParser,
    /// Indicates that the screen resolution is locked
    pub resolution_locked: bool,
    /// The way to get system specific paths
    dirs: directories::ProjectDirs,
}

impl LocalEmulatorDataClone {
    /// Returns the path to use for save states
    pub fn save_path(&self) -> std::path::PathBuf {
        Self::get_save_path(&self.dirs)
    }

    /// Retrieve the path for other files that get saved
    pub fn get_save_other(&self) -> std::path::PathBuf {
        self.dirs.data_dir().to_path_buf()
    }

    /// Retrieve the default path for roms. The user folder
    pub fn default_rom_path(&self) -> std::path::PathBuf {
        if let Some(pdirs) = directories::UserDirs::new() {
            if let Some(d) = pdirs.document_dir() {
                d.to_path_buf()
            } else {
                pdirs.home_dir().to_path_buf()
            }
        } else if let Some(bdirs) = directories::BaseDirs::new() {
            bdirs.home_dir().to_path_buf()
        } else {
            self.dirs.data_local_dir().to_path_buf()
        }
    }

    /// Convenience function for the new function
    fn get_save_path(dirs: &directories::ProjectDirs) -> std::path::PathBuf {
        let mut pb = dirs.data_dir().to_path_buf();
        pb.push("saves");
        if !pb.exists() {
            std::fs::create_dir_all(&pb);
        }
        pb
    }

    /// Returns the path of where to save recordings to
    pub fn record_path(&self) -> std::path::PathBuf {
        let mut pb = self.dirs.data_dir().to_path_buf();
        pb.push("recordings");
        if !pb.exists() {
            std::fs::create_dir_all(&pb);
        }
        pb
    }

    /// Finds roms for the system
    pub fn find_roms(&mut self, dir: &str) {
        self.parser
            .find_roms(dir, self.save_path(), self.get_save_other())
    }

    /// Process the list of roms
    pub fn process_roms(&mut self) {
        self.parser.process_roms(self.save_path())
    }
}

impl Default for LocalEmulatorDataClone {
    fn default() -> Self {
        let dirs = directories::ProjectDirs::from("com", "uglyoldbob", "nes_emulator").unwrap();

        let mut user_path = dirs.config_dir().to_path_buf();
        user_path.push("config.toml");
        println!("Config path is {}", user_path.display());
        let user_config = EmulatorConfiguration::load(user_path);

        let config = user_config;
        Self {
            configuration: config,
            parser: crate::romlist::RomListParser::default(),
            roms: RomList::load_list(Self::get_save_path(&dirs)),
            #[cfg(feature = "rom_status")]
            rom_test: crate::rom_status::RomListTestParser::new(dirs.data_dir().to_path_buf()),
            resolution_locked: false,
            dirs,
        }
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
    nmi: [bool; 5],
    /// Used for triggering the cpu irq line
    prev_irq: bool,
    /// A large counter used to indicate how many clock cycles have passed. This wraps eventually.
    big_counter: u64,
    /// Indicates vblank was just set
    vblank_just_set: u8,
    #[serde(skip)]
    /// Local emulator data that does not get stored into save states
    pub local: LocalEmulatorDataClone,
    /// Local emulator data that cannot be cloned
    #[serde(skip)]
    pub olocal: Option<LocalEmulatorData>,
}

#[cfg(feature = "egui-multiwin")]
impl NesEmulatorData {
    pub fn process_event(&mut self, _event: egui_multiwin::NoEvent) -> Vec<NewWindowRequest> {
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
            nmi: [false; 5],
            prev_irq: false,
            big_counter: 0,
            vblank_just_set: 0,
            local: LocalEmulatorDataClone::default(),
            olocal: Some(LocalEmulatorData::default()),
        }
    }

    /// Finds roms for the system
    pub fn find_roms(&mut self, dir: &str) {
        self.local.find_roms(dir)
    }

    /// Process the list of roms
    pub fn process_roms(&mut self) {
        self.local.process_roms()
    }

    /// serialize the structure, returning the raw data
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    /// deserialize the structure from the given data
    pub fn deserialize(&mut self, data: Vec<u8>) -> Result<(), Box<bincode::ErrorKind>> {
        match bincode::deserialize::<Self>(&data) {
            Ok(r) => {
                let lcl = self.local.clone();
                let olcl = self.olocal.take();
                let screen = self.cpu_peripherals.ppu.backup_frame();
                let controller1 = self.mb.get_controller(0);
                let controller2 = self.mb.get_controller(1);
                let controller3 = self.mb.get_controller(2);
                let controller4 = self.mb.get_controller(3);
                let cd = self.mb.cartridge_mut().map(|c| c.save_cart_data());
                *self = r;
                cd.and_then(|cd| {
                    self.mb
                        .cartridge_mut()
                        .map(|c| c.restore_cart_data(cd, self.local.save_path()))
                });
                self.mb.set_controller(0, controller1);
                self.mb.set_controller(1, controller2);
                self.mb.set_controller(2, controller3);
                self.mb.set_controller(3, controller4);
                self.local = lcl;
                self.olocal = olcl;
                self.cpu_peripherals.ppu.set_frame(screen);
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
        let controller1 = self.mb.get_controller(0);
        let controller2 = self.mb.get_controller(1);
        let controller3 = self.mb.get_controller(2);
        let controller4 = self.mb.get_controller(3);
        let mb: NesMotherboard = NesMotherboard::new();
        let ppu = NesPpu::new();
        let mut apu = NesApu::new();

        let breakpoints = self.cpu.breakpoints.clone();
        self.cpu = NesCpu::new();
        self.cpu.breakpoints = breakpoints;

        self.cpu_peripherals = NesCpuPeripherals::new(ppu, apu);
        self.mb = mb;

        self.mb.set_controller(0, controller1);
        self.mb.set_controller(1, controller2);
        self.mb.set_controller(2, controller3);
        self.mb.set_controller(3, controller4);

        self.cpu_clock_counter = rand::random::<u8>() % 16;
        self.ppu_clock_counter = rand::random::<u8>() % 4;
        self.last_frame_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        self.nmi = [false; 5];
        self.prev_irq = false;
        if let Some(cart) = cart {
            let name = cart.rom_name();
            let cart = NesCartridge::load_cartridge(name, &self.local.save_path());
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
        self.local
            .configuration
            .set_startup(cart.rom_name().to_owned());
        self.mb.insert_cartridge(cart);
    }

    /// Run a single cycle of the cpu and ppu system, dividing the input as necessary
    pub fn cycle_step(
        &mut self,
        sound: &mut Vec<&mut AudioProducerWithRate>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.big_counter += 1;

        if self.vblank_just_set > 0 {
            self.vblank_just_set -= 1;
        }

        self.ppu_clock_counter += 1;

        if self.ppu_clock_counter >= 4 {
            self.ppu_clock_counter = 0;
            self.cpu_peripherals.ppu_cycle(&mut self.mb);
            if self.cpu_peripherals.ppu.vblank_just_set {
                self.vblank_just_set = 1;
            }
            self.nmi[0] = self.nmi[1];
            self.nmi[1] = self.nmi[2];
            self.nmi[2] = self.nmi[3];
            self.nmi[3] = self.nmi[4];
            self.nmi[4] = self.cpu_peripherals.ppu_irq();
        }

        self.cpu_clock_counter += 1;
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            let nmi = self.nmi[2];

            self.cpu_peripherals.apu.clock_slow(sound, filter);
            let irq = self.cpu_peripherals.apu.irq();
            self.cpu.set_dma_input(self.cpu_peripherals.apu.dma());
            let cart_irq = self.mb.cartridge().map(|cart| cart.irq()).unwrap_or(false);
            self.cpu.cycle(
                &mut self.mb,
                &mut self.cpu_peripherals,
                nmi,
                self.prev_irq | cart_irq,
            );
            if self.cpu_peripherals.ppu.vblank_clear && self.vblank_just_set > 0 {
                self.cpu_peripherals.ppu.suppress_nmi();
                self.nmi[0] = false;
                self.nmi[1] = false;
                self.nmi[2] = false;
                self.nmi[3] = false;
                self.nmi[4] = false;
            }
            self.prev_irq = irq;
        }

        if self.ppu_clock_counter == 0 {}
    }
}
