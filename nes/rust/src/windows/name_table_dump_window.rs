//! This module is for the window that dumps ppu name table information.
use crate::{ppu::RgbImage, NesEmulatorData};
use egui_multiwin::{
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping ppu nametable data
#[cfg(feature = "egui-multiwin")]
pub struct DumpWindow {
    /// The image to use for the dump
    buf: Box<RgbImage>,
    /// The image for the attribute table
    buf2: Box<RgbImage>,
    /// The texture used for rendering the image.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    texture: Option<egui_multiwin::egui::TextureHandle>,
    /// The texture used for rendering the attribute table.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    texture2: Option<egui_multiwin::egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DumpWindow {
                buf: Box::new(RgbImage::new(512, 480)),
                buf2: Box::new(RgbImage::new(512, 480)),
                texture: None,
                texture2: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 1048.0,
                    height: 768.0,
                })
                .with_title("UglyOldBob NES PPU Name Table Dump"),
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
            ui.label("PPU Name Table Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                c.cpu_peripherals.ppu.render_nametable(&mut self.buf, &c.mb);
                c.cpu_peripherals
                    .ppu
                    .render_attribute_table(&mut self.buf2, &c.mb);
                let image = self.buf.to_egui();
                let image2 = self.buf2.to_egui();
                if self.texture.is_none() {
                    self.texture = Some(egui.egui_ctx.load_texture(
                        "NES_PPU",
                        image,
                        egui_multiwin::egui::TextureOptions::NEAREST,
                    ));
                } else if let Some(t) = &mut self.texture {
                    t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
                }
                if self.texture2.is_none() {
                    self.texture2 = Some(egui.egui_ctx.load_texture(
                        "NES_PPU",
                        image2,
                        egui_multiwin::egui::TextureOptions::NEAREST,
                    ));
                } else if let Some(t) = &mut self.texture2 {
                    t.set_partial([0, 0], image2, egui_multiwin::egui::TextureOptions::NEAREST);
                }
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        if let Some(t) = &self.texture {
                            let zoom = 1.0;
                            let r = ui.image(
                                t,
                                egui_multiwin::egui::Vec2 {
                                    x: zoom * self.buf.width as f32,
                                    y: zoom * self.buf.height as f32,
                                },
                            );

                            if r.hovered() {
                                if let Some(cursor) = r.hover_pos() {
                                    let pos = cursor - r.rect.left_top();
                                    if pos.x >= 0.0 && pos.y >= 0.0 {
                                        let pixelx = (pos.x / zoom).floor() as u8;
                                        let pixely = (pos.y / zoom).floor() as u8;
                                        c.cpu_peripherals.ppu.bg_debug = Some((pixelx, pixely));
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
                                        let pix_x =
                                            (((pos.x / (zoom)).floor() as usize) & 0xFF) as u8;
                                        let pix_y =
                                            (((pos.y / (zoom)).floor() as usize) % 240) as u8;
                                        ui.label(format!(
                                            "Coordinate {},{} {:x}",
                                            pix_x, pix_y, table
                                        ));
                                        let addr =
                                            c.cpu_peripherals.ppu.render_nametable_pixel_address(
                                                table, pix_x, pix_y, &c.mb,
                                            );
                                        let pixel_entry = c.mb.ppu_palette_read(addr) & 63;
                                        let ntaddr = 0x2000
                                            + 0x400 * table as usize
                                            + col as usize
                                            + row as usize * 32;
                                        ui.label(format!(
                                            "Palette address is {:x} {:x}",
                                            addr, pixel_entry
                                        ));
                                        ui.label(format!(
                                            "Tile address {},{} is {:x}={:x}",
                                            col, row, ntaddr, c.mb.ppu_peek(ntaddr as u16),
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
                                        let pix_x =
                                            (((pos.x / (zoom)).floor() as usize) & 0xFF) as u8;
                                        let pix_y =
                                            (((pos.y / (zoom)).floor() as usize) % 240) as u8;
                                        ui.label(format!(
                                            "Coordinate {},{} {:x}",
                                            pix_x, pix_y, table
                                        ));
                                        let ntaddr = 0x23C0
                                            + 0x400 * table as usize
                                            + col as usize
                                            + row as usize * 8;
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
                            let r = ui.image(
                                t,
                                egui_multiwin::egui::Vec2 {
                                    x: zoom * self.buf.width as f32,
                                    y: zoom * self.buf.height as f32,
                                },
                            );

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
                                        let pix_x =
                                            (((pos.x / (zoom)).floor() as usize) & 0xFF) as u8;
                                        let pix_y =
                                            (((pos.y / (zoom)).floor() as usize) % 240) as u8;
                                        ui.label(format!(
                                            "Coordinate {},{} {:x}",
                                            pix_x, pix_y, table
                                        ));
                                        let addr =
                                            c.cpu_peripherals.ppu.render_nametable_pixel_address(
                                                table, pix_x, pix_y, &c.mb,
                                            );
                                        let ntaddr = 0x23C0
                                            + 0x400 * table as usize
                                            + col as usize
                                            + row as usize * 8;
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
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
