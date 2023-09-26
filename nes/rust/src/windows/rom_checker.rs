//! The module for the main debug window

use crate::{NesEmulatorData, cartridge::NesCartridge, rom_status::RomStatus};
use egui_multiwin::{
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The structure for a debug window of the emulator.
#[cfg(feature = "egui-multiwin")]
pub struct Window {
    /// The index into the large rom list
    index: usize,
    /// The next rom in the list
    next_rom: Option<NesCartridge>,
    /// The input field for entering bug status
    bug: String,
    /// Set to some when it is desired to scan for a rom with a status
    want_status: Option<Option<RomStatus>>,
}

#[cfg(feature = "egui-multiwin")]
impl Window {
    /// Create a new request for a Debug window.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(Window {
                index: 0,
                next_rom: None,
                bug: "".to_string(),
                want_status: None,
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 640.0,
                    height: 480.0,
                })
                .with_title("UglyOldBob NES ROM CHECKER"),
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
            ui.label("Rom checking window");
            let mut save_state = None;
            if let Some(rom) = c.mb.cartridge() {
                ui.label(format!("Current rom is {}", rom.rom_name()));
                ui.label(format!("HASH of current rom is {}", rom.hash()));
                if let Some((_hash, result)) = c.rom_test.list().elements.get_key_value(&rom.hash()) {
                    match result {
                        RomStatus::CompletelyBroken => {
                            ui.label("Rom is completely broken");
                        }
                        RomStatus::Bug(b, state) => {
                            ui.label(format!("ROM affected by bug\n{}", b));
                            if ui.button("Load save state").clicked() {
                                save_state = state.clone();
                            }
                        }
                        RomStatus::Working => {
                            ui.label("Rom is working so far");
                        }
                    }
                }
                if ui.button("Set status to no bugs").clicked() {
                    c.rom_test.put_entry(rom.hash(), RomStatus::Working);
                }
                if ui.button("Set status to completely broken").clicked() {
                    c.rom_test.put_entry(rom.hash(), RomStatus::CompletelyBroken);
                }
                ui.text_edit_multiline(&mut self.bug);
                if ui.button("Set status to has a bug").clicked() {
                    c.rom_test.put_entry(rom.hash(), RomStatus::Bug(self.bug.to_owned(), Some(c.serialize())));
                }
            }
            ui.label(format!("There are {} known roms", c.parser.list().elements.len()));
            if let Some(state) = save_state {
                let _e = c.deserialize(state);
            }

            if self.next_rom.is_none() {
                println!("Next rom is none");
                if let Some((path, _romentry)) = c.roms.elements.iter().skip(self.index).next() {
                    if let Ok(cart) = crate::NesCartridge::load_cartridge(path.to_str().unwrap().into()) {
                        let hash = cart.hash();
                        match c.rom_test.list().elements.get_key_value(&hash) {
                            Some((_hash, status)) => {
                                if let Some(desired) = &self.want_status {
                                    if let Some(desired) = desired {
                                        if status.match_category(desired) {
                                            println!("Found next status rom: {}", cart.rom_name());
                                            self.next_rom = Some(cart);
                                        }
                                        else {
                                            println!("Looking for status rom");
                                            self.index += 1;
                                        }
                                    }
                                    else {
                                        println!("Looking for unlisted rom");
                                        self.index += 1;
                                    }
                                }
                                else {
                                    println!("Found next rom");
                                    self.next_rom = Some(cart);
                                }
                            }
                            None => {
                                if let Some(desired) = &self.want_status {
                                    if desired.is_some() {
                                        println!("Looking for status rom");
                                        self.index += 1;
                                    }
                                    else {
                                        println!("Found next status rom: {}", cart.rom_name());
                                        self.next_rom = Some(cart);
                                    }
                                }
                                else {
                                    println!("Found next rom");
                                    self.next_rom = Some(cart);
                                }
                            }
                        }
                    }
                    else {
                        println!("Looking for next valid rom");
                        self.index += 1;
                    }
                }
                else {
                    println!("No roms?");
                }
            }

            if let Some((path, _romentry)) = c.roms.elements.iter().skip(self.index).next() {
                ui.label(format!("The next rom is {}", path.display()));

                let mut new_rom = None;
                if self.next_rom.is_some() && self.want_status.is_some() {
                    new_rom = self.next_rom.take();
                    self.want_status = None;
                }
                if ui.button("Load next rom").clicked() {
                    println!("Load next rom");
                    new_rom = self.next_rom.take();
                    self.index += 1;
                }
                if ui.button("Find next bug").clicked() {
                    println!("Find next bug");
                    self.want_status = Some(Some(RomStatus::Bug("".to_string(), None)));
                    self.index += 1;
                    self.next_rom = None;
                }
                if ui.button("Find next completely broken").clicked() {
                    println!("Find next borken");
                    self.want_status = Some(Some(RomStatus::CompletelyBroken));
                    self.index += 1;
                    self.next_rom = None;
                }
                if ui.button("Find next unlisted").clicked() {
                    self.want_status = Some(None);
                    self.index += 1;
                    self.next_rom = None;
                }
                if ui.button("Restart").clicked() {
                    self.index = 0;
                    self.next_rom = None;
                }
                if let Some(nc) = new_rom {
                    c.remove_cartridge();
                    c.insert_cartridge(nc);
                    c.reset();
                    self.index += 1;
                }
            }
        });
        if self.index >= c.roms.elements.len() {
            self.index = 0;
            self.want_status = None;
        }
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
