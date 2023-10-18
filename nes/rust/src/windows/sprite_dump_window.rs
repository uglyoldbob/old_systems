//! This module is for the window that dumps ppu name table information.
use crate::{ppu::RgbImage, NesEmulatorData};

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
    egui,
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping ppu nametable data
pub struct DumpWindow {
    /// The image to use for the dump
    buf: Box<RgbImage>,
    /// The image for the palette dump
    palette: Box<RgbImage>,
    /// The texture used for rendering the image.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    texture: Option<egui::TextureHandle>,
    /// The texture for the palette
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    texture2: Option<egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DumpWindow {
                buf: Box::new(RgbImage::new(128, 64)),
                palette: Box::new(RgbImage::new(16, 2)),
                texture: None,
                texture2: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 1024.0,
                    height: 768.0,
                })
                .with_title("UglyOldBob NES PPU Sprite Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for DumpWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<NesEmulatorData> {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("PPU Sprite Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                c.cpu_peripherals.ppu.render_sprites(&mut self.buf, &c.mb);
                c.cpu_peripherals
                    .ppu
                    .render_palette(&mut self.palette, &c.mb);
                let image = self.buf.to_pixels_egui().to_egui();
                if self.texture.is_none() {
                    self.texture = Some(egui.egui_ctx.load_texture(
                        "NES_PPU_SPRITES",
                        image,
                        egui_multiwin::egui::TextureOptions::NEAREST,
                    ));
                } else if let Some(t) = &mut self.texture {
                    t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
                }
                let image2 = self.palette.to_pixels_egui().to_egui();
                if self.texture2.is_none() {
                    self.texture2 = Some(egui.egui_ctx.load_texture(
                        "NES_PPU_PALETTE",
                        image2,
                        egui_multiwin::egui::TextureOptions::NEAREST,
                    ));
                } else if let Some(t) = &mut self.texture2 {
                    t.set_partial([0, 0], image2, egui_multiwin::egui::TextureOptions::NEAREST);
                }
                let mut r = None;
                let zoom = 5.0;
                if let Some(t) = &self.texture {
                    r = Some(ui.add(egui_multiwin::egui::Image::from_texture(
                        egui_multiwin::egui::load::SizedTexture {
                            id: t.id(),
                            size: egui_multiwin::egui::Vec2 {
                                x: self.buf.width as f32 * zoom,
                                y: self.buf.height as f32 * zoom,
                            },
                        },
                    )));
                }
                if let Some(t) = &self.texture2 {
                    let zoom = 16.0;
                    let _r = ui.add(egui_multiwin::egui::Image::from_texture(
                        egui_multiwin::egui::load::SizedTexture {
                            id: t.id(),
                            size: egui_multiwin::egui::Vec2 {
                                x: self.palette.width as f32 * zoom,
                                y: self.palette.height as f32 * zoom,
                            },
                        },
                    ));
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
            });
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
