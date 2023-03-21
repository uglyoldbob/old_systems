//! Implements mapper00

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

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
    pub fn new(d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            mirror_vertical: d.mirroring,
            ppu_address: 0,
        })
    }
}

impl NesMapperTrait for Mapper00 {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                let mut addr2 = addr & 0x1fff;
                if !cart.prg_ram.is_empty() {
                    addr2 %= cart.prg_ram.len() as u16;
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

    fn memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _addr: u16, _data: u8) {}

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
        if cart.chr_rom.is_empty() {
            return None;
        }
        Some(cart.chr_rom[(self.ppu_address as usize) % cart.chr_rom.len()])
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8) {
        if !cart.chr_ram || cart.chr_rom.is_empty() {
            return;
        }
        let addr2 = self.ppu_address as u32 % cart.chr_rom.len() as u32;
        cart.chr_rom[addr2 as usize] = data;
    }

    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8) {
        let addr = addr as usize % cart.prg_rom.len();
        cart.prg_rom[addr] = new_byte;
    }
}
