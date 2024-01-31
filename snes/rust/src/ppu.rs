//! The ppu module for the emulator. Responsible for emulating the chip that generates all of the graphics for the snes.

use crate::motherboard::SnesMotherboard;
use common_emulator::video::RgbImage;
use egui_multiwin::egui::Vec2;
use serde_with::Bytes;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// The structure for the snes PPU (picture processing unit)
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesPpu {
    /// The frame data stored in the ppu for being displayed onto the screen later.
    frame_data: Box<RgbImage>,
    /// The frame number of the ppu, used for testing and debugging purposes.
    #[cfg(any(test, feature = "debugger"))]
    frame_number: u64,
    #[cfg(feature = "debugger")]
    /// For debugging pixel generation of the background
    pub bg_debug: Option<(u8, u8)>,
    /// The flag that indicates the end of a frame has occurred. Used for synchronizing frame rate of the emulator.
    frame_end: bool,
    /// The ppu registers
    #[serde_as(as = "Bytes")]
    pub registers: [u8; 64],
    /// The low ram chip
    #[serde_as(as = "Bytes")]
    ram1: [u8; 32768],
    /// The high ram chip
    #[serde_as(as = "Bytes")]
    ram2: [u8; 32768],
}

impl SnesPpu {
    /// Construct a new ppu
    pub fn new() -> Self {
        Self {
            frame_data: Box::new(RgbImage::new(256, 224)),
            #[cfg(any(test, feature = "debugger"))]
            frame_number: 0,
            #[cfg(any(test, feature = "debugger"))]
            bg_debug: None,
            frame_end: false,
            registers: [0; 64],
            ram1: [0; 32768],
            ram2: [0; 32768],
        }
    }

    /// Returns true if the frame has ended. Used for frame rate synchronizing.
    pub fn get_frame_end(&mut self) -> bool {
        let flag = self.frame_end;
        self.frame_end = false;
        flag
    }

    /// Return the frame number of the ppu, mostly used for testing and debugging the ppu
    #[cfg(any(test, feature = "debugger"))]
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Returns a reference to the frame data stored in the ppu.
    pub fn get_frame(&mut self) -> &RgbImage {
        &self.frame_data
    }

    /// Get a backup of the ppu frame
    pub fn backup_frame(&self) -> Box<RgbImage> {
        self.frame_data.clone()
    }

    /// Restore the frame from a backup
    pub fn set_frame(&mut self, f: &RgbImage) {
        self.frame_data = Box::new(f.clone());
    }

    /// Read a register on the ppu
    pub fn memory_cycle_read_a(&self, addr: u8) -> Option<u8> {
        if addr < 0x40 {
            Some(self.registers[addr as usize])
        } else {
            None
        }
    }

    /// Write a register on the ppu
    pub fn memory_cycle_write_a(&mut self, addr: u8, data: u8) {
        self.registers[(addr & 0x3f) as usize] = data;
    }

    /// Run a single clock cycle of the ppu
    pub fn cycle(&mut self, bus: &mut SnesMotherboard) {
        self.frame_end = true;
    }
}

/// The structure for the second snes PPU (picture processing unit)
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesPpu2 {}

impl SnesPpu2 {
    /// Construct the struct
    pub fn new() -> Self {
        Self {}
    }

    /// Read a register on the ppu
    pub fn memory_cycle_read_a(&self, addr: u8) -> Option<u8> {
        None
    }

    /// Write a register on the ppu
    pub fn memory_cycle_write_a(&mut self, addr: u8, data: u8) {}

    /// Run a single clock cycle of the ppu
    pub fn cycle(&mut self, bus: &mut SnesMotherboard) {}
}
