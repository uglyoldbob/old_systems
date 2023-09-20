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
    /// The texture used for rendering the image.
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    texture: Option<egui_multiwin::egui::TextureHandle>,
}

impl DumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DumpWindow {
                buf: Box::new(RgbImage::new(512, 480)),
                texture: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 1024.0,
                    height: 768.0,
                })
                .with_title("UglyOldBob NES PPU Pattern Table Dump"),
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
            ui.label("PPU Pattern Table Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                c.cpu_peripherals.ppu.render_nametable(&mut self.buf, &c.mb);
                let image = self.buf.to_egui();
                if self.texture.is_none() {
                    self.texture = Some(egui.egui_ctx.load_texture(
                        "NES_PPU",
                        image,
                        egui_multiwin::egui::TextureOptions::NEAREST,
                    ));
                } else if let Some(t) = &mut self.texture {
                    t.set_partial([0, 0], image, egui_multiwin::egui::TextureOptions::NEAREST);
                }
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
                                let pix_x = (((pos.x / (zoom)).floor() as usize) & 0xFF) as u8;
                                let pix_y = (((pos.y / (zoom)).floor() as usize) % 240) as u8;
                                ui.label(format!("Coordinate {},{} {:x}", pix_x,pix_y, table));
                                let addr = c.cpu_peripherals.ppu.render_nametable_pixel_address(table, pix_x, pix_y, &c.mb);
                                let pixel_entry = c.mb.ppu_palette_read(addr) & 63;
                                ui.label(format!("Address is {:x} {:x}", addr, pixel_entry));
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