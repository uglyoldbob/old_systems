#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

pub trait NesController {
    fn update_latch_bits(&mut self, data: [bool; 3]);
    fn read_data(&mut self) -> u8;
    fn provide_egui_ref(&mut self, data: &egui::InputState);
}

pub struct StandardController {
    controller_buttons: u8,
    shift_register: u8,
    strobe: bool,
}

const BUTTON_A: u8 = 0x01;
const BUTTON_B: u8 = 0x02;
const BUTTON_SELECT: u8 = 0x04;
const BUTTON_START: u8 = 0x08;
const BUTTON_UP: u8 = 0x10;
const BUTTON_DOWN: u8 = 0x20;
const BUTTON_LEFT: u8 = 0x40;
const BUTTON_RIGHT: u8 = 0x80;

impl StandardController {
    pub fn new() -> Self {
        Self {
            controller_buttons: 0xff,
            shift_register: 0xff,
            strobe: false,
        }
    }

    fn check_strobe(&mut self) {
        if self.strobe {
            self.shift_register = self.controller_buttons;
        }
    }
}

impl NesController for StandardController {
    fn update_latch_bits(&mut self, data: [bool; 3]) {
        self.strobe = data[0];
        self.check_strobe();
    }
    fn read_data(&mut self) -> u8 {
        self.check_strobe();
        let data = self.shift_register & 1;
        self.shift_register = (self.shift_register >> 1) | 0x00;
        data | 0x1e
    }
    fn provide_egui_ref(&mut self, data: &egui::InputState) {
        let kd = &data.keys_down;
        let mut newkeys: u8 = 0xff;
        if kd.contains(&egui::Key::F) {
            newkeys &= !BUTTON_A;
        }
        if kd.contains(&egui::Key::D) {
            newkeys &= !BUTTON_B;
        }
        if kd.contains(&egui::Key::ArrowUp) {
            newkeys &= !BUTTON_UP;
        } else if kd.contains(&egui::Key::ArrowDown) {
            newkeys &= !BUTTON_DOWN;
        }
        if kd.contains(&egui::Key::ArrowLeft) {
            newkeys &= !BUTTON_LEFT;
        } else if kd.contains(&egui::Key::ArrowRight) {
            newkeys &= !BUTTON_RIGHT;
        }
        if kd.contains(&egui::Key::A) {
            newkeys &= !BUTTON_START;
        }
        if kd.contains(&egui::Key::S) {
            newkeys &= !BUTTON_SELECT;
        }

        self.controller_buttons = newkeys;

        self.check_strobe();
    }
}
