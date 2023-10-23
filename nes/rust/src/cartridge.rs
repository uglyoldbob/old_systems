//! This module is responsible for cartridge related emulation, including mapper emulation.

mod mapper00;
mod mapper01;
mod mapper03;

use std::{collections::BTreeMap, path::PathBuf};

use mapper00::Mapper00;
use mapper01::Mapper01;
use mapper03::Mapper03;

use serde::{Deserialize, Serialize};

/// All mappers must implement this.
#[enum_dispatch::enum_dispatch]
trait NesMapperTrait {
    /// Dump some data from the cart
    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8>;
    /// Run a cpu memory read cycle
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8>;
    /// Run a cpu memory write cycle
    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8);
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
}

/// The mapper for an nes cartridge
#[non_exhaustive]
#[enum_dispatch::enum_dispatch(NesMapperTrait)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum NesMapper {
    Mapper00,
    Mapper01,
    Mapper03,
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

/// A visitor for PersistentStorage
pub struct PersistentVisitor;

impl<'de> serde::de::Visitor<'de> for PersistentVisitor {
    type Value = PersistentStorage;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("byte array")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let len = std::cmp::min(visitor.size_hint().unwrap_or(0), 4096);
        let mut bytes = Vec::with_capacity(len);

        while let Some(b) = visitor.next_element()? {
            bytes.push(b);
        }
        Ok(PersistentStorage::new_volatile(bytes))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PersistentStorage::new_volatile(v.to_vec()))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PersistentStorage::new_volatile(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PersistentStorage::new_volatile(v.as_bytes().to_vec()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PersistentStorage::new_volatile(v.into_bytes()))
    }
}

/// A vec that could be battery backed
pub enum PersistentStorage {
    /// The vec is battery backed by a file
    Persistent(memmap2::MmapMut),
    /// The vec is simply a plain vector
    Volatile(Vec<u8>),
}

impl Clone for PersistentStorage {
    fn clone(&self) -> Self {
        match self {
            Self::Persistent(arg0) => Self::Volatile(arg0.to_vec()),
            Self::Volatile(arg0) => Self::Volatile(arg0.clone()),
        }
    }
}

impl serde::Serialize for PersistentStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.contents().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PersistentStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(PersistentVisitor)
    }
}

impl PersistentStorage {
    /// Create a new persistent storage object
    fn new_persistent(p: PathBuf, v: Vec<u8>) -> Option<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&p)
            .ok()?;
        let mm = unsafe { memmap2::MmapMut::map_mut(&file) }.ok()?;
        Some(PersistentStorage::Persistent(mm))
    }

    /// Create a new volatile storage object
    fn new_volatile(v: Vec<u8>) -> Self {
        PersistentStorage::Volatile(v)
    }

    /// Retrieve a reference to the contents
    fn contents(&self) -> &[u8] {
        match self {
            PersistentStorage::Persistent(mm) => mm.as_ref(),
            PersistentStorage::Volatile(v) => &v[..],
        }
    }

    /// Retrieve a mutable reference to the contents
    fn contents_mut(&mut self) -> &mut [u8] {
        match self {
            PersistentStorage::Persistent(mm) => mm.as_mut(),
            PersistentStorage::Volatile(v) => &mut v[..],
        }
    }
}

/// The data for a cartridge.
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCartridgeData {
    #[serde(skip)]
    /// An optional trainer for the cartridge
    pub trainer: Option<Vec<u8>>,
    #[serde(skip)]
    /// The prg rom, where code typically goes.
    pub prg_rom: Vec<u8>,
    #[serde(skip)]
    /// The chr rom, where graphics are generally stored
    pub chr_rom: Vec<u8>,
    #[serde(skip)]
    /// inst_rom ?
    pub inst_rom: Option<Vec<u8>>,
    #[serde(skip)]
    /// prom?
    pub prom: Option<(Vec<u8>, Vec<u8>)>,
    /// The potentially volatile cartridge data
    pub volatile: VolatileCartridgeData,
}

/// Volatile storage for cartridge data
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct VolatileCartridgeData {
    /// Program ram
    pub prg_ram: Vec<u8>,
    /// Battery backup for prog ram active?
    pub battery_backup: bool,
    /// True for vertical mirroring, false for horizontal mirroring
    pub mirroring: bool,
    /// The mapper number
    pub mapper: u32,
    /// chr_ram ?
    pub chr_ram: bool,
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
    /// The data in the cartridge, including ram and everything else
    data: VolatileCartridgeData,
    /// The convenience name of the rom
    rom_name: String,
}

/// The types of errors that can occur when loading a rom
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CartridgeError {
    /// There can be a filesystem error opening the file
    FsError(String),
    /// It might not be any known type of rom
    InvalidRom,
    /// The rom might be incompatible (unparsed format)
    IncompatibleRom,
    /// The rom might use a mapper that is not yet implemented
    IncompatibleMapper(u32),
    /// The rom might be too short, indicating some bytes got cut off of the end, or that it has been corrupted/modified
    RomTooShort,
    /// The rom has bytes that were not parsed
    RomTooLong,
}

/// Calculate the sha256 of a chunk of data, and return it in a hex encoded string.
fn calc_sha256(data: &[u8]) -> String {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    context.update(data);
    let digest = context.finish();
    data_encoding::HEXLOWER.encode(digest.as_ref())
}

impl NesCartridge {
    /// "Parses" an obsolete ines rom
    fn load_obsolete_ines(_name: String, _rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        Err(CartridgeError::IncompatibleRom)
    }

    /// Saves the contents of the cartridge data so that it can be restored after loading a save state.
    pub fn save_cart_data(&self) -> NesCartridgeBackup {
        NesCartridgeBackup {
            data: self.data.volatile.clone(),
            rom_name: self.rom_name.clone(),
        }
    }

    /// Restore previously saved data after loading a save state.
    pub fn restore_cart_data(&mut self, old_data: NesCartridgeBackup) {
        self.data.volatile = old_data.data;
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
        self.save.clone()
    }

    /// Retrieve the mapper number for the cartridge
    pub fn mappernum(&self) -> u32 {
        self.mappernum
    }

    /// Builds a mapper for the rom
    fn get_mapper(mapper: u32, rom_data: &NesCartridgeData) -> Result<NesMapper, CartridgeError> {
        let mapper = match mapper {
            0 => mapper00::Mapper00::new(rom_data),
            1 => mapper01::Mapper01::new(rom_data),
            3 => mapper03::Mapper03::new(rom_data),
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

        let mut chr_rom_size = rom_contents[5] as usize * 8192;
        let chr_ram = chr_rom_size == 0;
        if chr_ram {
            //TODO determine correct size for chr-ram
            chr_rom_size = 8192;
        }
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
        if chr_rom_size != 0 {
            for i in 0..chr_rom_size {
                let data = if !chr_ram {
                    if rom_contents.len() <= (file_offset + i) {
                        return Err(CartridgeError::RomTooShort);
                    }
                    rom_contents[file_offset + i]
                } else {
                    rand::random()
                };
                chr_rom.push(data);
            }
            if !chr_ram {
                file_offset += chr_rom_size;
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
        } else {
            if rom_contents[8] != 0 {
                rom_contents[8] as usize * 8192
            } else {
                8192
            }
        };
        let mut prg_ram = Vec::with_capacity(ram_size);
        for _i in 0..ram_size {
            let v = rand::random();
            prg_ram.push(v);
        }

        let mappernum = (rom_contents[6] >> 4) | (rom_contents[7] & 0xf0);

        let vol = VolatileCartridgeData {
            prg_ram,
            battery_backup: (rom_contents[6] & 2) != 0,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
            chr_ram,
        };

        let rom_data = NesCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            volatile: vol,
            inst_rom,
            prom: None,
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
        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
            rom_format: RomFormat::Ines1,
            hash: hash.to_owned(),
            save: format!("{}.save", hash),
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
            prg_ram: Vec::new(),
            battery_backup: (rom_contents[6] & 2) != 0,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
            chr_ram: false,
        };

        let rom_data = NesCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            volatile: vol,
            inst_rom: None,
            prom: None,
        };

        let mapper = NesCartridge::get_mapper(mappernum as u32, &rom_data)?;

        let hash = calc_sha256(rom_contents);
        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
            rom_format: RomFormat::Ines2,
            hash: hash.to_owned(),
            save: format!("{}.save", hash),
            rom_name: name.to_owned(),
        })
    }

    /// Load a cartridge, returning an error or the new cartridge
    pub fn load_cartridge(name: String) -> Result<Self, CartridgeError> {
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
        if (rom_contents[7] & 0xC) == 8 {
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
        }
    }
}

impl NesCartridge {
    ///Retrieve a reference to the cartridge data
    pub fn cartridge(&self) -> &NesCartridgeData {
        &self.data
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
        self.mapper.memory_cycle_read(&mut self.data, addr)
    }

    /// Drive a cpu memory write cycle
    pub fn memory_write(&mut self, addr: u16, data: u8) {
        self.mapper.memory_cycle_write(&mut self.data, addr, data);
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
