mod mapper00;
mod mapper01;

pub trait NesMapper {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8>;
    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8);
    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8>;
    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8);
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
    inst_rom: Option<Vec<u8>>,
    prom: Option<(Vec<u8>, Vec<u8>)>,
    prg_ram: Vec<u8>,
}

pub struct NesCartridge {
    data: NesCartridgeData,
    mapper: Box<dyn NesMapper>,
}

#[derive(Debug)]
pub enum CartridgeError {
    FsError(std::io::Error),
    InvalidRom,
    IncompatibleRom,
    IncompatibleMapper(u16),
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

        let chr_rom_size = rom_contents[5] as usize * 8192;
        let mut chr_rom = Vec::with_capacity(chr_rom_size);
        let mut file_offset: usize = 16;
        let trainer = if (rom_contents[6] & 8) != 0 {
            let mut trainer = Vec::with_capacity(512);
            for i in 0..512 {
                trainer.push(rom_contents[file_offset + i]);
            }
            file_offset += 512;
            Some(trainer)
        } else {
            None
        };
        for i in 0..prg_rom_size {
            prg_rom.push(rom_contents[file_offset + i]);
        }
        file_offset += prg_rom_size;
        if chr_rom_size != 0 {
            for i in 0..chr_rom_size {
                chr_rom.push(rom_contents[file_offset + i]);
            }
            file_offset += chr_rom_size;
        }
        let inst_rom = if (rom_contents[7] & 2) != 0 {
            let mut irom = Vec::with_capacity(8192);
            for i in 0..8192 {
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
        let mapper = match mapper {
            0 => mapper00::Mapper::new(),
            1 => mapper01::Mapper::new(),
            _ => {
                return Err(CartridgeError::IncompatibleMapper(mapper as u16));
            }
        };

        let rom_data = NesCartridgeData {
            trainer: trainer,
            prg_rom: prg_rom,
            chr_rom: chr_rom,
            inst_rom: inst_rom,
            prom: None,
            prg_ram: prg_ram,
        };

        Ok(Self {
            data: rom_data,
            mapper: mapper,
        })
    }

    fn load_ines2(_rom_contents: &[u8]) -> Result<Self, CartridgeError> {
        unimplemented!()
    }

    pub fn load_cartridge(name: String) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name);
        if let Err(e) = rom_contents {
            return Err(CartridgeError::FsError(e));
        }
        let rom_contents = rom_contents.unwrap();
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

    pub fn rom_byte_hack(&mut self, addr: u32, new_byte: u8) {
        self.mapper.rom_byte_hack(&mut self.data, addr, new_byte);
    }
}
