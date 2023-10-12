//! This module is responsible for emulating controllers for the nes system.

use std::time::{Duration, Instant};

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;
use egui_multiwin::egui::InputState;

/// The types of user input that can be accepted
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub enum UserInput {
    Egui(egui_multiwin::egui::Key),
    NoInput,
}

/// Defines how inputs get from user to the ButtonCombination
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub struct ControllerConfig {
    buttons: [UserInput; 15],
}

impl ControllerConfig {
    /// Create a new blank configuration
    pub fn new() -> Self {
        Self {
            buttons: [UserInput::NoInput; 15],
        }
    }

    /// Set the given button with egui data
    pub fn set_key_egui(&mut self, index: usize, k: egui_multiwin::egui::Key) {
        self.buttons[index] = UserInput::Egui(k);
    }
}

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
pub const BUTTON_COMBO_A: usize = 0;
/// The index into the button combination array for turbo A
pub const BUTTON_COMBO_TURBOA: usize = 1;
/// The index into the button combination array for turbo B
pub const BUTTON_COMBO_TURBOB: usize = 2;
/// The index into the button combination array for button b
pub const BUTTON_COMBO_B: usize = 3;
/// The index into the button combination array for button start
pub const BUTTON_COMBO_START: usize = 4;
/// The index into the button combination array for button slow
pub const BUTTON_COMBO_SLOW: usize = 5;
/// The index into the button combination array for button select
pub const BUTTON_COMBO_SELECT: usize = 6;
/// The index into the button combination array for button up
pub const BUTTON_COMBO_UP: usize = 7;
/// The index into the button combination array for button down
pub const BUTTON_COMBO_DOWN: usize = 8;
/// The index into the button combination array for button left
pub const BUTTON_COMBO_LEFT: usize = 9;
/// The index into the button combination array for button right
pub const BUTTON_COMBO_RIGHT: usize = 10;
/// The index into the button combination array for fire/trigger
pub const BUTTON_COMBO_FIRE: usize = 11;
/// The index into the button combination array for a light sensor
pub const BUTTON_COMBO_LIGHT: usize = 12;
/// The index into the button combination array for a potentiometer
pub const BUTTON_COMBO_POTENTIOMETER: usize = 13;
/// The extra button for the power pad
pub const BUTTON_COMBO_POWERPAD: usize = 14;

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

    /// Update what buttons can be updated with an egui input
    pub fn update_egui_buttons(&mut self, i: &InputState, config: &ControllerConfig) {
        for (index, b) in config.buttons.iter().enumerate() {
            match b {
                UserInput::Egui(b) => {
                    if i.key_down(*b) {
                        self.set_button(index, 0);
                    }
                    else {
                        self.clear_button(index);
                    }
                }
                _ => {}
            }
        }
    }

    /// Set the status of a single button for a button combination. val is for the potentiometer value. Only applies for index BUTTON_COMBO_POTENTIOMETER
    fn set_button(&mut self, i: usize, val: u16) {
        match i {
            BUTTON_COMBO_A => {
                self.buttons[BUTTON_COMBO_A] = Some(Button::A);
            }
            BUTTON_COMBO_TURBOA => {
                if let Some(Button::TurboA(_enabled, rate)) = self.buttons[BUTTON_COMBO_TURBOA] {
                    self.buttons[BUTTON_COMBO_TURBOA] = Some(Button::TurboA(true, rate));
                }
            }
            BUTTON_COMBO_TURBOB => {
                if let Some(Button::TurboB(_enabled, rate)) = self.buttons[BUTTON_COMBO_TURBOB] {
                    self.buttons[BUTTON_COMBO_TURBOB] = Some(Button::TurboB(true, rate));
                }
            }
            BUTTON_COMBO_B => {
                self.buttons[BUTTON_COMBO_B] = Some(Button::B);
            }
            BUTTON_COMBO_START => {
                self.buttons[BUTTON_COMBO_START] = Some(Button::Start);
            }
            BUTTON_COMBO_SLOW => {
                self.buttons[BUTTON_COMBO_SLOW] = Some(Button::Slow);
            }
            BUTTON_COMBO_SELECT => {
                self.buttons[BUTTON_COMBO_SELECT] = Some(Button::Select);
            }
            BUTTON_COMBO_UP => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_DOWN].is_none() {
                    self.buttons[BUTTON_COMBO_UP] = Some(Button::Up);
                }
            }
            BUTTON_COMBO_DOWN => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_UP].is_none() {
                    self.buttons[BUTTON_COMBO_DOWN] = Some(Button::Down);
                }
            }
            BUTTON_COMBO_LEFT => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_RIGHT].is_none() {
                    self.buttons[BUTTON_COMBO_LEFT] = Some(Button::Left);
                }
            }
            BUTTON_COMBO_RIGHT => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_LEFT].is_none() {
                    self.buttons[BUTTON_COMBO_RIGHT] = Some(Button::Right);
                }
            }
            BUTTON_COMBO_FIRE => {
                self.buttons[BUTTON_COMBO_FIRE] = Some(Button::Fire);
            }
            BUTTON_COMBO_LIGHT => {
                self.buttons[BUTTON_COMBO_LIGHT] = Some(Button::LightSensor);
            }
            BUTTON_COMBO_POTENTIOMETER => {
                self.buttons[BUTTON_COMBO_POTENTIOMETER] = Some(Button::Potentiometer(val));
            }
            BUTTON_COMBO_POWERPAD => {
                self.buttons[BUTTON_COMBO_POWERPAD] = Some(Button::PowerPad);
            }
            _ => {}
        }
    }

    /// Clear a button for a button combination.
    fn clear_button(&mut self, i: usize) {
        match i {
            BUTTON_COMBO_TURBOA => {
                if let Some(Button::TurboA(_enabled, rate)) = self.buttons[BUTTON_COMBO_TURBOA] {
                    self.buttons[BUTTON_COMBO_TURBOA] = Some(Button::TurboA(false, rate));
                }
            }
            BUTTON_COMBO_TURBOB => {
                if let Some(Button::TurboB(_enabled, rate)) = self.buttons[BUTTON_COMBO_TURBOB] {
                    self.buttons[BUTTON_COMBO_TURBOB] = Some(Button::TurboB(false, rate));
                }
            }
            BUTTON_COMBO_POTENTIOMETER => {
            }
            _ => {
                self.buttons[i] = None;
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
            let controller_buttons = if self.combo[0].buttons[BUTTON_COMBO_A].is_none() {
                0
            } else {
                BUTTON_A
            } | if self.combo[0].buttons[BUTTON_COMBO_B].is_some() {
                0
            } else {
                BUTTON_B
            } | if self.combo[0].buttons[BUTTON_COMBO_START].is_some() {
                0
            } else {
                BUTTON_START
            } | if self.combo[0].buttons[BUTTON_COMBO_SELECT].is_some() {
                0
            } else {
                BUTTON_SELECT
            } | if self.combo[0].buttons[BUTTON_COMBO_UP].is_some() {
                0
            } else {
                BUTTON_UP
            } | if self.combo[0].buttons[BUTTON_COMBO_DOWN].is_some() {
                0
            } else {
                BUTTON_DOWN
            } | if self.combo[0].buttons[BUTTON_COMBO_LEFT].is_some() {
                0
            } else {
                BUTTON_LEFT
            } | if self.combo[0].buttons[BUTTON_COMBO_RIGHT].is_some() {
                0
            } else {
                BUTTON_RIGHT
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
