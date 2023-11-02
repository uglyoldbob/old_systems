//! This modules contains the window for editing controller properties

use crate::NesEmulatorData;
use strum::IntoEnumIterator;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
    egui,
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping ppu nametable data
pub struct Window {}

impl Window {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(Window {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Configuration"),
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
            ui.label("Emulator Configuration Window");

            let mut save_config = false;

            if ui
                .checkbox(&mut c.local.configuration.sticky_rom, "Remember last rom")
                .changed()
            {
                if !c.local.configuration.sticky_rom {
                    c.local.configuration.set_startup("".to_string());
                }
            }

            let mut scaler = c.local.configuration.scaler;
            if !c.local.resolution_locked {
                egui::ComboBox::from_label("Scaling algorithm")
                    .selected_text(
                        scaler
                            .map(|i| format!("{}", i))
                            .unwrap_or("None".to_string())
                            .to_string(),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut scaler, None, "None");
                        for opt in crate::ppu::ScalingAlgorithm::iter() {
                            ui.selectable_value(&mut scaler, Some(opt), opt.to_string());
                        }
                    });
                if scaler != c.local.configuration.scaler {
                    c.local.configuration.scaler = scaler;
                    save_config = true;
                }
            } else {
                ui.label(format!(
                    "Scaling algorithm: {}",
                    scaler
                        .map(|i| format!("{}", i))
                        .unwrap_or("None".to_string())
                        .to_string()
                ));
            }

            ui.label("Folder for roms:");
            if ui.add(egui::Label::new(c.local.configuration.get_rom_path()).sense(egui::Sense::click())).clicked() {
                println!("Clicked to change rom path");
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
