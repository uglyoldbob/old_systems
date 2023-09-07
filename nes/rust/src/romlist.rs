//! This is responsible for parsing roms from the filesystem, determining which ones are valid for the emulator to load.
//! Emulator inaccuracies may prevent a rom that this module determines to be valid fromm operating correctly.

use crate::cartridge::{CartridgeError, NesCartridge};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Data gathered fromm a successful rom load
#[derive(Serialize, Deserialize)]
pub struct RomListResult {
    pub mapper: u32,
}

/// A single entry for a potentially valid rom for the emulator
#[derive(Serialize, Deserialize)]
pub struct RomListEntry {
    /// Stores whether or not the rom is valid, and what kind of error was encountered.
    pub result: Option<Result<RomListResult, CartridgeError>>,
    /// The time when the rom file was last modified. USed for rechecking mmodified roms.
    pub modified: Option<std::time::SystemTime>,
}

/// A list of roms for the emulator.
#[derive(Serialize, Deserialize)]
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
pub struct RomListParser {
    /// The list of roms
    list: RomList,
    /// True when a scan has been performed on the list of roms.
    scan_complete: bool,
    /// True when all of the roms have been processed.
    update_complete: bool,
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
    pub fn find_roms(&mut self, dir: &str) {
        if !self.scan_complete {
            println!("Searching in {} for roms", dir);
            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| !e.file_type().is_dir())
            {
                let meta = entry.metadata();
                if meta.is_ok() {
                    let m = entry.clone().into_path();
                    let name = m.clone().into_os_string().into_string().unwrap();
                    println!("Checking2 {}", name);
                    if NesCartridge::load_cartridge(name.clone()).is_ok() {
                        println!("{} is ok", name);
                        self.list.elements.entry(m).or_insert_with(|| RomListEntry {
                            result: None,
                            modified: None,
                        });
                    } else {
                        println!("{} NOT GOOD", name);
                    }
                }
            }
            let _e = self.list.save_list();
            self.scan_complete = true;
        }
    }

    /// Responsbile for checking to see if an update has been performed. An update consists of checking to see if any roms have changed since the last scan through the filesystem.
    pub fn process_roms(&mut self) {
        if !self.update_complete {
            for (p, entry) in self.list.elements.iter_mut() {
                let metadata = p.metadata();
                if let Ok(metadata) = metadata {
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                    let last_modified = entry.modified.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if modified > last_modified {
                        println!("Checking {}", p.display());
                        let romcheck = NesCartridge::load_cartridge(
                            p.as_os_str().to_str().unwrap().to_string(),
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
