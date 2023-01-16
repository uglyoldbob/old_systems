use crate::{
    cartridge::NesCartridge,
    cpu::{NesCpuPeripherals, NesMemoryBus},
};

pub struct NesMotherboard {
    cart: Option<NesCartridge>,
    ram: [u8; 2048],
    vram: [u8; 2048],
    ppu_palette_ram: [u8; 32],
    vram_address: Option<u16>,
    last_ppu_cycle: u8,
}

impl NesMotherboard {
    pub fn new() -> Self {
        //board ram is random on startup
        let mut main_ram: [u8; 2048] = [0; 2048];
        for i in main_ram.iter_mut() {
            *i = rand::random();
        }

        let mut vram: [u8; 2048] = [0; 2048];
        for i in vram.iter_mut() {
            *i = rand::random();
        }

        let mut pram: [u8; 32] = [0; 32];
        for i in vram.iter_mut() {
            *i = rand::random();
        }
        Self {
            cart: None,
            ram: main_ram,
            vram: vram,
            ppu_palette_ram: pram,
            vram_address: None,
            last_ppu_cycle: 2,
        }
    }

    pub fn insert_cartridge(&mut self, c: NesCartridge) {
        if let None = self.cart {
            self.cart = Some(c);
        }
    }

    #[cfg(test)]
    pub fn check_vram(&self, addr: u16, check: &[u8]) -> bool {
        for (i, data) in check.iter().enumerate() {
            if self.vram[(addr + i as u16) as usize] != *data {
                return false;
            }
        }
        return true;
    }
}

impl NesMemoryBus for NesMotherboard {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        _out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) -> u8 {
        let mut response: u8 = 0;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = self.ram[addr as usize];
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                if let Some(r) = per.ppu_read(addr) {
                    response = r;
                    if addr == 7 {
                        let a = per.ppu.vram_address();
                        if a >= 0x3f00 {
                            let addr = a & 0x1f;
                            let addr2 = match addr {
                                0x10 => 0,
                                0x14 => 4,
                                0x18 => 8,
                                0x1c => 0xc,
                                _ => addr,
                            };
                            println!("READ PALETTE {:x} as {:x}", addr2, self.ppu_palette_ram[addr2 as usize]);
                            response |= self.ppu_palette_ram[addr2 as usize];
                            per.ppu.increment_vram();
                        }
                    }
                } else {
                    //TODO open bus implementation
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    let resp = cart.memory_read(addr);
                    if let Some(v) = resp {
                        response = v;
                    }
                }
            }
        }
        response
    }
    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        _out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) {
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                self.ram[addr as usize] = data;
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                //ppu registers
                per.ppu_write(addr, data);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                println!("TODO implement functionality {:x}", addr);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
                //println!("TODO implement functionality {:x}", addr);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    cart.memory_write(addr, data);
                }
            }
        }
    }

    fn ppu_cycle_1(&mut self, addr: u16) {
        if self.last_ppu_cycle != 2 {
            println!("ERROR PPU CYCLING a");
        }
        self.last_ppu_cycle = 1;
        if let Some(cart) = &mut self.cart {
            let (a10, vram_enable) = cart.ppu_cycle_1(addr);
            self.vram_address = if !vram_enable {
                if addr >= 0x2000 && addr <= 0x3fff {
                    Some(addr | ((a10 as u16) << 10))
                } else {
                    None
                }
            } else {
                None
            };
        }
    }
    fn ppu_cycle_2_write(&mut self, data: u8) {
        if self.last_ppu_cycle != 1 {
            println!("ERROR PPU CYCLING b");
        }
        self.last_ppu_cycle = 2;
        if let Some(addr) = self.vram_address {
            match addr {
                0..=0x3eff => {
                    let addr2 = addr & 0x7ff;
                    self.vram[addr2 as usize] = data;
                }
                0x3f00..=0x3fff => {
                    let mut addr2 = addr & 0x1F;
                    addr2 = match addr2 {
                        0x10 => 0,
                        0x14 => 4,
                        0x18 => 8,
                        0x1c => 0xc,
                        _ => addr2,
                    };
                    println!("SET PALETTE {:x} to {:x}", addr2, data);
                    self.ppu_palette_ram[addr2 as usize] = data;
                }
                _ => {}
            }
        } else {
            if let Some(cart) = &mut self.cart {
                cart.ppu_cycle_write(data);
            }
        }
    }
    fn ppu_cycle_2_read(&mut self) -> u8 {
        if self.last_ppu_cycle != 1 {
            println!("ERROR PPU CYCLING c");
        }
        self.last_ppu_cycle = 2;
        if let Some(addr) = self.vram_address {
            match addr {
                0..=0x3eff => {
                    let addr2 = addr & 0x7ff;
                    self.vram[addr2 as usize]
                }
                0x3f00..=0x3fff => {
                    let mut addr2 = addr & 0x1F;
                    addr2 = match addr2 {
                        0x10 => 0,
                        0x14 => 4,
                        0x18 => 8,
                        0x1c => 0xc,
                        _ => addr2,
                    };
                    self.ppu_palette_ram[addr2 as usize]
                }
                _ => 42,
            }
        } else if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_read()
        } else {
            42
        }
    }

    fn ppu_palette_read(&self, addr: u16) -> u8 {
        let addr2: usize = (addr as usize) & 0x1f;
        self.ppu_palette_ram[addr2]
    }
}
