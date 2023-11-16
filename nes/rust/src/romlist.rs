//! This is responsible for parsing roms from the filesystem, determining which ones are valid for the emulator to load.
//! Emulator inaccuracies may prevent a rom that this module determines to be valid fromm operating correctly.

use crate::cartridge::{CartridgeError, NesCartridge};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

#[derive(Clone, Serialize, Deserialize, PartialEq, strum::EnumIter)]
/// The ranking system for roms, consists of 5 hate, 5 love, and one neutral ranking
pub enum RomRanking {
    /// 5 positive stars
    Love5,
    /// 4 positive stars
    Love4,
    /// 3 positive stars
    Love3,
    /// 2 positive stars
    Love2,
    /// 1 positive star
    Love1,
    /// 0 stars, (default)
    Neutral,
    /// 1 negative star
    Hate1,
    /// 2 negative stars
    Hate2,
    /// 3 negative stars
    Hate3,
    /// 4 negative stars
    Hate4,
    /// 5 negative stars
    Hate5,
}

impl std::fmt::Display for RomRanking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RomRanking::Love5 => "5 Stars Love",
            RomRanking::Love4 => "4 Stars Love",
            RomRanking::Love3 => "3 Stars Love",
            RomRanking::Love2 => "2 Stars Love",
            RomRanking::Love1 => "1 Star Love",
            RomRanking::Neutral => "Unranked",
            RomRanking::Hate1 => "1 star Hate",
            RomRanking::Hate2 => "2 stars Hate",
            RomRanking::Hate3 => "3 stars Hate",
            RomRanking::Hate4 => "4 stars Hate",
            RomRanking::Hate5 => "5 stars Hate",
        })
    }
}

impl RomRanking {
    /// Increase ranking by one star
    pub fn increase(&mut self) {
        *self = match self {
            RomRanking::Love5 => RomRanking::Love5,
            RomRanking::Love4 => RomRanking::Love5,
            RomRanking::Love3 => RomRanking::Love4,
            RomRanking::Love2 => RomRanking::Love3,
            RomRanking::Love1 => RomRanking::Love2,
            RomRanking::Neutral => RomRanking::Love1,
            RomRanking::Hate1 => RomRanking::Neutral,
            RomRanking::Hate2 => RomRanking::Hate1,
            RomRanking::Hate3 => RomRanking::Hate2,
            RomRanking::Hate4 => RomRanking::Hate3,
            RomRanking::Hate5 => RomRanking::Hate4,
        }
    }

    /// Decrease ranking by one star
    pub fn decrease(&mut self) {
        *self = match self {
            RomRanking::Love5 => RomRanking::Love4,
            RomRanking::Love4 => RomRanking::Love3,
            RomRanking::Love3 => RomRanking::Love2,
            RomRanking::Love2 => RomRanking::Love1,
            RomRanking::Love1 => RomRanking::Neutral,
            RomRanking::Neutral => RomRanking::Hate1,
            RomRanking::Hate1 => RomRanking::Hate2,
            RomRanking::Hate2 => RomRanking::Hate3,
            RomRanking::Hate3 => RomRanking::Hate4,
            RomRanking::Hate4 => RomRanking::Hate5,
            RomRanking::Hate5 => RomRanking::Hate5,
        }
    }
}

/// Data gathered from a successful rom load
#[derive(Clone, Serialize, Deserialize)]
pub struct RomListResult {
    /// The mapper number of the rom
    pub mapper: u32,
    /// The ranking for the rom, higher is better. Negative is worse.
    pub ranking: RomRanking,
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
        for rs in self.elements.values() {
            if rs.result.is_none() {
                quant += 1;
            }
        }
        quant
    }

    /// Returns the quantity of roms that are broken
    pub fn get_broken_quantity(&self) -> u32 {
        let mut quant = 0;
        for rs in self.elements.values() {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(_rom) => {}
                    Err(romerr) => match romerr {
                        CartridgeError::IncompatibleMapper(_m) => {}
                        CartridgeError::FsError(_e) => {}
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
        for rs in self.elements.values() {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(_rom) => {}
                    Err(romerr) => match romerr {
                        CartridgeError::IncompatibleMapper(_m) => {}
                        CartridgeError::FsError(_e) => {}
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
        for rs in self.elements.values() {
            if let Some(rs) = &rs.result {
                match rs {
                    Ok(rom) => {
                        mq.insert(rom.mapper, mq.get(&rom.mapper).unwrap_or(&0) + 1);
                    }
                    Err(romerr) => {
                        if let CartridgeError::IncompatibleMapper(m) = romerr {
                            mq.insert(*m, mq.get(m).unwrap_or(&0) + 1);
                        }
                    }
                }
            }
        }
        mq
    }

    /// Load the rom list from disk
    pub fn load_list(mut pb: PathBuf) -> Self {
        pb.push("roms.bin");
        println!("Load actual rom list from {}", pb.display());
        let contents = std::fs::read(pb);
        if let Err(e) = contents {
            println!("Error loading list {:?}, getting new list", e);
            return RomList::new();
        }
        let contents = contents.unwrap();
        let config: Result<RomList, Box<bincode::ErrorKind>> = bincode::deserialize(&contents[..]);
        match config {
            Ok(list) => {
                println!("There are {} entries in the rom list", list.elements.len());
                list
            }
            Err(e) => {
                println!("Error deserializing list: {:?}", e);
                RomList::new()
            }
        }
    }

    /// Save the rom list to disk
    pub fn save_list(&self, mut pb: PathBuf) -> std::io::Result<()> {
        pb.push("roms.bin");
        println!("Save list to {}", pb.display());
        let encoded = bincode::serialize(&self).unwrap();
        std::fs::write(pb, encoded)
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
        Self::new(PathBuf::from_str("./").unwrap())
    }
}

impl RomListParser {
    /// Create a new rom list parser object. It loads the file that lists previously parsed roms.
    pub fn new(pb: PathBuf) -> Self {
        println!("Load romlist from {}", pb.display());
        Self {
            list: RomList::load_list(pb),
            scan_complete: false,
            update_complete: false,
        }
    }

    /// Returns a reference to the list of roms, for presentation to the user or some other purpose.
    pub fn list(&self) -> &RomList {
        &self.list
    }

    /// Returns a mutable reference to the list of roms.
    pub fn list_mut(&mut self) -> &mut RomList {
        &mut self.list
    }

    /// Performs a recursive search for files in the filesystem. It currently uses all files in the specified roms folder (dir).
    pub fn find_roms(&mut self, dir: &str, sp: PathBuf, bin: PathBuf) {
        if !self.scan_complete {
            println!(
                "There are {} roms currently in the list",
                self.list.elements.len()
            );
            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| !e.file_type().is_dir())
            {
                let meta = entry.metadata();
                if meta.is_ok() {
                    let m = entry.clone().into_path();
                    let name = m.clone().into_os_string().into_string().unwrap();
                    let cart = NesCartridge::load_cartridge(name.clone(), &sp);
                    match cart {
                        Ok(_cart) => {
                            let e = self.list.elements.entry(m.clone());
                            e.or_insert_with(|| {
                                println!("inserting new success entry {}", m.display());
                                RomListEntry {
                                    result: None,
                                    modified: None,
                                }
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
            let _e = self.list.save_list(bin);
            self.scan_complete = true;
        }
    }

    /// Responsbile for checking to see if an update has been performed. An update consists of checking to see if any roms have changed since the last scan through the filesystem.
    pub fn process_roms(&mut self, sp: PathBuf) {
        if !self.update_complete {
            for (p, entry) in self.list.elements.iter_mut() {
                let metadata = p.metadata();
                if let Ok(metadata) = metadata {
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                    let last_modified = entry.modified.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if modified > last_modified {
                        let romcheck = NesCartridge::load_cartridge(
                            p.as_os_str().to_str().unwrap().to_string(),
                            &sp,
                        );
                        entry.result = Some(romcheck.map(|i| RomListResult {
                            mapper: i.mappernum(),
                            ranking: RomRanking::Neutral,
                        }));
                        entry.modified = Some(modified);
                    }
                }
            }
            let _e = self.list.save_list(sp);
            self.update_complete = true;
        }
    }
}
