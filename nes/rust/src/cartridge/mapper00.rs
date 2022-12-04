use crate::cartridge::NesMapper;

pub struct Mapper {}

impl Mapper {
    pub fn new() -> Box<dyn NesMapper> {
        Box::new(Self {})
    }
}

impl NesMapper for Mapper {}
