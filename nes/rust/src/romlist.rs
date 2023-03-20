use crate::cartridge::{CartridgeError, NesCartridge};
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
        if let Err(_e) = contents {
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

pub struct RomListParser {
    list: RomList,
    scan_complete: bool,
    update_complete: bool,
}

impl RomListParser {
    pub fn new() -> Self {
        Self {
            list: RomList::load_list(),
            scan_complete: false,
            update_complete: false,
        }
    }

    pub fn list(&self) -> &RomList {
        &self.list
    }

    pub fn count(&self) -> usize {
        self.list.elements.len()
    }

    pub fn find_roms(&mut self, dir: &str) {
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
                    if NesCartridge::load_cartridge(name).is_ok() {
                        if !self.list.elements.contains_key(&m) {
                            let new_entry = RomListEntry {
                                result: None,
                                modified: None,
                            };
                            self.list.elements.insert(m, new_entry);
                        }
                    }
                }
            }
            let _e = self.list.save_list();
            self.scan_complete = true;
        }
    }

    pub fn process_roms(&mut self) {
        if !self.update_complete {
            for (p, entry) in self.list.elements.iter_mut() {
                let metadata = p.metadata();
                if let Ok(metadata) = metadata {
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                    let last_modified = entry.modified.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if modified > last_modified {
                        let romcheck = NesCartridge::load_cartridge(
                            p.as_os_str().to_str().unwrap().to_string(),
                        );
                        entry.result = Some(romcheck.map(|_i| ()));
                        entry.modified = Some(modified);
                    }
                }
            }
            let _e = self.list.save_list();
            self.update_complete = true;
        }
    }
}
