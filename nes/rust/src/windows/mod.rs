//! The module containing all of the windows for the emulator

use crate::{
    egui_multiwin_dynamic::tracked_window::{RedrawResponse, TrackedWindow},
    ppu,
};
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
pub mod pattern_table_dump_window;
pub mod ppu_memory_dump_window;
pub mod rom_finder;
pub mod sprite_dump_window;

#[cfg(feature = "rom_status")]
pub mod rom_checker;

#[enum_dispatch(TrackedWindow)]
pub enum Windows {
    Main(main::MainNesWindow),
    CartridgeDump(cartridge_dump::CartridgeMemoryDumpWindow),
    CartridgePrgRamDump(cartridge_prg_ram_dump::CartridgeMemoryDumpWindow),
    Configuration(configuration::Window),
    Controllers(controllers::Window),
    CpuMemoryDumpWindow(cpu_memory_dump_window::CpuMemoryDumpWindow),
    Debug(debug_window::DebugNesWindow),
    NametableDump(name_table_dump_window::DumpWindow),
    PatternTableDump(pattern_table_dump_window::DumpWindow),
    PpuMemoryDump(ppu_memory_dump_window::PpuMemoryDumpWindow),
    RomChecker(rom_checker::Window),
    RomFinder(rom_finder::RomFinder),
    SpriteDump(sprite_dump_window::DumpWindow),
}
