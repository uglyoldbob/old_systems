//! This module handles all of the wiring and memory for the Snes system.

use crate::controller::DummyController;
use crate::controller::SnesController;
use crate::controller::SnesControllerTrait;
use crate::ppu::SnesPpu;
use crate::{cartridge::SnesCartridge, cpu::SnesCpuPeripherals};
use serde_with::Bytes;

/// A struct for the Snes motherboard, containing accessories to the main chips.
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesMotherboard {
    /// The cartridge to use in the system
    cart: Option<SnesCartridge>,
    /// The cpu ram
    #[serde_as(as = "Bytes")]
    ram: Box<[u8; 128 * 1024]>,
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
    #[serde(skip)]
    /// The controllers for the system
    controllers: [SnesController; 2],
    /// The speed ratio applied to the emulator
    pub speed_ratio: f32,
}

impl SnesMotherboard {
    /// Create a new Snes motherboard
    pub fn new() -> Self {
        //board ram is random on startup
        let mut main_ram: Box<[u8; 128 * 1024]> = Box::new([0; 128 * 1024]);
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
            controllers: [SnesController::default(), SnesController::default()],
            speed_ratio: 1.0,
        }
    }

    /// Get one of the four possible controllers for the system, cloned
    pub fn get_controller(&self, index: u8) -> SnesController {
        self.controllers[index as usize].clone()
    }

    /// Get one of the four possible controllers, non-mutably
    pub fn get_controller_ref(&mut self, index: u8) -> &SnesController {
        &self.controllers[index as usize]
    }

    /// Get one of the four possible controllers, mutably
    pub fn get_controller_mut(&mut self, index: u8) -> &mut SnesController {
        &mut self.controllers[index as usize]
    }

    /// Set one of the four possible controllers for the system
    pub fn set_controller(&mut self, index: u8, nc: SnesController) {
        // Modify regular two controller setup
        if index < 2 {
            self.controllers[index as usize] = nc;
        }
    }

    /// Return a reference to the cartridge if it exists
    pub fn cartridge(&self) -> Option<&SnesCartridge> {
        self.cart.as_ref()
    }

    /// Return a mutable reference to the cartridge if it exists
    pub fn cartridge_mut(&mut self) -> Option<&mut SnesCartridge> {
        self.cart.as_mut()
    }

    /// Remove any cartridge that may exist in the system.
    pub fn remove_cartridge(&mut self) -> Option<SnesCartridge> {
        self.cart.take()
    }

    /// Insert a cartridge into the system, but only if one is not already present
    pub fn insert_cartridge(&mut self, c: SnesCartridge) {
        if self.cart.is_none() {
            self.cart = Some(c);
        }
    }

    /// Used by testing code for automated testing.
    #[cfg(test)]
    pub fn check_vram(&self, addr: u16, check: &[u8]) -> bool {
        for (i, data) in check.iter().enumerate() {
            if self.vram[(addr + i as u16) as usize] != *data {
                return false;
            }
        }
        true
    }

    /// Signals a change in the three outputs fromm the cpu related to the controllers
    pub fn joy_out_signal(&mut self, out: [bool; 3]) {
        self.controllers[0].parallel_signal(out[0]);
        self.controllers[1].parallel_signal(out[0]);
        //TODO handle expansion port here
    }

    /// Signals a change in signal for the joystick outputs. right true means the right joystick signal. signal is the actual signal level (active level is false).
    pub fn joy_clock_signal(&mut self, right: bool, signal: bool) {
        if !right {
            self.controllers[0].clock(signal);
        } else {
            self.controllers[1].clock(signal);
        }
        //TODO clock expansion port for both left and right
    }

    /// Perform a read operation on the cpu memory bus, but doesn;t have any side effects like a normal read might
    pub fn memory_dump(&self, bank: u8, addr: u16, per: &SnesCpuPeripherals) -> Option<u8> {
        let mut response: Option<u8> = None;
        match (bank, addr) {
            ((0..=0x3f), (0..=0x1fff)) => {
                response = Some(self.ram[addr as usize]);
            }
            ((0x80..=0xbf), (0..=0x1fff)) => {
                response = Some(self.ram[addr as usize]);
            }
            ((0..=0x3f), (0x2100..=0x213f)) => {
                if let Some(r) = per.ppu.memory_cycle_read_a((addr & 0x3f) as u8) {
                    response = Some(r);
                }
            }
            ((0x80..=0xbf), (0x2100..=0x213f)) => {
                if let Some(r) = per.ppu.memory_cycle_read_a((addr & 0x3f) as u8) {
                    response = Some(r);
                }
            }
            ((0x7e..=0x7f), a) => {
                let combined = ((bank as u32 & 1) << 16) | a as u32;
                response = Some(self.ram[combined as usize]);
            }
            _ => {
                if let Some(cart) = &self.cart {
                    response = cart.memory_dump(bank, addr);
                }
            }
        }
        response
    }

    /// Perform a read operation on the cpu memory bus a
    pub fn memory_cycle_read_a(
        &mut self,
        bank: u8,
        addr: u16,
        _controllers: [bool; 2],
        per: &mut SnesCpuPeripherals,
    ) -> u8 {
        let mut response: u8 = self.last_cpu_data;
        match (bank, addr) {
            ((0..=0x3f), (0..=0x1fff)) => {
                response = self.ram[addr as usize];
            }
            ((0x80..=0xbf), (0..=0x1fff)) => {
                response = self.ram[addr as usize];
            }
            ((0..=0x3f), (0x2100..=0x213f)) => {
                if let Some(r) = per.ppu.memory_cycle_read_a((addr & 0x3f) as u8) {
                    response = r;
                }
            }
            ((0x80..=0xbf), (0x2100..=0x213f)) => {
                if let Some(r) = per.ppu.memory_cycle_read_a((addr & 0x3f) as u8) {
                    response = r;
                }
            }
            ((0x7e..=0x7f), a) => {
                let combined = ((bank as u32 & 1) << 16) | a as u32;
                response = self.ram[combined as usize];
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    if let Some(r) = cart.memory_read(bank, addr) {
                        response = r;
                    }
                }
            }
        }
        println!("Read address {:X} {:X} = {:X}", bank, addr, response);
        response
    }

    /// Perform a write operation on the cpu memory bus a
    pub fn memory_cycle_write_a(
        &mut self,
        bank: u8,
        addr: u16,
        data: u8,
        _controllers: [bool; 2],
        per: &mut SnesCpuPeripherals,
    ) {
        self.last_cpu_data = data;
        match (bank, addr) {
            ((0..=0x3f), (0..=0x1fff)) => {
                self.ram[addr as usize] = data;
            }
            ((0x80..=0xbf), (0..=0x1fff)) => {
                self.ram[addr as usize] = data;
            }
            ((0..=0x3f), (0x2100..=0x213f)) => {
                per.ppu.memory_cycle_write_a((addr & 0x3f) as u8, data);
            }
            ((0x80..=0xbf), (0x2100..=0x213f)) => {
                per.ppu.memory_cycle_write_a((addr & 0x3f) as u8, data);
            }
            ((0x7e..=0x7f), a) => {
                let combined = ((bank as u32 & 1) << 16) | a as u32;
                self.ram[combined as usize] = data;
            }
            _ => {}
        }
        println!("Write address {:X} {:X} = {:X}", bank, addr, data);
    }
}
