//! This is responsible for parsing roms from the filesystem, determining which ones are valid for the emulator to load.
//! Emulator inaccuracies may prevent a rom that this module determines to be valid fromm operating correctly.

use crate::{cartridge::{CartridgeError, NesCartridge}, emulator_data::{NesEmulatorData, LocalEmulatorDataClone}};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Data gathered from a successful rom load
#[derive(Clone, Serialize, Deserialize)]
pub struct RomListResult {
    /// The mapper number of the rom
    pub mapper: u32,
}

/// A single entry for a potentially valid rom for the emulator
#[derive(Clone, Serialize, Deserialize)]
pub struct RomListEntry {
    /// Stores whether or not the rom is valid, and what kind of error was encountered.
    pub result: Option<Result<RomListResult, CartridgeError>>,
    /// The time when the rom file was last modified. USed for rechecking mmodified roms.
    pub modified: Option<std::time::SystemTime>,
}

/// A list of roms for the emulator.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct RomList {
    /// The tree of roms.
    pub elements: std::collections::BTreeMap<PathBuf, RomListEntry>,
}

impl RomList {
    /// Create a new blank list of roms
    fn new() -> Self {
        Self {
            elements: std::collections::BTreeMap::new(),
        }
    }

    /// Returns the quantity of roms that are unknown.
    pub fn get_unknown_quantity(&self) -> u32 {
        let mut quant = 0;
        for (_s, rs) in &self.elements {
            if rs.result.is_none() {
                quant += 1;
            }
        }
        quant
    }

    /// Returns the quantity of roms that are broken
    pub fn get_broken_quantity(&self) -> u32 {
        let mut quant = 0;
        for (s, rs) in &self.elements {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(rom) => {}
                    Err(romerr) => match romerr {
                        CartridgeError::IncompatibleMapper(m) => {}
                        CartridgeError::FsError(e) => {}
                        _ => {
                            quant += 1;
                        }
                    },
                }
            }
        }
        quant
    }

    /// Retrieves the number of roms that are not usable
    pub fn get_bad_quantity(&self) -> u32 {
        let mut quant = 0;
        for (s, rs) in &self.elements {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(rom) => {}
                    Err(romerr) => match romerr {
                        CartridgeError::IncompatibleMapper(m) => {}
                        CartridgeError::FsError(e) => {}
                        CartridgeError::InvalidRom => {}
                        _ => {
                            quant += 1;
                        }
                    },
                }
            }
        }
        quant
    }

    /// Get a mapper count tree. Maps mappernumber to quantity
    pub fn get_mapper_quantity(&self) -> std::collections::BTreeMap<u32, u32> {
        let mut mq = std::collections::BTreeMap::new();
        for (s, rs) in &self.elements {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(rom) => {
                        mq.insert(rom.mapper, mq.get(&rom.mapper).unwrap_or(&0) + 1);
                    }
                    Err(romerr) => match romerr {
                        CartridgeError::IncompatibleMapper(m) => {
                            mq.insert(*m, mq.get(m).unwrap_or(&0) + 1);
                        }
                        _ => {}
                    },
                }
            }
        }
        mq
    }

    /// Load the rom list from disk
    pub fn load_list() -> Self {
        let contents = std::fs::read("./roms.bin");
        if let Err(_e) = contents {
            return RomList::new();
        }
        let contents = contents.unwrap();
        let config = bincode::deserialize(&contents[..]);
        config.ok().unwrap_or(RomList::new())
    }

    /// Save the rom list to disk
    pub fn save_list(&self) -> std::io::Result<()> {
        let encoded = bincode::serialize(&self).unwrap();
        std::fs::write("./roms.bin", encoded)
    }
}

/// A struct for listing and parsing valid roms for the emulator.
#[derive(Clone)]
pub struct RomListParser {
    /// The list of roms
    list: RomList,
    /// True when a scan has been performed on the list of roms.
    scan_complete: bool,
    /// True when all of the roms have been processed.
    update_complete: bool,
}

impl Default for RomListParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RomListParser {
    /// Create a new rom list parser object. It loads the file that lists previously parsed roms.
    pub fn new() -> Self {
        Self {
            list: RomList::load_list(),
            scan_complete: false,
            update_complete: false,
        }
    }

    /// Returns a reference to the list of roms, for presentation to the user or some other purpose.
    pub fn list(&self) -> &RomList {
        &self.list
    }

    /// Performs a recursive search for files in the filesystem. It currently uses all files in the specified roms folder (dir).
    pub fn find_roms(&mut self, dir: &str, sp: &PathBuf) {
        if !self.scan_complete {
            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| !e.file_type().is_dir())
            {
                let meta = entry.metadata();
                if meta.is_ok() {
                    let m = entry.clone().into_path();
                    let name = m.clone().into_os_string().into_string().unwrap();
                    let cart = NesCartridge::load_cartridge(name.clone(), sp);
                    match cart {
                        Ok(_cart) => {
                            self.list.elements.entry(m).or_insert_with(|| RomListEntry {
                                result: None,
                                modified: None,
                            });
                        }
                        Err(e) => {
                            self.list.elements.entry(m).or_insert_with(|| RomListEntry {
                                result: Some(Err(e)),
                                modified: None,
                            });
                        }
                    }
                }
            }
            let _e = self.list.save_list();
            self.scan_complete = true;
        }
    }

    /// Responsbile for checking to see if an update has been performed. An update consists of checking to see if any roms have changed since the last scan through the filesystem.
    pub fn process_roms(&mut self, sp: &PathBuf) {
        if !self.update_complete {
            for (p, entry) in self.list.elements.iter_mut() {
                let metadata = p.metadata();
                if let Ok(metadata) = metadata {
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                    let last_modified = entry.modified.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if modified > last_modified {
                        let romcheck = NesCartridge::load_cartridge(
                            p.as_os_str().to_str().unwrap().to_string(),
                            sp
                        );
                        entry.result = Some(romcheck.map(|i| RomListResult {
                            mapper: i.mappernum(),
                        }));
                        entry.modified = Some(modified);
                    }
                }
            }
            let _e = self.list.save_list();
            self.update_complete = true;
        }
    }
}
