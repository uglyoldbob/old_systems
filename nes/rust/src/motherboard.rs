//! This module handles all of the wiring and memory for the nes system.

use crate::controller::NesController;
use crate::controller::NesControllerTrait;
use crate::ppu::NesPpu;
use crate::{cartridge::NesCartridge, cpu::NesCpuPeripherals};
use serde_with::Bytes;

/// A struct for the nes motherboard, containing accessories to the main chips.
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesMotherboard {
    /// The cartridge to use in the system
    cart: Option<NesCartridge>,
    /// The cpu ram
    #[serde_as(as = "Bytes")]
    ram: [u8; 2048],
    /// The ppu vram, physically outside the ppu, so this makes perfect sense.
    #[serde_as(as = "Bytes")]
    vram: [u8; 2048],
    /// The palette ram for the ppu, technically belongs in the ppu.
    ppu_palette_ram: [u8; 32],
    /// The vram address fromm the last ppu address cycle
    vram_address: Option<u16>,
    /// Used for detecting sequence problems in the ppu
    last_ppu_cycle: u8,
    /// The last coordinates for ppu access
    last_ppu_coordinates: (u16, u16),
    /// Used for open bus implementation of the cpu memory bus
    last_cpu_data: u8,
    /// The controllers for the system
    pub controllers: [Option<NesController>; 2],
}

impl NesMotherboard {
    /// Create a new nes motherboard
    pub fn new() -> Self {
        //board ram is random on startup
        let mut main_ram: [u8; 2048] = [0; 2048];
        for i in main_ram.iter_mut() {
            *i = rand::random();
        }

        let mut vram: [u8; 2048] = [0; 2048];
        for i in vram.iter_mut() {
            *i = rand::random();
        }

        let mut pram: [u8; 32] = [0; 32];
        for i in pram.iter_mut() {
            *i = rand::random();
        }
        Self {
            cart: None,
            ram: main_ram,
            vram,
            ppu_palette_ram: pram,
            vram_address: None,
            last_ppu_cycle: 2,
            last_cpu_data: 0,
            last_ppu_coordinates: (0, 0),
            controllers: [None, None],
        }
    }

    /// Return a reference to the cartridge if it exists
    pub fn cartridge(&self) -> Option<&NesCartridge> {
        if let Some(c) = &self.cart {
            Some(c)
        } else {
            None
        }
    }

    /// Remove any cartridge that may exist in the system.
    pub fn remove_cartridge(&mut self) -> Option<NesCartridge> {
        self.cart.take()
    }

    /// Insert a cartridge into the system, but only if one is not already present
    pub fn insert_cartridge(&mut self, c: NesCartridge) {
        if self.cart.is_none() {
            self.cart = Some(c);
        }
    }

    /// Insert a controller into the controller 1 port, removing any existing controller
    pub fn insert_controller1(&mut self, c: NesController) {
        self.controllers[0] = Some(c);
    }

    /// Insert a controller into the controller 2 port, removing any existing controller
    pub fn insert_controller2(&mut self, c: NesController) {
        self.controllers[1] = Some(c);
    }

    /// Used by testing code for automated testing.
    #[cfg(test)]
    pub fn check_vram(&self, addr: u16, check: &[u8]) -> bool {
        for (i, data) in check.iter().enumerate() {
            if self.vram[(addr + i as u16) as usize] != *data {
                return false;
            }
        }
        return true;
    }

    /// Perform a read operation on the cpu memory bus, but doesn;t have any side effects like a normal read might
    pub fn memory_dump(&self, addr: u16, per: &NesCpuPeripherals) -> Option<u8> {
        let mut response: Option<u8> = None;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = Some(self.ram[addr as usize]);
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                if let Some(r) = per.ppu_dump(addr) {
                    response = Some(r);
                    if addr == 7 {
                        let a = per.ppu.vram_address();
                        if a >= 0x3f00 {
                            let addr = a & 0x1f;
                            let addr2 = match addr {
                                0x10 => 0,
                                0x14 => 4,
                                0x18 => 8,
                                0x1c => 0xc,
                                _ => addr,
                            };
                            let palette_data = self.ppu_palette_ram[addr2 as usize];
                            response = Some(r | palette_data)
                        }
                    }
                } else {
                    //TODO open bus implementation
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                match addr {
                    0x4000..=0x4014 => {}
                    0x4015 => {
                        response = Some(per.apu.dump(addr & 0x1f));
                    }
                    0x4016 => {
                        if let Some(c) = &self.controllers[0] {
                            let d = c.dump_data() & 0x1f;
                            response = Some((d ^ 0x1f) | (self.last_cpu_data & 0xe0));
                        } else {
                            response = None;
                        }
                    }
                    0x4017 => {
                        if let Some(c) = &self.controllers[1] {
                            let d = c.dump_data() & 0x1f;
                            response = Some((d ^ 0x1f) | (self.last_cpu_data & 0xe0));
                        } else {
                            response = None;
                        }
                    }
                    _ => {}
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
            }
            _ => {
                if let Some(cart) = &self.cart {
                    let resp = cart.memory_dump(addr);
                    response = resp;
                }
            }
        }
        response
    }

    /// Perform a read operation on the cpu memory bus
    pub fn memory_cycle_read(
        &mut self,
        addr: u16,
        _out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) -> u8 {
        let mut response: u8 = self.last_cpu_data;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = self.ram[addr as usize];
                self.last_cpu_data = response;
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                if let Some(r) = per.ppu_read(addr, &self.ppu_palette_ram) {
                    response = r;
                } else {
                    //TODO open bus implementation
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                match addr {
                    0x4000..=0x4014 => {}
                    0x4015 => {
                        response = per.apu.read(addr & 0x1f);
                        self.last_cpu_data = response;
                    }
                    0x4016 => {
                        if let Some(c) = &mut self.controllers[0] {
                            let d = c.read_data() & 0x1f;
                            response = (d ^ 0x1f) | (self.last_cpu_data & 0xe0);
                        } else {
                            response = self.last_cpu_data & 0xe0;
                        }
                        self.last_cpu_data = response;
                    }
                    0x4017 => {
                        if let Some(c) = &mut self.controllers[1] {
                            let d = c.read_data() & 0x1f;
                            response = (d ^ 0x1f) | (self.last_cpu_data & 0xe0);
                        } else {
                            response = self.last_cpu_data & 0xe0;
                        }
                        self.last_cpu_data = response;
                    }
                    _ => {}
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    let resp = cart.memory_read(addr);
                    if let Some(v) = resp {
                        response = v;
                        self.last_cpu_data = v;
                    }
                }
            }
        }
        response
    }

    /// Perform a write operation on the cpu memory bus
    pub fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) {
        self.last_cpu_data = data;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                self.ram[addr as usize] = data;
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                //ppu registers
                per.ppu_write(addr, data, &mut self.ppu_palette_ram);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                if addr == 0x4016 {
                    for mut c in &mut self.controllers {
                        if let Some(con) = &mut c {
                            con.update_latch_bits(out);
                        }
                    }
                }
                match addr {
                    0x4014 => {}
                    _ => {
                        per.apu.write(addr & 0x1f, data);
                    }
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and io functionality
                //test mode
                //println!("TODO implement functionality {:x}", addr);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    cart.memory_write(addr, data);
                }
            }
        }
    }

    /// Performs a non-modifying ppu read
    pub fn ppu_peek(&self, addr: u16) -> u8 {
        if let Some(cart) = &self.cart {
            let (a10, vram_enable, data) = cart.ppu_peek_1(addr);
            let vram_address = if !vram_enable {
                if (0x2000..=0x3fff).contains(&addr) {
                    Some(addr | ((a10 as u16) << 10))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(addr) = vram_address {
                match addr {
                    0..=0x3eff => {
                        let addr2 = addr & 0x7ff;
                        self.vram[addr2 as usize]
                    }
                    0x3f00..=0x3fff => {
                        let mut addr2 = addr & 0x1F;
                        addr2 = match addr2 {
                            0x10 => 0,
                            0x14 => 4,
                            0x18 => 8,
                            0x1c => 0xc,
                            _ => addr2,
                        };
                        self.ppu_palette_ram[addr2 as usize]
                    }
                    _ => 42,
                }
            } else if let Some(cart) = &self.cart {
                if let Some(a) = data {
                    a
                } else {
                    42
                }
            } else {
                41
            }
        } else {
            0
        }
    }

    /// Perform the address part of a ppu memory cycle
    pub fn ppu_cycle_1(&mut self, addr: u16, ppu: &NesPpu) {
        if self.last_ppu_cycle != 2 {
            println!(
                "ERROR PPU CYCLING a @ {},{} from {:?}",
                ppu.row(),
                ppu.column(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.row(), ppu.column());
        self.last_ppu_cycle = 1;
        if let Some(cart) = &mut self.cart {
            let (a10, vram_enable) = cart.ppu_cycle_1(addr);
            self.vram_address = if !vram_enable {
                if (0x2000..=0x3fff).contains(&addr) {
                    Some(addr | ((a10 as u16) << 10))
                } else {
                    None
                }
            } else {
                None
            };
        }
    }

    /// Perform the write portion of a ppu memory cycle
    pub fn ppu_cycle_2_write(&mut self, data: u8, ppu: &NesPpu) {
        if self.last_ppu_cycle != 1 {
            println!(
                "ERROR PPU CYCLING b @ {},{} from {:?}",
                ppu.row(),
                ppu.column(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.row(), ppu.column());
        self.last_ppu_cycle = 2;
        if let Some(addr) = self.vram_address {
            match addr {
                0..=0x3eff => {
                    let addr2 = addr & 0x7ff;
                    self.vram[addr2 as usize] = data;
                }
                0x3f00..=0x3fff => {
                    let mut addr2 = addr & 0x1F;
                    addr2 = match addr2 {
                        0x10 => 0,
                        0x14 => 4,
                        0x18 => 8,
                        0x1c => 0xc,
                        _ => addr2,
                    };
                    self.ppu_palette_ram[addr2 as usize] = data;
                }
                _ => {}
            }
        } else if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_write(data);
        }
    }

    /// Perform the read portion of a ppu memory cycle
    pub fn ppu_cycle_2_read(&mut self, ppu: &NesPpu) -> u8 {
        if self.last_ppu_cycle != 1 {
            println!(
                "ERROR PPU CYCLING c @ {},{} from {:?}",
                ppu.row(),
                ppu.column(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.row(), ppu.column());
        self.last_ppu_cycle = 2;
        if let Some(addr) = self.vram_address {
            match addr {
                0..=0x3eff => {
                    let addr2 = addr & 0x7ff;
                    self.vram[addr2 as usize]
                }
                0x3f00..=0x3fff => {
                    let mut addr2 = addr & 0x1F;
                    addr2 = match addr2 {
                        0x10 => 0,
                        0x14 => 4,
                        0x18 => 8,
                        0x1c => 0xc,
                        _ => addr2,
                    };
                    self.ppu_palette_ram[addr2 as usize]
                }
                _ => 42,
            }
        } else if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_read()
        } else {
            42
        }
    }

    /// Read a palette address
    pub fn ppu_palette_read(&self, addr: u16) -> u8 {
        let addr2: usize = (addr as usize) & 0x1f;
        self.ppu_palette_ram[addr2]
    }
}
