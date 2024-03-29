//! Implements mapper 03

use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper 03
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper03 {
    /// Flag determining if mirroring is vertical
    mirror_vertical: bool,
    /// The ppu address for ppu addressing
    ppu_address: u16,
    /// The bank select register
    bank: u8,
}

impl Mapper03 {
    /// Create a new mapper03
    pub fn new(d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            mirror_vertical: d.volatile.mirroring,
            ppu_address: 0,
            bank: 0,
        })
    }
    /// Check the mirroring bit for the ppu addressing.
    fn check_mirroring(&self, addr: u16) -> (bool, bool) {
        let a10 = if self.mirror_vertical {
            (addr & 1 << 10) != 0
        } else {
            (addr & 1 << 11) != 0
        };
        (a10, false)
    }
    /// Perform a ppu read operation
    fn ppu_read(&self, addr: u16, cart: &NesCartridgeData) -> Option<u8> {
        let v = Vec::new();
        let chr = if !cart.volatile.chr_ram.is_empty() {
            &cart.volatile.chr_ram
        } else if !cart.nonvolatile.chr_rom.is_empty() {
            &cart.nonvolatile.chr_rom
        } else {
            &v
        };
        if chr.is_empty() {
            return None;
        }
        let addr2 = (addr | (self.bank as u16 * 0x2000)) & (chr.len() - 1) as u16;
        Some(chr[addr2 as usize])
    }
}

impl NesMapperTrait for Mapper03 {
    fn irq(&self) -> bool {
        false
    }

    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("Mirror".to_string(), self.mirror_vertical as u8);
        hm.insert("Mapper".to_string(), 3);
        hm.insert("PPU BANK".to_string(), self.bank);
        hm
    }

    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                let mut addr2 = addr & 0x1fff;
                if !cart.volatile.prg_ram.is_empty() {
                    addr2 %= cart.volatile.prg_ram.len() as u16;
                    Some(cart.volatile.prg_ram[addr2 as usize])
                } else {
                    None
                }
            }
            0x8000..=0xffff => {
                let addr2 = addr & 0x7fff;
                let addr3 = addr2 as u32 % cart.nonvolatile.prg_rom.len() as u32;
                Some(cart.nonvolatile.prg_rom[addr3 as usize])
            }
            _ => None,
        }
    }

    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                if cart.nonvolatile.trainer.is_some() && (0x7000..=0x71ff).contains(&addr) {
                    let c = cart.nonvolatile.trainer.as_mut().unwrap();
                    let addr = addr & 0x1ff;
                    Some(c[addr as usize])
                } else {
                    let mut addr2 = addr & 0x1fff;
                    if !cart.volatile.prg_ram.is_empty() {
                        addr2 %= cart.volatile.prg_ram.len() as u16;
                        Some(cart.volatile.prg_ram[addr2 as usize])
                    } else {
                        None
                    }
                }
            }
            0x8000..=0xffff => {
                let addr2 = addr & 0x7fff;
                let addr3 = addr2 as u32 & (cart.nonvolatile.prg_rom.len() - 1) as u32;
                Some(cart.nonvolatile.prg_rom[addr3 as usize])
            }
            _ => None,
        }
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.bank = data;
        } else if (0x6000..=0x7fff).contains(&addr) {
            if cart.nonvolatile.trainer.is_some() && (0x7000..=0x71ff).contains(&addr) {
                let c = cart.nonvolatile.trainer.as_mut().unwrap();
                let addr = addr & 0x1ff;
                c[addr as usize] = data;
            } else {
                let mut addr2 = addr & 0x1fff;
                if !cart.volatile.prg_ram.is_empty() {
                    addr2 %= cart.volatile.prg_ram.len() as u16;
                    cart.volatile.prg_ram[addr2 as usize] = data;
                }
            }
        }
    }

    fn ppu_peek_address(&self, addr: u16, cart: &NesCartridgeData) -> (bool, bool, Option<u8>) {
        let (mirror, thing) = self.check_mirroring(addr);
        let data = self.ppu_read(addr, cart);
        (mirror, thing, data)
    }

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        self.check_mirroring(addr)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        self.ppu_read(self.ppu_address, cart)
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8) {
        let addr = self.ppu_address;
        let mut v = Vec::new();
        let chr = if !cart.volatile.chr_ram.is_empty() {
            &mut cart.volatile.chr_ram
        } else {
            &mut v
        };
        if chr.is_empty() {
            return;
        }
        let addr2 = (addr | (self.bank as u16 * 0x2000)) & (chr.len() - 1) as u16;
        chr[addr2 as usize] = data;
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
