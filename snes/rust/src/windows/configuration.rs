//! This modules contains the window for editing controller properties

use crate::SnesEmulatorData;
use strum::IntoEnumIterator;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// Defines messages that can some from other threads
enum Message {
    ///A path has been selected for roms
    NewRomPath(std::path::PathBuf),
}

/// The window for dumping ppu nametable data
pub struct Window {
    /// The message channel for communicating with the main thread, when needed.
    message_channel: (
        std::sync::mpsc::Sender<Message>,
        std::sync::mpsc::Receiver<Message>,
    ),
}

impl Window {
    /// Create a request to create a new window of self.
    #[cfg(feature = "egui-multiwin")]
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::Configuration(Window {
                message_channel: std::sync::mpsc::channel(),
            }),
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
        c: &mut SnesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("Emulator Configuration Window");

            let mut save_config = false;

            while let Ok(message) = self.message_channel.1.try_recv() {
                match message {
                    Message::NewRomPath(pb) => {
                        c.local.configuration.set_rom_path(pb);
                    }
                }
            }

            if ui
                .checkbox(&mut c.local.configuration.sticky_rom, "Remember last rom")
                .changed()
                && !c.local.configuration.sticky_rom
            {
                c.local.configuration.set_startup("".to_string());
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
                ));
            }

            ui.label("Folder for roms:");
            if ui
                .add(
                    egui::Label::new(c.local.configuration.get_rom_path())
                        .sense(egui::Sense::click()),
                )
                .clicked()
            {
                let f = rfd::AsyncFileDialog::new()
                    .set_title("Select rom folder")
                    .set_directory(c.local.default_rom_path())
                    .pick_folder();
                let message_sender = self.message_channel.0.clone();
                crate::execute(async move {
                    let file = f.await;
                    if let Some(file) = file {
                        let fname = file.path().to_path_buf();
                        message_sender.send(Message::NewRomPath(fname)).ok();
                    }
                });
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
