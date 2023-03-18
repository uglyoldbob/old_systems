use crate::cartridge::CartridgeError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct RomListEntry {
    pub result: Option<Result<(), CartridgeError>>,
    pub modified: Option<std::time::SystemTime>,
}

#[derive(Serialize, Deserialize)]
pub struct RomList {
    pub elements: std::collections::BTreeMap<PathBuf, RomListEntry>,
}

impl RomList {
    fn new() -> Self {
        Self {
            elements: std::collections::BTreeMap::new(),
        }
    }

    pub fn load_list() -> Self {
        let contents = std::fs::read("./roms.bin");
        if let Err(e) = contents {
            return RomList::new();
        }
        let contents = contents.unwrap();
        let config = bincode::deserialize(&contents[..]);
        config.ok().unwrap_or(RomList::new())
    }

    pub fn save_list(&self) -> std::io::Result<()> {
        let encoded = bincode::serialize(&self).unwrap();
        std::fs::write("./roms.bin", encoded)
    }
}
