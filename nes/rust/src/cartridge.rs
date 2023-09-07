//! This module is responsible for cartridge related emulation, including mapper emulation.

mod mapper00;
mod mapper01;
mod mapper03;

use mapper00::Mapper00;
use mapper01::Mapper01;
use mapper03::Mapper03;

use serde::{Deserialize, Serialize};

/// All mappers must implement this.
#[enum_dispatch::enum_dispatch]
trait NesMapperTrait {
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
    /// Modify a byte for the cartridge rom
    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8);
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

/// The data for a cartridge.
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCartridgeData {
    /// An optional trainer for the cartridge
    trainer: Option<Vec<u8>>,
    /// The prg rom, where code typically goes.
    prg_rom: Vec<u8>,
    /// The chr rom, where graphics are generally stored
    chr_rom: Vec<u8>,
    /// chr_ram ?
    chr_ram: bool,
    /// inst_rom ?
    inst_rom: Option<Vec<u8>>,
    /// prom?
    prom: Option<(Vec<u8>, Vec<u8>)>,
    /// Program ram
    prg_ram: Vec<u8>,
    /// True for vertical mirroring, false for horizontal mirroring
    mirroring: bool,
    /// The mapper number
    mapper: u32,
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
}

/// The types of errors that can occur when loading a rom
#[derive(Serialize, Deserialize, Debug)]
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
}

impl NesCartridge {
    /// "Parses" an obsolete ines rom
    fn load_obsolete_ines(_rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        Err(CartridgeError::IncompatibleRom)
    }

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
    fn load_ines1(rom_contents: &[u8]) -> Result<Self, CartridgeError> {
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
        let trainer = if (rom_contents[6] & 8) != 0 {
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
            file_offset += chr_rom_size;
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

        let ram_size = rom_contents[8] as usize * 8192;
        let mut prg_ram = Vec::with_capacity(ram_size);
        for _i in 0..ram_size {
            let v = rand::random();
            prg_ram.push(v);
        }

        let mappernum = (rom_contents[6] >> 4) | (rom_contents[7] & 0xf0);
        let rom_data = NesCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            chr_ram,
            inst_rom,
            prom: None,
            prg_ram,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
        };
        let mapper = Self::get_mapper(mappernum as u32, &rom_data)?;

        Ok(Self {
            data: rom_data,
            mapper: mapper,
            mappernum: mappernum as u32,
        })
    }

    /// Parses an ines2 format rom
    fn load_ines2(rom_contents: &[u8]) -> Result<Self, CartridgeError> {
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
            println!("Didn't use the entire rom file, I should report this as a failure");
        }

        let mappernum = (rom_contents[6] >> 4) as u16
            | (rom_contents[7] & 0xf0) as u16
            | (rom_contents[8] as u16) << 8;
        let rom_data = NesCartridgeData {
            trainer,
            prg_rom,
            chr_rom,
            chr_ram: false,
            inst_rom: None,
            prom: None,
            prg_ram: Vec::new(),
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mappernum as u32,
        };

        let mapper = NesCartridge::get_mapper(mappernum as u32, &rom_data)?;

        Ok(Self {
            data: rom_data,
            mapper,
            mappernum: mappernum as u32,
        })
    }

    /// Load a cartridge, returning an error or the new cartridge
    pub fn load_cartridge(name: String) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name);
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
            Self::load_ines2(&rom_contents)
        } else if (rom_contents[7] & 0xC) == 4 {
            Self::load_obsolete_ines(&rom_contents)
        } else if (rom_contents[7] & 0xC) == 0
            && rom_contents[12] == 0
            && rom_contents[13] == 0
            && rom_contents[14] == 0
            && rom_contents[15] == 0
        {
            Self::load_ines1(&rom_contents)
        } else {
            //or ines 0.7
            Self::load_obsolete_ines(&rom_contents)
        }
    }
}

impl NesCartridge {
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
