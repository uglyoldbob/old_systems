//! This module is for manual testing of roms

use serde::{Deserialize, Serialize};

/// Indicates the tested status of a rom
#[derive(Serialize, Deserialize, PartialEq, Clone)]
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
            RomStatus::Working => matches!(s, RomStatus::Working),
            RomStatus::CompletelyBroken => matches!(s, RomStatus::CompletelyBroken),
            RomStatus::Bug(_b, _save) => matches!(s, RomStatus::Bug(_c, _save)),
        }
    }
}

/// A list of roms for the emulator.
#[derive(Serialize, Deserialize, Clone)]
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
    pub fn load_list(mut pb: std::path::PathBuf) -> Self {
        pb.push("testing.bin");
        let contents = std::fs::read(pb);
        if let Err(_e) = contents {
            return RomList::new();
        }
        let contents = contents.unwrap();
        let config = bincode::deserialize(&contents[..]);
        config.ok().unwrap_or(RomList::new())
    }

    /// Save the rom list to disk
    pub fn save_list(&self, mut pb: std::path::PathBuf) -> std::io::Result<()> {
        pb.push("testing.bin");
        let encoded = bincode::serialize(&self).unwrap();
        std::fs::write(pb, encoded)
    }
}

/// A struct for listing and parsing valid roms for the emulator.
#[derive(Clone)]
pub struct RomListTestParser {
    /// The list of roms
    list: RomList,
}

impl RomListTestParser {
    /// Create a new rom list parser object. It loads the file that lists previously parsed roms.
    pub fn new(pb: std::path::PathBuf) -> Self {
        Self {
            list: RomList::load_list(pb),
        }
    }

    /// Returns a reference to the list of roms, for presentation to the user or some other purpose.
    pub fn list(&self) -> &RomList {
        &self.list
    }

    /// Put an entry into the list, over-writing any previously existing entry
    pub fn put_entry(&mut self, hash: String, r: RomStatus, pb: std::path::PathBuf) {
        self.list.elements.insert(hash, r);
        let _e = self.list.save_list(pb);
    }
}
