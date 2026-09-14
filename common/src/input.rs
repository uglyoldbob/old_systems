//! for user input related code

use eframe::egui;

/// The types of user input that can be accepted
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum UserInput {
    /// User input provided by egui input layer
    Egui(egui::Key),
    /// User input provided by a button from gilrs
    GilrsButton(gilrs::GamepadId, gilrs::ev::Code),
    /// User input button provided by an axis from gilrs, true means positive direction
    GilrsAxisButton(gilrs::GamepadId, gilrs::ev::Code, bool),
    /// A bluetooth controller input
    /// bluetooth address and button index
    BluetoothButton([u8; 6], u8),
    /// No input at all
    NoInput,
}

impl UserInput {
    /// Convert the user input to a string, suitable for the user to see.
    pub fn as_string(&self) -> String {
        match self {
            UserInput::BluetoothButton(address, index) => {
                format!("{:x?} {}", address, index)
            }
            UserInput::Egui(k) => {
                format!("{:?}", k)
            }
            UserInput::GilrsButton(id, b) => {
                format!("Gamepad {} {:?}", id, b)
            }
            UserInput::GilrsAxisButton(id, a, dir) => {
                format!(
                    "Gamepad {} {:?} {}",
                    id,
                    a,
                    if *dir { " Positive" } else { " Negative" }
                )
            }
            UserInput::NoInput => "None".to_string(),
        }
    }
}
