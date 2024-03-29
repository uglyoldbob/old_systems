//! This modules contains the window for editing controller properties

use std::collections::{HashMap, HashSet};

use crate::{controller::NesController, NesEmulatorData};

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

use strum::IntoEnumIterator;

/// The window for dumping ppu nametable data
pub struct Window {
    /// The controller currently being shown to the user
    selected_controller: Option<u8>,
    /// Indicates which button on the current controller needs a button input
    waiting_for_input: Option<usize>,
    /// The last set of known pressed keys fromm egui
    known_keys: HashSet<egui::Key>,
    /// The set of data from gilrs
    gilrs_last_known: (
        HashSet<(gilrs::GamepadId, gilrs::ev::Code)>,
        HashMap<(gilrs::GamepadId, gilrs::ev::Code), f32>,
    ),
}

impl Window {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::Controllers(Window {
                selected_controller: None,
                known_keys: HashSet::new(),
                waiting_for_input: None,
                gilrs_last_known: (HashSet::new(), HashMap::new()),
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
            let (old_gilrs_button, old_gilrs_axis) = self.gilrs_last_known.clone();
            if let Some(olocal) = &mut c.olocal {
                let gilrs = &mut olocal.gilrs;

                let mut bhash = HashSet::new();
                let mut ahash = HashMap::new();
                for (id, gp) in gilrs.gamepads() {
                    let state = gp.state();
                    for (code, bd) in state.buttons() {
                        if bd.is_pressed() {
                            bhash.insert((id, code));
                        }
                    }
                    for (code, ad) in state.axes() {
                        ahash.insert((id, code), ad.value());
                    }
                }
                self.gilrs_last_known = (bhash, ahash);
            }
            let (new_b, new_a) = &self.gilrs_last_known;
            let mut diff_b = new_b.difference(&old_gilrs_button);
            let diff_a: Vec<(&(gilrs::GamepadId, gilrs::ev::Code), &f32)> = new_a
                .iter()
                .filter(|(complex_id, val)| {
                    if let Some(v2) = old_gilrs_axis.get(complex_id) {
                        *v2 == **val
                    } else {
                        true
                    }
                })
                .collect();

            let first_joy_button = diff_b.next();
            let first_joy_axis = diff_a.first();

            let newkeys = ui.input(|i| i.keys_down.clone());

            let oldkeys = self.known_keys.clone();

            let mut diff = newkeys.difference(&oldkeys);
            let first_key = diff.next();
            self.known_keys = newkeys.clone();
            ui.label("Controller Configuration Window");

            egui::ComboBox::from_label("Select a controller")
                .selected_text(
                    self.selected_controller
                        .map(|i| format!("{}", i))
                        .unwrap_or("Select a controller".to_string())
                        .to_string(),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_controller, Some(0), "First");
                    ui.selectable_value(&mut self.selected_controller, Some(1), "Second");
                    ui.selectable_value(&mut self.selected_controller, Some(2), "Third");
                    ui.selectable_value(&mut self.selected_controller, Some(3), "Fourth");
                });
            let mut save_config = false;
            let mut controller_mod = None;
            if let Some(i) = self.selected_controller {
                let mut controller = c.mb.get_controller(i);
                let controllerr = egui_multiwin::egui::ComboBox::from_id_source("Controller Type")
                    .selected_text(controller.to_string())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for opt in NesController::iter() {
                            if ui
                                .selectable_value(&mut controller, opt.clone(), opt.to_string())
                                .changed()
                            {
                                changed = true;
                                c.local.configuration.controller_type[i as usize] =
                                    controller.get_type();
                                controller_mod = Some((i, opt));
                            }
                        }
                        changed
                    });
                if controllerr.inner.is_some_and(|t| t) {
                    save_config = true;
                }

                let config = &mut c.local.configuration.controller_config[i as usize];
                let mut set_turboa = None;
                let mut set_turbob = None;

                if let Some(index) = self.waiting_for_input {
                    if let Some(key) = first_key {
                        config.set_key_egui(index, *key);
                        self.waiting_for_input = None;
                        save_config = true;
                    } else if let Some((id, joybutton)) = first_joy_button {
                        config.set_key_gilrs_button(index, *id, *joybutton);
                        self.waiting_for_input = None;
                        save_config = true;
                    } else if let Some(((id, code), axis)) = first_joy_axis {
                        if **axis < -0.5 {
                            config.set_key_gilrs_axis(index, *id, *code, false);
                            self.waiting_for_input = None;
                            save_config = true;
                        } else if **axis > 0.5 {
                            config.set_key_gilrs_axis(index, *id, *code, true);
                            self.waiting_for_input = None;
                            save_config = true;
                        }
                    }
                }

                let keys = config.get_keys();

                let controller = c.mb.get_controller_mut(i);
                match controller {
                    crate::controller::NesController::FourScore(_fs) => {
                        ui.label("A problem occurred");
                    }
                    crate::controller::NesController::StandardController(_) => {
                        ui.horizontal(|ui| {
                            ui.label("Button A:");
                            if ui
                                .button(
                                    if let Some(crate::controller::BUTTON_COMBO_A) =
                                        self.waiting_for_input
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
                                    if let Some(crate::controller::BUTTON_COMBO_B) =
                                        self.waiting_for_input
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
                                self.waiting_for_input =
                                    Some(crate::controller::BUTTON_COMBO_TURBOA);
                            }
                            let mut val = config.get_rate(0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut val, 0.5..=25.0).text("Rapid fire rate"),
                                )
                                .changed()
                            {
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
                                self.waiting_for_input =
                                    Some(crate::controller::BUTTON_COMBO_TURBOB);
                            }
                            let mut val = config.get_rate(1);
                            if ui
                                .add(
                                    egui::Slider::new(&mut val, 0.5..=25.0).text("Rapid fire rate"),
                                )
                                .changed()
                            {
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
                                self.waiting_for_input =
                                    Some(crate::controller::BUTTON_COMBO_START);
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
                                self.waiting_for_input =
                                    Some(crate::controller::BUTTON_COMBO_SELECT);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Button Up:");
                            if ui
                                .button(
                                    if let Some(crate::controller::BUTTON_COMBO_UP) =
                                        self.waiting_for_input
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
                                self.waiting_for_input =
                                    Some(crate::controller::BUTTON_COMBO_RIGHT);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Trigger:");
                            if ui
                                .button(
                                    if let Some(crate::controller::BUTTON_COMBO_FIRE) =
                                        self.waiting_for_input
                                    {
                                        "Waiting for input".to_string()
                                    } else {
                                        keys[crate::controller::BUTTON_COMBO_FIRE].to_string()
                                    },
                                )
                                .clicked()
                            {
                                self.waiting_for_input = Some(crate::controller::BUTTON_COMBO_FIRE);
                            }
                        });
                    }
                    crate::controller::NesController::Zapper(_) => {
                        ui.label("Zapper controlled by mouse");
                    }
                    crate::controller::NesController::DummyController(_) => {}
                }

                if let Some(r) = set_turboa {
                    config.set_rate(0, r);
                    save_config = true;
                }
                if let Some(r) = set_turbob {
                    config.set_rate(1, r);
                    save_config = true;
                }
            }

            if let Some((i, cont)) = controller_mod {
                c.mb.set_controller(i, cont);
            }
            if save_config {
                c.local.configuration.save();
            }
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
