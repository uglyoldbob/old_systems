//! This modules contains the window for editing controller properties

use std::collections::HashSet;

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

use crate::controller::NesControllerTrait;

/// The window for dumping ppu nametable data
pub struct Window {
    selected_controller: Option<u8>,
    waiting_for_input: Option<usize>,
    known_keys: HashSet<egui::Key>,
}

impl Window {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(Window {
                selected_controller: None,
                known_keys: HashSet::new(),
                waiting_for_input: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 1024.0,
                    height: 768.0,
                })
                .with_title("UglyOldBob NES Controller Configuration"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for Window {
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
            let newkeys = ui.input(|i| i.keys_down.clone());

            let oldkeys = self.known_keys.clone();

            let mut diff = newkeys.difference(&oldkeys);
            let first_key = diff.next();
            self.known_keys = newkeys.clone();
            ui.label("Controller Configuration Window");

            egui::ComboBox::from_label("Select a controller")
                .selected_text(format!(
                    "{}",
                    self.selected_controller
                        .map(|i| format!("{}", i))
                        .unwrap_or("Select a controller".to_string())
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_controller, Some(0), "First");
                    ui.selectable_value(&mut self.selected_controller, Some(1), "Second");
                    ui.selectable_value(&mut self.selected_controller, Some(2), "Third");
                    ui.selectable_value(&mut self.selected_controller, Some(3), "Fourth");
                });
            let mut save_config = false;
            if let Some(i) = self.selected_controller {
                let config = &mut c.configuration.controller_config[i as usize];
                let mut set_turboa = None;
                let mut set_turbob = None;

                if let Some(index) = self.waiting_for_input {
                    if let Some(key) = first_key {
                        config.set_key_egui(index as usize, *key);
                        self.waiting_for_input = None;
                        save_config = true;
                    }
                }

                let keys = config.get_keys();

                ui.horizontal(|ui| {
                    ui.label("Button A:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_A) = self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_A].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_A);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button B:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_B) = self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_B].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_B);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Turbo A:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_TURBOA) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_TURBOA].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_TURBOA);
                    }
                    let mut val = config.get_rate(0);
                    if ui.add(egui::Slider::new(&mut val, 0.5..=25.0).text("Rapid fire rate")).changed() {
                        set_turboa = Some(val);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Turbo B:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_TURBOB) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_TURBOB].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_TURBOB);
                    }
                    let mut val = config.get_rate(1);
                    if ui.add(egui::Slider::new(&mut val, 0.5..=25.0).text("Rapid fire rate")).changed() {
                        set_turbob = Some(val);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Start:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_START) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_START].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_START);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Select:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_SELECT) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_SELECT].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_SELECT);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Up:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_UP) = self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_UP].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_UP);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Button Down:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_DOWN) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_DOWN].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_DOWN);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Button Left:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_LEFT) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_LEFT].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_LEFT);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Button Right:");
                    if ui
                        .button(
                            if let Some(crate::controller::BUTTON_COMBO_RIGHT) =
                                self.waiting_for_input
                            {
                                "Waiting for input".to_string()
                            } else {
                                keys[crate::controller::BUTTON_COMBO_RIGHT].to_string()
                            },
                        )
                        .clicked()
                    {
                        self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_RIGHT);
                    }
                });
                if let Some(r) = set_turboa {
                    config.set_rate(0, r);
                    save_config = true;
                }
                if let Some(r) = set_turbob {
                    config.set_rate(1, r);
                    save_config = true;
                }
            }
            if save_config {
                c.configuration.save();
            }
            
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}