mod mapper00;
mod mapper01;
mod mapper03;

use serde::{Deserialize, Serialize};

pub trait NesMapper {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8>;
    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8);
    fn memory_cycle_nop(&mut self);
    #[must_use]
    //performs the first half of a ppu memory cycle
    //returns A10 for internal VRAM and the motherboard CS line (for internal VRAM)
    //A10 is straight forward, CS line is active low like the electronics would be
    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool);
    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8>;
    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8);
    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8);
}

pub trait NesMemoryBusDevice {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        out: [bool; 3],
        controllers: [bool; 2],
    ) -> Option<u8>;
    fn memory_cycle_write(&mut self, addr: u16, data: u8);
}

pub struct NesCartridgeData {
    trainer: Option<Vec<u8>>,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: bool,
    inst_rom: Option<Vec<u8>>,
    prom: Option<(Vec<u8>, Vec<u8>)>,
    prg_ram: Vec<u8>,
    /// True for vertical mirroring, false for horizontal mirroring
    mirroring: bool,
    mapper: u32,
}

pub struct NesCartridge {
    data: NesCartridgeData,
    mapper: Box<dyn NesMapper>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CartridgeError {
    FsError(String),
    InvalidRom,
    IncompatibleRom,
    IncompatibleMapper(u16),
    RomTooShort,
}

impl NesCartridge {
    fn load_obsolete_ines(_rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        Err(CartridgeError::IncompatibleRom)
    }

    fn load_ines1(rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        if rom_contents[0] != 'N' as u8
            || rom_contents[1] != 'E' as u8
            || rom_contents[2] != 'S' as u8
            || rom_contents[3] != 0x1a
        {
            return Err(CartridgeError::InvalidRom);
        }
        let prg_rom_size = rom_contents[4] as usize * 16384;
        let mut prg_rom = Vec::with_capacity(prg_rom_size);

        let mut chr_rom_size = rom_contents[5] as usize * 8192;
        let chr_ram = chr_rom_size == 0;
        if chr_ram {
            //TODO determine correct size for chr-ram
            chr_rom_size = 8192 * 1;
        }
        let mut chr_rom = Vec::with_capacity(chr_rom_size);
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
        for i in 0..prg_rom_size {
            if rom_contents.len() <= (file_offset + i) {
                return Err(CartridgeError::RomTooShort);
            }
            prg_rom.push(rom_contents[file_offset + i]);
        }
        file_offset += prg_rom_size;
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

        let mapper = (rom_contents[6] >> 4) as u8 | (rom_contents[7] & 0xf0) as u8;
        let rom_data = NesCartridgeData {
            trainer: trainer,
            prg_rom: prg_rom,
            chr_rom: chr_rom,
            chr_ram: chr_ram,
            inst_rom: inst_rom,
            prom: None,
            prg_ram: prg_ram,
            mirroring: (rom_contents[6] & 1) != 0,
            mapper: mapper as u32,
        };
        let mapper = match mapper {
            0 => mapper00::Mapper::new(&rom_data),
            1 => mapper01::Mapper::new(&rom_data),
            3 => mapper03::Mapper::new(&rom_data),
            _ => {
                return Err(CartridgeError::IncompatibleMapper(mapper as u16));
            }
        };

        Ok(Self {
            data: rom_data,
            mapper: mapper,
        })
    }

    fn load_ines2(_rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        Err(CartridgeError::IncompatibleRom)
    }

    pub fn load_cartridge(name: String) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name);
        if let Err(e) = rom_contents {
            return Err(CartridgeError::FsError(e.kind().to_string()));
        }
        let rom_contents = rom_contents.unwrap();
        if rom_contents.len() < 16 {
            return Err(CartridgeError::InvalidRom);
        }
        if rom_contents[0] != 'N' as u8
            || rom_contents[1] != 'E' as u8
            || rom_contents[2] != 'S' as u8
            || rom_contents[3] != 0x1a
        {
            return Err(CartridgeError::InvalidRom);
        }
        if (rom_contents[7] & 0xC) == 8 {
            return Self::load_ines2(&rom_contents);
        } else if (rom_contents[7] & 0xC) == 4 {
            return Self::load_obsolete_ines(&rom_contents);
        } else if (rom_contents[7] & 0xC) == 0
            && rom_contents[12] == 0
            && rom_contents[13] == 0
            && rom_contents[14] == 0
            && rom_contents[15] == 0
        {
            return Self::load_ines1(&rom_contents);
        } else {
            //or ines 0.7
            return Self::load_obsolete_ines(&rom_contents);
        }
    }
}

impl NesCartridge {
    pub fn memory_read(&mut self, addr: u16) -> Option<u8> {
        self.mapper.memory_cycle_read(&mut self.data, addr)
    }

    pub fn memory_write(&mut self, addr: u16, data: u8) {
        self.mapper.memory_cycle_write(&mut self.data, addr, data);
    }

    pub fn memory_nop(&mut self) {
        self.mapper.memory_cycle_nop();
    }

    #[must_use]
    pub fn ppu_cycle_1(&mut self, addr: u16) -> (bool, bool) {
        self.mapper.ppu_memory_cycle_address(addr)
    }

    pub fn ppu_cycle_write(&mut self, data: u8) {
        self.mapper.ppu_memory_cycle_write(&mut self.data, data);
    }

    pub fn ppu_cycle_read(&mut self) -> u8 {
        if let Some(a) = self.mapper.ppu_memory_cycle_read(&mut self.data) {
            a
        } else {
            //TODO implement open bus behavior
            42
        }
    }

    #[cfg(test)]
    pub fn rom_byte_hack(&mut self, addr: u32, new_byte: u8) {
        self.mapper.rom_byte_hack(&mut self.data, addr, new_byte);
    }
}
