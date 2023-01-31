#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

pub trait NesController {
    fn update_latch_bits(&mut self, data: [bool; 3]);
    fn read_data(&mut self) -> u8;
    fn provide_egui_ref(&mut self, data: &egui::InputState);
}

pub struct StandardController {}

impl StandardController {
    pub fn new() -> Self {
        Self {}
    }
}

impl NesController for StandardController {
    fn update_latch_bits(&mut self, data: [bool; 3]) {}
    fn read_data(&mut self) -> u8 {
        42
    }
    fn provide_egui_ref(&mut self, data: &egui::InputState) {}
}
