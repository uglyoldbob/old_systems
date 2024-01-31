//! The module for finding nes roms

use crate::{cartridge::SnesCartridge, SnesEmulatorData};
use common_emulator::romlist::RomRanking;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui::Sense, egui_glow::EguiGlow};
use strum::IntoEnumIterator;

use crate::egui_multiwin_dynamic::{
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
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::RomFinder(RomFinder { scrolled: false }),
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
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow for RomFinder {
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
        let mut quit = false;
        let windows_to_create = vec![];

        //scan for roms if needed
        let rp = c.local.configuration.get_rom_path().to_owned();
        c.find_roms(&rp);
        //process to see if any new roms need to be checked
        c.process_roms();

        let mut save_list = false;
        let sp = c.local.save_path();
        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                let mut new_rom = None;
                for ranking in RomRanking::iter() {
                    let mut have_entry = false;
                    for (p, entry) in c.local.parser.list_mut().elements.iter_mut() {
                        if let Some(Ok(r)) = &mut entry.result {
                            if r.ranking == ranking {
                                have_entry = true;
                                ui.horizontal(|ui| {
                                    if ui.button("-").clicked() {
                                        r.ranking.decrease();
                                        save_list = true;
                                    }
                                    if ui.button("+").clicked() {
                                        r.ranking.increase();
                                        save_list = true;
                                    }

                                    ui.label(r.ranking.to_string());

                                    let resp = ui.add(
                                        egui_multiwin::egui::Label::new(format!(
                                            "{:x}: {}",
                                            r.mapper,
                                            p.display()
                                        ))
                                        .sense(Sense::click()),
                                    );
                                    if let Some(cart) = c.mb.cartridge() {
                                        if p.display().to_string() == cart.rom_name()
                                            && !self.scrolled
                                        {
                                            resp.scroll_to_me(Some(
                                                egui_multiwin::egui::Align::TOP,
                                            ));
                                            self.scrolled = true;
                                        }
                                    }

                                    if resp.double_clicked() {
                                        new_rom = Some(
                                            SnesCartridge::load_cartridge(
                                                p.to_str().unwrap().into(),
                                                &sp,
                                            )
                                            .unwrap(),
                                        );
                                        quit = true;
                                    }
                                });
                            }
                        }
                    }
                    if have_entry {
                        ui.separator();
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

        if save_list {
            let p = c.local.save_path();
            if c.local.parser.list().save_list(p).is_ok() {
                println!("Saved rom list");
            }
        }

        self.scrolled = true;

        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
