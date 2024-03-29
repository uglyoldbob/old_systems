//! The module containing all of the windows for the emulator

use crate::egui_multiwin_dynamic::tracked_window::{RedrawResponse, TrackedWindow};
use egui_multiwin::egui_glow::EguiGlow;
use egui_multiwin::enum_dispatch::enum_dispatch;
use std::sync::Arc;

pub mod cartridge_dump;
pub mod cartridge_prg_ram_dump;
pub mod configuration;
pub mod controllers;
pub mod cpu_memory_dump_window;
pub mod debug_window;
pub mod genie;
pub mod main;
pub mod network;
pub mod rom_finder;

#[cfg(feature = "rom_status")]
pub mod rom_checker;

/// The list of windows that can exist in the emulator.
#[enum_dispatch(TrackedWindow)]
pub enum Windows {
    Main(crate::windows::main::MainSnesWindow),
    CartridgeDump(crate::windows::cartridge_dump::CartridgeMemoryDumpWindow),
    CartridgePrgRamDump(crate::windows::cartridge_prg_ram_dump::CartridgeMemoryDumpWindow),
    Configuration(crate::windows::configuration::Window),
    Controllers(crate::windows::controllers::Window),
    CpuMemoryDumpWindow(crate::windows::cpu_memory_dump_window::CpuMemoryDumpWindow),
    Debug(crate::windows::debug_window::DebugSnesWindow),
    Genie(crate::windows::genie::Window),
    Network(crate::windows::network::Window),
    RomChecker(crate::windows::rom_checker::Window),
    RomFinder(crate::windows::rom_finder::RomFinder),
}
