//! This modules contains the window for editing game genie codes for the current game

use crate::NesEmulatorData;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping ppu nametable data
pub struct Window {
    /// The string for a new game genie code
    code: String,
}

impl Window {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::Genie(Window {
                code: Default::default(),
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 480.0,
                })
                .with_title("UglyOldBob NES Game Genie Codes"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow for Window {
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
            if let Some(cart) = c.mb.cartridge_mut() {
                let mut delete = None;
                for code in &cart.cartridge().volatile.genie {
                    ui.horizontal(|ui| {
                        ui.label(format!("Code: {}", code.code()));
                        if ui.button("Delete").clicked() {
                            delete = Some(code.to_owned());
                        }
                    });
                }
                if let Some(code) = delete {
                    cart.cartridge_volatile_mut().remove_code(&code);
                }
                if cart.cartridge().volatile.genie.len() > 0 {
                    ui.separator();
                }
                ui.label("Enter game genie code");
                ui.text_edit_singleline(&mut self.code);
                if let Ok(v) = crate::genie::GameGenieCode::from_str(&self.code) {
                    if ui.button("Add game genie code").clicked() {
                        cart.cartridge_volatile_mut().genie.push(v);
                    }
                }
            }
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
