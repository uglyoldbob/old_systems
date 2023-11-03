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
pub mod main;
pub mod name_table_dump_window;
pub mod network;
pub mod pattern_table_dump_window;
pub mod ppu_memory_dump_window;
pub mod rom_finder;
pub mod sprite_dump_window;

#[cfg(feature = "rom_status")]
pub mod rom_checker;

#[enum_dispatch(TrackedWindow)]
pub enum Windows {
    Main(crate::windows::main::MainNesWindow),
    CartridgeDump(crate::windows::cartridge_dump::CartridgeMemoryDumpWindow),
    CartridgePrgRamDump(crate::windows::cartridge_prg_ram_dump::CartridgeMemoryDumpWindow),
    Configuration(crate::windows::configuration::Window),
    Controllers(crate::windows::controllers::Window),
    CpuMemoryDumpWindow(crate::windows::cpu_memory_dump_window::CpuMemoryDumpWindow),
    Debug(crate::windows::debug_window::DebugNesWindow),
    NametableDump(crate::windows::name_table_dump_window::DumpWindow),
    Network(crate::windows::network::Window),
    PatternTableDump(crate::windows::pattern_table_dump_window::DumpWindow),
    PpuMemoryDump(crate::windows::ppu_memory_dump_window::PpuMemoryDumpWindow),
    RomChecker(crate::windows::rom_checker::Window),
    RomFinder(crate::windows::rom_finder::RomFinder),
    SpriteDump(crate::windows::sprite_dump_window::DumpWindow),
}
