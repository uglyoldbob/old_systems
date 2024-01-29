//! Implements mapper00

use std::collections::BTreeMap;

use crate::cartridge::SnesCartridgeData;
use crate::cartridge::{SnesMapper, SnesMapperTrait};

/// Mapper00
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper00 {
    /// True when mirroring is vertical
    mirror_vertical: bool,
    /// The address for ppu memory cycles
    ppu_address: u16,
}

impl Mapper00 {
    /// Create a new mapper00
    pub fn new(d: &SnesCartridgeData) -> SnesMapper {
        SnesMapper::from(Self {
            mirror_vertical: d.volatile.mirroring,
            ppu_address: 0,
        })
    }
    /// Check the mirroring bit for the ppu addressing.
    fn check_mirroring(&self, addr: u16) -> (bool, bool) {
        let a10 = if self.mirror_vertical {
            (addr & (1 << 10)) != 0
        } else {
            (addr & (1 << 11)) != 0
        };
        (a10, false)
    }
}

impl SnesMapperTrait for Mapper00 {
    fn irq(&self) -> bool {
        false
    }

    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("Mirror".to_string(), self.mirror_vertical as u8);
        hm.insert("Mapper".to_string(), 0);
        hm
    }

    fn memory_cycle_dump(&self, cart: &SnesCartridgeData, bank: u8, addr: u16) -> Option<u8> {
        if (0x8000..=0xffff).contains(&addr) {
            let big_addr = ((bank as u32) << 15) | ((addr & 0x7fff) as u32);
            let big_addr = big_addr & (cart.nonvolatile.rom_largest - 1);
            if big_addr < cart.nonvolatile.rom_first {
                Some(cart.nonvolatile.prg_rom[big_addr as usize])
            } else {
                Some(42)
            }
        } else {
            None
        }
    }

    fn memory_cycle_read(
        &mut self,
        cart: &mut SnesCartridgeData,
        bank: u8,
        addr: u16,
    ) -> Option<u8> {
        if (0x8000..=0xffff).contains(&addr) {
            let big_addr = ((bank as u32) << 15) | ((addr & 0x7fff) as u32);
            let big_addr = big_addr & (cart.nonvolatile.rom_largest - 1);
            if big_addr < cart.nonvolatile.rom_first {
                Some(cart.nonvolatile.prg_rom[big_addr as usize])
            } else {
                todo!("Map the second half of rom");
            }
        } else {
            None
        }
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, cart: &mut SnesCartridgeData, bank: u8, addr: u16, data: u8) {}

    fn ppu_peek_address(&self, addr: u16, cart: &SnesCartridgeData) -> (bool, bool, Option<u8>) {
        let (mirror, thing) = self.check_mirroring(addr);
        let data = 42;
        (mirror, thing, Some(data))
    }

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        self.check_mirroring(addr)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut SnesCartridgeData) -> Option<u8> {
        None
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut SnesCartridgeData, data: u8) {
        if cart.volatile.chr_ram.is_empty() {
            return;
        }
        let addr2 = self.ppu_address as u32 % cart.volatile.chr_ram.len() as u32;
        cart.volatile.chr_ram[addr2 as usize] = data;
    }

    fn rom_byte_hack(&mut self, cart: &mut SnesCartridgeData, addr: u32, new_byte: u8) {
        let addr = addr as usize % cart.nonvolatile.prg_rom.len();
        cart.nonvolatile.prg_rom[addr] = new_byte;
    }
}
