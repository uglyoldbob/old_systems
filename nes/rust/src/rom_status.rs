//! This module is for manual testing of roms

use crate::cartridge::{CartridgeError, NesCartridge};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Indicates the tested status of a rom
#[derive(Serialize, Deserialize, PartialEq)]
pub enum RomStatus {
    /// The rom is completely unusable
    CompletelyBroken,
    /// There is a bug affecting the rom
    Bug(String, Option<Vec<u8>>),
    /// No known bugs for the rom
    Working,
}

impl RomStatus {
    /// Returns true if the category of both elements are the same
    pub fn match_category(&self, s: &RomStatus) -> bool {
        match self {
            RomStatus::Working => match s {
                RomStatus::Working => true,
                _ => false,
            },
            RomStatus::CompletelyBroken => match s {
                RomStatus::CompletelyBroken => true,
                _ => false,
            },
            RomStatus::Bug(_b, _save) => match s {
                RomStatus::Bug(_c, _save) => true,
                _ => false,
            },
        }
    }
}

/// A list of roms for the emulator.
#[derive(Serialize, Deserialize)]
pub struct RomList {
    /// The tree of roms.
    pub elements: std::collections::BTreeMap<String, RomStatus>,
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
        let contents = std::fs::read("./testing.bin");
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
        std::fs::write("./testing.bin", encoded)
    }
}

/// A struct for listing and parsing valid roms for the emulator.
pub struct RomListTestParser {
    /// The list of roms
    list: RomList,
    /// True when a scan has been performed on the list of roms.
    scan_complete: bool,
    /// True when all of the roms have been processed.
    update_complete: bool,
}

impl Default for RomListTestParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RomListTestParser {
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

    /// Put an entry into the list, over-writing any previously existing entry
    pub fn put_entry(&mut self, hash: String, r: RomStatus) {
        self.list.elements.insert(hash, r);
        let _e = self.list.save_list();
    }
}
