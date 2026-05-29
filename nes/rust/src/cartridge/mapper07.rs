//! Implements mapper 07 (AxROM)

#[cfg(feature = "debugger")]
use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper 07 (AxROM)
///
/// PRG banking:
///   bits 0-3 = 32KB PRG bank
///
/// Mirroring:
///   bit 4 = one-screen mirroring
///     0 = lower bank
///     1 = upper bank
///
/// CHR:
///   Fixed 8KB CHR RAM/ROM at $0000-$1FFF
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper07 {
    /// Current PPU address
    ppu_address: u16,
    /// Bank register
    bank: u8,
}

impl Mapper07 {
    /// Create a new mapper07
    pub fn new(_d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            ppu_address: 0,
            bank: 0,
        })
    }

    /// Check one-screen mirroring configuration.
    fn check_mirroring(&self, _addr: u16) -> (bool, bool) {
        // AxROM uses one-screen mirroring.
        // bit 4 selects which nametable page is used.
        let upper_bank = (self.bank & 0x10) != 0;

        (upper_bank, false)
    }

    /// Get current 32KB PRG bank
    fn prg_bank(&self) -> usize {
        (self.bank & 0x0f) as usize
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

        let addr2 = (addr as usize) & 0x1fff;
        let addr3 = addr2 % chr.len();

        Some(chr[addr3])
    }
}

impl NesMapperTrait for Mapper07 {
    fn irq(&self) -> bool {
        false
    }

    #[cfg(feature = "debugger")]
    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();

        hm.insert("Mapper".to_string(), 7);
        hm.insert("BANK".to_string(), self.bank);
        hm.insert("PRG BANK".to_string(), self.prg_bank() as u8);
        hm.insert("MIRROR PAGE".to_string(), ((self.bank >> 4) & 0x01) as u8);

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
                if cart.nonvolatile.prg_rom.is_empty() {
                    return None;
                }

                let bank_base = self.prg_bank() * 0x8000;
                let addr2 = bank_base | ((addr as usize) & 0x7fff);
                let addr3 = addr2 % cart.nonvolatile.prg_rom.len();

                Some(cart.nonvolatile.prg_rom[addr3])
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
                if cart.nonvolatile.prg_rom.is_empty() {
                    return None;
                }

                let bank_base = self.prg_bank() * 0x8000;
                let addr2 = bank_base | ((addr as usize) & 0x7fff);
                let addr3 = addr2 % cart.nonvolatile.prg_rom.len();

                Some(cart.nonvolatile.prg_rom[addr3])
            }

            _ => None,
        }
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {
        if addr >= 0x8000 {
            // Mapper 07 (AxROM)
            // bits 0-3 = 32KB PRG bank
            // bit 4 = one-screen mirroring select
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

    #[cfg(feature = "debugger")]
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

        let addr2 = ((addr as usize) & 0x1fff) % chr.len();

        chr[addr2] = data;
    }

    #[cfg(test)]
    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
