//! Implements mapper01

use std::collections::BTreeMap;

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
    /// Extended bank select for prg rom, starts at bit 14
    rom_bank: u8,
    /// Extended bank select for prg ram, starts at bit 13
    ram_bank: u8,
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
            rom_bank: 0,
            ram_bank: 0,
        })
    }

    /// Update a register in the mapper
    fn update_register(&mut self, adr: u8, data: u8) {
        self.registers[adr as usize] = data;
    }
    /// Check the mirroring bit for the ppu addressing.
    fn check_mirroring(&self, addr: u16) -> (bool, bool) {
        let control = self.registers[0];
        let a10 = match control & 3 {
            0 | 1 => (control & 1) != 0,
            2 => (addr & (1 << 10)) != 0,
            _ => (addr & (1 << 11)) != 0,
        };
        (a10, false)
    }
    /// Perform a ppu read operation
    fn ppu_read(&self, addr: u16, cart: &NesCartridgeData) -> Option<u8> {
        if (self.registers[0] & 0x10) != 0 {
            //two separate 4kb banks
            match addr {
                0..=0x0fff => {
                    let addr2 = addr & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[1] as u32 & 0x1F) << 12;
                    Some(cart.chr_rom[addr3 as usize & (cart.chr_rom.len() - 1)])
                }
                0x1000..=0x1fff => {
                    let addr2 = addr & 0x0fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[2] as u32 & 0x1F) << 12;
                    Some(cart.chr_rom[addr3 as usize & (cart.chr_rom.len() - 1)])
                }
                _ => None,
            }
        } else {
            //one 8kb bank
            match addr {
                0..=0x1fff => {
                    let addr2 = addr & 0x1fff;
                    let mut addr3 = addr2 as u32 % cart.chr_rom.len() as u32;
                    addr3 |= (self.registers[1] as u32 & 0x1E) << 12;
                    Some(cart.chr_rom[addr3 as usize & (cart.chr_rom.len() - 1)])
                }
                _ => None,
            }
        }
    }
}

impl NesMapperTrait for Mapper01 {
    fn cartridge_registers(&self) -> BTreeMap<String, u8> {
        let mut hm = BTreeMap::new();
        hm.insert("Control".to_string(), self.registers[0]);
        hm.insert("Chr0".to_string(), self.registers[1]);
        hm.insert("Chr1".to_string(), self.registers[2]);
        hm.insert("prg bank".to_string(), self.registers[3]);
        hm.insert("Mapper".to_string(), 1);
        hm.insert("Extended Prg rom bank".to_string(), self.rom_bank);
        hm.insert("Extended Prg ram bank".to_string(), self.ram_bank);
        hm
    }

    fn memory_cycle_dump(&self, cart: &NesCartridgeData, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7fff => {
                if cart.trainer.is_some() && (0x7000..=0x71ff).contains(&addr) {
                    let c = cart.trainer.as_ref().unwrap();
                    let addr = addr & 0x1ff;
                    Some(c[addr as usize])
                } else {
                    let mut addr2 = addr & 0x1fff;
                    if !cart.prg_ram.is_empty() {
                        addr2 %= cart.prg_ram.len() as u16;
                        Some(cart.prg_ram[addr2 as usize])
                    } else {
                        None
                    }
                }
            }
            0x8000..=0xffff => {
                match (self.registers[0] & 0x0C) >> 2 {
                    0 | 1 => {
                        //32kb bankswitch
                        let addr2 = addr & 0x7fff;
                        let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                        let addr4 = (self.rom_bank as u32 | (self.registers[3] as u32 & 0xC)) << 14;
                        let addr5 = addr3 | addr4;
                        Some(cart.prg_rom[addr5 as usize & (cart.prg_rom.len() - 1)])
                    }
                    2 => {
                        //first half fixed, second half switched
                        if addr < 0xc000 {
                            //fixed to first bank
                            let addr2 = addr & 0x3fff;
                            let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            let addr4 = (self.rom_bank as u32) << 14 | addr3;
                            Some(cart.prg_rom[addr4 as usize & (cart.prg_rom.len() - 1)])
                        } else {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            addr3 |= (self.registers[3] as u32 & 0xF) << 14;
                            let addr4 = (self.rom_bank as u32) << 14 | addr3;
                            Some(cart.prg_rom[addr4 as usize & (cart.prg_rom.len() - 1)])
                        }
                    }
                    _ => {
                        //first half switched, second half fixed
                        if addr < 0xc000 {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32;
                            addr3 |= (self.rom_bank as u32 | self.registers[3] as u32 & 0xF) << 14;
                            Some(cart.prg_rom[addr3 as usize & (cart.prg_rom.len() - 1)])
                        } else {
                            //fixed to last bank
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32;
                            addr3 |= ((cart.prg_rom.len() - 1) & 0x3c000) as u32;
                            addr3 |= (self.rom_bank as u32) << 14;
                            Some(cart.prg_rom[addr3 as usize & (cart.prg_rom.len() - 1)])
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

    fn memory_cycle_write(&mut self, cart: &mut NesCartridgeData, addr: u16, data: u8) {
        if addr >= 0x8000 && !self.shift_locked {
            self.shift_locked = true;
            if (data & 0x80) != 0 {
                self.shift_counter = 0;
                self.shift_register = 0;
                self.registers[0] |= 0x0C;
            } else {
                self.shift_counter += 1;
                self.shift_register >>= 1;
                if (data & 1) != 0 {
                    self.shift_register |= 0x10;
                }
            }
            if self.shift_counter == 5 {
                let adr_select = (addr & 0x6000) >> 13;
                self.update_register(adr_select as u8, self.shift_register);
                if adr_select == 2 || adr_select == 1 {
                    let chr = self.shift_register;
                    let ram_a13 = if cart.prg_ram.len() == 32768 {
                        (chr & 4) >> 2
                    } else if cart.prg_ram.len() == 16384 {
                        (chr & 8) >> 3
                    } else {
                        0
                    };
                    let ram_a14 = if cart.prg_ram.len() == 32768 {
                        (chr & 8) >> 2
                    } else {
                        0
                    };
                    let rom_a18 = if cart.prg_rom.len() == (512 * 1024) {
                        chr & 0x10
                    } else {
                        0
                    };
                    self.rom_bank = rom_a18;
                    self.ram_bank = ram_a13 | (ram_a14 << 1);
                }
                self.shift_counter = 0;
                self.shift_register = 0;
            }
        } else if (0x6000..=0x7fff).contains(&addr) {
            if cart.trainer.is_some() && (0x7000..=0x71ff).contains(&addr) {
                let c = cart.trainer.as_mut().unwrap();
                let addr = addr & 0x1ff;
                c[addr as usize] = data;
            } else {
                let mut addr2 = addr & 0x1fff;
                if !cart.prg_ram.is_empty() {
                    addr2 %= cart.prg_ram.len() as u16;
                    cart.prg_ram[addr2 as usize] = data;
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
        if cart.chr_rom.is_empty() {
            return None;
        }
        self.ppu_read(self.ppu_address, cart)
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
                addr3 &= (cart.chr_rom.len() - 1) as u32;
                cart.chr_rom[addr3 as usize] = data;
            }
        }
    }

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
