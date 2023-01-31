use crate::cartridge::NesCartridgeData;
use crate::cartridge::NesMapper;

pub struct Mapper {
    mirror_vertical: bool,
    ppu_address: u16,
}

impl Mapper {
    pub fn new(d: &NesCartridgeData) -> Box<dyn NesMapper> {
        Box::new(Self {
            mirror_vertical: d.mirroring,
            ppu_address: 0,
        })
    }
}

impl NesMapper for Mapper {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                let mut addr2 = addr & 0x1fff;
                if cart.prg_ram.len() != 0 {
                    addr2 = addr2 % cart.prg_ram.len() as u16;
                    Some(cart.prg_ram[addr2 as usize])
                } else {
                    None
                }
            }
            0x8000..=0xffff => {
                let addr2 = addr & 0x7fff;
                let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                Some(cart.prg_rom[addr3 as usize])
            }
            _ => None,
        }
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, addr: u16, data: u8) {}

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        let a10 = if self.mirror_vertical {
            (addr & 1 << 10) != 0
        } else {
            (addr & 1 << 11) != 0
        };
        (a10, false)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        if cart.chr_rom.len() == 0 {
            return None;
        }
        Some(cart.chr_rom[self.ppu_address as usize])
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8) {}

    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8) {}
}
