//! This module handles all of the wiring and memory for the nes system.

use crate::controller::DummyController;
use crate::controller::FourScore;
use crate::controller::NesController;
use crate::controller::NesControllerTrait;
use crate::ppu::NesPpu;
use crate::{cartridge::NesCartridge, cpu::NesCpuPeripherals};
use serde_with::Bytes;

/// A struct for the nes motherboard, containing accessories to the main chips.
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesMotherboard {
    /// The cartridge to use in the system
    cart: Option<NesCartridge>,
    /// The cpu ram
    #[serde_as(as = "Bytes")]
    ram: [u8; 2048],
    /// The ppu vram, physically outside the ppu, so this makes perfect sense.
    #[serde_as(as = "Bytes")]
    vram: [u8; 2048],
    /// The palette ram for the ppu, technically belongs in the ppu.
    ppu_palette_ram: [u8; 32],
    /// The vram address fromm the last ppu address cycle
    vram_address: Option<u16>,
    /// Used for detecting sequence problems in the ppu
    last_ppu_cycle: u8,
    /// The last coordinates for ppu access
    last_ppu_coordinates: (u16, u16),
    /// Used for open bus implementation of the cpu memory bus
    last_cpu_data: u8,
    #[serde(skip)]
    /// The controllers for the system
    controllers: [NesController; 2],
}

impl NesMotherboard {
    /// Create a new nes motherboard
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
        for i in pram.iter_mut() {
            *i = rand::random();
        }
        Self {
            cart: None,
            ram: main_ram,
            vram,
            ppu_palette_ram: pram,
            vram_address: None,
            last_ppu_cycle: 2,
            last_cpu_data: 0,
            last_ppu_coordinates: (0, 0),
            controllers: [NesController::default(), NesController::default()],
        }
    }

    /// Get one of the four possible controllers for the system, cloned
    pub fn get_controller(&self, index: u8) -> NesController {
        match index {
            0 | 2 => match &self.controllers[0] {
                NesController::FourScore(fs) => fs.get_controller(index >> 1),
                any => {
                    if index == 0 {
                        any.clone()
                    } else {
                        NesController::DummyController(DummyController::default())
                    }
                }
            },
            _ => match &self.controllers[1] {
                NesController::FourScore(fs) => fs.get_controller(index >> 1),
                any => {
                    if index == 1 {
                        any.clone()
                    } else {
                        NesController::DummyController(DummyController::default())
                    }
                }
            },
        }
    }

    /// Get one of the four possible controllers, mutably
    pub fn get_controller_mut(&mut self, index: u8) -> &mut NesController {
        match index {
            0 | 2 => match &mut self.controllers[0] {
                NesController::FourScore(fs) => fs.get_controller_mut(index >> 1),
                any => any,
            },
            _ => match &mut self.controllers[1] {
                NesController::FourScore(fs) => fs.get_controller_mut(index >> 1),
                any => any,
            },
        }
    }

    /// Set one of the four possible controllers for the system
    pub fn set_controller(&mut self, index: u8, nc: NesController) {
        let p3_current = if let NesController::FourScore(fs) = &self.controllers[0] {
            fs.has_second_controller()
        } else {
            false
        };
        let p4_current = if let NesController::FourScore(fs) = &self.controllers[1] {
            fs.has_second_controller()
        } else {
            false
        };
        let fourscore_current = p3_current || p4_current;
        let fourscore_new = match index {
            0 | 1 => fourscore_current,
            2 => match nc {
                NesController::StandardController(_) => true,
                NesController::Zapper(_) => true,
                NesController::DummyController(_) => p4_current,
                NesController::FourScore(_) => false,
            },
            3 => match nc {
                NesController::StandardController(_) => true,
                NesController::Zapper(_) => true,
                NesController::DummyController(_) => p3_current,
                NesController::FourScore(_) => false,
            },
            _ => fourscore_current,
        };
        if !fourscore_current && fourscore_new {
            //reconfigure the controllers to insert four score adapters
            let mut fs1 = FourScore::default();
            let mut fs2 = FourScore::default();
            fs1.set_controller(0, self.controllers[0].clone());
            fs2.set_controller(0, self.controllers[1].clone());
            if index == 2 {
                fs1.set_controller(1, nc);
            } else if index == 3 {
                fs2.set_controller(1, nc);
            }
            self.controllers[0] = NesController::FourScore(fs1);
            self.controllers[1] = NesController::FourScore(fs2);
        } else if fourscore_current && !fourscore_new {
            //reconfigure the controllers to remove existing four score adapters
            let mut p1 = NesController::DummyController(DummyController::default());
            let mut p2 = NesController::DummyController(DummyController::default());

            if let NesController::FourScore(fs) = &self.controllers[0] {
                p1 = fs.get_controller(0);
            }
            if let NesController::FourScore(fs) = &self.controllers[1] {
                p2 = fs.get_controller(0);
            }
            self.controllers[0] = p1;
            self.controllers[1] = p2;
        } else if fourscore_current {
            // modify existing four score adapters
            match index {
                0 => {
                    if let NesController::FourScore(fs) = &mut self.controllers[0] {
                        fs.set_controller(0, nc);
                    }
                }
                1 => {
                    if let NesController::FourScore(fs) = &mut self.controllers[1] {
                        fs.set_controller(0, nc);
                    }
                }
                2 => {
                    if let NesController::FourScore(fs) = &mut self.controllers[0] {
                        fs.set_controller(1, nc);
                    }
                }
                _ => {
                    if let NesController::FourScore(fs) = &mut self.controllers[1] {
                        fs.set_controller(1, nc);
                    }
                }
            }
        } else {
            // Modify regular two controller setup
            self.controllers[index as usize] = nc;
        }
    }

    /// Return a reference to the cartridge if it exists
    pub fn cartridge(&self) -> Option<&NesCartridge> {
        self.cart.as_ref()
    }

    /// Return a mutable reference to the cartridge if it exists
    pub fn cartridge_mut(&mut self) -> Option<&mut NesCartridge> {
        self.cart.as_mut()
    }

    /// Remove any cartridge that may exist in the system.
    pub fn remove_cartridge(&mut self) -> Option<NesCartridge> {
        self.cart.take()
    }

    /// Insert a cartridge into the system, but only if one is not already present
    pub fn insert_cartridge(&mut self, c: NesCartridge) {
        if self.cart.is_none() {
            self.cart = Some(c);
        }
    }

    /// Used by testing code for automated testing.
    #[cfg(test)]
    pub fn check_vram(&self, addr: u16, check: &[u8]) -> bool {
        for (i, data) in check.iter().enumerate() {
            if self.vram[(addr + i as u16) as usize] != *data {
                return false;
            }
        }
        true
    }

    /// Signals a change in the three outputs fromm the cpu related to the controllers
    pub fn joy_out_signal(&mut self, out: [bool; 3]) {
        self.controllers[0].parallel_signal(out[0]);
        self.controllers[1].parallel_signal(out[0]);
        //TODO handle expansion port here
    }

    /// Signals a change in signal for the joystick outputs. right true means the right joystick signal. signal is the actual signal level (active level is false).
    pub fn joy_clock_signal(&mut self, right: bool, signal: bool) {
        if !right {
            self.controllers[0].clock(signal);
        } else {
            self.controllers[1].clock(signal);
        }
        //TODO clock expansion port for both left and right
    }

    /// Perform a read operation on the cpu memory bus, but doesn;t have any side effects like a normal read might
    pub fn memory_dump(&self, addr: u16, per: &NesCpuPeripherals) -> Option<u8> {
        let mut response: Option<u8> = None;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = Some(self.ram[addr as usize]);
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                if let Some(r) = per.ppu_dump(addr) {
                    response = Some(r);
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
                            let palette_data = self.ppu_palette_ram[addr2 as usize];
                            response = Some(r | palette_data)
                        }
                    }
                } else {
                    //TODO open bus implementation
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                match addr {
                    0x4000..=0x4014 => {}
                    0x4015 => {
                        response = Some(per.apu.dump(addr & 0x1f));
                    }
                    0x4016 => {
                        let d = self.controllers[0].dump_data() & 0x1f;
                        response = Some((d ^ 0x1f) | (self.last_cpu_data & 0xe0));
                    }
                    0x4017 => {
                        let d = self.controllers[1].dump_data() & 0x1f;
                        response = Some((d ^ 0x1f) | (self.last_cpu_data & 0xe0));
                    }
                    _ => {}
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
            }
            _ => {
                if let Some(cart) = &self.cart {
                    let resp = cart.memory_dump(addr);
                    response = resp;
                }
            }
        }
        response
    }

    /// Perform a read operation on the cpu memory bus
    pub fn memory_cycle_read(
        &mut self,
        addr: u16,
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) -> u8 {
        let mut response: u8 = self.last_cpu_data;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = self.ram[addr as usize];
                self.last_cpu_data = response;
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                if let Some(r) = per.ppu_read(addr, &self.ppu_palette_ram) {
                    response = r;
                } else {
                    //TODO open bus implementation
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                match addr {
                    0x4000..=0x4014 => {}
                    0x4015 => {
                        response = per.apu.read(addr & 0x1f);
                        self.last_cpu_data = response;
                    }
                    0x4016 => {
                        let d = self.controllers[0].read_data() & 0x1f;
                        response = (d ^ 0x1f) | (self.last_cpu_data & 0xe0);
                        self.last_cpu_data = response;
                    }
                    0x4017 => {
                        let d = self.controllers[1].read_data() & 0x1f;
                        response = (d ^ 0x1f) | (self.last_cpu_data & 0xe0);
                        self.last_cpu_data = response;
                    }
                    _ => {}
                }
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
                        self.last_cpu_data = v;
                    }
                }
            }
        }
        response
    }

    /// Perform a write operation on the cpu memory bus
    pub fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) {
        self.last_cpu_data = data;
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
                per.ppu_write(addr, data, &mut self.ppu_palette_ram);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                match addr {
                    0x4014 => {}
                    _ => {
                        per.apu.write(addr & 0x1f, data);
                    }
                }
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and io functionality
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

    /// Performs a non-modifying ppu read
    pub fn ppu_peek(&self, addr: u16) -> u8 {
        if let Some(cart) = &self.cart {
            let (a10, vram_enable, data) = cart.ppu_peek_1(addr);
            let vram_address = if !vram_enable {
                if (0x2000..=0x3fff).contains(&addr) {
                    Some(addr | ((a10 as u16) << 10))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(addr) = vram_address {
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
            } else if let Some(a) = data {
                a
            } else {
                42
            }
        } else {
            0
        }
    }

    /// Perform the address part of a ppu memory cycle
    pub fn ppu_cycle_1(&mut self, addr: u16, ppu: &NesPpu) {
        if self.last_ppu_cycle != 2 {
            println!(
                "ERROR PPU CYCLING a @ {},{} from {:?}",
                ppu.column(),
                ppu.row(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.column(), ppu.row());
        self.last_ppu_cycle = 1;
        if let Some(cart) = &mut self.cart {
            let (a10, vram_enable) = cart.ppu_cycle_1(addr);
            let addr = addr & !0x400;
            self.vram_address = if !vram_enable {
                if (0x2000..=0x3fff).contains(&addr) {
                    Some(addr | ((a10 as u16) << 10))
                } else {
                    None
                }
            } else {
                None
            };
        }
    }

    /// Perform the write portion of a ppu memory cycle
    pub fn ppu_cycle_2_write(&mut self, data: u8, ppu: &NesPpu) {
        if self.last_ppu_cycle != 1 {
            println!(
                "ERROR PPU CYCLING b @ {},{} from {:?}",
                ppu.column(),
                ppu.row(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.column(), ppu.row());
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
                    self.ppu_palette_ram[addr2 as usize] = data;
                }
                _ => {}
            }
        } else if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_write(data);
        }
    }

    /// Perform the read portion of a ppu memory cycle
    pub fn ppu_cycle_2_read(&mut self, ppu: &NesPpu) -> u8 {
        if self.last_ppu_cycle != 1 {
            println!(
                "ERROR PPU CYCLING c @ {},{} from {:?}",
                ppu.column(),
                ppu.row(),
                self.last_ppu_coordinates
            );
        }
        self.last_ppu_coordinates = (ppu.column(), ppu.row());
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

    /// Read a palette address
    pub fn ppu_palette_read(&self, addr: u16) -> u8 {
        let addr2: usize = (addr as usize) & 0x1f;
        self.ppu_palette_ram[addr2]
    }
}
