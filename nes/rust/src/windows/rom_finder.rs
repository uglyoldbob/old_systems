//! The module for finding nes roms

use crate::{cartridge::NesCartridge, NesEmulatorData};

use common_emulator::romlist::RomRanking;

use eframe::egui;

use strum::IntoEnumIterator;

/// The structure for a window that helps a user select a rom to load.
pub struct RomFinder {
    /// Set when the initial scroll to the currently loaded rom has occurred
    pub scrolled: bool,
    /// Found roms
    found_roms: bool,
}

impl RomFinder {
    pub fn new() -> Self {
        RomFinder {
            scrolled: false,
            found_roms: false,
        }
    }
}

impl RomFinder {
    pub fn show(&mut self, ui: &mut egui::Ui, quit_rom_window: &mut bool, c: &mut NesEmulatorData) {
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("ROM_LOAD_WINDOW"),
            egui::ViewportBuilder::default()
                .with_title("ROM LOAD")
                .with_inner_size([800.0, 640.0]),
            |ui, _class| {
                if !self.found_roms {
                    //scan for roms if needed
                    let rp = c.local.configuration.get_rom_path().to_owned();
                    c.find_roms(&rp);
                    //process to see if any new roms need to be checked
                    c.process_roms();
                    self.found_roms = true;
                }

                let mut save_list = false;
                let sp = c.local.save_path();
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
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
                                                egui::Label::new(format!(
                                                    "{}: {}",
                                                    r.mapper,
                                                    p.display()
                                                ))
                                                .sense(egui::Sense::click()),
                                            );
                                            if let Some(cart) = c.mb.cartridge() {
                                                if p.display().to_string() == cart.rom_name()
                                                    && !self.scrolled
                                                {
                                                    resp.scroll_to_me(Some(egui::Align::TOP));
                                                    self.scrolled = true;
                                                }
                                            }

                                            if resp.double_clicked() {
                                                new_rom = Some(
                                                    NesCartridge::load_cartridge(
                                                        p.to_str().unwrap().into(),
                                                        &sp,
                                                    )
                                                    .unwrap(),
                                                );
                                                *quit_rom_window = true;
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
                        log::info!("Saved rom list");
                    }
                }

                self.scrolled = true;
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    *quit_rom_window = true;
                }
            },
        );
    }
}
