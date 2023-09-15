//! This module is for the window that dumps the ppu pattern tables.

use crate::{ppu::RgbImage, NesEmulatorData};
use egui_multiwin::{
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping cartridge program data
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
                buf: Box::new(RgbImage::new(256, 128)),
                texture: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 640.0,
                    height: 480.0,
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
                c.cpu_peripherals
                    .ppu
                    .render_pattern_table(&mut self.buf, &c.mb);
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
                    ui.image(
                        t,
                        egui_multiwin::egui::Vec2 {
                            x: 2.0 * self.buf.width as f32,
                            y: 2.0 * self.buf.height as f32,
                        },
                    );
                }
            });
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
