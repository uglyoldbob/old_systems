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
}

impl Mapper04 {
    /// Create a new Mapper04
    pub fn new(d: &NesCartridgeData) -> NesMapper {
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
            _ => { None }
        }
    }
}

impl NesMapperTrait for Mapper04 {
    fn irq(&self) -> bool {
        false
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
            0x8000..=0xffff => {
                None
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
                }
                else {
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
                }
                else {
                    self.registers[3] = data;
                }
            }
            0xC000..=0xDFFF => {
                if (addr & 1) == 0 {
                    self.registers[4] = data;
                }
                else {
                    self.irq_counter = 0;
                    self.reload_irq = true;
                    self.registers[5] = data;
                }
            }
            0xE000..=0xFFFF => {
                if (addr & 1) == 0 {
                    self.registers[6] = data;
                    self.irq_enabled = true;
                    self.irq_pending = false;
                }
                else {
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
        self.check_mirroring(addr)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        self.ppu_read(self.ppu_address, cart)
    }

    fn ppu_memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _data: u8) {
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
