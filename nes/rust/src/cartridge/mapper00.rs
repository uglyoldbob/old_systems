use crate::cartridge::NesCartridgeData;
use crate::cartridge::NesMapper;

pub struct Mapper {}

impl Mapper {
    pub fn new() -> Box<dyn NesMapper> {
        Box::new(Self {})
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

    fn ppu_memory_cycle_address(&mut self, addr: u16) {}

    fn ppu_memory_cycle_read(&mut self, _cart: &mut NesCartridgeData) -> Option<u8> {
        None
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
