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
    zesty_nes::run()
}
