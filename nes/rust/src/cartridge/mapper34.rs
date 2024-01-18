//! Implements mapper34

use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper00
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper34 {
    /// True when mirroring is vertical
    mirror_vertical: bool,
    /// The address for ppu memory cycles
    ppu_address: u16,
    /// The bank select register
    bank: u8,
    /// Extra registers
    regs: [u8; 2],
}

impl Mapper34 {
    /// Create a new mapper
    pub fn new(d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            mirror_vertical: d.volatile.mirroring,
            ppu_address: 0,
            bank: 0xff,
            regs: [0; 2],
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

impl NesMapperTrait for Mapper34 {
    fn irq(&self) -> bool {
        false
    }

    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("Mirror".to_string(), self.mirror_vertical as u8);
        hm.insert("Mapper".to_string(), 0);
        hm
    }

    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                if cart.nonvolatile.trainer.is_some() && (0x7000..=0x71ff).contains(&addr) {
                    let c = cart.nonvolatile.trainer.as_ref().unwrap();
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
            0x8000..=0xbfff => {
                let addr2 = addr & 0x3fff;
                let mut addr3 = addr2 as u32 % cart.nonvolatile.prg_rom.len() as u32;
                addr3 |= (self.bank as u32 & 0xF) << 14;
                let addr4 = addr3;
                Some(
                    cart.nonvolatile.prg_rom
                        [addr4 as usize & (cart.nonvolatile.prg_rom.len() - 1)],
                )
            }
            0xc000..=0xffff => {
                let addr2 = addr & 0x3fff;
                let mut addr3 = addr2 as u32 % cart.nonvolatile.prg_rom.len() as u32;
                addr3 |= 0xFFFF << 14;
                let addr4 = addr3;
                Some(
                    cart.nonvolatile.prg_rom
                        [addr4 as usize & (cart.nonvolatile.prg_rom.len() - 1)],
                )
            }
            _ => None,
        }
    }

    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
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
                let mut addr3 = addr2 as u32 % cart.nonvolatile.prg_rom.len() as u32;
                addr3 |= (self.bank as u32 & 0xF) << 15;
                let addr4 = addr3;
                Some(
                    cart.nonvolatile.prg_rom
                        [addr4 as usize & (cart.nonvolatile.prg_rom.len() - 1)],
                )
            }
            _ => None,
        }
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7ffc => {
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
            0x7ffd => {
                self.bank = data;
            }
            0x7ffe => {
                self.regs[1] = data;
            }
            0x7fff => {
                self.regs[2] = data;
            }
            0x8000..=0xffff => {
                self.bank = data;
            }
            _ => {}
        }
    }

    fn ppu_peek_address(&self, addr: u16, cart: &NesCartridgeData) -> (bool, bool, Option<u8>) {
        let (mirror, thing) = self.check_mirroring(addr);
        let data = cart.nonvolatile.chr_rom[(addr as usize) % cart.nonvolatile.chr_rom.len()];
        (mirror, thing, Some(data))
    }

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        self.check_mirroring(addr)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        let mut v = Vec::new();
        let chr = if !cart.volatile.chr_ram.is_empty() {
            &mut cart.volatile.chr_ram
        } else if !cart.nonvolatile.chr_rom.is_empty() {
            &mut cart.nonvolatile.chr_rom
        } else {
            &mut v
        };
        if chr.is_empty() {
            return None;
        }
        Some(chr[(self.ppu_address as usize) % chr.len()])
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8) {
        if cart.volatile.chr_ram.is_empty() {
            return;
        }
        let addr2 = self.ppu_address as u32 % cart.volatile.chr_ram.len() as u32;
        cart.volatile.chr_ram[addr2 as usize] = data;
    }

    fn rom_byte_hack(&mut self, cart: &mut NesCartridgeData, addr: u32, new_byte: u8) {
        let addr = addr as usize % cart.nonvolatile.prg_rom.len();
        cart.nonvolatile.prg_rom[addr] = new_byte;
    }
}
