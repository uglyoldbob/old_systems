use crate::cartridge::NesCartridgeData;
use crate::cartridge::NesMapper;

pub struct Mapper {
    ppu_address: u16,
    mirroring: bool,
}

impl Mapper {
    pub fn new(d: &NesCartridgeData) -> Box<dyn NesMapper> {
        Box::new(Self {
            ppu_address: 0,
            mirroring: d.mirroring,
        })
    }
}

impl NesMapper for Mapper {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7FFF => {
                if cart.prg_ram.len() > 0 {
                    let mask = cart.prg_ram.len() - 1;
                    let adr = addr as usize & mask;
                    Some(cart.prg_ram[adr])
                } else {
                    None
                }
            }
            0x8000..=0xFFFF => {
                if cart.prg_rom.len() > 0 {
                    let mask = cart.prg_rom.len() - 1;
                    let adr = addr as usize & mask;
                    Some(cart.prg_rom[adr])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _addr: u16, _data: u8) {}

    fn memory_cycle_nop(&mut self) {}

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        let a10 = if !self.mirroring {
            (addr & 0x400) != 0
        } else {
            (addr & 0x200) != 0
        };
        (a10, false)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        if cart.chr_rom.len() > 0 {
            let mask = cart.chr_rom.len() - 1;
            let adr = self.ppu_address as usize & mask;
            Some(cart.chr_rom[adr])
        } else {
            None
        }
    }

    fn ppu_memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _data: u8) {}

    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8) {
        match addr {
            0x8000..=0xFFFF => {
                if cart.prg_rom.len() > 0 {
                    let mask = cart.prg_rom.len() - 1;
                    let adr = addr as usize & mask;
                    cart.prg_rom[adr] = new_byte;
                }
            }
            _ => {}
        }
    }
}
