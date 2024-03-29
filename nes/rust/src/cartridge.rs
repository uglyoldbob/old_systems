//! This module is responsible for cartridge related emulation, including mapper emulation.

mod mapper00;
mod mapper01;
mod mapper02;
mod mapper03;
mod mapper04;
mod mapper05;
mod mapper34;
mod mapper71;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use common_emulator::{storage::PersistentStorage, CartridgeError};
use mapper00::Mapper00;
use mapper01::Mapper01;
use mapper02::Mapper02;
use mapper03::Mapper03;
use mapper04::Mapper04;
use mapper05::Mapper05;
use mapper34::Mapper34;
use mapper71::Mapper71;

use crate::genie::GameGenieCode;

/// All mappers must implement this.
#[enum_dispatch::enum_dispatch]
trait NesMapperTrait {
    /// Dump some data from the cart
    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8>;
    /// Run a cpu memory read cycle
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8>;
    /// A read cycle that does not target cartridge memory. Used for mappers that monitor reads like mmc5.
    fn other_memory_read(&mut self, cart: &mut NesCartridgeData, addr: u16) {}
    /// Run a cpu memory write cycle
    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8);
    /// A write cycle that does not target cartridge memory. Used for mappers that monitor writes like mmc5.
    fn other_memory_write(&mut self, _addr: u16, _data: u8) {}
    /// Runs a memory cycle that does nothing, for mappers that need to do special things.
    fn memory_cycle_nop(&mut self);
    #[must_use]
    /// performs the first half of a ppu memory cycle
    /// returns A10 for internal VRAM and the motherboard CS line (for internal VRAM)
    /// A10 is straight forward, CS line is active low like the electronics would be
    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool);
    /// Run a ppu read cycle
    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8>;
    /// Run a ppu write cycle
    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8);
    /// Peek at a ppu memory address
    fn ppu_peek_address(&self, adr: u16, cart: &NesCartridgeData) -> (bool, bool, Option<u8>);
    /// Modify a byte for the cartridge rom
    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8);
    /// Returns a list of registers used by the cartridge
    fn cartridge_registers(&self) -> BTreeMap<String, u8>;
    /// Retrieve the irq signal
    fn irq(&self) -> bool;
    /// Checks for active game genie codes and acts appropriately
    fn genie(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        if cart.volatile.genie.len() > 0 {
            let a = if (0xe000..=0xffff).contains(&addr) {
                let mut a = self.memory_cycle_read(cart, addr);
                for code in &cart.volatile.genie {
                    if code.address() == addr {
                        if let Some(check) = code.check() {
                            let lv = self.memory_cycle_dump(cart, addr ^ 0x8000);
                            if let Some(lv) = lv {
                                if a == Some(check) {
                                    a = Some(lv & code.value());
                                }
                            } else {
                                if a == Some(check) {
                                    a = Some(code.value());
                                }
                            }
                        } else {
                            let lv = self.memory_cycle_dump(cart, addr ^ 0x8000);
                            if let Some(lv) = lv {
                                a = Some(lv & code.value());
                            } else {
                                a = Some(code.value());
                            }
                        }
                    }
                }
                a
            } else {
                let mut a = self.memory_cycle_read(cart, addr);
                for code in &cart.volatile.genie {
                    if code.address() == addr {
                        if let Some(check) = code.check() {
                            if a == Some(check) {
                                a = Some(code.value());
                            }
                        } else {
                            a = Some(code.value());
                        }
                    }
                }
                a
            };
            a
        } else {
            self.memory_cycle_read(cart, addr)
        }
    }
}

/// The mapper for an nes cartridge
#[non_exhaustive]
#[enum_dispatch::enum_dispatch(NesMapperTrait)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum NesMapper {
    Mapper00,
    Mapper01,
    Mapper02,
    Mapper03,
    Mapper04,
    Mapper05,
    Mapper34,
    Mapper71,
}

/// The trait for cpu memory reads and writes, implemented by devices on the bus
pub trait NesMemoryBusDevice {
    /// Run a cpu memory read cycle on the cartridge
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        out: [bool; 3],
        controllers: [bool; 2],
    ) -> Option<u8>;
    /// Run a cpu memory write cycle on the cartridge
    fn memory_cycle_write(&mut self, addr: u16, data: u8);
}

/// The data for a cartridge.
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCartridgeData {
    #[serde(skip)]
    /// The nonvolatile data for the cartridge
    pub nonvolatile: NonvolatileCartridgeData,
    /// The potentially volatile cartridge data
    pub volatile: VolatileCartridgeData,
}

/// Nonvolatile storage for cartridge data
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct NonvolatileCartridgeData {
    /// An optional trainer for the cartridge
    pub trainer: Option<Vec<u8>>,
    /// The prg rom, where code typically goes.
    pub prg_rom: Vec<u8>,
    /// The chr rom, where graphics are generally stored
    pub chr_rom: Vec<u8>,
    /// inst_rom ?
    pub inst_rom: Option<Vec<u8>>,
    /// prom?
    pub prom: Option<(Vec<u8>, Vec<u8>)>,
}

/// Volatile storage for cartridge data
#[derive(serde::Serialize, serde::Deserialize)]
pub struct VolatileCartridgeData {
    /// Program ram
    pub prg_ram: PersistentStorage,
    /// Battery backup for prog ram active?
    pub battery_backup: bool,
    /// True for vertical mirroring, false for horizontal mirroring
    pub mirroring: bool,
    /// The mapper number
    pub mapper: u32,
    /// Where chr-ram is stored
    pub chr_ram: Vec<u8>,
    /// A list of game genie codes
    pub genie: Vec<crate::genie::GameGenieCode>,
}

impl VolatileCartridgeData {
    /// Remove a game genie code
    pub fn remove_code(&mut self, code: &GameGenieCode) {
        let mut codes = Vec::new();
        let len = self.genie.len();
        for i in 0..len {
            let c = self.genie.pop().unwrap();
            if c != *code {
                codes.push(c);
            }
        }
        self.genie = codes;
    }
}

#[non_exhaustive]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// The format that a rom file is loaded from
pub enum RomFormat {
    /// The ines1 rom format
    Ines1,
    /// The ines2 rom format
    Ines2,
}

/// A cartridge, including the mapper structure
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCartridge {
    /// The data in the cartridge, including ram and everything else
    data: NesCartridgeData,
    /// The mapper
    mapper: NesMapper,
    /// The mapper number
    mappernum: u32,
    /// Rom format loaded from
    pub rom_format: RomFormat,
    /// The hash of the rom contents
    hash: String,
    /// The name to use for save games
    save: String,
    #[serde(skip)]
    /// The convenience name of the rom
    rom_name: String,
}

/// The data from a cartridge that needs to be saved when loading a save state
pub struct NesCartridgeBackup {
    /// The non-volatile data in the cartridge
    data: NonvolatileCartridgeData,
    /// The convenience name of the rom
    rom_name: String,
}

/// Calculate the sha256 of a chunk of data, and return it in a hex encoded string.
fn calc_sha256(data: &[u8]) -> String {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    context.update(data);
    let digest = context.finish();
    data_encoding::HEXLOWER.encode(digest.as_ref())
}

impl common_emulator::romlist::GetMapperNumber for NesCartridge {
    fn mappernum(&self) -> u32 {
        self.mappernum
    }
}

impl NesCartridge {
    /// Helper function to get the irq signal from the cartridge
    pub fn irq(&self) -> bool {
        self.mapper.irq()
    }

    /// "Parses" an obsolete ines rom
    fn load_obsolete_ines(_name: String, _rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        Err(CartridgeError::IncompatibleRom)
    }

    /// Saves the contents of the cartridge data so that it can be restored after loading a save state.
    pub fn save_cart_data(&mut self) -> NesCartridgeBackup {
        NesCartridgeBackup {
            data: self.data.nonvolatile.clone(),
            rom_name: self.rom_name.clone(),
        }
    }

    /// Restore previously saved data after loading a save state.
    pub fn restore_cart_data(&mut self, old_data: NesCartridgeBackup, mut pb: PathBuf) {
        self.data.nonvolatile = old_data.data;
        pb.push(format!("{}.prgram", self.save));
        self.data.volatile.prg_ram.upgrade_to_persistent(pb);
        self.rom_name = old_data.rom_name;
    }

    /// Retrieve the hash of the rom contents
    pub fn hash(&self) -> String {
        self.hash.to_owned()
    }

    /// Retrieve the convenience name
    pub fn rom_name(&self) -> String {
        self.rom_name.clone()
    }

    /// Retrieve the save name for the cartridge
    pub fn save_name(&self) -> String {
        format!("{}.save", self.save)
    }

    /// Builds a mapper for the rom
    fn get_mapper(mapper: u32, rom_data: &NesCartridgeData) -> Result<NesMapper, CartridgeError> {
        let mapper = match mapper {
            0 => mapper00::Mapper00::new(rom_data),
            1 => mapper01::Mapper01::new(rom_data),
            2 => mapper02::Mapper02::new(rom_data),
            3 => mapper03::Mapper03::new(rom_data),
            4 => mapper04::Mapper04::new(rom_data),
            5 => mapper05::Mapper05::new(rom_data),
            34 => mapper34::Mapper34::new(rom_data),
            71 => mapper71::Mapper71::new(rom_data),
            _ => {
                return Err(CartridgeError::IncompatibleMapper(mapper));
            }
        };
        Ok(mapper)
    }

    /// Parses an ines1 format rom
    fn load_ines1(name: String, rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        if rom_contents[0] != b'N'
            || rom_contents[1] != b'E'
            || rom_contents[2] != b'S'
            || rom_contents[3] != 0x1a
        {
            return Err(CartridgeError::InvalidRom);
        }
        let prg_rom_size = rom_contents[4] as usize * 16384;

        let chr_rom_size = rom_contents[5] as usize * 8192;
        let mut file_offset: usize = 16;
        let trainer = if (rom_contents[6] & 4) != 0 {
            let mut trainer = Vec::with_capacity(512);
            for i in 0..512 {
                if rom_contents.len() <= (file_offset + i) {
                    return Err(CartridgeError::RomTooShort);
                }
                trainer.push(rom_contents[file_offset + i]);
            }
            file_offset += 512;
            Some(trainer)
        } else {
            None
        };

        let mut prg_rom = Vec::with_capacity(prg_rom_size);
        for i in 0..prg_rom_size {
            if rom_contents.len() <= (file_offset + i) {
                return Err(CartridgeError::RomTooShort);
            }
            prg_rom.push(rom_contents[file_offset + i]);
        }
        file_offset += prg_rom_size;

        let mut chr_rom = Vec::with_capacity(chr_rom_size);
        let mut chr_ram = Vec::with_capacity(8192);
        if chr_rom_size != 0 {
            for i in 0..chr_rom_size {
                let data = {
                    if rom_contents.len() <= (file_offset + i) {
                        return Err(CartridgeError::RomTooShort);
                    }
                    rom_contents[file_offset + i]
                };
                chr_rom.push(data);
            }
            file_offset += chr_rom_size;
        } else {
            let chr_ram_size = 8192;
            for _i in 0..chr_ram_size {
                let data = rand::random();
                chr_ram.push(data);
            }
        }
        let inst_rom = if (rom_contents[7] & 2) != 0 {
            let mut irom = Vec::with_capacity(8192);
            for i in 0..8192 {
                if rom_contents.len() <= (file_offset + i) {
                    return Err(CartridgeError::RomTooShort);
                }
                irom.push(rom_contents[file_offset + i]);
            }
            //file_offset += 8192;
            Some(irom)
        } else {
            None
        };

        let ram_size = if (rom_contents[6] & 2) != 0 {
            0x2000
        } else if rom_contents[8] != 0 {
            rom_contents[8] as usize * 8192
        } else {
            8192
        };
        let mut prg_ram = Vec::with_capacity(ram_size);
        for _i in 0..ram_size {
            let v = rand::random();
            prg_ram.push(v);
        }

        let mappernum = (rom_contents[6] >> 4) | (rom_contents[7] & 0xf0);

        let vol = VolatileCartridgeData {
            prg_ram: PersistentStorage::Volatile(prg_ram),
            battery_backup: (rom_contents[6] & 2) != 0,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
            chr_ram,
            genie: Vec::new(),
        };

        let nonvol = NonvolatileCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            inst_rom,
            prom: None,
        };

        let rom_data = NesCartridgeData {
            volatile: vol,
            nonvolatile: nonvol,
        };
        let mapper = Self::get_mapper(mappernum as u32, &rom_data)?;

        if file_offset != rom_contents.len() {
            return Err(CartridgeError::RomTooLong);
            /*println!(
                "Expected to read {:x} bytes, read {:x}",
                rom_contents.len(),
                file_offset
            );*/
        }
        let hash = calc_sha256(rom_contents);

        let pb = <PathBuf as std::str::FromStr>::from_str(&name).unwrap();

        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
            rom_format: RomFormat::Ines1,
            hash: hash.to_owned(),
            save: pb
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap()
                .to_string(),
            rom_name: name.to_owned(),
        })
    }

    /// Parses an ines2 format rom
    fn load_ines2(name: String, rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        if rom_contents[0] != b'N'
            || rom_contents[1] != b'E'
            || rom_contents[2] != b'S'
            || rom_contents[3] != 0x1a
        {
            return Err(CartridgeError::InvalidRom);
        }
        let prg_rom_size = if (rom_contents[9] & 0xF) < 0xF {
            (rom_contents[4] as usize | ((rom_contents[9] & 0xF) as usize) << 8) * 16384
        } else {
            let mult = (rom_contents[4] & 3) as usize * 2 + 1;
            let exp = 2 << ((rom_contents[4] >> 2) as usize);
            exp * mult
        };

        let mut file_offset: usize = 16;
        let trainer = if (rom_contents[6] & (1 << 2)) != 0 {
            let mut trainer = Vec::with_capacity(512);
            for i in 0..512 {
                if rom_contents.len() <= (file_offset + i) {
                    return Err(CartridgeError::RomTooShort);
                }
                trainer.push(rom_contents[file_offset + i]);
            }
            file_offset += 512;
            Some(trainer)
        } else {
            None
        };

        let mut prg_rom = Vec::with_capacity(prg_rom_size);
        for i in 0..prg_rom_size {
            if rom_contents.len() <= (file_offset + i) {
                return Err(CartridgeError::RomTooShort);
            }
            prg_rom.push(rom_contents[file_offset + i]);
        }
        file_offset += prg_rom_size;

        let chr_rom_size = if (rom_contents[9] & 0xF) < 0xF {
            (rom_contents[4] as usize | ((rom_contents[9] & 0xF0) as usize) << 4) * 8192
        } else {
            let mult = (rom_contents[4] & 3) as usize * 2 + 1;
            let exp = 2 << ((rom_contents[4] >> 2) as usize);
            exp * mult
        };

        let mut chr_rom = Vec::with_capacity(chr_rom_size);

        if chr_rom_size != 0 {
            for i in 0..chr_rom_size {
                if rom_contents.len() <= (file_offset + i) {
                    return Err(CartridgeError::RomTooShort);
                }
                chr_rom.push(rom_contents[file_offset + i]);
            }
            file_offset += chr_rom_size;
        }

        if file_offset < rom_contents.len() {
            return Err(CartridgeError::RomTooLong);
            //println!("Didn't use the entire rom file, I should report this as a failure");
        }

        let mappernum = (rom_contents[6] >> 4) as u16
            | (rom_contents[7] & 0xf0) as u16
            | (rom_contents[8] as u16) << 8;

        let vol = VolatileCartridgeData {
            prg_ram: PersistentStorage::Volatile(Vec::new()),
            battery_backup: (rom_contents[6] & 2) != 0,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
            chr_ram: Vec::new(),
            genie: Vec::new(),
        };

        let nonvol = NonvolatileCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            inst_rom: None,
            prom: None,
        };

        let rom_data = NesCartridgeData {
            nonvolatile: nonvol,
            volatile: vol,
        };

        let mapper = NesCartridge::get_mapper(mappernum as u32, &rom_data)?;

        let pb = <PathBuf as std::str::FromStr>::from_str(&name).unwrap();

        let hash = calc_sha256(rom_contents);
        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
            rom_format: RomFormat::Ines2,
            hash: hash.to_owned(),
            save: pb
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap()
                .to_string(),
            rom_name: name.to_owned(),
        })
    }

    /// Load a cartridge, returning an error or the new cartridge
    pub fn load_cartridge(name: String, sp: &Path) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name.clone());
        if let Err(e) = rom_contents {
            return Err(CartridgeError::FsError(e.kind().to_string()));
        }
        let rom_contents = rom_contents.unwrap();
        if rom_contents.len() < 16 {
            return Err(CartridgeError::InvalidRom);
        }
        if rom_contents[0] != b'N'
            || rom_contents[1] != b'E'
            || rom_contents[2] != b'S'
            || rom_contents[3] != 0x1a
        {
            return Err(CartridgeError::InvalidRom);
        }
        let mut cart = if (rom_contents[7] & 0xC) == 8 {
            Self::load_ines2(name, &rom_contents)
        } else if (rom_contents[7] & 0xC) == 4 {
            Self::load_obsolete_ines(name, &rom_contents)
        } else if (rom_contents[7] & 0xC) == 0
            && rom_contents[12] == 0
            && rom_contents[13] == 0
            && rom_contents[14] == 0
            && rom_contents[15] == 0
        {
            Self::load_ines1(name, &rom_contents)
        } else {
            //or ines 0.7
            Self::load_obsolete_ines(name, &rom_contents)
        };

        if let Ok(c) = &mut cart {
            let mut pb: PathBuf = sp.to_path_buf();
            pb.push(format!("{}.prgram", c.save));
            if c.data.volatile.battery_backup {
                c.data.volatile.prg_ram.convert_to_nonvolatile(pb);
            }
        }

        cart
    }
}

impl NesCartridge {
    ///Retrieve a reference to the cartridge data
    pub fn cartridge(&self) -> &NesCartridgeData {
        &self.data
    }

    /// Retrieve a mutable reference to the cartridge volatile data
    pub fn cartridge_volatile_mut(&mut self) -> &mut VolatileCartridgeData {
        &mut self.data.volatile
    }

    /// Retrieve a list of cartridge registers
    pub fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        self.mapper.cartridge_registers()
    }

    /// Perform a dump of a cartridge
    pub fn memory_dump(&self, addr: u16) -> Option<u8> {
        self.mapper.memory_cycle_dump(&self.data, addr)
    }

    /// Drive a cpu memory read cycle
    pub fn memory_read(&mut self, addr: u16) -> Option<u8> {
        self.mapper.genie(&mut self.data, addr)
    }

    /// Drive a cpu memory write cycle
    pub fn memory_write(&mut self, addr: u16, data: u8) {
        self.mapper.memory_cycle_write(&mut self.data, addr, data);
    }

    /// Drive the other memory read cycle, this cycle does not perform a read, but drives logic that depends on the read
    pub fn other_memory_read(&mut self, addr: u16) {
        self.mapper.other_memory_read(&mut self.data, addr);
    }

    /// Drive the other memory write cycle
    pub fn other_memory_write(&mut self, addr: u16, data: u8) {
        self.mapper.other_memory_write(addr, data);
    }

    /// A nop for the cpu bus, for driving mapper logic that needs it.
    pub fn memory_nop(&mut self) {
        self.mapper.memory_cycle_nop();
    }

    /// Perform a peek on ppu memory
    pub fn ppu_peek_1(&self, addr: u16) -> (bool, bool, Option<u8>) {
        self.mapper.ppu_peek_address(addr, &self.data)
    }

    /// Run a ppu address cycle
    #[must_use]
    pub fn ppu_cycle_1(&mut self, addr: u16) -> (bool, bool) {
        self.mapper.ppu_memory_cycle_address(addr)
    }

    /// Run a ppu write cyle
    pub fn ppu_cycle_write(&mut self, data: u8) {
        self.mapper.ppu_memory_cycle_write(&mut self.data, data);
    }

    /// Run a ppu read cycle
    pub fn ppu_cycle_read(&mut self) -> u8 {
        if let Some(a) = self.mapper.ppu_memory_cycle_read(&mut self.data) {
            a
        } else {
            //TODO implement open bus behavior
            42
        }
    }

    ///Used in testing to over-write the contents of a specific byte in the rom image
    #[cfg(test)]
    pub fn rom_byte_hack(&mut self, addr: u32, new_byte: u8) {
        self.mapper.rom_byte_hack(&mut self.data, addr, new_byte);
    }
}
