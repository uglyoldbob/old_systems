//! The module for finding nes roms

use crate::{cartridge::NesCartridge, NesEmulatorData};
use egui_multiwin::{
    egui::Sense,
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The structure for a window that helps a user select a rom to load.
#[cfg(feature = "egui-multiwin")]
pub struct RomFinder {
    /// The element responsible for parsing the list of roms known by the emulator.
    parser: crate::romlist::RomListParser,
}

#[cfg(feature = "egui-multiwin")]
impl RomFinder {
    /// Create a new request to make a RomFinder window.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(RomFinder {
                parser: crate::romlist::RomListParser::new(),
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Rom Select"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for RomFinder {
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
        let mut quit = false;
        let windows_to_create = vec![];

        //scan for roms if needed
        self.parser.find_roms("./roms");
        //process to see if any new roms need to be checked
        self.parser.process_roms();

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                let mut new_rom = None;
                for (p, entry) in self.parser.list().elements.iter() {
                    if let Some(Ok(r)) = &entry.result {
                        if ui
                            .add(
                                egui_multiwin::egui::Label::new(format!(
                                    "{:x}: {}",
                                    r.mapper,
                                    p.display()
                                ))
                                .sense(Sense::click()),
                            )
                            .double_clicked()
                        {
                            new_rom = Some(
                                NesCartridge::load_cartridge(p.to_str().unwrap().into()).unwrap(),
                            );
                            quit = true;
                        }
                    }
                }
                ui.label("Unsupported roms below here");
                for (p, entry) in self.parser.list().elements.iter() {
                    if let Some(Err(r)) = &entry.result {
                        ui.label(format!("Rom: {}: {:?}", p.display(), r));
                    }
                }
                if let Some(nc) = new_rom {
                    c.remove_cartridge();
                    c.insert_cartridge(nc);
                    c.reset();
                }
            });
        });

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
