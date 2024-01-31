use serde::{Deserialize, Serialize};

pub mod audio;
pub mod recording;
pub mod rom_status;
pub mod romlist;
pub mod video;

/// The types of errors that can occur when loading a rom
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CartridgeError {
    /// There can be a filesystem error opening the file
    FsError(String),
    /// It might not be any known type of rom
    InvalidRom,
    /// The rom might be incompatible (unparsed format)
    IncompatibleRom,
    /// The rom might use a mapper that is not yet implemented
    IncompatibleMapper(u32),
    /// The rom might be too short, indicating some bytes got cut off of the end, or that it has been corrupted/modified
    RomTooShort,
    /// The rom has bytes that were not parsed
    RomTooLong,
    /// The cartridge length is not a multiple of 512
    BadLength,
    /// The rom header was not found
    HeaderNotFound,
}

/// A constant that defines when the code was compiled
pub const COMPILE_TIME: &'static str = compile_time::datetime_str!();

/// Returns when the code was compiled
pub fn get_compile_time() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_str(COMPILE_TIME, "%+").unwrap()
}

pub fn convert_hex_to_decimal(d: char) -> u8 {
    match d {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' | 'a' => 10,
        'B' | 'b' => 11,
        'C' | 'c' => 12,
        'D' | 'd' => 13,
        'E' | 'e' => 14,
        'F' | 'f' => 15,
        _ => 0,
    }
}
