//! Implements mapper 03

use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper 03
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper04 {
    /// The ppu address for ppu addressing
    ppu_address: u16,
    /// Registers: Bank select, Bank data, mirroring, prg ram protect, irq latch, irq reload, irq disable, irq enable
    registers: [u8; 8],
    /// prg rom bank registers
    prg_roms: [u16; 4],
    /// chr rom bank registers
    chr_roms: [u16; 6],
}

impl Mapper04 {
    /// Create a new Mapper04
    pub fn new(d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            ppu_address: 0,
            registers: [0; 8],
            prg_roms: [0; 4],
            chr_roms: [0; 6],
        })
    }
    /// Check the mirroring bit for the ppu addressing.
    fn check_mirroring(&self, addr: u16) -> (bool, bool) {
        let a10 = if (self.registers[2] & 1) != 0 {
            (addr & 1 << 10) != 0
        } else {
            (addr & 1 << 11) != 0
        };
        (a10, false)
    }
    /// Perform a ppu read operation
    fn ppu_read(&self, addr: u16, cart: &NesCartridgeData) -> Option<u8> {
        if cart.nonvolatile.chr_rom.is_empty() {
            return None;
        }
        match addr {
            _ => {}
        }
        let addr2 =
            (addr | (self.bank as u16 * 0x2000)) & (cart.nonvolatile.chr_rom.len() - 1) as u16;
        Some(cart.nonvolatile.chr_rom[addr2 as usize])
    }
}

impl NesMapperTrait for Mapper04 {
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

    fn ppu_memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _data: u8) {}

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
