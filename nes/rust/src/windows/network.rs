//! This module is for the window that allows a user to set network options

#[cfg(feature = "eframe")]
use eframe::egui;

use egui_multiwin::egui::TextEdit;
#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};
use crate::emulator_data::NesEmulatorData;

pub struct Window {}

#[cfg(feature = "egui-multiwin")]
impl Window {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::Network(Window {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Network Configuration"),
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
            if let Some(olocal) = &mut c.olocal {
                if olocal.network.is_none() {
                    ui.label("Network is not active");
                    if ui.button("Enable networking").clicked() {
                        if let Some(proxy) = c.local.get_proxy() {
                            olocal.network = Some(crate::network::Network::new(proxy));
                        }
                    }
                } else {
                    ui.label("Network is active");
                    if ui.button("Disable networking").clicked() {
                        olocal.network = None;
                    }
                }
                if let Some(network) = &mut olocal.network {
                    let na = network.get_addresses();
                    if na.len() > 0 {
                        ui.label("Currently listening on:");
                        for a in na {
                            let mut t = a.to_string();
                            let te = TextEdit::singleline(&mut t);
                            ui.add(te);
                        }
                    }

                    if ui.button("Start server").clicked() {
                        network.start_server();
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
