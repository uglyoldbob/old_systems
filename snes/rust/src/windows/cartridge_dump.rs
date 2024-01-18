//! The module for the cartridge program data dump window

use crate::SnesEmulatorData;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping cartridge program data
#[cfg(feature = "egui-multiwin")]
pub struct CartridgeMemoryDumpWindow {}

#[cfg(feature = "egui-multiwin")]
impl CartridgeMemoryDumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::CartridgeDump(CartridgeMemoryDumpWindow {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Cartridge ROM Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow for CartridgeMemoryDumpWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut SnesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("Cartridge Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    if let Some(cart) = c.mb.cartridge() {
                        for (i, chunk) in cart.cartridge().nonvolatile.prg_rom.chunks(8).enumerate()
                        {
                            ui.label(format!(
                                "{:04X}: {:02X} {:02X} {:02X} {:02X}\t{:02X} {:02X} {:02X} {:02X}",
                                i * 8,
                                chunk[0],
                                chunk[1],
                                chunk[2],
                                chunk[3],
                                chunk[4],
                                chunk[5],
                                chunk[6],
                                chunk[7],
                            ));
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
