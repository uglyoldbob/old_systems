//! This module is for the window that dumps ppu name table information.
use crate::NesEmulatorData;

use common_emulator::video::RgbImage;

use eframe::egui;

/// The window for dumping ppu nametable data
pub struct DumpWindow {
    /// The image to use for the dump
    buf: Box<RgbImage>,
    /// The image for the attribute table
    buf2: Box<RgbImage>,
    /// The texture used for rendering the image.
    texture: Option<egui::TextureHandle>,
    /// The texture used for rendering the attribute table.
    texture2: Option<egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        DumpWindow {
            buf: Box::new(RgbImage::new(512, 480)),
            buf2: Box::new(RgbImage::new(512, 480)),
            texture: None,
            texture2: None,
        }
    }
}

impl DumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("NAMETABLE_DUMP_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Nametable dump")
                .with_inner_size([400.0, 300.0]),
            |ui, _class| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.label("PPU Name Table Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        c.cpu_peripherals.ppu.render_nametable(&mut self.buf, &c.mb);
                        c.cpu_peripherals
                            .ppu
                            .render_attribute_table(&mut self.buf2, &c.mb);
                        let image = self.buf.to_pixels_egui().to_egui();
                        let image2 = self.buf2.to_pixels_egui().to_egui();
                        if self.texture.is_none() {
                            self.texture = Some(ui.ctx().load_texture(
                                "NES_PPU",
                                image,
                                egui::TextureOptions::NEAREST,
                            ));
                        } else if let Some(t) = &mut self.texture {
                            t.set_partial([0, 0], image, egui::TextureOptions::NEAREST);
                        }
                        if self.texture2.is_none() {
                            self.texture2 = Some(ui.ctx().load_texture(
                                "NES_PPU",
                                image2,
                                egui::TextureOptions::NEAREST,
                            ));
                        } else if let Some(t) = &mut self.texture2 {
                            t.set_partial([0, 0], image2, egui::TextureOptions::NEAREST);
                        }
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                if let Some(t) = &self.texture {
                                    let zoom = 1.0;
                                    let r = ui.add(egui::Image::from_texture(
                                        egui::load::SizedTexture {
                                            id: t.id(),
                                            size: egui::Vec2 {
                                                x: self.buf.width as f32 * zoom,
                                                y: self.buf.height as f32 * zoom,
                                            },
                                        },
                                    ));

                                    if r.hovered() {
                                        if let Some(cursor) = r.hover_pos() {
                                            let pos = cursor - r.rect.left_top();
                                            if pos.x >= 0.0 && pos.y >= 0.0 {
                                                let pixelx = (pos.x / zoom).floor() as u8;
                                                let pixely = (pos.y / zoom).floor() as u8;
                                                #[cfg(feature = "debugger")]
                                                {
                                                    c.cpu_peripherals.ppu.bg_debug =
                                                        Some((pixelx, pixely));
                                                }
                                                let x = (pos.x / (8.0 * zoom)).floor() as usize;
                                                let y = (pos.y / (8.0 * zoom)).floor() as usize;
                                                let left = x < 32;
                                                let top = y < 30;
                                                let col = x & 0x1f;
                                                let row = y % 30;
                                                let table = match (left, top) {
                                                    (true, true) => 0,
                                                    (false, true) => 1,
                                                    (true, false) => 2,
                                                    (false, false) => 3,
                                                };
                                                let pix_x = (((pos.x / (zoom)).floor() as usize)
                                                    & 0xFF)
                                                    as u8;
                                                let pix_y = (((pos.y / (zoom)).floor() as usize)
                                                    % 240)
                                                    as u8;
                                                ui.label(format!(
                                                    "Coordinate {},{} {:x}",
                                                    pix_x, pix_y, table
                                                ));
                                                let addr = c
                                                    .cpu_peripherals
                                                    .ppu
                                                    .render_nametable_pixel_address(
                                                        table, pix_x, pix_y, &c.mb,
                                                    );
                                                let pixel_entry = c.mb.ppu_palette_read(addr) & 63;
                                                let ntaddr = 0x2000
                                                    + 0x400 * table as usize
                                                    + col
                                                    + row * 32;
                                                ui.label(format!(
                                                    "Palette address is {:x} {:x}",
                                                    addr, pixel_entry
                                                ));
                                                ui.label(format!(
                                                    "Tile address {},{} is {:x}={:x}",
                                                    col,
                                                    row,
                                                    ntaddr,
                                                    c.mb.ppu_peek(ntaddr as u16),
                                                ));

                                                let x = (pos.x / (32.0 * zoom)).floor() as usize;
                                                let y = (pos.y / (32.0 * zoom)).floor() as usize;
                                                let left = x < 32;
                                                let top = y < 30;
                                                let col = x & 0x7;
                                                let row = y % 8;
                                                let table = match (left, top) {
                                                    (true, true) => 0,
                                                    (false, true) => 1,
                                                    (true, false) => 2,
                                                    (false, false) => 3,
                                                };
                                                let pix_x = (((pos.x / (zoom)).floor() as usize)
                                                    & 0xFF)
                                                    as u8;
                                                let pix_y = (((pos.y / (zoom)).floor() as usize)
                                                    % 240)
                                                    as u8;
                                                ui.label(format!(
                                                    "Coordinate {},{} {:x}",
                                                    pix_x, pix_y, table
                                                ));
                                                let ntaddr =
                                                    0x23C0 + 0x400 * table as usize + col + row * 8;
                                                ui.label(format!(
                                                    "Attribute address {},{} is {:x}",
                                                    col, row, ntaddr
                                                ));
                                            }
                                        }
                                    }
                                }
                            });
                            ui.vertical(|ui| {
                                if let Some(t) = &self.texture2 {
                                    let zoom = 1.0;
                                    let r = ui.add(egui::Image::from_texture(
                                        egui::load::SizedTexture {
                                            id: t.id(),
                                            size: egui::Vec2 {
                                                x: self.buf2.width as f32 * zoom,
                                                y: self.buf2.height as f32 * zoom,
                                            },
                                        },
                                    ));

                                    if r.hovered() {
                                        if let Some(cursor) = r.hover_pos() {
                                            let pos = cursor - r.rect.left_top();
                                            if pos.x >= 0.0 && pos.y >= 0.0 {
                                                let x = (pos.x / (32.0 * zoom)).floor() as usize;
                                                let y = (pos.y / (32.0 * zoom)).floor() as usize;
                                                let left = x < 32;
                                                let top = y < 30;
                                                let col = x & 0x7;
                                                let row = y % 8;
                                                let table = match (left, top) {
                                                    (true, true) => 0,
                                                    (false, true) => 1,
                                                    (true, false) => 2,
                                                    (false, false) => 3,
                                                };
                                                let pix_x = (((pos.x / (zoom)).floor() as usize)
                                                    & 0xFF)
                                                    as u8;
                                                let pix_y = (((pos.y / (zoom)).floor() as usize)
                                                    % 240)
                                                    as u8;
                                                ui.label(format!(
                                                    "Coordinate {},{} {:x}",
                                                    pix_x, pix_y, table
                                                ));
                                                let _addr = c
                                                    .cpu_peripherals
                                                    .ppu
                                                    .render_nametable_pixel_address(
                                                        table, pix_x, pix_y, &c.mb,
                                                    );
                                                let ntaddr =
                                                    0x23C0 + 0x400 * table as usize + col + row * 8;
                                                ui.label(format!(
                                                    "Tile address {},{} is {:x}",
                                                    col, row, ntaddr
                                                ));
                                            }
                                        }
                                    }
                                }
                            });
                        });
                    });
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            },
        );
    }
}
