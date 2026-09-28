//! This module is for the window that dumps the ppu pattern tables.

use crate::NesEmulatorData;
use common_emulator::video::RgbImage;

use eframe::egui;

/// The window for dumping cartridge program data
pub struct DumpWindow {
    /// The image to use for the dump
    buf: Box<RgbImage>,
    /// The texture used for rendering the image.
    texture: Option<egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        DumpWindow {
            buf: Box::new(RgbImage::new(256, 128)),
            texture: None,
        }
    }
}

impl DumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("PATTERN_TABLE_DUMP_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Pattern Table Dump")
                .with_inner_size([400.0, 300.0]),
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("PPU Pattern Table Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        c.cpu_peripherals
                            .ppu
                            .render_pattern_table(&mut self.buf, &c.mb);
                        let image = self.buf.to_pixels_egui().to_egui();
                        if self.texture.is_none() {
                            self.texture = Some(ui.ctx().load_texture(
                                "NES_PPU",
                                image,
                                egui::TextureOptions::NEAREST,
                            ));
                        } else if let Some(t) = &mut self.texture {
                            t.set_partial([0, 0], image, egui::TextureOptions::NEAREST);
                        }
                        if let Some(t) = &self.texture {
                            let zoom = 2.0;
                            let r = ui.add(egui::Image::from_texture(egui::load::SizedTexture {
                                id: t.id(),
                                size: egui::Vec2 {
                                    x: self.buf.width as f32 * zoom,
                                    y: self.buf.height as f32 * zoom,
                                },
                            }));
                            if r.hovered() {
                                if let Some(cursor) = r.hover_pos() {
                                    let pos = cursor - r.rect.left_top();
                                    if pos.x >= 0.0 && pos.y >= 0.0 {
                                        let x = (pos.x / (8.0 * zoom)).floor() as usize;
                                        let y = (pos.y / (8.0 * zoom)).floor() as usize;
                                        let col = x & 15;
                                        let second = (x & !0xF) != 0;
                                        let row = y;
                                        let tilenum = col + row * 16 + if second { 256 } else { 0 };

                                        ui.label(format!("Coordinate {},{}", col, row));
                                        ui.label(format!("Tile number is {:x}", tilenum));
                                    }
                                }
                            }
                        }
                    });
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            },
        );
    }
}
