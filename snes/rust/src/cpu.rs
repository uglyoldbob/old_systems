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

/// A struct for implementing the snes cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpu {
}