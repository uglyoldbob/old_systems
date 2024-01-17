//! Implements mapper 05

use std::collections::BTreeMap;

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

use serde_with::Bytes;

/// Mapper 05
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper05 {
    /// The ppu address for ppu addressing
    ppu_address: u16,
    /// First group of registers 0x5105 - 0x5107
    registers: [u8; 8],
    /// Second group of registers 0x5113 - 0x5117
    registers2: [u8; 5],
    /// Third group of registers, 0x5120 - 0x5130
    registers3: [u8; 17],
    /// Fourth group of registers: 0x5200 - 0x5206
    registers4: [u8; 7],
    /// A block of internal ram
    #[serde_as(as = "Bytes")]
    int_ram: [u8; 1024],
    /// The intercepted value for the ppu ctrl register
    ppuctrl: u8,
    /// The intercepted value for the ppu mask register
    ppumask: u8,
    /// The irq register
    irq: u8,
}

impl Mapper05 {
    /// Create a new Mapper05
    pub fn new(_d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            ppu_address: 0,
            registers: [0xff; 8],
            registers2: [0xff; 5],
            registers3: [0xff; 17],
            registers4: [0xff; 7],
            int_ram: [0xff; 1024],
            ppuctrl: 0,
            ppumask: 0,
            irq: 0,
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
        None
    }
}

impl NesMapperTrait for Mapper05 {
    fn irq(&self) -> bool {
        false
    }

    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("Mirroring".to_string(), self.registers[2]);
        hm.insert("PRG RAM".to_string(), self.registers[3]);

        hm.insert("Mapper".to_string(), 5);
        hm
    }

    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x5200..=0x5203 => {
                let i = addr & 7;
                Some(self.registers4[i as usize])
            }
            0x5204 => Some(self.irq),
            0x5205 => {
                let mul: u16 = (self.registers4[5] as u16) * (self.registers4[6] as u16);
                let mul = (mul & 0xFF) as u8;
                Some(mul)
            }
            0x5206 => {
                let mul: u16 = (self.registers4[5] as u16) * (self.registers4[6] as u16);
                let mul = (mul >> 8) as u8;
                Some(mul)
            }
            0x6000..=0x7fff => {
                let a = (self.registers2[0] & 0x7f) as u32;
                let addr = (addr as u32 & 0x1FFF) | (a << 13);
                let addr = addr & (cart.volatile.prg_ram.len() as u32 - 1);
                Some(cart.volatile.prg_ram[addr as usize])
            },
            0x8000..=0x9fff => match self.registers[0] & 3 {
                0 => {
                    let a = (self.registers2[4] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr as usize])
                }
                1 | 2 => {
                    let a = (self.registers2[2] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let rom = (self.registers2[2] & 0x80) != 0;
                    if rom {
                        let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                        Some(cart.nonvolatile.prg_rom[addr as usize])
                    } else {
                        let addr = addr & (cart.volatile.prg_ram.len() as u32 - 1);
                        Some(cart.volatile.prg_ram[addr as usize])
                    }
                }
                3 => {
                    let a = (self.registers2[1] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let rom = (self.registers2[1] & 0x80) != 0;
                    if rom {
                        let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                        Some(cart.nonvolatile.prg_rom[addr as usize])
                    } else {
                        let addr = addr & (cart.volatile.prg_ram.len() as u32 - 1);
                        Some(cart.volatile.prg_ram[addr as usize])
                    }
                }
                _ => unreachable!(),
            },
            0xa000..=0xbfff => match self.registers[0] & 3 {
                0 => {
                    let a = (self.registers2[4] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr as usize])
                }
                1 | 2 | 3 => {
                    let a = (self.registers2[2] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let rom = (self.registers2[2] & 0x80) != 0;
                    if rom {
                        let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                        Some(cart.nonvolatile.prg_rom[addr as usize])
                    } else {
                        let addr = addr & (cart.volatile.prg_ram.len() as u32 - 1);
                        Some(cart.volatile.prg_ram[addr as usize])
                    }
                }
                _ => unreachable!(),
            },
            0xc000..=0xdfff => match self.registers[0] & 3 {
                0 | 1 => {
                    let a = (self.registers2[4] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                    Some(cart.nonvolatile.prg_rom[addr as usize])
                }
                2 | 3 => {
                    let a = (self.registers2[3] & 0x7f) as u32;
                    let addr = (addr as u32 & 0x1FFF) | (a << 13);
                    let rom = (self.registers2[3] & 0x80) != 0;
                    if rom {
                        let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                        Some(cart.nonvolatile.prg_rom[addr as usize])
                    } else {
                        let addr = addr & (cart.volatile.prg_ram.len() as u32 - 1);
                        Some(cart.volatile.prg_ram[addr as usize])
                    }
                }
                _ => unreachable!(),
            },
            0xe000..=0xffff => {
                let a = (self.registers2[4] & 0x7f) as u32;
                let addr = (addr as u32 & 0x1FFF) | (a << 13);
                let addr = addr & (cart.nonvolatile.prg_rom.len() as u32 - 1);
                Some(cart.nonvolatile.prg_rom[addr as usize])
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
            0x5100..=0x5107 => {
                let i = addr & 7;
                self.registers[i as usize] = data;
            }
            0x5113..=0x5117 => {
                let i = addr - 0x5113;
                self.registers2[i as usize] = data;
            }
            0x5120..=0x5130 => {
                let i = addr - 0x5120;
                self.registers3[i as usize] = data;
            }
            0x5200..=0x5206 => {
                let i = addr - 0x5200;
                self.registers4[i as usize] = data;
            }
            0x5c00..=0x5fff => {
                match self.registers[4] & 3 {
                    // internal ram is read only
                    3 => {}
                    // writes work normally when rendering is enabled
                    _ => {
                        let data = if (self.ppumask & 0x18) != 0 { data } else { 0 };
                        let i = addr & 0x3fff;
                        self.int_ram[i as usize] = data;
                    }
                }
            }
            0x6000..=0x7fff => match self.registers[0] & 3 {
                0 => {}
                1 => {}
                2 => {}
                3 => {}
                _ => unreachable!(),
            },
            0x8000..=0x9fff => match self.registers[0] & 3 {
                0 => {}
                1 => {}
                2 => {}
                3 => {}
                _ => unreachable!(),
            },
            0xa000..=0xbfff => match self.registers[0] & 3 {
                0 => {}
                1 => {}
                2 => {}
                3 => {}
                _ => unreachable!(),
            },
            0xc000..=0xdfff => match self.registers[0] & 3 {
                0 => {}
                1 => {}
                2 => {}
                3 => {}
                _ => unreachable!(),
            },
            0xe000..=0xffff => {
                //always rom so do nothing here
            }
            _ => {}
        }
    }

    fn other_memory_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x2000 => {
                self.ppuctrl = data;
            }
            0x2001 => {
                self.ppumask = data;
            }
            _ => {}
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
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
