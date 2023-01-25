use crate::cpu::NesMemoryBus;

#[cfg(feature = "eframe")]
use eframe::egui;

pub struct NesPpu {
    registers: [u8; 8],
    scanline_number: u16,
    scanline_cycle: u16,
    frame_end: bool,
    address_bit: bool,
    data_bit: bool,
    vblank_clear: bool,
    vblank_nmi: bool,
    frame_odd: bool,
    write_ignore_counter: u16,
    nametable_data: u8,
    attributetable_data: u8,
    attributetable_shift: [u8; 2],
    patterntable_tile: u16,
    patterntable_shift: [u16; 2],
    frame_data: Box<[u8; 3 * 256 * 240]>,
    pend_vram_write: Option<u8>,
    pend_vram_read: Option<u16>,
    vram_address: u16,
    #[cfg(any(test, debug_assertions))]
    frame_number: u64,
    last_nmi: bool,
    ppudata_buffer: u8,
    last_cpu_data: u8,
    last_cpu_counter: [u32; 2],
    oam: [u8; 256],
    oamaddress: u8,
    cycle1_done: bool,
    debug_special: bool,
}

const PPU_REGISTER0_NAMETABLE_BASE: u8 = 0x03;
const PPU_REGISTER0_VRAM_ADDRESS_INCREMENT: u8 = 0x04;
const PPU_REGISTER0_SPRITETABLE_BASE: u8 = 0x08;
const PPU_REGISTER0_BACKGROUND_PATTERNTABLE_BASE: u8 = 0x10;
const PPU_REGISTER0_SPRITE_SIZE: u8 = 0x20;
const PPU_REGISTER0_GENERATE_NMI: u8 = 0x80;

const PPU_REGISTER1_GREYSCALE: u8 = 0x01;
const PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN: u8 = 0x02;
const PPU_REGISTER1_DRAW_SPRITES_FIRST_COLUMN: u8 = 0x04;
const PPU_REGISTER1_DRAW_BACKGROUND: u8 = 0x08;
const PPU_REGISTER1_DRAW_SPRITES: u8 = 0x10;
const PPU_REGISTER1_EMPHASIZE_RED: u8 = 0x20;
const PPU_REGISTER1_EMPHASIZE_GREEN: u8 = 0x40;
const PPU_REGISTER1_EMPHASIZE_BLUE: u8 = 0x80;

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
        let mut oam = [0; 256];
        for i in &mut oam {
            *i = rand::random();
        }
        Self {
            scanline_number: 0,
            scanline_cycle: 0,
            registers: [0, 0, reg2, 0, 0, 0, 0, 0],
            address_bit: false,
            data_bit: false,
            vblank_nmi: false,
            vblank_clear: false,
            frame_end: false,
            frame_odd: false,
            write_ignore_counter: 0,
            nametable_data: 0,
            attributetable_data: 0,
            attributetable_shift: [0, 0],
            patterntable_tile: 0,
            patterntable_shift: [0, 0],
            frame_data: Box::new([0; 3 * 256 * 240]),
            pend_vram_write: None,
            pend_vram_read: None,
            vram_address: 0,
            #[cfg(any(test, debug_assertions))]
            frame_number: 0,
            last_nmi: false,
            ppudata_buffer: 0,
            last_cpu_data: 0,
            last_cpu_counter: [0, 0],
            oam: oam,
            oamaddress: 0,
            cycle1_done: false,
            debug_special: false,
        }
    }

    #[cfg(any(test, debug_assertions))]
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    pub fn reset(&mut self) {
        self.registers[0] = 0;
        self.registers[1] = 0;
        #[cfg(any(test, debug_assertions))]
        {
            self.frame_number = 0;
        }
    }

    pub fn vram_address(&self) -> u16 {
        self.vram_address
    }

    pub fn increment_vram(&mut self) {
        self.vram_address = self.vram_address.wrapping_add(1);
    }

    pub fn provide_palette_data(&mut self, data: u8) {
        let data2 = data & 0x3f;
        self.last_cpu_data = data2;
        self.last_cpu_counter[1] = 893420;
    }

    pub fn read(&mut self, addr: u16) -> Option<u8> {
        match addr {
            0 | 1 | 3 | 5 | 6 => Some(self.last_cpu_data),
            4 => {
                let mut data = self.oam[self.oamaddress as usize];
                match self.oamaddress & 3 {
                    2 => {
                        data &= 0xe3;
                        self.last_cpu_data = data;
                        self.last_cpu_counter[0] = 893420;
                        self.last_cpu_counter[1] = 893420;
                    }
                    _ => {}
                }
                Some(data)
            }
            7 => match self.vram_address {
                0..=0x3eff => {
                    self.pend_vram_read = Some(self.vram_address);
                    if (self.registers[0] & PPU_REGISTER0_VRAM_ADDRESS_INCREMENT) == 0 {
                        self.vram_address = self.vram_address.wrapping_add(1);
                    } else {
                        self.vram_address = self.vram_address.wrapping_add(32);
                    }
                    self.last_cpu_data = self.ppudata_buffer;
                    self.last_cpu_counter[0] = 893420;
                    self.last_cpu_counter[1] = 893420;
                    Some(self.ppudata_buffer)
                }
                _ => {
                    self.pend_vram_read = Some(self.vram_address);
                    Some(self.last_cpu_data & 0xC0)
                }
            },
            _ => {
                let mut val = self.registers[addr as usize];
                if addr == 2 {
                    self.address_bit = false;
                    self.data_bit = false;
                    self.vblank_clear = true;
                    val = (val & 0xE0) | self.last_cpu_data & 0x1f;
                    self.last_cpu_data = (self.last_cpu_data & 0x1f) | (val & 0xE0);
                    self.last_cpu_counter[1] = 893420;
                }
                Some(val)
            }
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.last_cpu_data = data;
        self.last_cpu_counter[0] = 893420;
        self.last_cpu_counter[1] = 893420;
        match addr {
            0 | 1 | 5 | 6 => {
                if self.write_ignore_counter >= PPU_STARTUP_CYCLE_COUNT {
                    match addr {
                        5 => {
                            self.address_bit = !self.address_bit;
                        }
                        6 => {
                            if !self.address_bit {
                                self.registers[addr as usize] = data;
                                self.address_bit = !self.address_bit;
                            } else {
                                self.vram_address = (self.registers[6] as u16) << 8 | data as u16;
                                self.address_bit = !self.address_bit;
                            }
                        }
                        _ => {
                            self.registers[addr as usize] = data;
                        }
                    }
                }
            }
            3 => {
                self.oamaddress = data;
            }
            4 => {
                self.oam[self.oamaddress as usize] = data;
                self.oamaddress = self.oamaddress.wrapping_add(1);
            }
            7 => match self.vram_address {
                0..=0x3eff => {
                    self.pend_vram_write = Some(data);
                }
                _ => {}
            },
            2 => {}
            _ => {
                self.registers[addr as usize] = data;
            }
        }
    }

    fn increment_scanline_cycle(&mut self) {
        self.scanline_cycle += 1;
        if self.scanline_cycle == 341 {
            self.scanline_cycle = 0;
            self.scanline_number += 1;
            if self.scanline_number == 262 {
                self.frame_odd = !self.frame_odd;
                self.scanline_number = 0;
            }
            if self.scanline_cycle == 0
                && self.scanline_number == 0
                && self.frame_odd
                && ((self.registers[1]
                    & (PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN | PPU_REGISTER1_DRAW_BACKGROUND))
                    != 0)
            {
                self.scanline_cycle += 1;
            }
        }
    }

    fn nametable_base(&self) -> u16 {
        match self.registers[0] & PPU_REGISTER0_NAMETABLE_BASE {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            _ => 0x2c00,
        }
    }

    fn attributetable_base(&self) -> u16 {
        match self.registers[0] & PPU_REGISTER0_NAMETABLE_BASE {
            0 => 0x23c0,
            1 => 0x27c0,
            2 => 0x2bc0,
            _ => 0x2fc0,
        }
    }

    fn patterntable_base(&self) -> u16 {
        if (self.registers[0] & PPU_REGISTER0_BACKGROUND_PATTERNTABLE_BASE) == 0 {
            0
        } else {
            0x1000
        }
    }

    fn should_render_background(&self, cycle: u16) -> bool {
        if cycle < 8 {
            (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN) != 0
        } else {
            (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND) != 0
        }
    }

    fn should_render_sprites(&self, cycle: u16) -> bool {
        if cycle == 0 {
            (self.registers[1] & PPU_REGISTER1_DRAW_SPRITES_FIRST_COLUMN) != 0
        } else {
            (self.registers[1] & PPU_REGISTER1_DRAW_SPRITES) != 0
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
        let (x, y) = self.compute_xy(cycle, 16);
        match (cycle / 2) % 4 {
            0 => {
                if (cycle & 1) == 0 {
                    //nametable byte
                    let base = self.nametable_base();
                    let offset = (y / 8) << 5 | (x / 8);
                    bus.ppu_cycle_1(base + offset);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    self.nametable_data = bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
                }
            }
            1 => {
                //attribute table byte
                if (cycle & 1) == 0 {
                    let base = self.attributetable_base();
                    let offset = (y / 32) << 3 | (x / 32);
                    bus.ppu_cycle_1(base + offset);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    self.attributetable_data = bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
                }
            }
            2 => {
                //pattern table tile low
                if (cycle & 1) == 0 {
                    let base = self.patterntable_base();
                    let offset = (self.nametable_data as u16) << 4;
                    let calc = base + offset + self.scanline_number % 8;
                    bus.ppu_cycle_1(calc);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    let mut pt = self.patterntable_tile.to_le_bytes();
                    pt[0] = bus.ppu_cycle_2_read();
                    self.patterntable_tile = u16::from_le_bytes(pt);
                    self.cycle1_done = false;
                }
            }
            3 => {
                //pattern table tile high
                if (cycle & 1) == 0 {
                    let base = self.patterntable_base();
                    let offset = (self.nametable_data as u16) << 4;
                    let calc = 8 + base + offset + self.scanline_number % 8;
                    bus.ppu_cycle_1(calc);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    let mut pt = self.patterntable_tile.to_le_bytes();
                    pt[1] = bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
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

    fn idle_operation(&mut self, bus: &mut dyn NesMemoryBus, cycle: u16) {
        if (cycle & 1) == 0 {
            if let Some(_a) = self.pend_vram_write {
                bus.ppu_cycle_1(self.vram_address);
                self.cycle1_done = true;
            } else if let Some(a) = self.pend_vram_read {
                bus.ppu_cycle_1(a & 0x2fff);
                self.cycle1_done = true;
            }
        } else if self.cycle1_done {
            if let Some(a) = self.pend_vram_write {
                bus.ppu_cycle_2_write(a);
                self.cycle1_done = false;
                if (self.registers[0] & PPU_REGISTER0_VRAM_ADDRESS_INCREMENT) == 0 {
                    self.vram_address = self.vram_address.wrapping_add(1);
                } else {
                    self.vram_address = self.vram_address.wrapping_add(32);
                }
                self.pend_vram_write = None;
            } else if let Some(_a) = self.pend_vram_read {
                self.ppudata_buffer = bus.ppu_cycle_2_read();
                self.cycle1_done = false;
                self.pend_vram_read = None;
            }
        }
    }

    pub fn cycle(&mut self, bus: &mut dyn NesMemoryBus) {
        if self.write_ignore_counter < PPU_STARTUP_CYCLE_COUNT {
            self.write_ignore_counter += 1;
        }

        for c in &mut self.last_cpu_counter {
            if *c > 0 {
                *c -= 1;
            }
        }
        if self.last_cpu_counter[0] == 0 {
            self.last_cpu_data &= 0xe0;
        }
        if self.last_cpu_counter[1] == 0 {
            self.last_cpu_data &= 0x1f;
        }

        let vblank_flag = (self.registers[2] & 0x80) != 0;

        //if else chain allows the constants to be changed later to variables
        //to allow for ntsc/pal to be emulated
        if self.scanline_number < 240 {
            if self.scanline_cycle == 0 {
                //idle cycle
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 256 {
                //each cycle here renders a single pixel
                let cycle = self.scanline_cycle - 1;
                if self.should_render_background(cycle) {
                    let index = 7 - cycle % 8;
                    let pt = self.patterntable_shift[0].to_le_bytes();
                    let upper_bit = (pt[1] >> index) & 1;
                    let lower_bit = (pt[0] >> index) & 1;

                    let modx = (cycle / 16) & 1;
                    let mody = (self.scanline_number / 16) & 1;
                    let combined = mody << 1 | modx;
                    let extra_palette_bits = if combined != 0 {
                        (self.attributetable_shift[0] >> (2 * combined)) & 3
                    } else {
                        0
                    };

                    let mut palette_entry =
                        (extra_palette_bits << 2 | upper_bit << 1 | lower_bit) as u16;
                    if (self.registers[1] & PPU_REGISTER1_GREYSCALE) != 0 {
                        palette_entry &= 0x30;
                    }
                    let pixel_entry = bus.ppu_palette_read(0x3f00 + palette_entry) & 63;
                    if (self.registers[1]
                        & (PPU_REGISTER1_EMPHASIZE_BLUE
                            | PPU_REGISTER1_EMPHASIZE_GREEN
                            | PPU_REGISTER1_EMPHASIZE_RED))
                        != 0
                    {
                        //TODO implement color emphasis
                        println!("TODO: implement color emphasis");
                    }
                    let pixel = PPU_PALETTE[pixel_entry as usize];
                    self.frame_data[((self.scanline_number * 256 + cycle) as u32 * 3) as usize] =
                        pixel[0];
                    self.frame_data
                        [((self.scanline_number * 256 + cycle) as u32 * 3 + 1) as usize] = pixel[1];
                    self.frame_data
                        [((self.scanline_number * 256 + cycle) as u32 * 3 + 2) as usize] = pixel[2];
                    self.background_fetch(bus, cycle);
                } else {
                    self.idle_operation(bus, cycle);
                    let palette_entry = 13;
                    let pixel = PPU_PALETTE[palette_entry];
                    self.frame_data[((self.scanline_number * 256 + cycle) as u32 * 3) as usize] =
                        pixel[0];
                    self.frame_data
                        [((self.scanline_number * 256 + cycle) as u32 * 3 + 1) as usize] = pixel[1];
                    self.frame_data
                        [((self.scanline_number * 256 + cycle) as u32 * 3 + 2) as usize] = pixel[2];
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 320 {
                //sprite rendering data to be fetched
                let cycle = self.scanline_cycle - 257;
                if cycle > 0 {
                    if self.should_render_sprites(cycle) {
                        match (cycle / 2) % 4 {
                            0 => {
                                if (cycle & 1) == 0 {
                                    //nametable byte
                                    let base = self.nametable_base();
                                    let x = cycle / 8;
                                    let y = self.scanline_number / 8;
                                    let offset = y << 5 | x;
                                    bus.ppu_cycle_1(base + offset); //TODO verify this calculation
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    bus.ppu_cycle_2_read();
                                    self.cycle1_done = false;
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
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    bus.ppu_cycle_2_read();
                                    self.cycle1_done = false;
                                }
                            }
                            2 => {
                                //pattern table tile low
                                if (cycle & 1) == 0 {
                                    let base = 0; //TODO calculate this correctly
                                    let offset = cycle % 8; //TODO calculate this value correctly
                                    bus.ppu_cycle_1(base + offset);
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    let mut pt = self.patterntable_tile.to_le_bytes();
                                    pt[0] = bus.ppu_cycle_2_read();
                                    self.patterntable_tile = u16::from_le_bytes(pt);
                                    self.cycle1_done = false;
                                }
                            }
                            3 => {
                                //pattern table tile high
                                if (cycle & 1) == 0 {
                                    let base = 0; //TODO calculate this correctly
                                    let offset = cycle % 8; //TODO calculate this value correctly
                                    bus.ppu_cycle_1(base + offset);
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    let mut pt = self.patterntable_tile.to_le_bytes();
                                    pt[1] = bus.ppu_cycle_2_read();
                                    self.patterntable_tile = u16::from_le_bytes(pt);
                                    self.cycle1_done = false
                                }
                            }
                            _ => {}
                        }
                    } else {
                        self.idle_operation(bus, self.scanline_cycle - 1);
                    }
                }
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 336 {
                //background renderer control
                let cycle = self.scanline_cycle - 321;
                if cycle > 0 {
                    //self.background_fetch(bus, cycle);
                    self.idle_operation(bus, self.scanline_cycle - 1);
                }
                self.increment_scanline_cycle();
            } else {
                //do nothing
                let cycle = self.scanline_cycle - 337;
                if (cycle & 1) == 0 {
                    let base = 0; //TODO calculate this correctly
                    let offset = cycle % 8; //TODO calculate this value correctly
                    bus.ppu_cycle_1(base + offset);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
                }
                self.increment_scanline_cycle();
            }
        } else if self.scanline_number == 240 {
            if self.scanline_cycle > 0 {
                self.idle_operation(bus, self.scanline_cycle - 1);
            }
            self.increment_scanline_cycle();
        } else if self.scanline_number <= 260 {
            //vblank lines
            if self.scanline_cycle == 1 && self.scanline_number == 241 {
                self.registers[2] |= 0x80;
                self.frame_end = true;
                #[cfg(any(test, debug_assertions))]
                {
                    self.frame_number = self.frame_number.wrapping_add(1);
                }
            }
            if self.scanline_cycle > 0 {
                self.idle_operation(bus, self.scanline_cycle - 1);
            }
            self.increment_scanline_cycle();
        } else {
            if self.scanline_number == 261 && self.scanline_cycle == 1 {
                self.registers[2] &= !0xE0; //vblank, sprite 0, sprite overflow
            }
            if self.scanline_cycle > 0 {
                self.idle_operation(bus, self.scanline_cycle - 1);
            }
            self.increment_scanline_cycle();
        }
        if self.vblank_clear {
            self.vblank_clear = false;
            self.registers[2] &= !0x80; //clear vblank flag
        }
        self.last_nmi = self.vblank_nmi;
        self.vblank_nmi = ((self.registers[2] & 0x80) != 0)
            & ((self.registers[0] & PPU_REGISTER0_GENERATE_NMI) != 0);
    }

    pub fn get_frame_end(&mut self) -> bool {
        let flag = self.frame_end;
        self.frame_end = false;
        flag
    }

    pub fn get_frame(&mut self) -> &Box<[u8; 256 * 240 * 3]> {
        &self.frame_data
    }

    pub fn irq(&self) -> bool {
        self.vblank_nmi
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
