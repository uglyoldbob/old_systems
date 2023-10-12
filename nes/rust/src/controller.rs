//! This module is responsible for emulating controllers for the nes system.

use std::time::{Duration, Instant};

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// The various buttons used by controllers
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Button {
    /// The A button on standard controllers.
    A,
    /// The turbo rate adjustment for the A button. Boolean enables and disables the turbo action. Duration is the length of time between toggles.
    TurboA(bool, Duration),
    /// The turbo rate adjustment for the B button. Boolean enables and disables the turbo action. Duration is the length of time between toggles.
    TurboB(bool, Duration),
    /// The B button on standard controllers.
    B,
    /// The start button on standard controllers.
    Start,
    /// The slow button on the nes advantage
    Slow,
    /// The select button on standard controllers.
    Select,
    /// The up button on standard controllers.
    Up,
    /// The down button on standard controllers.
    Down,
    /// The left button on standard controllers.
    Left,
    /// The right button on standard controllers.
    Right,
    /// The fire button for a zapper or the arkanoid controller.
    Fire,
    /// The light sensor for a zapper
    LightSensor,
    /// Potentiometer for the arkanoid controller
    Potentiometer(u16),
    /// An extra button for the powerpad
    PowerPad,
}

/// The index into the button combination array for button A
const BUTTON_COMBO_A: usize = 0;
/// The index into the button combination array for turbo A
const BUTTON_COMBO_TURBOA: usize = 1;
/// The index into the button combination array for turbo B
const BUTTON_COMBO_TURBOB: usize = 2;
/// The index into the button combination array for button b
const BUTTON_COMBO_B: usize = 3;
/// The index into the button combination array for button start
const BUTTON_COMBO_START: usize = 4;
/// The index into the button combination array for button slow
const BUTTON_COMBO_SLOW: usize = 5;
/// The index into the button combination array for button select
const BUTTON_COMBO_SELECT: usize = 6;
/// The index into the button combination array for button up
const BUTTON_COMBO_UP: usize = 7;
/// The index into the button combination array for button down
const BUTTON_COMBO_DOWN: usize = 8;
/// The index into the button combination array for button left
const BUTTON_COMBO_LEFT: usize = 9;
/// The index into the button combination array for button right
const BUTTON_COMBO_RIGHT: usize = 10;
/// The index into the button combination array for fire/trigger
const BUTTON_COMBO_FIRE: usize = 11;
/// The index into the button combination array for a light sensor
const BUTTON_COMBO_LIGHT: usize = 12;
/// The index into the button combination array for a potentiometer
const BUTTON_COMBO_POTENTIOMETER: usize = 13;
/// The extra button for the power pad
const BUTTON_COMBO_POWERPAD: usize = 14;

/// The combination of all possible buttons on a controller.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ButtonCombination {
    buttons: [Option<Button>; 15],
    /// Prevents up and down at the same time, and left and right at the same time when active.
    arrow_restrict: bool,
}

impl ButtonCombination {
    /// Create a blank set of button combinations. No buttons are pressed.
    pub fn new() -> Self {
        Self {
            buttons: [
                None,
                Some(Button::TurboA(false, Duration::from_millis(50))),
                Some(Button::TurboB(false, Duration::from_millis(50))),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(Button::Potentiometer(127)),
                None,
            ],
            arrow_restrict: true,
        }
    }

    /// Set the status of a single button for a button combination
    pub fn set_button(&mut self, b: Button) {
        match b {
            Button::A => {
                self.buttons[BUTTON_COMBO_A] = Some(b);
            }
            Button::TurboA(enabled, rate) => {
                self.buttons[BUTTON_COMBO_TURBOA] = Some(Button::TurboA(enabled, rate));
            }
            Button::TurboB(enabled, rate) => {
                self.buttons[BUTTON_COMBO_TURBOB] = Some(Button::TurboB(enabled, rate));
            }
            Button::B => {
                self.buttons[BUTTON_COMBO_B] = Some(b);
            }
            Button::Start => {
                self.buttons[BUTTON_COMBO_START] = Some(b);
            }
            Button::Slow => {
                self.buttons[BUTTON_COMBO_SLOW] = Some(b);
            }
            Button::Select => {
                self.buttons[BUTTON_COMBO_SELECT] = Some(b);
            }
            Button::Up => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_DOWN].is_none() {
                    self.buttons[BUTTON_COMBO_UP] = Some(b);
                }
            }
            Button::Down => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_UP].is_none() {
                    self.buttons[BUTTON_COMBO_DOWN] = Some(b);
                }
            }
            Button::Left => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_RIGHT].is_none() {
                    self.buttons[BUTTON_COMBO_LEFT] = Some(b);
                }
            }
            Button::Right => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_LEFT].is_none() {
                    self.buttons[BUTTON_COMBO_RIGHT] = Some(b);
                }
            }
            Button::Fire => {
                self.buttons[BUTTON_COMBO_FIRE] = Some(b);
            }
            Button::LightSensor => {
                self.buttons[BUTTON_COMBO_LIGHT] = Some(b);
            }
            Button::Potentiometer(val) => {
                self.buttons[BUTTON_COMBO_POTENTIOMETER] = Some(Button::Potentiometer(val));
            }
            Button::PowerPad => {
                self.buttons[BUTTON_COMBO_POWERPAD] = Some(b);
            }
        }
    }

    /// Clear a button for a button combination.
    pub fn clear_button(&mut self, b: Button) {
        match b {
            Button::A => {
                self.buttons[BUTTON_COMBO_A] = None;
            }
            Button::TurboA(enabled, rate) => {
                self.buttons[BUTTON_COMBO_TURBOA] = Some(Button::TurboA(enabled, rate));
            }
            Button::TurboB(enabled, rate) => {
                self.buttons[BUTTON_COMBO_TURBOB] = Some(Button::TurboB(enabled, rate));
            }
            Button::B => {
                self.buttons[BUTTON_COMBO_B] = None;
            }
            Button::Start => {
                self.buttons[BUTTON_COMBO_START] = None;
            }
            Button::Slow => {
                self.buttons[BUTTON_COMBO_SLOW] = None;
            }
            Button::Select => {
                self.buttons[BUTTON_COMBO_SELECT] = None;
            }
            Button::Up => {
                self.buttons[BUTTON_COMBO_UP] = None;
            }
            Button::Down => {
                self.buttons[BUTTON_COMBO_DOWN] = None;
            }
            Button::Left => {
                self.buttons[BUTTON_COMBO_LEFT] = None;
            }
            Button::Right => {
                self.buttons[BUTTON_COMBO_RIGHT] = None;
            }
            Button::Fire => {
                self.buttons[BUTTON_COMBO_FIRE] = None;
            }
            Button::LightSensor => {
                self.buttons[BUTTON_COMBO_LIGHT] = None;
            }
            Button::Potentiometer(val) => {
                self.buttons[BUTTON_COMBO_POTENTIOMETER] = Some(Button::Potentiometer(val));
            }
            Button::PowerPad => {
                self.buttons[BUTTON_COMBO_POWERPAD] = None;
            }
        }
    }
}

/// The trait the all controllers must implement. This must be capable of handling all the various types of controllers.
///
/// Standard controller with 8 buttons.
///
/// Arkanoid controller with 9-bit potentiometer and fire button.
///
/// Four score adapter - allows four controllers and turbo buttons for all.
///
/// Power pad - 12 buttons.
///
/// Zapper - trigger and light sensor.
#[enum_dispatch::enum_dispatch]
pub trait NesControllerTrait {
    /// Clock signal for the controller, must implement additional logic for edge sensitive behavior
    fn clock(&mut self, c: bool);
    /// Used to operate the rapid fire mechanisms. time is the time since the last call
    fn rapid_fire(&mut self, time: Duration);
    /// Update the serial/parallel input
    fn parallel_signal(&mut self, s: bool);
    /// Get a mutable iterator of all button combinations for this controller.
    fn get_buttons_iter_mut(&mut self) -> std::slice::IterMut<'_, ButtonCombination>;
    /// Dump data from the controller. No side effects.
    fn dump_data(&self) -> u8;
    /// Read data from the controller.
    fn read_data(&mut self) -> u8;
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
    /// The standard button combination
    combo: [ButtonCombination; 1],
    /// The contents of the shift register
    shift_register: u8,
    /// The strobe signal triggers loading the controller data into the shift register
    strobe: bool,
    /// The previous clock signal
    prevclk: bool,
    /// The mask for rapid fire operation
    rapid_fire: [bool; 3],
    /// The time since a toggle of rapid_fire
    rapid_time: [Duration; 3],
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
            combo: [ButtonCombination::new()],
            shift_register: 0xff,
            strobe: false,
            prevclk: false,
            rapid_fire: [false; 3],
            rapid_time: [Duration::from_millis(0); 3],
        })
        .into()
    }

    ///convenience function to check the strobe, to determine of the buttons should be loaded to the shift register
    fn check_strobe(&mut self) {
        if self.strobe {
            let controller_buttons = if self.combo[0].buttons[BUTTON_COMBO_A].is_some() {
                BUTTON_A
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_B].is_some() {
                BUTTON_B
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_START].is_some() {
                BUTTON_START
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_SELECT].is_some() {
                BUTTON_SELECT
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_UP].is_some() {
                BUTTON_UP
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_DOWN].is_some() {
                BUTTON_DOWN
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_LEFT].is_some() {
                BUTTON_LEFT
            } else {
                0
            } | if self.combo[0].buttons[BUTTON_COMBO_RIGHT].is_some() {
                BUTTON_RIGHT
            } else {
                0
            };
            self.shift_register = controller_buttons;
        }
    }
}

impl NesControllerTrait for StandardController {
    fn parallel_signal(&mut self, s: bool) {
        self.strobe = s;
        self.check_strobe();
    }

    fn rapid_fire(&mut self, time: Duration) {}

    fn get_buttons_iter_mut(&mut self) -> std::slice::IterMut<'_, ButtonCombination> {
        self.combo.iter_mut()
    }

    fn clock(&mut self, c: bool) {
        let active_high_edge = c && !self.prevclk;
        if active_high_edge && !self.strobe {
            self.shift_register >>= 1;
        }
        self.prevclk = c;
    }

    fn dump_data(&self) -> u8 {
        let data = self.shift_register & 1;
        data | 0x1e
    }

    fn read_data(&mut self) -> u8 {
        let data = self.shift_register & 1;
        data | 0x1e
    }
}
