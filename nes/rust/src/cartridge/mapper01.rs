//! Implements mapper01

use crate::cartridge::NesCartridgeData;
use crate::cartridge::{NesMapper, NesMapperTrait};

/// Mapper01
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mapper01 {
    /// The conteents of the shift register
    shift_register: u8,
    /// The counter used for inputting data into the shift register
    shift_counter: u8,
    /// The shift register is locked
    shift_locked: bool,
    /// Control, chr bank 0, chr bank 1, prg bank
    registers: [u8; 4],
    /// The ppu address for ppu cycles
    ppu_address: u16,
}

impl Mapper01 {
    /// Create a new mapper01
    pub fn new(_d: &NesCartridgeData) -> NesMapper {
        NesMapper::from(Self {
            shift_register: 0,
            shift_counter: 0,
            shift_locked: false,
            registers: [0x0c, 0, 0, 0],
            ppu_address: 0,
        })
    }

    /// Update a register in the mapper
    fn update_register(&mut self, adr: u8, data: u8) {
        self.registers[adr as usize] = data;
    }
}

impl NesMapperTrait for Mapper01 {
    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                let mut addr2 = addr & 0x1fff;
                if !cart.prg_ram.is_empty() {
                    addr2 %= cart.prg_ram.len() as u16;
                    Some(cart.prg_ram[addr2 as usize & (cart.prg_ram.len() - 1)])
                } else {
                    None
                }
            }
            0x8000..=0xffff => {
                match (self.registers[0] & 0x0C) >> 2 {
                    0 | 1 => {
                        //32kb bankswitch
                        let addr2 = addr & 0x7fff;
                        let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                        let addr4 = (self.registers[3] as u32 & 0xE) << 14;
                        let addr5 = addr3 | addr4;
                        Some(cart.prg_rom[addr5 as usize % (cart.prg_rom.len() - 1)])
                    }
                    2 => {
                        //first half fixed, second half switched
                        if addr < 0xc000 {
                            //fixed to first bank
                            let addr2 = addr & 0x3fff;
                            let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            Some(cart.prg_rom[addr3 as usize % (cart.prg_rom.len() - 1)])
                        } else {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            addr3 |= (self.registers[3] as u32 & 0xF) * 16384;
                            Some(cart.prg_rom[addr3 as usize % (cart.prg_rom.len() - 1)])
                        }
                    }
                    _ => {
                        //first half switched, second half fixed
                        if addr < 0xc000 {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            addr3 |= (self.registers[3] as u32 & 0xF) * 16384;
                            Some(cart.prg_rom[addr3 as usize % (cart.prg_rom.len() - 1)])
                        } else {
                            //fixed to last bank
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32;
                            addr3 |= ((cart.prg_rom.len() - 1) & !0x3fff) as u32;
                            Some(cart.prg_rom[addr3 as usize % (cart.prg_rom.len() - 1)])
                        }
                    }
                }
            }
            _ => None,
        }
    }

    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        self.shift_locked = false;
        self.memory_cycle_dump(cart, addr)
    }

    fn memory_cycle_nop(&mut self) {
        self.shift_locked = false;
    }

    fn memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, addr: u16, data: u8) {
        if addr >= 0x8000 && !self.shift_locked {
            self.shift_locked = true;
            if (data & 0x80) != 0 {
                self.shift_counter = 0;
                self.shift_register = 0;
                self.registers[0] |= 0x0C;
            } else if self.shift_counter < 5 {
                self.shift_counter += 1;
                self.shift_register >>= 1;
                if (data & 1) != 0 {
                    self.shift_register |= 0x10;
                }
            } else {
                let adr_select = (addr & 0x6000) >> 13;
                self.update_register(adr_select as u8, self.shift_register);
                self.shift_counter = 0;
                self.shift_register = 0;
            }
        }
    }

    fn ppu_memory_cycle_address(&mut self, addr: u16) -> (bool, bool) {
        self.ppu_address = addr;
        let control = self.registers[0];
        let a10 = match control & 3 {
            0 | 1 => (control & 1) != 0,
            2 => (addr & 1 << 10) != 0,
            _ => (addr & 1 << 11) != 0,
        };
        (a10, false)
    }

    fn ppu_memory_cycle_read(&mut self, cart: &mut NesCartridgeData) -> Option<u8> {
        if cart.chr_rom.is_empty() {
            return None;
        }
        if (self.registers[0] & 0x10) != 0 {
            //two separate 4kb banks
            match self.ppu_address {
                0..=0x0fff => {
                    let addr2 = self.ppu_address & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[1] as u32 & 0x1F) << 12;
                    Some(cart.chr_rom[addr3 as usize])
                }
                0x1000..=0x1fff => {
                    let addr2 = self.ppu_address & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[2] as u32 & 0x1F) << 12;
                    Some(cart.chr_rom[addr3 as usize])
                }
                _ => None,
            }
        } else {
            //one 8kb bank
            match self.ppu_address {
                0..=0x1fff => {
                    let addr2 = self.ppu_address & 0x1fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[1] as u32 & 0x1E) << 12;
                    Some(cart.chr_rom[addr3 as usize & (cart.chr_rom.len() - 1)])
                }
                _ => None,
            }
        }
    }

    fn ppu_memory_cycle_write(&mut self, cart: &mut NesCartridgeData, data: u8) {
        if !cart.chr_ram {
            return;
        }
        if cart.chr_rom.is_empty() {
            return;
        }
        if (self.registers[0] & 0x10) != 0 {
            //two separate 4kb banks
            match self.ppu_address {
                0..=0x0fff => {
                    let addr2 = self.ppu_address & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[1] as u32 & 0x1F) << 12;
                    cart.chr_rom[addr3 as usize] = data;
                }
                0x1000..=0x1fff => {
                    let addr2 = self.ppu_address & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[2] as u32 & 0x1F) << 12;
                    cart.chr_rom[addr3 as usize] = data;
                }
                _ => {}
            }
        } else {
            //one 8kb bank
            if let 0..=0x1fff = self.ppu_address {
                let addr2 = self.ppu_address & 0x1fff;
                let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                addr3 |= (self.registers[1] as u32 & 0x1E) << 12;
                cart.chr_rom[addr3 as usize] = data;
            }
        }
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
