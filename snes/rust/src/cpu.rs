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
}

/// A struct for implementing the snes cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpu {
    /// A list of breakpoints for the cpu
    #[cfg(feature = "debugger")]
    pub breakpoints: Vec<u16>,
}

impl SnesCpu {
    /// Build a new cpu
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "debugger")]
            breakpoints: Vec::new(),
        }
    }

    /// Reset the cpu
    pub fn reset(&mut self) {
        
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