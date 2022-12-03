pub trait NesMemoryBusDevice {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        out: [bool; 3],
        controllers: [bool; 2],
    ) -> Option<u8>;
    fn memory_cycle_write(&mut self, addr: u16, data: u8, out: [bool; 3], controllers: [bool; 2]);
}

pub struct NesCartridge {
    trainer: Option<[u8; 512]>,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    inst_rom: Option<[u8; 8192]>,
    prom: Option<([u8; 16], [u8; 16])>,
}

#[derive(Debug)]
pub enum CartridgeError {}

impl NesCartridge {
    pub fn load_cartridge(name: String) -> Result<Self, CartridgeError> {
        let rom_contents = std::fs::read(name);
        Ok(Self {
            trainer: None,
            prg_rom: Vec::new(),
            chr_rom: Vec::new(),
            inst_rom: None,
            prom: None,
        })
    }
}
