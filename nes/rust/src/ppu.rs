//! The ppu module for the emulator. Responsible for emulating the chip that generates all of the graphics for the nes.

use crate::motherboard::NesMotherboard;
use serde_with::Bytes;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// A rgb image of variable size. Each pixel is 8 bits per channel, red, green, blue.
pub struct RgbImage {
    /// The raw data of the image
    data: Vec<u8>,
    /// The width of the image in pixels.
    pub width: u16,
    /// The height of the image in pixels.
    pub height: u16,
}

impl RgbImage {
    /// Create a blank rgb image of the specified dimensions.
    pub fn new(w: u16, h: u16) -> Self {
        let cap = w as usize * h as usize * 3;
        let m = vec![0; cap];
        Self {
            data: m,
            width: w,
            height: h,
        }
    }

    /// Converts to egui format.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub fn to_egui(&self) -> egui::ColorImage {
        let pixels = self
            .data
            .chunks_exact(3)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        egui::ColorImage {
            size: [self.width as usize, self.height as usize],
            pixels,
        }
    }
}

/// The various modes of evaluating sprites for a scanline
#[non_exhaustive]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum PpuSpriteEvalMode {
    /// Look for sprites on the current scanline
    Normal,
    /// A sprite has been found, copy the other bytes of that sprite
    CopyCurrentSprite,
    /// The are currently 8 sprites that have been found for the current scanline
    Sprites8,
    /// Sprites are done being evaluated
    Done,
}

/// A struct for a single sprite of the ppu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
pub struct PpuSprite {
    /// The y coordinate for the sprite on screen
    y: u8,
    /// The tile number for the sprite
    tile: u8,
    /// The attribute data for the sprite
    attribute: u8,
    /// The x coordinate of the sprite on screen
    x: u8,
    /// The pattern table data for the sprite
    patterntable_data: u16,
}

impl PpuSprite {
    /// Create a new sprite, off of the rendered screen
    fn new() -> Self {
        Self {
            y: 0xff,
            tile: 0xff,
            attribute: 0xff,
            x: 0xff,
            patterntable_data: 0,
        }
    }

    /// Returns the x coordinate
    pub fn x(&self) -> u8 {
        self.x
    }

    /// Returns the y coordinate
    pub fn y(&self) -> u8 {
        self.y
    }

    /// Returns the tile data for the sprite.
    pub fn tile(&self) -> u8 {
        self.tile
    }

    /// Returns the pallete for the sprite
    pub fn pallete(&self) -> u16 {
        ((self.attribute & 3) as u16) << 2
    }

    /// Returns the tile number to fetch for this sprite.
    pub fn tile_num(&self, scanline: u8, height: u8) -> u16 {
        let mask = if height == 16 { 0xFE } else { 0xFF };
        let calc = self.tile as u16 & mask;
        let adder: u16 = if scanline >= self.y {
            if (self.attribute & 0x80) == 0 {
                if (scanline - self.y) < 8 {
                    0
                } else {
                    1
                }
            } else {
                if (scanline - self.y) < 8 {
                    1
                } else {
                    0
                }
            }
        } else {
            0
        };
        (adder + calc) * 0x10
    }

    /// Returns the line number to render of the sprite, given the scanline being rendered
    pub fn line_number(&self, scanline: u8) -> u8 {
        if self.y <= scanline {
            let sprite_line = (scanline - self.y) % 8;
            if (self.attribute & 0x80) == 0 {
                sprite_line
            } else {
                7 - sprite_line
            }
        } else {
            0
        }
    }
}

/// The structure for the nes PPU (picture processing unit)
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NesPpu {
    /// The registers for the ppu
    registers: [u8; 8],
    /// The index of the current rendering row
    scanline_number: u16,
    /// The index of the current rendering column
    scanline_cycle: u16,
    /// The flag that indicates the end of a frame has occurred. Used for synchronizing frame rate of the emulator.
    frame_end: bool,
    /// Controls access to registers 5 and 6 for writes by the cpu
    address_bit: bool,
    /// Used for clearing the vblank flag
    vblank_clear: bool,
    /// Flag used for generating irq signals used by the cpu.
    vblank_nmi: bool,
    /// Indicates that an odd frame is currently being rendered.
    frame_odd: bool,
    /// Used to ignore writes to certain registers during ppu startup. Used with PPU_STARTUP_CYCLE_COUNT
    write_ignore_counter: u16,
    /// The data for the previous tile from the nametable, for the background
    prev_nametable_data: u8,
    /// The data from the nametable, used for background fetching
    nametable_data: u8,
    /// The attribute table data, used for background rendering
    attributetable_data: u8,
    /// The shift register for the attribute table, used for background rendering
    attributetable_shift: [u8; 2],
    /// The patterntable data, for displaying background data.
    patterntable_tile: u16,
    /// The shift register for the patterntable, used for background rendering
    #[serde_as(as = "[_; 2]")]
    patterntable_shift: [u16; 2],
    /// The frame data stored in the ppu for being displayed onto the screen later.
    #[serde_as(as = "Bytes")]
    frame_data: Box<[u8; 3 * 256 * 240]>,
    /// Indicates that there is a pending write from the cpu
    pend_vram_write: Option<u8>,
    /// Indicates that there is a pending read from the cpu
    pend_vram_read: Option<u16>,
    /// The vram address for accessing ppu vram (different from oam)
    vram_address: u16,
    /// The temporary vram address used in the scrolling algorithm
    temporary_vram_address: u16,
    /// The frame number of the ppu, used for testing and debugging purposes.
    #[cfg(any(test, feature = "debugger"))]
    frame_number: u64,
    /// For read operations by the cpu
    ppudata_buffer: u8,
    /// The data for emulating pen bus behavior
    last_cpu_data: u8,
    /// The counter used for emulating open bus behavior of the ppu
    last_cpu_counter: [u32; 2],
    #[serde_as(as = "Bytes")]
    /// The memory for holding up to 64 sprites for the entire frame.
    oam: [u8; 256],
    /// The memory for storing evaluated sprites for the next scanline
    secondary_oam: [u8; 32],
    /// The sprites for the current scanline being rendered
    sprites: [PpuSprite; 8],
    /// The address to use for secondary oam access
    secondaryoamaddress: u8,
    /// The data retrieved from the oam
    oamdata: u8,
    /// The address to use for oam access
    oamaddress: u8,
    /// The mode for sprite evaluation in the sprite_eval function
    sprite_eval_mode: PpuSpriteEvalMode,
    /// Indicates that the first half of the ppu memory cycle has been completed.
    cycle1_done: bool,
    /// The fine horizontal scroll amount, in pixels
    scrollx: u8,
}

/// The flags that set the nametable base
const PPU_REGISTER0_NAMETABLE_BASE: u8 = 0x03;
/// The flag that sets the vram address increment amount
const PPU_REGISTER0_VRAM_ADDRESS_INCREMENT: u8 = 0x04;
/// The flag to select the sprite table base
const PPU_REGISTER0_SPRITETABLE_BASE: u8 = 0x08;
/// The flag that selects the second half of the pattern table for the background
const PPU_REGISTER0_BACKGROUND_PATTERNTABLE_BASE: u8 = 0x10;
/// The flag that indicates a larger sprite size of 16 pixels height instead of 8 pixels
const PPU_REGISTER0_SPRITE_SIZE: u8 = 0x20;
/// The flag that indicates that the nmi should be generated
const PPU_REGISTER0_GENERATE_NMI: u8 = 0x80;

/// The flag that indicates that everything should be in grayscale
const PPU_REGISTER1_GREYSCALE: u8 = 0x01;
/// The flag that indicates that the background should be drawn in the first column
const PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN: u8 = 0x02;
/// The flag that indicates sprites should be drawn in the first column
const PPU_REGISTER1_DRAW_SPRITES_FIRST_COLUMN: u8 = 0x04;
/// The flag that indicates the background should be drawn
const PPU_REGISTER1_DRAW_BACKGROUND: u8 = 0x08;
/// The flag that indicates sprites should be drawn
const PPU_REGISTER1_DRAW_SPRITES: u8 = 0x10;
/// The flag for emphasizing the red channel
const PPU_REGISTER1_EMPHASIZE_RED: u8 = 0x20;
/// The flag for emphasizing the green channel
const PPU_REGISTER1_EMPHASIZE_GREEN: u8 = 0x40;
/// The flag for emphasizing the blue channel
const PPU_REGISTER1_EMPHASIZE_BLUE: u8 = 0x80;

/// The number of cycles where the ppu is in a special state on startup.
const PPU_STARTUP_CYCLE_COUNT: u16 = 29658;

/// The palette for the ppu
const PPU_PALETTE: [[u8; 3]; 64] = palette_generator(); //TODO put in correct colors into the palette

/// Build a palette for the ppu.
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
    /// Return a new ppu.
    pub fn new() -> Self {
        let reg2: u8 = rand::random::<u8>() & !0x40;
        let mut oam = [0; 256];
        for i in &mut oam {
            *i = rand::random();
        }

        let mut oam2 = [0; 32];
        for i in &mut oam2 {
            *i = rand::random();
        }
        Self {
            scanline_number: 0,
            scanline_cycle: 0,
            registers: [0, 0, reg2, 0, 0, 0, 0, 0],
            address_bit: false,
            vblank_nmi: false,
            vblank_clear: false,
            frame_end: false,
            frame_odd: false,
            write_ignore_counter: 0,
            prev_nametable_data: 0,
            nametable_data: 0,
            attributetable_data: 0,
            attributetable_shift: [0, 0],
            patterntable_tile: 0,
            patterntable_shift: [0, 0],
            frame_data: Box::new([0; 3 * 256 * 240]),
            pend_vram_write: None,
            pend_vram_read: None,
            vram_address: 0,
            temporary_vram_address: 0,
            #[cfg(any(test, feature = "debugger"))]
            frame_number: 0,
            ppudata_buffer: 0,
            last_cpu_data: 0,
            last_cpu_counter: [0, 0],
            oam,
            secondary_oam: oam2,
            sprites: [PpuSprite::new(); 8],
            secondaryoamaddress: 0,
            oamdata: 0,
            sprite_eval_mode: PpuSpriteEvalMode::Normal,
            oamaddress: 0,
            cycle1_done: false,
            scrollx: 0,
        }
    }

    /// Return the frame number of the ppu, mostly used for testing and debugging the ppu
    #[cfg(any(test, feature = "debugger"))]
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Reset the ppu
    pub fn reset(&mut self) {
        self.registers[0] = 0;
        self.registers[1] = 0;
        #[cfg(any(test, feature = "debugger"))]
        {
            self.frame_number = 0;
        }
    }

    /// Returns the vram address of the ppu
    pub fn vram_address(&self) -> u16 {
        self.vram_address
    }

    /// Increment the vram address, ignoring any wrapping
    pub fn increment_vram(&mut self) {
        self.vram_address = self.vram_address.wrapping_add(1);
    }

    /// Allows providing palette data directly to the ppu
    pub fn provide_palette_data(&mut self, data: u8) {
        let data2 = data & 0x3f;
        self.last_cpu_data = data2;
        self.last_cpu_counter[1] = 893420;
    }

    /// Returns a copy of the sprites in the ppu memory
    #[cfg(any(test, feature = "debugger"))]
    pub fn get_64_sprites(&self) -> [PpuSprite; 64] {
        let mut s: [PpuSprite; 64] = [PpuSprite::new(); 64];
        for (i, e) in s.iter_mut().enumerate() {
            e.y = self.oam[i * 4];
            e.tile = self.oam[1 + i * 4];
            e.attribute = self.oam[2 + i * 4];
            e.x = self.oam[3 + i * 4];
        }
        s
    }

    /// Performs a dump of the ppu without side effects.
    pub fn dump(&self, addr: u16) -> Option<u8> {
        match addr {
            0 | 1 | 3 | 5 | 6 => Some(self.last_cpu_data),
            4 => {
                let mut data = self.oam[self.oamaddress as usize];
                if (self.oamaddress & 3) == 2 {
                    data &= 0xe3;
                }
                Some(data)
            }
            7 => match self.vram_address {
                0..=0x3eff => Some(self.ppudata_buffer),
                _ => Some(self.last_cpu_data & 0xC0),
            },
            _ => {
                let mut val = self.registers[addr as usize];
                if addr == 2 {
                    val = (val & 0xE0) | self.last_cpu_data & 0x1f;
                }
                Some(val)
            }
        }
    }

    /// Perform reads done by the cpu.
    pub fn read(&mut self, addr: u16) -> Option<u8> {
        match addr {
            0 | 1 | 3 | 5 | 6 => Some(self.last_cpu_data),
            4 => {
                let mut data = self.oam[self.oamaddress as usize];
                if (self.oamaddress & 3) == 2 {
                    data &= 0xe3;
                    self.last_cpu_data = data;
                    self.last_cpu_counter[0] = 893420;
                    self.last_cpu_counter[1] = 893420;
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
                    self.vblank_clear = true;
                    val = (val & 0xE0) | self.last_cpu_data & 0x1f;
                    self.last_cpu_data = (self.last_cpu_data & 0x1f) | (val & 0xE0);
                    self.last_cpu_counter[1] = 893420;
                }
                Some(val)
            }
        }
    }

    /// Perform writes done by the cpu.
    pub fn write(&mut self, addr: u16, data: u8) {
        self.last_cpu_data = data;
        self.last_cpu_counter[0] = 893420;
        self.last_cpu_counter[1] = 893420;
        match addr {
            0 | 1 | 5 | 6 => {
                if self.write_ignore_counter >= PPU_STARTUP_CYCLE_COUNT {
                    match addr {
                        0 => {
                            self.registers[0] = data;
                            self.temporary_vram_address =
                                (self.temporary_vram_address & 0x73FF) | (data as u16 & 3) << 10;
                        }
                        5 => {
                            if !self.address_bit {
                                self.temporary_vram_address =
                                    (self.temporary_vram_address & !0x1F) | (data as u16) >> 3;
                                self.scrollx = data & 7;
                            } else {
                                let t1 = (data as u16 & 7) << 12;
                                let t2 = (data as u16 & 0xF8) << 5;
                                self.temporary_vram_address =
                                    (self.temporary_vram_address & 0x0C1F) | t1 | t2;
                            }
                            self.address_bit = !self.address_bit;
                        }
                        6 => {
                            if !self.address_bit {
                                self.registers[addr as usize] = data;
                                self.temporary_vram_address = (self.temporary_vram_address & 0xFF)
                                    | (data as u16 & 0x3F) << 8;
                                self.address_bit = !self.address_bit;
                            } else {
                                self.temporary_vram_address =
                                    (self.temporary_vram_address & 0x7F00) | data as u16;
                                self.vram_address = self.temporary_vram_address;
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
            7 => {
                if let 0..=0x3eff = self.vram_address {
                    self.pend_vram_write = Some(data);
                }
            }
            2 => {}
            _ => {
                self.registers[addr as usize] = data;
            }
        }
    }

    /// Increment the vram address by a horizontal ammount of 1
    fn increment_horizontal_position(&mut self) {
        if (self.vram_address & 0x1F) == 0x1F {
            self.vram_address = (self.vram_address & !0x1F) ^ 0x400;
        } else {
            self.vram_address += 1;
        }
    }

    /// Increment the vram address by a vertical amount of 1
    fn increment_vertical_position(&mut self) {
        if (self.vram_address & 0x7000) != 0x7000 {
            self.vram_address += 0x1000;
        } else {
            self.vram_address &= !0x7000;
            let cy = (self.vram_address & 0x3E0) >> 5;
            let y = if cy == 29 {
                self.vram_address ^= 0x800;
                0
            } else if cy == 31 {
                0
            } else {
                cy + 1
            };
            self.vram_address = (self.vram_address & !0x3E0) | (y as u16) << 5;
        }
    }

    /// Copy horizontal information from temporary to actual vram address
    fn transfer_horizontal_position(&mut self) {
        let mask = 0x41F;
        self.vram_address = (self.vram_address & !mask) | (self.temporary_vram_address & mask);
    }

    /// Copy vertical information from temporary to actual vram address
    fn transfer_vertical_position(&mut self) {
        let mask = 0x7BE0;
        self.vram_address = (self.vram_address & !mask) | (self.temporary_vram_address & mask);
    }

    /// This increments the scanline cycle machine, sweeping across every scanline, and down every row sequentially.
    fn increment_scanline_cycle(&mut self) {
        if self.should_render_background() {
            if self.scanline_number < 240 || self.scanline_number == 261 {
                match self.scanline_cycle {
                    256 => {
                        self.increment_vertical_position();
                    }
                    257 => {
                        self.transfer_horizontal_position();
                    }
                    _ => {}
                }
                if (self.scanline_cycle >= 328 || self.scanline_cycle <= 256)
                    && self.scanline_cycle != 0
                {
                    if (self.scanline_cycle & 7) == 0 {
                        self.increment_horizontal_position();
                    }
                }
            }
            if self.scanline_number == 261 {
                if (280..=304).contains(&self.scanline_cycle) {
                    self.transfer_vertical_position();
                }
            }
        }
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

    /// Calculates the base for the name table, taking into account x and y scrolling
    fn nametable_base(&self) -> u16 {
        match self.registers[0] & PPU_REGISTER0_NAMETABLE_BASE {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            _ => 0x2c00,
        }
    }

    /// Calculates the coordinates for the name table, taking into account x and y scrolling
    fn nametable_coordinates(&self, x: u8, y: u8) -> (u16, u8, u8) {
        let mut base = self.nametable_base();
        let (x, ox) = x.overflowing_add(self.scrollx);
        if ox {
            base ^= 0x400;
        }
        let y = y as u16;
        if y > 240 {
            base ^= 0x800;
        }
        (base, x, (y % 240) as u8)
    }

    /// Calculates the base for the attribute table, taking into account x and y scrolling
    fn attributetable_base(&self) -> u16 {
        match self.registers[0] & PPU_REGISTER0_NAMETABLE_BASE {
            0 => 0x23c0,
            1 => 0x27c0,
            2 => 0x2bc0,
            _ => 0x2fc0,
        }
    }

    /// Calculates the coordinates for the attribute table, taking into account x and y scrolling
    fn attributetable_coordinates(&self, x: u8, y: u8) -> (u16, u8, u8) {
        let mut base = self.attributetable_base();
        let (x, ox) = x.overflowing_add(self.scrollx);
        if ox {
            base ^= 0x400;
        }
        let y = y as u16;
        if y > 240 {
            base ^= 0x800;
        }
        (base, x, (y % 240) as u8)
    }

    /// Returns the pattern table base for the sprites.
    fn sprite_patterntable_base(&self, spr: &PpuSprite) -> u16 {
        if self.sprite_height() == 16 {
            if (spr.tile & 1) == 0 {
                0
            } else {
                0x1000
            }
        } else {
            if (self.registers[0] & PPU_REGISTER0_SPRITE_SIZE) != 0
                || (self.registers[0] & PPU_REGISTER0_SPRITETABLE_BASE) == 0
            {
                0
            } else {
                0x1000
            }
        }
    }

    /// Returns the pattern table base for the background.
    fn background_patterntable_base(&self) -> u16 {
        if (self.registers[0] & PPU_REGISTER0_BACKGROUND_PATTERNTABLE_BASE) == 0 {
            0
        } else {
            0x1000
        }
    }

    /// Returns true when any part of the background should be rendered
    fn should_render_background(&self) -> bool {
        (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN) != 0
            || (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND) != 0
    }

    /// Returns true when the background should be rendered on the given cycle.
    fn should_render_background_cycle(&self, cycle: u8) -> bool {
        if cycle < 8 {
            (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND_FIRST_COLUMN) != 0
        } else {
            (self.registers[1] & PPU_REGISTER1_DRAW_BACKGROUND) != 0
        }
    }

    /// Returns true when sprites should be rendered.
    fn should_render_sprites(&self, cycle: u8) -> bool {
        if cycle == 0 {
            (self.registers[1] & PPU_REGISTER1_DRAW_SPRITES_FIRST_COLUMN) != 0
        } else {
            (self.registers[1] & PPU_REGISTER1_DRAW_SPRITES) != 0
        }
    }

    /// Computes the xy coordinates to be used by the background fetcher.
    fn compute_xy(&self, cycle: u16, offset: u8) -> (u8, u8) {
        let (cycle2, ox) = cycle.overflowing_add(offset as u16);
        let mut scanline = self.scanline_number;
        if ox {
            scanline = (scanline + 1) % 240;
        }

        ((cycle2 & 0xFF) as u8, scanline as u8)
    }

    /// Performs fetches for the background data of the ppu.
    fn background_fetch(&mut self, bus: &mut NesMotherboard, cycle: u16) {
        let (x, y) = self.compute_xy(cycle, 16);
        match (cycle / 2) % 4 {
            0 => {
                if (cycle & 1) == 0 {
                    //nametable byte
                    let (base, x, y) = self.nametable_coordinates(x, y);
                    let offset = (y as u16 / 8) << 5 | (x as u16 / 8);
                    let calc = base + offset;
                    let calc2 = 0x2000 | (self.vram_address & 0xFFF);
                    bus.ppu_cycle_1(calc2);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    self.prev_nametable_data = self.nametable_data;
                    self.nametable_data = bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
                }
            }
            1 => {
                //attribute table byte
                if (cycle & 1) == 0 {
                    let calc2 = 0x23C0
                        | (self.vram_address & 0x0C00)
                        | ((self.vram_address >> 4) & 0x38)
                        | ((self.vram_address >> 2) & 0x07);
                    bus.ppu_cycle_1(calc2);
                    self.cycle1_done = true;
                } else if self.cycle1_done {
                    self.attributetable_data = bus.ppu_cycle_2_read();
                    self.cycle1_done = false;
                }
            }
            2 => {
                //pattern table tile low
                if (cycle & 1) == 0 {
                    let base = self.background_patterntable_base();
                    let offset = (self.nametable_data as u16) << 4;
                    let calc = base + offset + ((7 + (self.vram_address) >> 12) & 7);
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
                    let base = self.background_patterntable_base();
                    let offset = (self.nametable_data as u16) << 4;
                    let calc = 8 + base + offset + ((7 + (self.vram_address) >> 12) & 7);
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

    /// Allows for cpu operations to read and write to ppu vram
    fn idle_operation(&mut self, bus: &mut NesMotherboard, cycle: u16) {
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

    /// Returns the height of sprites, by examining the current configuration of the ppu
    fn sprite_height(&self) -> u8 {
        let big = (self.registers[0] & PPU_REGISTER0_SPRITE_SIZE) != 0;
        if big {
            16
        } else {
            8
        }
    }

    /// Evaluate sprites in the ppu, condensing 64 sprites down to 8 sprites for a single scanline.
    /// This function operates "asynchronously", requiring multiple calls to perform the entire job for any given scanline.
    fn sprite_eval(&mut self) {
        if self.scanline_number < 240 {
            let row = (self.scanline_number + 1) as u8;
            match self.scanline_cycle {
                0 => {
                    //TODO: trigger bug that occurs when oamaddress is nonzero
                    //it copies 8 bytes of sprite data
                    self.oamaddress = 0;
                    self.secondaryoamaddress = 0;
                }
                1..=64 => {
                    if (self.scanline_cycle & 1) == 0 {
                        self.secondary_oam[((self.scanline_cycle >> 1) - 1) as usize] = 0xff;
                    }
                }
                65..=256 => {
                    if self.scanline_cycle == 65 {
                        self.sprite_eval_mode = PpuSpriteEvalMode::Normal;
                    }
                    if (self.scanline_cycle & 1) == 1 {
                        self.oamdata = self.oam[self.oamaddress as usize];
                    } else {
                        match self.sprite_eval_mode {
                            PpuSpriteEvalMode::Normal => {
                                //range check
                                if row < 240
                                    && row >= self.oamdata
                                    && row < (self.oamdata + self.sprite_height())
                                {
                                    self.secondary_oam[self.secondaryoamaddress as usize] =
                                        self.oamdata;
                                    self.oamaddress = self.oamaddress.wrapping_add(1);
                                    self.secondaryoamaddress += 1;
                                    self.sprite_eval_mode = PpuSpriteEvalMode::CopyCurrentSprite;
                                } else {
                                    self.oamaddress = self.oamaddress.wrapping_add(4);
                                    if self.oamaddress == 0 {
                                        self.sprite_eval_mode = PpuSpriteEvalMode::Done;
                                    }
                                }
                            }
                            PpuSpriteEvalMode::Sprites8 => {
                                if row < 240
                                    && row >= self.oamdata
                                    && row < (self.oamdata + self.sprite_height())
                                {
                                    self.sprite_eval_mode = PpuSpriteEvalMode::Done;
                                    self.registers[2] |= 0x20; //the sprite overflow flag
                                    self.oamaddress = self.oamaddress.wrapping_add(1);
                                } else {
                                    self.oamaddress = self.oamaddress.wrapping_add(4);
                                    if self.oamaddress < 4 {
                                        self.sprite_eval_mode = PpuSpriteEvalMode::Done;
                                    }
                                }
                            }
                            PpuSpriteEvalMode::CopyCurrentSprite => {
                                self.secondary_oam[self.secondaryoamaddress as usize] =
                                    self.oamdata;
                                self.oamaddress = self.oamaddress.wrapping_add(1);
                                self.secondaryoamaddress += 1;
                                if (self.secondaryoamaddress & 3) == 0 {
                                    //done copying the sprite
                                    if self.oamaddress == 0 {
                                        //done checking all 64 sprites
                                        self.sprite_eval_mode = PpuSpriteEvalMode::Done;
                                    } else if self.secondaryoamaddress == 32 {
                                        //found 8 sprites already
                                        self.sprite_eval_mode = PpuSpriteEvalMode::Sprites8;
                                    } else {
                                        self.sprite_eval_mode = PpuSpriteEvalMode::Normal;
                                    }
                                }
                            }
                            PpuSpriteEvalMode::Done => {
                                self.oamaddress = self.oamaddress.wrapping_add(4);
                            }
                        }
                    }
                }
                257..=320 => {
                    let cycle = self.scanline_cycle - 257;
                    let sprite = cycle / 4;
                    let index = cycle % 4;
                    if sprite < 8 {
                        match index {
                            0 => {
                                self.sprites[sprite as usize].y = self.secondary_oam[cycle as usize]
                            }
                            1 => {
                                self.sprites[sprite as usize].tile =
                                    self.secondary_oam[cycle as usize]
                            }
                            2 => {
                                self.sprites[sprite as usize].attribute =
                                    self.secondary_oam[cycle as usize]
                            }
                            3 => {
                                self.sprites[sprite as usize].x = self.secondary_oam[cycle as usize]
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Run a single clock cycle of the ppu
    pub fn cycle(&mut self, bus: &mut NesMotherboard) {
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

        if self.should_render_sprites(0)
            || self.should_render_sprites(8)
            || self.should_render_background()
        {
            self.sprite_eval();
        }

        //if else chain allows the constants to be changed later to variables
        //to allow for ntsc/pal to be emulated
        if self.scanline_number < 240 {
            if self.scanline_cycle == 0 {
                //idle cycle
                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 256 {
                //each cycle here renders a single pixel
                let cycle = (self.scanline_cycle - 1) as u8;
                let bg_pixel = if self.should_render_background_cycle(cycle) {
                    let prev_tile = ((self.scrollx & 7) + (cycle & 7)) > 7;
                    let index = 7 - ((cycle.wrapping_add(self.scrollx)) % 8);
                    let pt = if !prev_tile {
                        self.patterntable_shift[0].to_le_bytes()
                    } else {
                        self.patterntable_shift[1].to_le_bytes()
                    };
                    let upper_bit = (pt[1] >> index) & 1;
                    let lower_bit = (pt[0] >> index) & 1;

                    let modx = (((cycle as u16 + self.scrollx as u16) / 16) & 1) as u8;
                    let mody = ((((self.scanline_number) % 240) / 16) & 1) as u8;
                    let combined = (mody << 1) | modx;
                    let attribute = if !prev_tile {
                        self.attributetable_shift[0]
                    } else {
                        self.attributetable_shift[1]
                    };
                    let extra_palette_bits = (attribute >> (2 * combined)) & 3;
                    let lower_bits = (upper_bit << 1) | lower_bit;

                    let mut palette_entry = if lower_bits == 0 {
                        0
                    }
                    else {
                        ((extra_palette_bits << 2) | lower_bits) as u16
                    };
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
                    Some(pixel_entry)
                } else {
                    None
                };
                if self.should_render_background() {
                    self.background_fetch(bus, cycle as u16);
                } else {
                    self.idle_operation(bus, cycle as u16);
                }
                let spr_pixel: Option<(usize, u8)> = if self.should_render_sprites(cycle) {
                    let mut sprite_pixels =
                        self.sprites.iter().enumerate().filter_map(|(index, e)| {
                            if cycle >= e.x && (cycle < (e.x.wrapping_add(8))) && e.y < 240 {
                                let index2 = if (e.attribute & 0x40) == 0 {
                                    7 - (cycle - e.x)
                                } else {
                                    cycle - e.x
                                };
                                let pt = e.patterntable_data.to_le_bytes();
                                let upper_bit = (pt[1] >> index2) & 1;
                                let lower_bit = (pt[0] >> index2) & 1;

                                if upper_bit != 0 || lower_bit != 0 {
                                    let mut palette_entry =
                                        e.pallete() | ((upper_bit << 1) | lower_bit) as u16;
                                    if (self.registers[1] & PPU_REGISTER1_GREYSCALE) != 0 {
                                        palette_entry &= 0x30;
                                    }
                                    let pixel_entry =
                                        bus.ppu_palette_read(0x3f10 | palette_entry) & 63;
                                    Some((index, pixel_entry))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        });
                    sprite_pixels.next()
                } else {
                    None
                };

                let pixel_entry = if let Some((index, spr)) = spr_pixel {
                    if bg_pixel.is_some() && index == 0 {
                        self.registers[2] |= 0x40; //sprite 0 hit
                    }
                    spr
                } else if let Some(bg) = bg_pixel {
                    bg
                } else {
                    0x0d
                };

                let pixel = PPU_PALETTE[pixel_entry as usize];
                self.frame_data
                    [((self.scanline_number * 256 + cycle as u16) as u32 * 3) as usize] = pixel[0];
                self.frame_data
                    [((self.scanline_number * 256 + cycle as u16) as u32 * 3 + 1) as usize] =
                    pixel[1];
                self.frame_data
                    [((self.scanline_number * 256 + cycle as u16) as u32 * 3 + 2) as usize] =
                    pixel[2];

                self.increment_scanline_cycle();
            } else if self.scanline_cycle <= 320 {
                //sprite rendering data to be fetched
                let cycle = self.scanline_cycle - 257;
                let sprite_num = cycle / 8;
                let row = self.scanline_number;
                if cycle > 0 {
                    if self.should_render_sprites(cycle as u8) {
                        match (cycle / 2) % 4 {
                            0 => {
                                if (cycle & 1) == 0 {
                                    //nametable byte
                                    let x = cycle as u8 / 8;
                                    let y = (row / 8) as u8;
                                    let base = self.nametable_base();
                                    let offset = y << 5 | x;
                                    bus.ppu_cycle_1(base + offset as u16); //TODO verify this calculation
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    bus.ppu_cycle_2_read();
                                    self.cycle1_done = false;
                                }
                            }
                            1 => {
                                if (cycle & 1) == 0 {
                                    //nametable byte
                                    let cycle = cycle & !2;
                                    let x = cycle as u8 / 8;
                                    let y = (row / 8) as u8;
                                    let base = self.nametable_base();
                                    let offset = y << 5 | x;
                                    bus.ppu_cycle_1(base + offset as u16); //TODO verify this calculation
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    bus.ppu_cycle_2_read();
                                    self.cycle1_done = false;
                                }
                            }
                            2 => {
                                //pattern table tile low
                                if (cycle & 1) == 0 {
                                    let base = self.sprite_patterntable_base(
                                        &self.sprites[sprite_num as usize],
                                    );
                                    let offset = self.sprites[sprite_num as usize]
                                        .tile_num(row as u8, self.sprite_height());
                                    let o2 = self.sprites[sprite_num as usize]
                                        .line_number(row as u8)
                                        as u16;
                                    let calc = base + offset + o2;
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
                                    let base = self.sprite_patterntable_base(
                                        &self.sprites[sprite_num as usize],
                                    );
                                    let offset = self.sprites[sprite_num as usize]
                                        .tile_num(row as u8, self.sprite_height());
                                    let o2 = self.sprites[sprite_num as usize]
                                        .line_number(row as u8)
                                        as u16;
                                    let calc = 8 + base + offset + o2;
                                    bus.ppu_cycle_1(calc);
                                    self.cycle1_done = true;
                                } else if self.cycle1_done {
                                    let mut pt = self.patterntable_tile.to_le_bytes();
                                    pt[1] = bus.ppu_cycle_2_read();
                                    self.cycle1_done = false;
                                    self.patterntable_tile = u16::from_le_bytes(pt);
                                    self.sprites[sprite_num as usize].patterntable_data =
                                        self.patterntable_tile;
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
                self.background_fetch(bus, self.scanline_cycle - 1);
                //self.idle_operation(bus, self.scanline_cycle - 1);
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
                #[cfg(any(test, feature = "debugger"))]
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
        self.vblank_nmi = ((self.registers[2] & 0x80) != 0)
            & ((self.registers[0] & PPU_REGISTER0_GENERATE_NMI) != 0);
    }

    /// Returns true if the frame has ended. Used for frame rate synchronizing.
    pub fn get_frame_end(&mut self) -> bool {
        let flag = self.frame_end;
        self.frame_end = false;
        flag
    }

    /// Returns a reference to the frame data stored in the ppu.
    pub fn get_frame(&mut self) -> &[u8; 256 * 240 * 3] {
        &self.frame_data
    }

    /// Renders all sprites
    pub fn render_sprites(&self, buf: &mut Box<RgbImage>, bus: &NesMotherboard) {
        let allsprites = self.get_64_sprites();
        for (i, pixel) in buf.data.chunks_exact_mut(3).enumerate() {
            let trow: u16 = (i as u16 / 128) as u16;
            let tcolumn: u8 = ((i % 128) / 8) as u8;
            let spritex: u8 = (i & 7) as u8;
            let spritey: u8 = (trow & 15) as u8;
            let spritenum: u8 = (tcolumn) + ((trow / 16) * 16) as u8;
            let sprite = allsprites[spritenum as usize];
            // zero out pixel first, in case sprite height changes
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;

            let base = self.sprite_patterntable_base(&sprite);
            let offset = sprite.tile_num(sprite.y + spritey as u8, self.sprite_height());
            let o2 = sprite.line_number(sprite.y + spritey as u8) as u16;
            let calc = base + offset + o2;
            let pattern_low = bus.ppu_peek(calc);
            let pattern_high = bus.ppu_peek(8 + calc);

            let index2 = if (sprite.attribute & 0x40) == 0 {
                7 - spritex
            } else {
                spritex
            };
            let upper_bit = (pattern_high >> index2) & 1;
            let lower_bit = (pattern_low >> index2) & 1;

            if upper_bit != 0 || lower_bit != 0 {
                let mut palette_entry = sprite.pallete() | ((upper_bit << 1) | lower_bit) as u16;
                if (self.registers[1] & PPU_REGISTER1_GREYSCALE) != 0 {
                    palette_entry &= 0x30;
                }
                let pixel_entry = bus.ppu_palette_read(0x3f10 | palette_entry) & 63;
                let p = PPU_PALETTE[pixel_entry as usize];
                pixel[0] = p[0];
                pixel[1] = p[1];
                pixel[2] = p[2];
            }
        }
    }

    /// Renders a nametable pixel, returning the palette entry
    pub fn render_nametable_pixel_address(&self, nametable: u8, x: u8, y: u8, bus: &NesMotherboard) -> u16 {
        let quadrant = nametable;
        let row = y;
        let col = x;
        let base_address = 0x2000 + 0x400 * quadrant as u16;
        let offset = (row as u16 / 8) << 5 | (col as u16 / 8);
        let nametable = bus.ppu_peek(base_address + offset);

        let base_address = match quadrant {
            0 => 0x23c0,
            1 => 0x27c0,
            2 => 0x2bc0,
            _ => 0x2fc0,
        };
        let offset = (row as u16 / 32) << 3 | (col as u16 / 32);
        let attribute = bus.ppu_peek(base_address + offset);

        let table = self.background_patterntable_base();
        let base = table;
        let offset = (nametable as u16) << 4;
        let calc = base + offset + (row as u16) % 8;

        let data_low = bus.ppu_peek(calc);
        let data_high = bus.ppu_peek(calc + 8);

        let index = 7 - (col % 8);
        let upper_bit = (data_high >> index) & 1;
        let lower_bit = (data_low >> index) & 1;

        let modx = ((col as u8) / 16) & 1;
        let mody = (((row as u16) / 16) & 1) as u8;
        let combined = (mody << 1) | modx;
        let extra_palette_bits = (attribute >> (2 * combined)) & 3;
        let lower_bits = (upper_bit << 1) | lower_bit;

        let mut palette_entry = if lower_bits == 0 {
            0
        }
        else {
            ((extra_palette_bits << 2) | lower_bits) as u16
        };
        if (self.registers[1] & PPU_REGISTER1_GREYSCALE) != 0 {
            palette_entry &= 0x30;
        }
        0x3f00 + palette_entry
    }

    /// Renders the entire nametable into the given buffer
    pub fn render_nametable(&self, buf: &mut Box<RgbImage>, bus: &NesMotherboard) {
        for (i, pixel) in buf.data.chunks_exact_mut(3).enumerate() {
            let col = i & 0xFF;
            let row = i / 512;
            let left = (i % 512) < 256;
            let top = (i / 512) < 240;
            let quadrant = match (left, top) {
                (true, true) => 0,
                (false, true) => 1,
                (true, false) => 2,
                (false, false) => 3,
            };
            let row = row % 240;

            let address = self.render_nametable_pixel_address(quadrant, col as u8, row as u8, bus);
            let pixel_entry = bus.ppu_palette_read(address) & 63;

            let p = PPU_PALETTE[pixel_entry as usize];
            pixel[0] = p[0];
            pixel[1] = p[1];
            pixel[2] = p[2];
        }
    }

    /// Renders the entire pattern table into the given buffer
    pub fn render_pattern_table(&self, buf: &mut Box<RgbImage>, bus: &NesMotherboard) {
        for (i, pixel) in buf.data.chunks_exact_mut(3).enumerate() {
            let column = (i & 0x78) >> 3;
            let row = (i & 0xF800) >> 11;
            let table = if (i & 0x80) != 0 { 0x1000 } else { 0 };
            let pattern = column + row * 16;
            let irow = (i & 0x700) >> 8;

            let base = table as u16;
            let offset = (pattern as u16) << 4;
            let calc = base + offset + (irow as u16) % 8;
            if i < 512 {
                println!("Address {:x} -> {:x}", i, calc);
            }
            let data_low = bus.ppu_peek(calc);
            let data_high = bus.ppu_peek(calc + 8);

            let index = 7 - (i % 8);
            let upper_bit = (data_high >> index) & 1;
            let lower_bit = (data_low >> index) & 1;

            let mut palette_entry = ((upper_bit << 1) | lower_bit) as u16;
            if (self.registers[1] & PPU_REGISTER1_GREYSCALE) != 0 {
                palette_entry &= 0x30;
            }
            let pixel_entry = bus.ppu_palette_read(0x3f00 + palette_entry) & 63;
            let p = PPU_PALETTE[pixel_entry as usize];
            pixel[0] = p[0];
            pixel[1] = p[1];
            pixel[2] = p[2];
        }
    }

    /// Returns the irq status for the ppu
    pub fn irq(&self) -> bool {
        self.vblank_nmi
    }

    /// Converts the data in the given reference (from this module usually), into a form that sdl2 can use directly.
    #[cfg(feature = "sdl2")]
    pub fn convert_for_sdl2(f: &[u8; 256 * 240 * 3], buf: &mut Vec<egui_sdl2_gl::egui::Color32>) {
        let pixels: Vec<egui_sdl2_gl::egui::Color32> = f
            .chunks_exact(3)
            .map(|p| egui_sdl2_gl::egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        *buf = pixels;
    }

    /// Converts the data in the given reference (from this module usually), into a form that egui can use directly.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub fn convert_to_egui(f: &[u8; 256 * 240 * 3]) -> egui::ColorImage {
        let pixels = f
            .chunks_exact(3)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        egui::ColorImage {
            size: [256, 240],
            pixels,
        }
    }
}
