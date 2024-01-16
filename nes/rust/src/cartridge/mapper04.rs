//! Implements mapper 04

use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper 04
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper04 {
    /// The ppu address for ppu addressing
    ppu_address: u16,
    /// Registers: Bank select, Bank data, mirroring, prg ram protect, irq latch, irq reload, irq disable, irq enable
    registers: [u8; 8],
    /// prg rom bank registers
    prg_roms: [u8; 2],
    /// chr rom bank registers
    chr_roms: [u8; 6],
    /// The irq counter for the mapper
    irq_counter: u8,
    /// The irq counter latch
    irq_latch: u8,
    /// Indicates that the irq counter should be reloaded
    reload_irq: bool,
    /// Indicates that interrupts are enabled
    irq_enabled: bool,
    /// Indicates that an irq is pending
    irq_pending: bool,
    /// Used to filter the ppu a12 signal for the irq clock
    irq_filter: u8,
}

impl Mapper04 {
    /// Create a new Mapper04
    pub fn new(_d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            ppu_address: 0,
            registers: [0; 8],
            prg_roms: [0; 2],
            chr_roms: [0; 6],
            irq_counter: 0,
            irq_latch: 0,
            reload_irq: false,
            irq_enabled: false,
            irq_pending: false,
            irq_filter: 0,
        })
    }
    /// Check the mirroring bit for the ppu addressing.
    fn check_mirroring(&self, addr: u16) -> (bool, bool) {
        let a10 = if (self.registers[2] & 1) == 0 {
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
        // The a12 inversion bit
        if (self.registers[0] & 0x80) == 0 {
            match addr {
                0..=0x7ff => {
                    let bank = ((self.chr_roms[0] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x800..=0xfff => {
                    let bank = ((self.chr_roms[1] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x1000..=0x13ff => {
                    let bank = (self.chr_roms[2] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x1400..=0x17ff => {
                    let bank = (self.chr_roms[3] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x1800..=0x1bff => {
                    let bank = (self.chr_roms[4] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x1c00..=0x1fff => {
                    let bank = (self.chr_roms[5] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                _ => None,
            }
        } else {
            match addr {
                0x1000..=0x17ff => {
                    let bank = ((self.chr_roms[0] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x1800..=0x1fff => {
                    let bank = ((self.chr_roms[1] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x000..=0x3ff => {
                    let bank = (self.chr_roms[2] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x400..=0x7ff => {
                    let bank = (self.chr_roms[3] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0x800..=0xbff => {
                    let bank = (self.chr_roms[4] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                0xc00..=0xfff => {
                    let bank = (self.chr_roms[5] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    Some(chr[addr3 as usize])
                }
                _ => None,
            }
        }
    }

    /// Runs when the irq is actually clocked
    fn irq_clock(&mut self) {
        if self.irq_counter == 0 || self.reload_irq {
            self.irq_counter = self.irq_latch;
        } else {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }

        self.reload_irq = false;
    }

    /// Runs for the input signal to the irq, eventually clocking the irq
    fn irq_filter(&mut self, clock: bool) {
        self.irq_filter = (self.irq_filter << 1) | clock as u8;
        if (self.irq_filter & 7) == 1 {
            self.irq_clock();
        }
    }
}

impl NesMapperTrait for Mapper04 {
    fn irq(&self) -> bool {
        self.irq_pending
    }

    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("PPU bank 0".to_string(), self.chr_roms[0]);
        hm.insert("PPU bank 1".to_string(), self.chr_roms[1]);
        hm.insert("PPU bank 2".to_string(), self.chr_roms[2]);
        hm.insert("PPU bank 3".to_string(), self.chr_roms[3]);

        hm.insert("CPU bank 0".to_string(), self.prg_roms[0]);
        hm.insert("CPU bank 1".to_string(), self.prg_roms[1]);

        hm.insert("Mirroring".to_string(), self.registers[2]);
        hm.insert("PRG RAM".to_string(), self.registers[3]);

        hm.insert("IRQ LATCH".to_string(), self.irq_latch);
        hm.insert("IRQ Counter".to_string(), self.irq_counter);
        hm.insert("IRQ Enabled".to_string(), self.irq_enabled as u8);
        hm.insert("IRQ Pending".to_string(), self.irq_pending as u8);

        hm.insert("Mapper".to_string(), 4);
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
            0x8000..=0x9fff => {
                if (self.registers[0] & 0x40) == 0 {
                    //banked with prg_rom[0]
                    let bank = (self.prg_roms[0] as u32) << 13;
                    let addr2 = (addr & 0x1fff) as u32 | bank;
                    let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr3 as usize])
                } else {
                    //fixed to second to last bank
                    let bank = 0x3E << 13;
                    let addr2 = (addr & 0x1fff) as u32 | bank;
                    let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr3 as usize])
                }
            }
            0xA000..=0xBFFF => {
                //banked with prg_rom[1]
                let bank = (self.prg_roms[1] as u32) << 13;
                let addr2 = (addr & 0x1fff) as u32 | bank;
                let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                Some(cart.nonvolatile.prg_rom[addr3 as usize])
            }
            0xC000..=0xDFFF => {
                if (self.registers[0] & 0x40) != 0 {
                    //banked with prg_rom[0]
                    let bank = (self.prg_roms[0] as u32) << 13;
                    let addr2 = (addr & 0x1fff) as u32 | bank;
                    let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr3 as usize])
                } else {
                    //fixed to second to last bank
                    let bank = 0x3E << 13;
                    let addr2 = (addr & 0x1fff) as u32 | bank;
                    let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr3 as usize])
                }
            }
            0xE000..=0xFFFF => {
                //fixed to second to last bank
                let bank = 0x3F << 13;
                let addr2 = (addr & 0x1fff) as u32 | bank;
                let addr3 = addr2 & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                Some(cart.nonvolatile.prg_rom[addr3 as usize])
            }
            _ => None,
        }
    }

    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        self.memory_cycle_dump(cart, addr)
    }

    fn memory_cycle_nop(&mut self) {}

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {
        match addr {
            0..=0x5FFF => {}
            0x6000..=0x7fff => {
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
            0x8000..=0x9FFF => {
                if (addr & 1) == 0 {
                    self.registers[0] = data;
                } else {
                    match self.registers[0] & 7 {
                        0..=5 => {
                            self.chr_roms[self.registers[0] as usize & 7] = data;
                        }
                        6 => {
                            self.prg_roms[0] = data;
                        }
                        7 => {
                            self.prg_roms[1] = data;
                        }
                        _ => {
                            unreachable!();
                        }
                    }
                    self.registers[1] = data;
                }
            }
            0xA000..=0xBFFF => {
                if (addr & 1) == 0 {
                    self.registers[2] = data;
                } else {
                    self.registers[3] = data;
                }
            }
            0xC000..=0xDFFF => {
                if (addr & 1) == 0 {
                    self.registers[4] = data;
                    self.irq_latch = data;
                } else {
                    self.irq_counter = 0;
                    self.reload_irq = true;
                    self.registers[5] = data;
                }
            }
            0xE000..=0xFFFF => {
                if (addr & 1) == 0 {
                    self.registers[6] = data;
                    self.irq_enabled = false;
                    self.irq_pending = false;
                } else {
                    self.registers[7] = data;
                    self.irq_enabled = true;
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
        if addr < 0x2000 {
            self.irq_filter((addr & (1 << 12)) != 0);
        }
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
        // The a12 inversion bit
        if (self.registers[0] & 0x80) == 0 {
            match addr {
                0..=0x7ff => {
                    let bank = ((self.chr_roms[0] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x800..=0xfff => {
                    let bank = ((self.chr_roms[1] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x1000..=0x13ff => {
                    let bank = (self.chr_roms[2] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x1400..=0x17ff => {
                    let bank = (self.chr_roms[3] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x1800..=0x1bff => {
                    let bank = (self.chr_roms[4] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x1c00..=0x1fff => {
                    let bank = (self.chr_roms[5] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                _ => {}
            }
        } else {
            match addr {
                0x1000..=0x17ff => {
                    let bank = ((self.chr_roms[0] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x1800..=0x1fff => {
                    let bank = ((self.chr_roms[1] as u32) & 0xFE) << 10;
                    let addr2 = (addr & 0x7ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x000..=0x3ff => {
                    let bank = (self.chr_roms[2] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x400..=0x7ff => {
                    let bank = (self.chr_roms[3] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0x800..=0xbff => {
                    let bank = (self.chr_roms[4] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                0xc00..=0xfff => {
                    let bank = (self.chr_roms[5] as u32) << 10;
                    let addr2 = (addr & 0x3ff) as u32 | bank;
                    let addr3 = addr2 & (chr.len() as u32 - 1);
                    chr[addr3 as usize] = data;
                }
                _ => {}
            }
        }
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
