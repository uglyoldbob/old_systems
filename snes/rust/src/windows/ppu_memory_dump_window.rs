//! The module for dumping all of the ppu address space

use crate::NesEmulatorData;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping cpu data
pub struct PpuMemoryDumpWindow {}

#[cfg(feature = "egui-multiwin")]
impl PpuMemoryDumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::PpuMemoryDump(PpuMemoryDumpWindow {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES PPU Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow for PpuMemoryDumpWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("PPU Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    for i in (0..=0x3FFF).step_by(8) {
                        let d: [u8; 8] = [
                            c.mb.ppu_peek(i),
                            c.mb.ppu_peek(i + 1),
                            c.mb.ppu_peek(i + 2),
                            c.mb.ppu_peek(i + 3),
                            c.mb.ppu_peek(i + 4),
                            c.mb.ppu_peek(i + 5),
                            c.mb.ppu_peek(i + 6),
                            c.mb.ppu_peek(i + 7),
                        ];
                        let a: [String; 8] = d.map(|d| format!("{:02X}", d));
                        let b = String::from_utf8_lossy(&d);
                        let display = format!(
                            "{:04X}: {} {} {} {}\t{} {} {} {}\t{}",
                            i, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], b,
                        );
                        ui.label(
                            egui_multiwin::egui::RichText::new(display)
                                .font(egui_multiwin::egui::FontId::monospace(12.0)),
                        );
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
