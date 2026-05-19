//! This module is for the window that allows a user to set network options

use eframe::egui;

use crate::emulator_data::NesEmulatorData;

/// The network configuration window.
pub struct Window {
    /// The server that the user desires to connect to.
    server: String,
}

impl Window {
    /// Create a new self.
    pub fn new() -> Self {
        Window {
            server: "".to_string(),
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
            let cpu_frequency = c.cpu_frequency();
            let framerate = c.ppu_frame_rate();

            if let Some(olocal) = &mut c.olocal {
                if olocal.network.is_none() {
                    ui.label("Network is not active");
                    if ui.button("Enable networking").clicked() {
                        if let Some(proxy) = c.local.get_proxy() {
                            let mut button = crate::controller::ButtonCombination::new();
                            button.clear_buttons();
                            let button = bincode::serialize(&button).unwrap();
                            olocal.network = Some(common_emulator::network::Network::new(
                                proxy,
                                c.local.get_sound_rate(),
                                button,
                                "/nes/0.0.1",
                            ));
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
                    if !na.is_empty() {
                        ui.label("Currently listening on:");
                        for a in na {
                            let mut t = a.to_string();
                            let te = TextEdit::singleline(&mut t);
                            ui.add(te);
                        }
                    }

                    if !network.is_server_running() {
                        if ui.button("Start server").clicked() {
                            let _e = network.start_server(
                                c.local.image.width,
                                c.local.image.height,
                                framerate as u8,
                                cpu_frequency,
                            );
                        }

                        ui.horizontal(|ui| {
                            let te = TextEdit::singleline(&mut self.server);
                            ui.label("Server to connect to: ");
                            ui.add(te);
                        });
                        if ui.button("Connect").clicked() {
                            let _e = network.try_connect(&self.server);
                        }
                    } else if ui.button("Stop server").clicked() {
                        let _e = network.stop_server().is_ok();
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
