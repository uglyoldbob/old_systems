//! The module containing all of the windows for the emulator

#[cfg(feature = "debugger")]
pub mod cartridge_dump;
#[cfg(feature = "debugger")]
pub mod cartridge_prg_ram_dump;
pub mod configuration;
pub mod controllers;
#[cfg(feature = "debugger")]
pub mod cpu_memory_dump_window;
#[cfg(feature = "debugger")]
pub mod debug_window;
pub mod genie;
pub mod main;
#[cfg(feature = "debugger")]
pub mod name_table_dump_window;
#[cfg(not(target_os = "android"))]
pub mod network;
#[cfg(feature = "debugger")]
pub mod pattern_table_dump_window;
#[cfg(feature = "debugger")]
pub mod ppu_memory_dump_window;
pub mod rom_finder;
#[cfg(feature = "debugger")]
pub mod sprite_dump_window;

#[cfg(feature = "rom_status")]
pub mod rom_checker;
