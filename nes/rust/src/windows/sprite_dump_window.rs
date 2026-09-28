//! This module is for the window that dumps ppu name table information.
use crate::NesEmulatorData;
use common_emulator::video::RgbImage;

use eframe::egui;

/// The window for dumping ppu nametable data
pub struct DumpWindow {
    /// The image to use for the dump
    buf: Box<RgbImage>,
    /// The image for the palette dump
    palette: Box<RgbImage>,
    /// The texture used for rendering the image.
    texture: Option<egui::TextureHandle>,
    /// The texture for the palette
    texture2: Option<egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a new self.
    pub fn new() -> Self {
        DumpWindow {
            buf: Box::new(RgbImage::new(128, 64)),
            palette: Box::new(RgbImage::new(16, 2)),
            texture: None,
            texture2: None,
        }
    }
}

impl DumpWindow {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("GENIE_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("Geme genie")
                .with_inner_size([400.0, 300.0]),
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("PPU Sprite Dump Window");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        #[cfg(feature = "debugger")]
                        {
                            c.cpu_peripherals.ppu.render_sprites(&mut self.buf, &c.mb);
                        }
                        c.cpu_peripherals
                            .ppu
                            .render_palette(&mut self.palette, &c.mb);
                        let image = self.buf.to_pixels_egui().to_egui();
                        if self.texture.is_none() {
                            self.texture = Some(ui.ctx().load_texture(
                                "NES_PPU_SPRITES",
                                image,
                                egui::TextureOptions::NEAREST,
                            ));
                        } else if let Some(t) = &mut self.texture {
                            t.set_partial([0, 0], image, egui::TextureOptions::NEAREST);
                        }
                        let image2 = self.palette.to_pixels_egui().to_egui();
                        if self.texture2.is_none() {
                            self.texture2 = Some(ui.ctx().load_texture(
                                "NES_PPU_PALETTE",
                                image2,
                                egui::TextureOptions::NEAREST,
                            ));
                        } else if let Some(t) = &mut self.texture2 {
                            t.set_partial([0, 0], image2, egui::TextureOptions::NEAREST);
                        }
                        let mut r = None;
                        let zoom = 5.0;
                        if let Some(t) = &self.texture {
                            r = Some(ui.add(egui::Image::from_texture(egui::load::SizedTexture {
                                id: t.id(),
                                size: egui::Vec2 {
                                    x: self.buf.width as f32 * zoom,
                                    y: self.buf.height as f32 * zoom,
                                },
                            })));
                        }
                        if let Some(t) = &self.texture2 {
                            let zoom = 16.0;
                            let _r = ui.add(egui::Image::from_texture(egui::load::SizedTexture {
                                id: t.id(),
                                size: egui::Vec2 {
                                    x: self.palette.width as f32 * zoom,
                                    y: self.palette.height as f32 * zoom,
                                },
                            }));
                        }
                        if let Some(r) = r {
                            if r.hovered() {
                                if let Some(cursor) = r.hover_pos() {
                                    let pos = cursor - r.rect.left_top();
                                    if pos.x >= 0.0 && pos.y >= 0.0 {
                                        let x = (pos.x / (8.0 * zoom)).floor() as usize;
                                        let y = (pos.y / (16.0 * zoom)).floor() as usize;
                                        let col = x & 15;
                                        let row = y & 3;
                                        let num = col + row * 16;

                                        ui.label(format!("Sprite number is {:x}", num));
                                        #[cfg(feature = "debugger")]
                                        {
                                            let sprites = c.cpu_peripherals.ppu.get_64_sprites();
                                            ui.label(format!(
                                                "Sprite tile is {:x} {:x}, attribute is {:x}",
                                                sprites[num].tile_num(sprites[num].y(), 16),
                                                sprites[num].tile_num(sprites[num].y() + 8, 16),
                                                sprites[num].attribute(),
                                            ));
                                            ui.label(format!(
                                                "Location is {},{}",
                                                sprites[num].x(),
                                                sprites[num].y()
                                            ));
                                        }
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
