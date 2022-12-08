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
        None
    }

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {}

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        None
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {}

    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8) {}
}
