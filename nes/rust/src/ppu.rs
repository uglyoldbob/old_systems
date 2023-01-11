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
    nametable_counter: u16,
    write_ignore_counter: u16,
    nametable_data: u8,
    attributetable_data: u8,
    attributetable_shift: [u8;2],
    patterntable_tile: u16,
    patterntable_shift: [u16; 2],
    frame_data: Box<[u8; 3 * 256 * 240]>,
    frame_number: u64,
    pend_vram_write: Option<u8>,
    vram_address: u16,
}

const PPU_STARTUP_CYCLE_COUNT: u16 = 29658;

const PPU_PALETTE: [[u8; 3]; 64] = palette_generator(); //TODO put in correct colors into the palette

const fn palette_generator() -> [[u8; 3]; 64] {
    let mut palette: [[u8; 3]; 64] = [[0; 3]; 64];
    palette[0] = [84, 84, 84];
    palette[1] = [0, 30, 116];
    palette[2] = [8, 16, 144];
    palette[3] = [48, 0, 136];
    palette[4] = [68, 0, 100];
    palette[5] = [92, 0, 48];
    palette[6] = [84, 4, 0];
    palette[7] = [60, 24, 0];
    palette[8] = [32, 42, 0];
    palette[9] = [8, 58, 0];
    palette[10] = [0, 64, 0];
    palette[11] = [0, 60, 0];
    palette[12] = [0, 50, 60];
    palette[13] = [0, 0, 0];
    palette[14] = [0, 0, 0];
    palette[15] = [0, 0, 0];

    palette[16] = [152, 150, 152];
    palette[17] = [8, 76, 196];
    palette[18] = [48, 50, 236];
    palette[19] = [92, 30, 228];
    palette[20] = [136, 20, 176];
    palette[21] = [160, 20, 100];
    palette[22] = [152, 34, 32];
    palette[23] = [120, 60, 0];
    palette[24] = [84, 90, 0];
    palette[25] = [40, 114, 0];
    palette[26] = [8, 124, 0];
    palette[27] = [0, 118, 40];
    palette[28] = [0, 102, 120];
    palette[29] = [0, 0, 0];
    palette[30] = [0, 0, 0];
    palette[31] = [0, 0, 0];

    palette[32] = [236, 238, 236];
    palette[33] = [76, 154, 236];
    palette[34] = [120, 124, 236];
    palette[35] = [176, 98, 236];
    palette[36] = [228, 84, 236];
    palette[37] = [236, 88, 180];
    palette[38] = [236, 106, 100];
    palette[39] = [212, 136, 32];
    palette[40] = [160, 170, 0];
    palette[41] = [116, 196, 0];
    palette[42] = [76, 208, 32];
    palette[43] = [56, 204, 108];
    palette[44] = [56, 180, 204];
    palette[45] = [60, 60, 60];
    palette[46] = [0, 0, 0];
    palette[47] = [0, 0, 0];

    palette[48] = [236, 238, 236];
    palette[49] = [168, 204, 236];
    palette[50] = [188, 188, 236];
    palette[51] = [212, 178, 236];
    palette[52] = [236, 174, 236];
    palette[53] = [236, 174, 212];
    palette[54] = [236, 180, 176];
    palette[55] = [228, 196, 144];
    palette[56] = [204, 210, 120];
    palette[57] = [180, 222, 120];
    palette[58] = [168, 226, 144];
    palette[59] = [152, 226, 180];
    palette[60] = [160, 214, 228];
    palette[61] = [160, 162, 160];
    palette[62] = [0, 0, 0];
    palette[63] = [0, 0, 0];
    palette
}

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
            nametable_counter: 0,
            write_ignore_counter: 0,
            nametable_data: 0,
            attributetable_data: 0,
            attributetable_shift: [0,0],
            patterntable_tile: 0,
            patterntable_shift: [0, 0],
            frame_data: Box::new([0; 3 * 256 * 240]),
            frame_number: 0,
            pend_vram_write: None,
            vram_address: 0,
        }
    }

    pub fn read(&mut self, addr: u16) -> Option<u8> {
        //println!("Read ppu register {:x}", addr);
        let val = self.registers[addr as usize];
        if addr == 2 {
            self.address_bit = false;
            self.data_bit = false;
            self.registers[2] &= !0x80; //clear vblank flag
                                        //TODO maybe clear the other vblank flags here?
        }
        Some(val)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0 | 1 | 5 | 6 => {
                if self.write_ignore_counter >= PPU_STARTUP_CYCLE_COUNT {
                    self.registers[addr as usize] = data;
                } else {
                    println!("Write ppu register {:x} with {:x}", addr, data);
                }
            }
            7 => {
                self.pend_vram_write = Some(data);
            }
            _ => {
                println!("Write ppu register {:x} with {:x}", addr, data);
                self.registers[addr as usize] = data;
            }
        }
    }

    fn increment_scanline_cycle(&mut self) {
        self.scanline_cycle += 1;
        if self.scanline_cycle == 340 || (self.frame_odd && self.scanline_cycle == 339) {
            self.frame_odd = !self.frame_odd;
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

    fn patterntable_base(&self) -> u16 {
        if (self.registers[0] & 0x10) == 0 {
            0
        } else {
            0x1000
        }
    }

    fn compute_xy(&self, cycle: u16, offset: u8) -> (u16, u16) {
        let mut cycle2 = cycle + offset as u16;
        let mut scanline = self.scanline_number;
        if cycle2 >= 256 {
            cycle2 -= 256;
            scanline += 1;
            if scanline >= 240 {
                scanline -= 240;
            }
        }
        (cycle2, scanline)
    }

    fn background_fetch(&mut self, bus: &mut dyn NesMemoryBus, cycle: u16) {
        let (x,y) = self.compute_xy(cycle, 16);
        match (cycle / 2) % 4 {
            0 => {
                if (cycle & 1) == 0 {
                    //nametable byte
                    let base = self.nametable_base();
                    let offset = (y/8) << 5 | (x/8);
                    bus.ppu_cycle_1(base + offset);
                } else {
                    self.nametable_data = bus.ppu_cycle_2_read();
                }
            }
            1 => {
                //attribute table byte
                if (cycle & 1) == 0 {
                    let base = self.attributetable_base();
                    let offset = (y/32) << 5 | (x/32);
                    bus.ppu_cycle_1(base + offset);
                } else {
                    self.attributetable_data = bus.ppu_cycle_2_read();
                }
            }
            2 => {
                //pattern table tile low
                if (cycle & 1) == 0 {
                    let base = self.patterntable_base();
                    let offset = (self.nametable_data as u16) << 3;
                    let offset = (y/8) << 8 | (x/8) << 4;
                    let calc = base + offset + self.scanline_number % 8;
                    bus.ppu_cycle_1(calc);
                } else {
                    let mut pt = self.patterntable_tile.to_le_bytes();
                    pt[0] = bus.ppu_cycle_2_read();
                }
            }
            3 => {
                //pattern table tile high
                if (cycle & 1) == 0 {
                    let base = self.patterntable_base();
                    let offset = (self.nametable_data as u16) << 3;
                    let offset = (y/8) << 8 | (x/8) << 4;
                    let calc = 8 + base + offset + self.scanline_number % 8;
                    bus.ppu_cycle_1(calc);
                } else {
                    let mut pt = self.patterntable_tile.to_le_bytes();
                    pt[1] = bus.ppu_cycle_2_read();
                    self.patterntable_tile = u16::from_le_bytes(pt);
                    self.attributetable_shift[0] = self.attributetable_shift[1];
                    self.attributetable_shift[1] = self.attributetable_data;
                    self.patterntable_shift[0] = self.patterntable_shift[1];
                    self.patterntable_shift[1] = self.patterntable_tile;
                }
            }
            _ => {}
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
                if cycle == 1 {
                    self.vblank = false;
                    self.vblank_nmi = false;
                    self.registers[2] &= !0xE0; //vblank, sprite 0, sprite overflow
                }
                if (cycle & 1) == 0 {
                    bus.ppu_cycle_1(0); //TODO put in proper address here
                } else {
                    bus.ppu_cycle_2_read();
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 256 {
                //each cycle here renders a single pixel
                let cycle = self.scanline_cycle - 1;
                let index = 7 - cycle % 8;
                let pt = self.patterntable_shift[0].to_le_bytes();
                let upper_bit = (pt[1] >> index) & 1;
                let lower_bit = (pt[0] >> index) & 1;

                let modx = (cycle / 16) & 1;
                let mody = (self.scanline_number / 16) & 1;
                let combined = mody<<1 | modx;
                let extra_palette_bits = (self.attributetable_shift[0]>>(2*combined)) & 3;

                let palette_entry = ((extra_palette_bits<<2 | upper_bit << 1 | lower_bit) as usize);
                let pixel = PPU_PALETTE[palette_entry];
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 * 3) as usize] =
                    pixel[0];
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 * 3 + 1) as usize] =
                    pixel[1];
                self.frame_data[((self.scanline_number * 256 + cycle) as u32 * 3 + 2) as usize] =
                    pixel[2];
                self.background_fetch(bus, cycle);
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 320 {
                //sprite rendering data to be fetched
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
                //background renderer control
                let cycle = self.scanline_cycle - 321;
                //self.background_fetch(bus, cycle);
                self.increment_scanline_cycle();
            } else {
                //do nothing
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
                self.registers[2] |= 0x80;
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

    pub fn get_frame(&mut self) -> &Box<[u8; 256 * 240 * 3]> {
        &self.frame_data
    }

    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    pub fn convert_to_egui(f: &Box<[u8; 256 * 240 * 3]>) -> egui::ColorImage {
        let data = &**f;
        let pixels = data
            .chunks_exact(3)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        egui::ColorImage {
            size: [256, 240],
            pixels: pixels,
        }
    }
}
