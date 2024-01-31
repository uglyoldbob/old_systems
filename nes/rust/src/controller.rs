//! This module is responsible for emulating controllers for the nes system.

use std::time::Duration;

use common_emulator::input::UserInput;
#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// Defines how inputs get from user to the ButtonCombination
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub struct ControllerConfig {
    /// The array of user input specified for a controller.
    buttons: [UserInput; 15],
    /// The turbo rates for the a and b buttons.
    rates: [Duration; 2],
}

impl ControllerConfig {
    /// Create a new blank configuration
    pub fn new() -> Self {
        Self {
            buttons: [UserInput::NoInput; 15],
            rates: [Duration::from_millis(50); 2],
        }
    }

    /// Retrieves the array of user inputs
    pub fn get_keys(&self) -> &[UserInput] {
        &self.buttons
    }

    /// Retrieve the rapid fire rate
    pub fn get_rate(&self, index: usize) -> f32 {
        500.0 / self.rates[index].as_millis() as f32
    }

    /// Sets the rapid fire rate for a and b turbo buttons.
    pub fn set_rate(&mut self, index: usize, r: f32) {
        self.rates[index] = Duration::from_millis((500.0 / r) as u64);
    }

    /// Set the given button with egui data
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub fn set_key_egui(&mut self, index: usize, k: egui::Key) {
        self.buttons[index] = UserInput::Egui(k);
    }

    /// Set the given button with gilrs code data
    pub fn set_key_gilrs_button(&mut self, index: usize, id: gilrs::GamepadId, c: gilrs::ev::Code) {
        self.buttons[index] = UserInput::GilrsButton(id, c);
    }

    /// Set the given button with gilrs axis data as a button
    pub fn set_key_gilrs_axis(
        &mut self,
        index: usize,
        id: gilrs::GamepadId,
        c: gilrs::ev::Code,
        dir: bool,
    ) {
        self.buttons[index] = UserInput::GilrsAxisButton(id, c, dir);
    }
}

/// The various buttons used by controllers
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Debug)]
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
    /// Not a button
    None,
}

impl Button {
    /// Returns true when the button is None
    pub fn is_none(&self) -> bool {
        Button::None == *self
    }

    /// Returns true when the button is not None
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
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
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Debug)]
pub struct ButtonCombination {
    /// The buttons for a controller. None generally means a button is not pressed, there are a few exceptions.
    buttons: [Button; 15],
    /// Prevents up and down at the same time, and left and right at the same time when active.
    arrow_restrict: bool,
}

impl ButtonCombination {
    /// Create a blank set of button combinations. No buttons are pressed.
    pub fn new() -> Self {
        Self {
            buttons: [
                Button::None,
                Button::TurboA(false, Duration::from_millis(50)),
                Button::TurboB(false, Duration::from_millis(50)),
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::None,
                Button::Potentiometer(127),
                Button::None,
            ],
            arrow_restrict: true,
        }
    }

    /// Clear all buttons on a button combo
    pub fn clear_buttons(&mut self) {
        for i in BUTTON_COMBO_A..=BUTTON_COMBO_POWERPAD {
            self.clear_button(i);
        }
    }

    /// Update button information with button data from gilrs
    pub fn update_gilrs_buttons(
        &mut self,
        gid: gilrs::GamepadId,
        code: gilrs::ev::Code,
        button: &gilrs::ev::state::ButtonData,
        config: &ControllerConfig,
    ) {
        for (index, b) in config.buttons.iter().enumerate() {
            if index == BUTTON_COMBO_TURBOA {
                self.try_set_rate(index, config.rates[0]);
            }
            if index == BUTTON_COMBO_TURBOB {
                self.try_set_rate(index, config.rates[1]);
            }
            if let UserInput::GilrsButton(id, c) = b {
                if *id == gid && *c == code {
                    if button.is_pressed() {
                        self.set_button(index, 0);
                    } else {
                        self.clear_button(index);
                    }
                }
            }
        }
    }

    /// Update button information with axis data fromm gilrs
    pub fn update_gilrs_axes(
        &mut self,
        gid: gilrs::GamepadId,
        code: gilrs::ev::Code,
        axis: &gilrs::ev::state::AxisData,
        config: &ControllerConfig,
    ) {
        for (index, b) in config.buttons.iter().enumerate() {
            if index == BUTTON_COMBO_TURBOA {
                self.try_set_rate(index, config.rates[0]);
            }
            if index == BUTTON_COMBO_TURBOB {
                self.try_set_rate(index, config.rates[1]);
            }
            if let UserInput::GilrsAxisButton(id, a, dir) = b {
                if *id == gid && *a == code {
                    if *dir {
                        if axis.value() > 0.5 {
                            self.set_button(index, 0);
                        } else {
                            self.clear_button(index);
                        }
                    } else if axis.value() < -0.5 {
                        self.set_button(index, 0);
                    } else {
                        self.clear_button(index);
                    }
                }
            }
        }
    }

    /// Update what buttons can be updated with an egui input
    #[cfg(any(feature = "eframe", feature = "egui-multiwin"))]
    pub fn update_egui_buttons(&mut self, i: &egui::InputState, config: &ControllerConfig) {
        for (index, b) in config.buttons.iter().enumerate() {
            if index == BUTTON_COMBO_TURBOA {
                self.try_set_rate(index, config.rates[0]);
            }
            if index == BUTTON_COMBO_TURBOB {
                self.try_set_rate(index, config.rates[1]);
            }
            if let UserInput::Egui(b) = b {
                if i.key_down(*b) {
                    self.set_button(index, 0);
                } else {
                    self.clear_button(index);
                }
            }
        }
    }

    /// Try to set the rate of the button
    pub fn try_set_rate(&mut self, i: usize, newrate: Duration) {
        match i {
            BUTTON_COMBO_TURBOA => {
                if let Button::TurboA(enabled, _rate) = self.buttons[BUTTON_COMBO_TURBOA] {
                    self.buttons[BUTTON_COMBO_TURBOA] = Button::TurboA(enabled, newrate);
                }
            }
            BUTTON_COMBO_TURBOB => {
                if let Button::TurboB(enabled, _rate) = self.buttons[BUTTON_COMBO_TURBOB] {
                    self.buttons[BUTTON_COMBO_TURBOB] = Button::TurboB(enabled, newrate);
                }
            }
            _ => {}
        }
    }

    /// Is the specified button pressed?
    pub fn pressed(&self, i: usize) -> bool {
        self.buttons[i].is_some()
    }

    /// Set the status of a single button for a button combination. val is for the potentiometer value. Only applies for index BUTTON_COMBO_POTENTIOMETER
    pub fn set_button(&mut self, i: usize, val: u16) {
        match i {
            BUTTON_COMBO_A => {
                self.buttons[BUTTON_COMBO_A] = Button::A;
            }
            BUTTON_COMBO_TURBOA => {
                if let Button::TurboA(_enabled, rate) = self.buttons[BUTTON_COMBO_TURBOA] {
                    self.buttons[BUTTON_COMBO_TURBOA] = Button::TurboA(true, rate);
                }
            }
            BUTTON_COMBO_TURBOB => {
                if let Button::TurboB(_enabled, rate) = self.buttons[BUTTON_COMBO_TURBOB] {
                    self.buttons[BUTTON_COMBO_TURBOB] = Button::TurboB(true, rate);
                }
            }
            BUTTON_COMBO_B => {
                self.buttons[BUTTON_COMBO_B] = Button::B;
            }
            BUTTON_COMBO_START => {
                self.buttons[BUTTON_COMBO_START] = Button::Start;
            }
            BUTTON_COMBO_SLOW => {
                self.buttons[BUTTON_COMBO_SLOW] = Button::Slow;
            }
            BUTTON_COMBO_SELECT => {
                self.buttons[BUTTON_COMBO_SELECT] = Button::Select;
            }
            BUTTON_COMBO_UP => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_DOWN].is_none() {
                    self.buttons[BUTTON_COMBO_UP] = Button::Up;
                }
            }
            BUTTON_COMBO_DOWN => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_UP].is_none() {
                    self.buttons[BUTTON_COMBO_DOWN] = Button::Down;
                }
            }
            BUTTON_COMBO_LEFT => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_RIGHT].is_none() {
                    self.buttons[BUTTON_COMBO_LEFT] = Button::Left;
                }
            }
            BUTTON_COMBO_RIGHT => {
                if !self.arrow_restrict || self.buttons[BUTTON_COMBO_LEFT].is_none() {
                    self.buttons[BUTTON_COMBO_RIGHT] = Button::Right;
                }
            }
            BUTTON_COMBO_FIRE => {
                self.buttons[BUTTON_COMBO_FIRE] = Button::Fire;
            }
            BUTTON_COMBO_LIGHT => {
                self.buttons[BUTTON_COMBO_LIGHT] = Button::LightSensor;
            }
            BUTTON_COMBO_POTENTIOMETER => {
                self.buttons[BUTTON_COMBO_POTENTIOMETER] = Button::Potentiometer(val);
            }
            BUTTON_COMBO_POWERPAD => {
                self.buttons[BUTTON_COMBO_POWERPAD] = Button::PowerPad;
            }
            _ => {}
        }
    }

    /// Clear a button for a button combination.
    pub fn clear_button(&mut self, i: usize) {
        match i {
            BUTTON_COMBO_TURBOA => {
                if let Button::TurboA(_enabled, rate) = self.buttons[BUTTON_COMBO_TURBOA] {
                    self.buttons[BUTTON_COMBO_TURBOA] = Button::TurboA(false, rate);
                }
            }
            BUTTON_COMBO_TURBOB => {
                if let Button::TurboB(_enabled, rate) = self.buttons[BUTTON_COMBO_TURBOB] {
                    self.buttons[BUTTON_COMBO_TURBOB] = Button::TurboB(false, rate);
                }
            }
            BUTTON_COMBO_POTENTIOMETER => {}
            _ => {
                self.buttons[i] = Button::None;
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
    /// Return the data for all button states
    fn button_data(&self) -> ButtonCombination;
}

/// A generic implementation of a NES controller
#[non_exhaustive]
#[enum_dispatch::enum_dispatch(NesControllerTrait)]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, strum::EnumIter, strum::Display, PartialEq,
)]
pub enum NesController {
    StandardController,
    Zapper,
    DummyController,
    FourScore,
}

impl Default for NesController {
    fn default() -> Self {
        NesController::DummyController(DummyController::default())
    }
}

impl NesController {
    /// Converts this type into the NesControllerType struct
    pub fn get_type(&self) -> NesControllerType {
        match self {
            NesController::StandardController(_) => NesControllerType::StandardController,
            NesController::Zapper(_) => NesControllerType::Zapper,
            NesController::DummyController(_) => NesControllerType::None,
            NesController::FourScore(_) => NesControllerType::FourScore,
        }
    }
}

/// The types of controllers that can be plugged into the emulator
#[derive(
    serde::Serialize, serde::Deserialize, Copy, Clone, strum::EnumIter, strum::Display, PartialEq,
)]
pub enum NesControllerType {
    /// A standard controller. More realistically this is like an nes advantage.
    StandardController,
    /// A standard zapper
    Zapper,
    /// Not a real controller. Signifies the lack of a controller.
    None,
    /// The four player controller adapter
    FourScore,
}

impl NesControllerType {
    /// Creates a new controller based on the type specified by this struct.
    pub fn make_controller(&self) -> NesController {
        match self {
            NesControllerType::StandardController => {
                NesController::StandardController(StandardController::default())
            }
            NesControllerType::Zapper => NesController::Zapper(Zapper::default()),
            NesControllerType::None => NesController::DummyController(DummyController::default()),
            NesControllerType::FourScore => NesController::FourScore(FourScore::default()),
        }
    }
}

/// Half of a four score controller adapter. The four score is modeled as two separate controllers.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct FourScore {
    /// The fourscore contains two potential controllers
    combo: [ButtonCombination; 2],
    /// The controllers that the four score uses
    controllers: [Box<NesController>; 2],
    /// The strobe signal triggers loading the controller data into the shift register
    strobe: bool,
    /// The previous clock signal
    prevclk: bool,
    /// The counter for the clocking operation
    clock_counter: u32,
}

impl Default for FourScore {
    fn default() -> Self {
        Self {
            combo: [ButtonCombination::new(); 2],
            controllers: [
                Box::new(NesController::DummyController(DummyController::default())),
                Box::new(NesController::DummyController(DummyController::default())),
            ],
            strobe: false,
            prevclk: false,
            clock_counter: 0,
        }
    }
}

impl FourScore {
    /// Retrieve a mutable reference to a controller
    pub fn get_controller_mut(&mut self, index: u8) -> &mut Box<NesController> {
        &mut self.controllers[index as usize]
    }

    /// Return a reference of the requested controller
    pub fn get_controller_ref(&self, index: u8) -> &NesController {
        self.controllers[index as usize].as_ref()
    }

    /// Return a clone of the requested controller
    pub fn get_controller(&self, index: u8) -> NesController {
        (*self.controllers[index as usize]).clone()
    }

    /// Set the controller of the four score to the given controller
    pub fn set_controller(&mut self, index: u8, nc: NesController) {
        self.controllers[index as usize] = Box::new(nc);
    }

    /// Returns true if the second controller is present
    pub fn has_second_controller(&self) -> bool {
        !matches!(*self.controllers[1], NesController::DummyController(_d))
    }
}

impl NesControllerTrait for FourScore {
    #[doc = " Clock signal for the controller, must implement additional logic for edge sensitive behavior"]
    fn clock(&mut self, c: bool) {
        let active_high_edge = c && !self.prevclk;
        match self.clock_counter {
            0..=7 => {
                self.controllers[0].clock(c);
            }
            8..=15 => {
                self.controllers[1].clock(c);
            }
            _ => {}
        }
        if active_high_edge && !self.strobe && self.clock_counter < 24 {
            self.clock_counter += 1;
        }
        self.prevclk = c;
    }

    #[doc = " Used to operate the rapid fire mechanisms. time is the time since the last call"]
    fn rapid_fire(&mut self, time: Duration) {
        self.controllers[0].rapid_fire(time);
        self.controllers[1].rapid_fire(time);
    }

    #[doc = " Update the serial/parallel input"]
    fn parallel_signal(&mut self, s: bool) {
        self.strobe = s;
        if s {
            self.clock_counter = 0;
        }
        self.controllers[0].parallel_signal(s);
        self.controllers[1].parallel_signal(s);
    }

    fn button_data(&self) -> ButtonCombination {
        self.combo[0]
    }

    #[doc = " Get a mutable iterator of all button combinations for this controller."]
    fn get_buttons_iter_mut(&mut self) -> std::slice::IterMut<'_, ButtonCombination> {
        self.combo.iter_mut()
    }

    #[doc = " Dump data from the controller. No side effects."]
    fn dump_data(&self) -> u8 {
        0
    }

    #[doc = " Read data from the controller."]
    fn read_data(&mut self) -> u8 {
        match self.clock_counter {
            0..=7 => self.controllers[0].read_data(),
            8..=15 => self.controllers[1].read_data(),
            16..=17 => 0,
            18 => 0xFF,
            19..=23 => 0,
            _ => 0xFF,
        }
    }
}

/// A standard nes controller implementation
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq)]
pub struct DummyController {
    /// Required to make the NesControllerTrait functional
    combo: [ButtonCombination; 1],
}

impl Default for DummyController {
    fn default() -> Self {
        Self {
            combo: [ButtonCombination::new()],
        }
    }
}

impl NesControllerTrait for DummyController {
    #[doc = " Clock signal for the controller, must implement additional logic for edge sensitive behavior"]
    fn clock(&mut self, _c: bool) {}

    #[doc = " Used to operate the rapid fire mechanisms. time is the time since the last call"]
    fn rapid_fire(&mut self, _time: Duration) {}

    #[doc = " Update the serial/parallel input"]
    fn parallel_signal(&mut self, _s: bool) {}

    fn button_data(&self) -> ButtonCombination {
        self.combo[0]
    }

    #[doc = " Get a mutable iterator of all button combinations for this controller."]
    fn get_buttons_iter_mut(&mut self) -> std::slice::IterMut<'_, ButtonCombination> {
        self.combo.iter_mut()
    }

    #[doc = " Dump data from the controller. No side effects."]
    fn dump_data(&self) -> u8 {
        0xff
    }

    #[doc = " Read data from the controller."]
    fn read_data(&mut self) -> u8 {
        0xff
    }
}

/// A standard nes controller implementation
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq)]
pub struct Zapper {
    /// The button combination
    combo: [ButtonCombination; 1],
}

impl Default for Zapper {
    fn default() -> Self {
        Self {
            combo: [ButtonCombination::new()],
        }
    }
}

impl Zapper {
    /// Provide zapper specific inputs
    pub fn provide_zapper_data(&mut self, trigger: bool, vision: bool) {
        self.combo[0].buttons[BUTTON_COMBO_FIRE] =
            if trigger { Button::Fire } else { Button::None };
        self.combo[0].buttons[BUTTON_COMBO_LIGHT] = if vision {
            Button::LightSensor
        } else {
            Button::None
        };
    }
}

impl NesControllerTrait for Zapper {
    #[doc = " Clock signal for the controller, must implement additional logic for edge sensitive behavior"]
    fn clock(&mut self, _c: bool) {}

    #[doc = " Used to operate the rapid fire mechanisms. time is the time since the last call"]
    fn rapid_fire(&mut self, _time: Duration) {}

    #[doc = " Update the serial/parallel input"]
    fn parallel_signal(&mut self, _s: bool) {}

    fn button_data(&self) -> ButtonCombination {
        self.combo[0]
    }

    #[doc = " Get a mutable iterator of all button combinations for this controller."]
    fn get_buttons_iter_mut(&mut self) -> std::slice::IterMut<'_, ButtonCombination> {
        self.combo.iter_mut()
    }

    #[doc = " Dump data from the controller. No side effects."]
    fn dump_data(&self) -> u8 {
        let d3 = self.combo[0].buttons[BUTTON_COMBO_LIGHT].is_some();
        let d4 = self.combo[0].buttons[BUTTON_COMBO_FIRE].is_some();
        0xE7 | if d3 { 1 << 3 } else { 0 } | if !d4 { 1 << 4 } else { 0 }
    }

    #[doc = " Read data from the controller."]
    fn read_data(&mut self) -> u8 {
        self.dump_data()
    }
}

/// A standard nes controller implementation
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq)]
pub struct StandardController {
    /// The standard button combination
    combo: [ButtonCombination; 1],
    /// The contents of the shift register
    shift_register: u8,
    /// The strobe signal triggers loading the controller data into the shift register
    strobe: bool,
    /// The previous clock signal
    prevclk: bool,
    /// The mask for and time since a toggle of rapid_fire
    rapid_fire: [(bool, Duration); 3],
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

impl Default for StandardController {
    fn default() -> Self {
        Self {
            combo: [ButtonCombination::new()],
            shift_register: 0xff,
            strobe: false,
            prevclk: false,
            rapid_fire: [(false, Duration::from_millis(0)); 3],
        }
    }
}

impl StandardController {
    ///convenience function to check the strobe, to determine of the buttons should be loaded to the shift register
    fn check_strobe(&mut self) {
        if self.strobe {
            let rapida =
                if let Button::TurboA(flag, _rate) = &self.combo[0].buttons[BUTTON_COMBO_TURBOA] {
                    if *flag {
                        self.rapid_fire[0].0
                    } else {
                        false
                    }
                } else {
                    false
                };
            let rapidb =
                if let Button::TurboB(flag, _rate) = &self.combo[0].buttons[BUTTON_COMBO_TURBOB] {
                    if *flag {
                        self.rapid_fire[1].0
                    } else {
                        false
                    }
                } else {
                    false
                };
            let slow = if let Button::Slow = self.combo[0].buttons[BUTTON_COMBO_SLOW] {
                self.rapid_fire[2].0
            } else {
                false
            };
            let controller_buttons =
                if rapida || self.combo[0].buttons[BUTTON_COMBO_A].is_some() {
                    0
                } else {
                    BUTTON_A
                } | if rapidb || self.combo[0].buttons[BUTTON_COMBO_B].is_some() {
                    0
                } else {
                    BUTTON_B
                } | if slow || self.combo[0].buttons[BUTTON_COMBO_START].is_some() {
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

    fn rapid_fire(&mut self, time: Duration) {
        for (index, (flag, t)) in &mut self.rapid_fire.iter_mut().enumerate() {
            *t += time;
            let req = match index {
                0 => {
                    if let Button::TurboA(_flag, rate) = &self.combo[0].buttons[BUTTON_COMBO_TURBOA]
                    {
                        *rate
                    } else {
                        Duration::from_millis(0)
                    }
                }
                1 => {
                    if let Button::TurboB(_flag, rate) = &self.combo[0].buttons[BUTTON_COMBO_TURBOB]
                    {
                        *rate
                    } else {
                        Duration::from_millis(0)
                    }
                }
                _ => Duration::from_millis(50),
            };
            if *t > req {
                *t -= req;
                *flag = !*flag;
            }
        }
    }

    fn button_data(&self) -> ButtonCombination {
        self.combo[0]
    }

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
        self.dump_data()
    }
}
