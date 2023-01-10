use crate::cpu::NesMemoryBus;

pub struct NesPpu {
    registers: [u8; 8],
    scanline_number: u16,
    scanline_cycle: u16,
    frame_end: bool,
    vblank: bool,
    address_bit: bool,
    data_bit: bool,
    vblank_nmi: bool,
    frame_odd: bool,
    cycle: u32,
    nametable_counter: u16,
    write_ignore_counter: u16,
    nametable_data: u8,
    attributetable_data: u8,
    patterntable_tile: u16,
    frame_data: [u8; 3*256 * 240],
    frame_number: u64,
}

const PPU_STARTUP_CYCLE_COUNT: u16 = 29658;

const PPU_PALETTE: [u16; 56] = [1; 56]; //TODO put in correct colors into the palette

impl NesPpu {
    pub fn new() -> Self {
        let reg2: u8 = rand::random::<u8>() & !0x40;
        Self {
            scanline_number: 0,
            scanline_cycle: 0,
            registers: [0, 0, reg2, 0, 0, 0, 0, 0],
            address_bit: false,
            data_bit: false,
            vblank: false,
            vblank_nmi: false,
            frame_end: false,
            frame_odd: false,
            cycle: 0,
            nametable_counter: 0,
            write_ignore_counter: 0,
            nametable_data: 0,
            attributetable_data: 0,
            patterntable_tile: 0,
            frame_data: [0; 3 * 256 * 240],
            frame_number: 0,
        }
    }

    pub fn read(&mut self, addr: u16) -> Option<u8> {
        //println!("Read ppu register {:x}", addr);
        if addr == 2 {
            self.address_bit = false;
            self.data_bit = false;
        }
        Some(self.registers[addr as usize])
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        //println!("Write ppu register {:x} with {:x}", addr, data);
        match addr {
            0 | 1 | 5 | 6 => {
                if self.write_ignore_counter >= PPU_STARTUP_CYCLE_COUNT {
                    self.registers[addr as usize] = data;
                }
            }
            _ => {
                self.registers[addr as usize] = data;
            }
        }
    }

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

    fn attributetable_base(&self) -> u16 {
        match self.registers[0] & 3 {
            0 => 0x23c0,
            1 => 0x27c0,
            2 => 0x2bc0,
            _ => 0x2fc0,
        }
    }

    pub fn cycle(&mut self, bus: &mut dyn NesMemoryBus) {
        if self.write_ignore_counter < PPU_STARTUP_CYCLE_COUNT {
            self.write_ignore_counter += 1;
        }

        //if else chain allows the constants to be changed later to variables
        //to allow for ntsc/pal to be emulated
        if self.scanline_number < 240 {
            if self.scanline_cycle == 0 {
                //idle cycle
                let cycle = self.scanline_cycle;
                if (cycle & 1) == 0 {
                    bus.ppu_cycle_1(0); //TODO put in proper address here
                } else {
                    bus.ppu_cycle_2_read();
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 256 {
                //each cycle here renders a single pixel
                let cycle = self.scanline_cycle - 1;
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 *3) as usize] = rand::random::<u8>();//(self.scanline_number & 0xFF) as u8;
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 *3+1) as usize] = (cycle & 0xFF) as u8;
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 *3+2) as usize] = (self.frame_number & 0xFF) as u8;
                match (cycle / 2) % 4 {
                    0 => {
                        if (cycle & 1) == 0 {
                            //nametable byte
                            let base = self.nametable_base();
                            let x = cycle / 8;
                            let y = self.scanline_number / 8;
                            let offset = y << 5 | x;
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            self.nametable_data = bus.ppu_cycle_2_read();
                        }
                    }
                    1 => {
                        //attribute table byte
                        if (cycle & 1) == 0 {
                            let base = self.attributetable_base();
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            self.attributetable_data = bus.ppu_cycle_2_read();
                        }
                    }
                    2 => {
                        //pattern table tile low
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[0] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    3 => {
                        //pattern table tile high
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[1] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    _ => {}
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 320 {
                let cycle = self.scanline_cycle - 257;
                match (cycle / 2) % 4 {
                    0 => {
                        if (cycle & 1) == 0 {
                            //nametable byte
                            let base = self.nametable_base();
                            let x = cycle / 8;
                            let y = self.scanline_number / 8;
                            let offset = y << 5 | x;
                            bus.ppu_cycle_1(base + offset); //TODO verify this calculation
                        } else {
                            bus.ppu_cycle_2_read();
                        }
                    }
                    1 => {
                        if (cycle & 1) == 0 {
                            //nametable byte
                            let base = self.nametable_base();
                            let x = cycle / 8;
                            let y = self.scanline_number / 8;
                            let offset = y << 5 | x;
                            bus.ppu_cycle_1(base + offset); //TODO verify this calculation
                        } else {
                            bus.ppu_cycle_2_read();
                        }
                    }
                    2 => {
                        //pattern table tile low
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[0] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    3 => {
                        //pattern table tile high
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[1] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    _ => {}
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 336 {
                let cycle = self.scanline_cycle - 321;
                match (cycle / 2) % 4 {
                    0 => {
                        if (cycle & 1) == 0 {
                            //nametable byte
                            let base = self.nametable_base();
                            let x = cycle / 8;
                            let y = self.scanline_number / 8;
                            let offset = y << 5 | x;
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            self.nametable_data = bus.ppu_cycle_2_read();
                        }
                    }
                    1 => {
                        //attribute table byte
                        if (cycle & 1) == 0 {
                            let base = self.attributetable_base();
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            self.attributetable_data = bus.ppu_cycle_2_read();
                        }
                    }
                    2 => {
                        //pattern table tile low
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[0] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    3 => {
                        //pattern table tile high
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            let mut pt = self.patterntable_tile.to_le_bytes();
                            pt[1] = bus.ppu_cycle_2_read();
                            self.patterntable_tile = u16::from_le_bytes(pt);
                        }
                    }
                    _ => {}
                }
                self.increment_scanline_cycle();
            } else {
                let cycle = self.scanline_cycle - 337;
                match (cycle / 2) % 2 {
                    0 => {
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            bus.ppu_cycle_2_read();
                        }
                    }
                    _ => {
                        if (cycle & 1) == 0 {
                            let base = 0; //TODO calculate this correctly
                            let offset = cycle % 8; //TODO calculate this value correctly
                            bus.ppu_cycle_1(base + offset);
                        } else {
                            bus.ppu_cycle_2_read();
                        }
                    }
                }
                self.increment_scanline_cycle();
            }
        } else if self.scanline_number == 240 {
            self.increment_scanline_cycle();
        } else if self.scanline_number <= 260 {
            //vblank lines
            if self.scanline_cycle == 1 && self.scanline_number == 241 {
                self.vblank = true;
                self.frame_end = true;
                self.frame_number = self.frame_number.wrapping_add(1);
            }
            self.increment_scanline_cycle();
        } else {
            self.increment_scanline_cycle();
        }
    }

    pub fn get_frame_end(&mut self) -> bool {
        let flag = self.frame_end;
        self.frame_end = false;
        flag
    }

    pub fn get_frame(&mut self) -> &[u8] {
        &self.frame_data
    }

    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }
}
