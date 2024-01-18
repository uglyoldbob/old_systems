//! This module is responsible for emulating the cpu of the snes.

use crate::apu::SnesApu;
use crate::motherboard::SnesMotherboard;
use crate::ppu::SnesPpu;

/// The peripherals for the cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpuPeripherals {
    /// The ppu for the nes system
    pub ppu: SnesPpu,
    /// The apu for the nes system
    pub apu: SnesApu,
}

impl SnesCpuPeripherals {
    /// Build a set of cpu peripherals
    pub fn new(ppu: SnesPpu, apu: SnesApu) -> Self {
        Self {
            ppu,
            apu,
        }
    }

    /// reset the ppu
    pub fn ppu_reset(&mut self) {

    }

    /// Run a ppu cycle
    pub fn ppu_cycle(&mut self, mb: &mut SnesMotherboard) {

    }

    /// Return the ppu frame number
    pub fn ppu_frame_number(&self) -> u64 {
        self.ppu.frame_number()
    }

    /// Has the current ppu frame ended?
    pub fn ppu_frame_end(&mut self) -> bool {
        self.ppu.get_frame_end()
    }

    /// Returns a reference to the frame data for the ppu
    pub fn ppu_get_frame(&mut self) -> &crate::ppu::RgbImage {
        self.ppu.get_frame()
    }
}

#[cfg(feature = "debugger")]
#[derive(serde::Serialize, serde::Deserialize)]
/// Stores the state of the cpu at the debugger point.
/// Single byte instructions make debugging without this weird, because the instruction has already taken effect
/// by the time the debugger is presenting the information
pub struct SnesCpuDebuggerPoint {
    /// The a register
    pub a: u8,
    /// The x register
    pub x: u8,
    /// The y register
    pub y: u8,
    /// The stack register
    pub s: u8,
    /// The flags register
    pub p: u8,
    /// The program counter
    pub pc: u16,
    /// The string that corresponds to the disassembly for the most recently fetched instruction
    pub disassembly: String,
}

impl SnesCpuDebuggerPoint {
    /// Build a new point
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0xfd,
            p: 0,
            pc: 0xfffc,
            disassembly: "RESET".to_string(),
        }
    }
}

/// A struct for implementing the snes cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpu {
    /// A list of breakpoints for the cpu
    #[cfg(feature = "debugger")]
    pub breakpoints: Vec<u16>,
    /// The debugger information
    #[cfg(feature = "debugger")]
    pub debugger: SnesCpuDebuggerPoint,
    /// True when the last byte of an instruction has been fetched
    #[cfg(feature = "debugger")]
    done_fetching: bool,
    /// The portion of an instruction currently being executed
    subcycle: u8,
}

impl SnesCpu {
    /// Build a new cpu
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "debugger")]
            breakpoints: Vec::new(),
            #[cfg(feature = "debugger")]
            debugger: SnesCpuDebuggerPoint::new(),
            #[cfg(feature = "debugger")]
            done_fetching: false,
            subcycle: 0,
        }
    }

    /// Reset the cpu
    pub fn reset(&mut self) {
        
    }

    /// Returns true when done fetching all bytes for an instruction.
    #[cfg(feature = "debugger")]
    pub fn breakpoint_option(&self) -> bool {
        self.done_fetching
    }

    /// Returns true when a breakpoint is active
    pub fn breakpoint(&self) -> bool {
        let mut b = false;
        if self.done_fetching {
            for v in &self.breakpoints {
                if self.debugger.pc == *v {
                    println!("subcycle for breakpoint is {}", self.subcycle);
                    b = true;
                }
            }
        }
        b
    }

    /// Show the disassembly of the current instruction
    #[cfg(feature = "debugger")]
    pub fn disassemble(&self) -> Option<String> {
        Some(self.debugger.disassembly.to_owned())
    }

    /// Run a single cycle of the cpu
    pub fn cycle(
        &mut self,
        bus: &mut SnesMotherboard,
        cpu_peripherals: &mut SnesCpuPeripherals,
        nmi: bool,
        irq: bool,
    ) {
    }
}