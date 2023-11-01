//! The module for finding nes roms

use crate::{cartridge::NesCartridge, NesEmulatorData};

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{
    egui::Sense,
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The structure for a window that helps a user select a rom to load.
pub struct RomFinder {
    /// Set when the initial scroll to the currently loaded rom has occurred
    scrolled: bool,
}

#[cfg(feature = "egui-multiwin")]
impl RomFinder {
    /// Create a new request to make a RomFinder window.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(RomFinder { scrolled: false }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 480.0,
                    height: 300.0,
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
        let rp = c.local.configuration.get_rom_path().to_owned();
        c.find_roms(&rp);
        //process to see if any new roms need to be checked
        c.process_roms();

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                let mut new_rom = None;
                for (p, entry) in c.local.parser.list().elements.iter() {
                    if let Some(Ok(r)) = &entry.result {
                        let resp = ui.add(
                            egui_multiwin::egui::Label::new(format!(
                                "{:x}: {}",
                                r.mapper,
                                p.display()
                            ))
                            .sense(Sense::click()),
                        );
                        if let Some(cart) = c.mb.cartridge() {
                            if p.display().to_string() == cart.rom_name() && !self.scrolled {
                                resp.scroll_to_me(Some(egui_multiwin::egui::Align::TOP));
                                self.scrolled = true;
                            }
                        }

                        if resp.double_clicked() {
                            new_rom = Some(
                                NesCartridge::load_cartridge(
                                    p.to_str().unwrap().into(),
                                    &c.local.save_path(),
                                )
                                .unwrap(),
                            );
                            quit = true;
                        }
                    }
                }
                ui.label("Unsupported roms below here");
                for (p, entry) in c.local.parser.list().elements.iter() {
                    if let Some(Err(r)) = &entry.result {
                        ui.label(format!("Rom: {}: {:?}", p.display(), r));
                    }
                }
                if let Some(nc) = new_rom {
                    c.remove_cartridge();
                    c.insert_cartridge(nc);
                    c.power_cycle();
                }
            });
        });

        self.scrolled = true;

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
