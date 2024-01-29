//! This module is responsible for cartridge related emulation, including mapper emulation.

mod mapper00;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    slice::ChunksExact,
};

use mapper00::Mapper00;

use serde::{Deserialize, Serialize};

use crate::genie::GameGenieCode;

/// All mappers must implement this.
#[enum_dispatch::enum_dispatch]
trait SnesMapperTrait {
    /// Dump some data from the cart
    fn memory_cycle_dump(&self, cart: &SnesCartridgeData, bank: u8, addr: u16) -> Option<u8>;
    /// Run a cpu memory read cycle
    fn memory_cycle_read(&mut self, cart: &mut SnesCartridgeData, bank: u8, addr: u16) -> Option<u8>;
    /// A read cycle that does not target cartridge memory. Used for mappers that monitor reads like mmc5.
    fn other_memory_read(&mut self, cart: &mut SnesCartridgeData, bank: u8, addr: u16) {}
    /// Run a cpu memory write cycle
    fn memory_cycle_write(&mut self, cart: &mut SnesCartridgeData, bank: u8, addr: u16, data: u8);
    /// A write cycle that does not target cartridge memory. Used for mappers that monitor writes like mmc5.
    fn other_memory_write(&mut self, bank: u8, _addr: u16, _data: u8) {}
    /// Runs a memory cycle that does nothing, for mappers that need to do special things.
    fn memory_cycle_nop(&mut self);
    #[must_use]
    /// performs the first half of a ppu memory cycle
    /// returns A10 for internal VRAM and the motherboard CS line (for internal VRAM)
    /// A10 is straight forward, CS line is active low like the electronics would be
    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool);
    /// Run a ppu read cycle
    fn ppu_memory_cycle_read(&mut self, cart: &mut SnesCartridgeData) -> Option<u8>;
    /// Run a ppu write cycle
    fn ppu_memory_cycle_write(&mut self, cart: &mut SnesCartridgeData, data: u8);
    /// Peek at a ppu memory address
    fn ppu_peek_address(&self, adr: u16, cart: &SnesCartridgeData) -> (bool, bool, Option<u8>);
    /// Modify a byte for the cartridge rom
    fn rom_byte_hack(&mut self, cart: &mut SnesCartridgeData, addr: u32, new_byte: u8);
    /// Returns a list of registers used by the cartridge
    fn cartridge_registers(&self) -> BTreeMap<String, u8>;
    /// Retrieve the irq signal
    fn irq(&self) -> bool;
    /// Checks for active game genie codes and acts appropriately
    fn genie(&mut self, cart: &mut SnesCartridgeData, bank: u8, addr: u16) -> Option<u8> {
        if cart.volatile.genie.len() > 0 {
            let a = if (0xe000..=0xffff).contains(&addr) {
                let mut a = self.memory_cycle_read(cart, bank, addr);
                for code in &cart.volatile.genie {
                    if code.address() == addr {
                        if let Some(check) = code.check() {
                            let lv = self.memory_cycle_dump(cart, bank, addr ^ 0x8000);
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
                            let lv = self.memory_cycle_dump(cart, bank, addr ^ 0x8000);
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
                let mut a = self.memory_cycle_read(cart, bank, addr);
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
            self.memory_cycle_read(cart, bank, addr)
        }
    }
}

/// The mapper for an Snes cartridge
#[non_exhaustive]
#[enum_dispatch::enum_dispatch(SnesMapperTrait)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum SnesMapper {
    Mapper00,
}

/// The trait for cpu memory reads and writes, implemented by devices on the bus
pub trait SnesMemoryBusDevice {
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
        formatter.write_str("byte sequence")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let len = std::cmp::min(visitor.size_hint().unwrap_or(0), 4096);
        let mut bytes = Vec::with_capacity(len);

        let t: Option<u8> = visitor.next_element()?;
        while let Some(b) = visitor.next_element()? {
            bytes.push(b);
        }
        if let Some(t) = t {
            match t {
                0 | 1 => Ok(PersistentStorage::new_should(bytes)),
                _ => Ok(PersistentStorage::new_volatile(bytes)),
            }
        } else {
            Ok(PersistentStorage::new_volatile(bytes))
        }
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let v = v.as_bytes().to_vec();
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let v = v.into_bytes();
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }
}

/// A vec that could be battery backed
pub enum PersistentStorage {
    /// The vec is battery backed by a file
    Persistent(PathBuf, memmap2::MmapMut),
    /// The vec should be persistent but it is not for some reason
    ShouldBePersistent(Vec<u8>),
    /// The vec is simply a plain vector
    Volatile(Vec<u8>),
}

impl Clone for PersistentStorage {
    fn clone(&self) -> Self {
        match self {
            Self::Persistent(_pb, arg0) => Self::ShouldBePersistent(arg0.to_vec()),
            Self::ShouldBePersistent(v) => Self::ShouldBePersistent(v.clone()),
            Self::Volatile(arg0) => Self::Volatile(arg0.clone()),
        }
    }
}

impl serde::Serialize for PersistentStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let elem = self.contents();
        let mut seq = serializer.serialize_seq(Some(elem.len() + 1))?;
        match self {
            PersistentStorage::Volatile(_v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &2_u8)?;
            }
            PersistentStorage::Persistent(_pb, _v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &0_u8)?;
            }
            PersistentStorage::ShouldBePersistent(_v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &1_u8)?;
            }
        }
        for e in elem {
            serde::ser::SerializeSeq::serialize_element(&mut seq, e)?;
        }
        serde::ser::SerializeSeq::end(seq)
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

impl std::ops::Index<usize> for PersistentStorage {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents()[index]
    }
}

impl std::ops::IndexMut<usize> for PersistentStorage {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.contents_mut()[index]
    }
}

impl Drop for PersistentStorage {
    fn drop(&mut self) {
        if let PersistentStorage::Persistent(_p, v) = self {
            let _ = v.flush();
        }
    }
}

impl PersistentStorage {
    /// Create a persistent storage object using the specified path and data. Overwrite will overwrite the contents of the file if set to true.
    fn make_persistent(p: PathBuf, v: Vec<u8>, overwrite: bool) -> Option<Self> {
        let file = if p.exists() {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&p);
            if let Ok(mut file) = file {
                if overwrite {
                    std::io::Write::write_all(&mut file, &v[..]).ok()?;
                }
                Some(file)
            } else {
                None
            }
        } else {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&p);
            if let Ok(mut file) = file {
                std::io::Write::write_all(&mut file, &v[..]).ok()?;
                Some(file)
            } else {
                None
            }
        };
        if let Some(file) = file {
            let mm = unsafe { memmap2::MmapMut::map_mut(&file) };
            if let Ok(mm) = mm {
                Some(PersistentStorage::Persistent(p, mm))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Reupgrade the object to be fully persistent
    fn upgrade_to_persistent(&mut self, p: PathBuf) {
        if let PersistentStorage::ShouldBePersistent(v) = self {
            if let Some(ps) = Self::make_persistent(p, v.clone(), true) {
                *self = ps;
            }
        }
    }

    /// Convert this volatile object to a non-volatile object, taking on the contents of the existing nonvolatile storage if it exists.
    /// If it does not exist, then the current contents are transferred over.
    fn convert_to_nonvolatile(&mut self, p: PathBuf) {
        let t = match self {
            PersistentStorage::Persistent(_pb, _a) => None,
            PersistentStorage::ShouldBePersistent(_v) => None,
            PersistentStorage::Volatile(v) => Self::make_persistent(p, v.clone(), false),
        };
        if let Some(t) = t {
            *self = t;
        }
    }

    /// Create a new object that should be persistent
    fn new_should(v: Vec<u8>) -> Self {
        PersistentStorage::ShouldBePersistent(v)
    }

    /// Create a new volatile storage object
    fn new_volatile(v: Vec<u8>) -> Self {
        PersistentStorage::Volatile(v)
    }

    /// Convenience function for determining if the contents are empty.
    fn is_empty(&self) -> bool {
        self.contents().is_empty()
    }

    /// The length of the contents
    pub fn len(&self) -> usize {
        self.contents().len()
    }

    /// Get chunks of the data
    pub fn chunks_exact(&self, cs: usize) -> ChunksExact<'_, u8> {
        self.contents().chunks_exact(cs)
    }

    /// Retrieve a reference to the contents
    fn contents(&self) -> &[u8] {
        match self {
            PersistentStorage::Persistent(_pb, mm) => mm.as_ref(),
            PersistentStorage::ShouldBePersistent(v) => &v[..],
            PersistentStorage::Volatile(v) => &v[..],
        }
    }

    /// Retrieve a mutable reference to the contents
    fn contents_mut(&mut self) -> &mut [u8] {
        match self {
            PersistentStorage::Persistent(_pb, mm) => mm.as_mut(),
            PersistentStorage::ShouldBePersistent(v) => &mut v[..],
            PersistentStorage::Volatile(v) => &mut v[..],
        }
    }
}

/// The data for a cartridge.
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCartridgeData {
    #[serde(skip)]
    /// The nonvolatile data for the cartridge
    pub nonvolatile: NonvolatileCartridgeData,
    /// The potentially volatile cartridge data
    pub volatile: VolatileCartridgeData,
}

/// Nonvolatile storage for cartridge data
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct NonvolatileCartridgeData {
    /// The prg rom
    pub prg_rom: Vec<u8>,
    /// The largest chunk of memory size for the rom
    pub rom_first: u32,
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
    /// The iSnes1 rom format
    ISnes1,
    /// The iSnes2 rom format
    ISnes2,
}

/// A cartridge, including the mapper structure
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCartridge {
    /// The data in the cartridge, including ram and everything else
    data: SnesCartridgeData,
    /// The mapper
    mapper: SnesMapper,
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
pub struct SnesCartridgeBackup {
    /// The non-volatile data in the cartridge
    data: NonvolatileCartridgeData,
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
    /// The cartridge length is not a multiple of 512
    BadLength,
    /// The rom header was not found
    HeaderNotFound,
}

/// Calculate the sha256 of a chunk of data, and return it in a hex encoded string.
fn calc_sha256(data: &[u8]) -> String {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    context.update(data);
    let digest = context.finish();
    data_encoding::HEXLOWER.encode(digest.as_ref())
}

struct RomHeaderOption {
    start: u32,
    mode: u8,
    method: MapperMethod,
}

enum MapperMethod {
    LoRom,
    HiRom,
    ExHirom,
}

/// An iterator over the contents of a rom for getting the checksum
struct MapperIter<'a> {
    /// The address
    addr: u32,
    /// The size of the main chunk of memory for the rom. This is the largest power of two equal to or less than the entire length of the rom.
    main_size: u32,
    /// The size of the repeat block for the minor portion of rom memory
    step: u32,
    /// The maximum address to read to for all of the rom
    max_size: u32,
    /// The map method
    map: &'a MapperMethod,
    /// The contents of the rom
    contents: &'a [u8],
}

impl<'a> Iterator for MapperIter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        let d = if self.addr < self.main_size {
            Some(self.contents[self.addr as usize])
        } else if self.addr < self.max_size {
            let index = self.addr % self.step;
            let small = self.contents.len() - self.main_size as usize;
            if index as usize >= small {
                Some(0)
            } else {
                Some(self.contents[(self.main_size + index) as usize])
            }
        } else {
            None
        };
        self.addr += 1;
        d
    }
}

impl<'a> MapperMethod {
    fn get_iter(&'a self, contents: &'a [u8]) -> MapperIter {
        let mut size = contents.len().next_power_of_two();
        if size > contents.len() {
            size /= 2;
        }

        let mut msize = contents.len() - size;

        let (max_size, step) = if msize > 0 {
            msize = msize.next_power_of_two();
            (size * 2, msize)
        } else {
            (size, 0)
        };
        MapperIter {
            addr: 0,
            main_size: size as u32,
            step: step as u32,
            max_size: max_size as u32,
            map: self,
            contents,
        }
    }
}

impl SnesCartridge {
    /// Helper function to get the irq signal from the cartridge
    pub fn irq(&self) -> bool {
        self.mapper.irq()
    }

    /// Saves the contents of the cartridge data so that it can be restored after loading a save state.
    pub fn save_cart_data(&mut self) -> SnesCartridgeBackup {
        SnesCartridgeBackup {
            data: self.data.nonvolatile.clone(),
            rom_name: self.rom_name.clone(),
        }
    }

    /// Restore previously saved data after loading a save state.
    pub fn restore_cart_data(&mut self, old_data: SnesCartridgeBackup, mut pb: PathBuf) {
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

    /// Retrieve the mapper number for the cartridge
    pub fn mappernum(&self) -> u32 {
        self.mappernum
    }

    /// Builds a mapper for the rom
    fn get_mapper(mapper: u32, rom_data: &SnesCartridgeData) -> Result<SnesMapper, CartridgeError> {
        let mapper = match mapper {
            0 => mapper00::Mapper00::new(rom_data),
            _ => {
                return Err(CartridgeError::IncompatibleMapper(mapper));
            }
        };
        Ok(mapper)
    }

    /// Parses an snes rom
    fn load_snes_rom(name: String, mapper: u8, rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        let mut prg_rom = Vec::with_capacity(rom_contents.len());
        for i in 0..rom_contents.len() {
            prg_rom.push(rom_contents[i]);
        }

        let mut size = rom_contents.len().next_power_of_two();
        if size > rom_contents.len() {
            size /= 2;
        }

        let mut msize = rom_contents.len() - size;

        let (max_size, step) = if msize > 0 {
            msize = msize.next_power_of_two();
            (size * 2, msize)
        } else {
            (size, 0)
        };

        let mappernum = mapper;

        let vol = VolatileCartridgeData {
            prg_ram: PersistentStorage::Volatile(Vec::new()),
            battery_backup: false,
            mirroring: false,
            mapper: mappernum as u32,
            chr_ram: Vec::new(),
            genie: Vec::new(),
        };

        let nonvol = NonvolatileCartridgeData {
            prg_rom,
            rom_first: size as u32,
        };

        let rom_data = SnesCartridgeData {
            nonvolatile: nonvol,
            volatile: vol,
        };

        let mapper = SnesCartridge::get_mapper(mappernum as u32, &rom_data)?;

        let pb = <PathBuf as std::str::FromStr>::from_str(&name).unwrap();

        let hash = calc_sha256(rom_contents);
        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
            rom_format: RomFormat::ISnes2,
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

    /// Finds the rom header and calls load_snes_rom
    fn find_rom_header(
        name: String,
        preheader: bool,
        contents: &[u8],
    ) -> Result<Self, CartridgeError> {
        let header_locations: [RomHeaderOption; 3] = [
            RomHeaderOption {
                start: 0x7fc0,
                mode: 0,
                method: MapperMethod::LoRom,
            },
            RomHeaderOption {
                start: 0xffc0,
                mode: 1,
                method: MapperMethod::HiRom,
            },
            RomHeaderOption {
                start: 0x40ffc0,
                mode: 5,
                method: MapperMethod::ExHirom,
            },
        ];

        let rom_contents: Vec<u8> = if preheader {
            contents[512..].iter().map(|f| *f).collect()
        } else {
            contents.iter().map(|f| *f).collect()
        };

        for o in header_locations {
            if rom_contents.len() < (o.start as usize + 32) {
                continue;
            }
            let maybe_header_contents = &rom_contents[o.start as usize..o.start as usize + 32];
            if (maybe_header_contents[0x15] & 0xF) != o.mode {
                continue;
            }

            let nchecksum: u16 =
                (maybe_header_contents[0x1c] as u16) | (maybe_header_contents[0x1d] as u16) << 8;
            let checksum: u16 =
                (maybe_header_contents[0x1e] as u16) | (maybe_header_contents[0x1f] as u16) << 8;
            if (checksum ^ nchecksum) != 0xffff {
                continue;
            }

            let mut cs1: u16 = 0;
            for d in o.method.get_iter(&rom_contents) {
                cs1 = cs1.wrapping_add(d as u16);
            }

            let checksum_good = (cs1 == checksum);

            return Self::load_snes_rom(name, o.mode, &rom_contents);
        }

        Err(CartridgeError::HeaderNotFound)
    }

    /// Load a cartridge, returning an error or the new cartridge
    pub fn load_cartridge(name: String, sp: &Path) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name.clone());
        if let Err(e) = rom_contents {
            return Err(CartridgeError::FsError(e.kind().to_string()));
        }
        let rom_contents = rom_contents.unwrap();

        let preheader = rom_contents.len() % 1024 == 512;
        if rom_contents.len() % 512 != 0 {
            return Err(CartridgeError::BadLength);
        }

        let mut cart = Self::find_rom_header(name, preheader, &rom_contents);

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

impl SnesCartridge {
    ///Retrieve a reference to the cartridge data
    pub fn cartridge(&self) -> &SnesCartridgeData {
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
    pub fn memory_dump(&self, bank: u8, addr: u16) -> Option<u8> {
        self.mapper.memory_cycle_dump(&self.data, bank, addr)
    }

    /// Drive a cpu memory read cycle
    pub fn memory_read(&mut self, bank: u8, addr: u16) -> Option<u8> {
        self.mapper.genie(&mut self.data, bank, addr)
    }

    /// Drive a cpu memory write cycle
    pub fn memory_write(&mut self, bank: u8, addr: u16, data: u8) {
        self.mapper.memory_cycle_write(&mut self.data, bank, addr, data);
    }

    /// Drive the other memory read cycle, this cycle does not perform a read, but drives logic that depends on the read
    pub fn other_memory_read(&mut self, bank: u8, addr: u16) {
        self.mapper.other_memory_read(&mut self.data, bank, addr);
    }

    /// Drive the other memory write cycle
    pub fn other_memory_write(&mut self, bank: u8, addr: u16, data: u8) {
        self.mapper.other_memory_write(bank, addr, data);
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
