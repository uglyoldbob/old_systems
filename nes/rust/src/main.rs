#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! This is the nes emulator written in rust. It is compatible with windows, linux, and osx.

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod genie;
mod motherboard;
mod ppu;

use emulator_data::NesEmulatorData;
use zesty_nes::*;

#[cfg(test)]
mod tests;

mod windows;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    #[cfg(target_os = "windows")]
    {
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .map(std::path::PathBuf::from)
            .ok_or("Could not determine executable directory")
            .unwrap();
        std::env::set_var("GST_PLUGIN_PATH", exe_dir);
    }
    simple_file_logger::init_logger("ZestyNes", simple_file_logger::LogLevel::Info).unwrap();
    let options = eframe::NativeOptions::default();
    zesty_nes::run(options)
}
