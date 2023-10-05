//! This module is responsible for emulating controllers for the nes system.

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// The trait the all controllers must implement
#[enum_dispatch::enum_dispatch]
pub trait NesControllerTrait {
    /// Update the latch bits on the controller
    fn update_latch_bits(&mut self, data: [bool; 3]);
    /// Dump data from the controller. No side effects.
    fn dump_data(&self) -> u8;
    /// Read data from the controller.
    fn read_data(&mut self) -> u8;
    /// Provides an egui input state to update the controller state.
    fn provide_egui_ref(&mut self, data: &egui::InputState);
    /// Convert controller state to a string
    fn to_string(&self) -> String;
}

/// A generic implementation of a NES controller
#[non_exhaustive]
#[enum_dispatch::enum_dispatch(NesControllerTrait)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum NesController {
    StandardController,
}

/// A standard nes controller implementation
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StandardController {
    /// The status of all 8 buttons
    controller_buttons: u8,
    /// The contents of the shift register
    shift_register: u8,
    /// The strobe signal triggers loading the controller data into the shift register
    strobe: bool,
}

/// Flag for the a button
const BUTTON_A: u8 = 0x01;
/// Flag for the b button
const BUTTON_B: u8 = 0x02;
/// Flag for the select button
const BUTTON_SELECT: u8 = 0x04;
/// Flag for the start button
const BUTTON_START: u8 = 0x08;
/// Flag for the up button
const BUTTON_UP: u8 = 0x10;
/// Flag for the down button
const BUTTON_DOWN: u8 = 0x20;
/// Flag for the left button
const BUTTON_LEFT: u8 = 0x40;
/// Flag for the right button
const BUTTON_RIGHT: u8 = 0x80;

impl StandardController {
    /// Create a new controller
    pub fn new() -> NesController {
        (Self {
            controller_buttons: 0xff,
            shift_register: 0xff,
            strobe: false,
        })
        .into()
    }

    ///convenience function to check the strobe, to determine of the buttons should be loaded to the shift register
    fn check_strobe(&mut self) {
        if self.strobe {
            self.shift_register = self.controller_buttons;
        }
    }
}

impl NesControllerTrait for StandardController {
    fn update_latch_bits(&mut self, data: [bool; 3]) {
        self.strobe = data[0];
        self.check_strobe();
    }
    fn dump_data(&self) -> u8 {
        let data = self.shift_register & 1;
        data | 0x1e
    }

    fn to_string(&self) -> String {
        format!(
            "{}{}{}{} {}{} {}{}",
            if (self.controller_buttons & BUTTON_LEFT) == 0 {
                "<"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_UP) == 0 {
                "^"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_DOWN) == 0 {
                "V"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_RIGHT) == 0 {
                ">"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_SELECT) == 0 {
                "Se"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_START) == 0 {
                "St"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_A) == 0 {
                "A"
            } else {
                ""
            },
            if (self.controller_buttons & BUTTON_B) == 0 {
                "B"
            } else {
                ""
            },
        )
    }

    fn read_data(&mut self) -> u8 {
        self.check_strobe();
        let data = self.shift_register & 1;
        self.shift_register >>= 1;
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
