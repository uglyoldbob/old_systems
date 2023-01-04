use crate::NesMemoryBus;

pub struct NesPpu {
    registers: [u8; 8],
    scanline_number: u16,
    scanline_cycle: u16,
    vblank: bool,
    vblank_nmi: bool,
    frame_odd: bool,
    cycle: u32,
    nametable_counter: u16,
}

impl NesPpu {
    pub fn new() -> Self {
        let reg2: u8 = rand::random::<u8>() & !0x40;
        Self {
            scanline_number: 0,
            scanline_cycle: 0,
            registers: [0, 0, reg2, 0, 0, 0, 0, 0],
            vblank: false,
            vblank_nmi: false,
            frame_odd: false,
            cycle: 0,
            nametable_counter: 0,
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        42
    }

    pub fn write(&mut self, addr: u16, data: u8) {}

    fn increment_scanline_cycle(&mut self) {
        self.scanline_cycle += 1;
        if self.scanline_cycle == 340 {
            self.scanline_cycle = 0;
            self.scanline_number += 1;
        }
        if self.scanline_number == 262 {
            self.scanline_number = 0;
        }
    }

    fn nametable_base(&self) -> u16 {
        match self.registers[0] & 3 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            _ => 0x2c00,
        }
    }

    pub fn cycle(&mut self, bus: &mut dyn NesMemoryBus) {
        //if else chain allows the constants to be changed later to variables
        //to allow for ntsc/pal to be emulated
        if self.scanline_number < 240 {
            if self.scanline_cycle == 0 {
                //idle cycle
                bus.ppu_cycle_1(0); //TODO put in proper address here
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 256 {
                let cycle = self.scanline_cycle - 1;
                if (cycle & 1) == 0 {
                    let addr = match cycle % 4 {
                        0 => {
                            //nametable byte
                            let base = self.nametable_base();
                            let x = cycle / 8;
                            let y = self.scanline_number / 8;
                            let offset = y * 32 | x;
                            base + offset
                        }
                        1 => {
                            //attribute table byte
                            42
                        }
                        2 => {
                            //pattern table tile low
                            42
                        }
                        3 => {
                            //pattern table tile high
                            42
                        }
                        _ => 42,
                    };
                    bus.ppu_cycle_1(addr);
                } else {
                    let _data = bus.ppu_cycle_2_read();
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 320 {
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 336 {
                self.increment_scanline_cycle();
            } else {
                self.increment_scanline_cycle();
            }
        } else if self.scanline_number == 240 {
            self.increment_scanline_cycle();
        } else if self.scanline_number <= 260 {
            //vblank lines
            if self.scanline_cycle == 1 {
                self.vblank = true;
            }
            self.increment_scanline_cycle();
        } else {
            self.increment_scanline_cycle();
        }
    }

    pub fn get_vblank(&self) -> bool {
        self.vblank_nmi
    }
}
