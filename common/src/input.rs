//! dfor user input related code

use eframe::egui;

/// The types of user input that can be accepted
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
pub enum UserInput {
    /// User input provided by egui input layer
    Egui(egui::Key),
    /// User input provided by a button from gilrs
    GilrsButton(gilrs::GamepadId, gilrs::ev::Code),
    /// User input button provided by an axis from gilrs, true means positive direction
    GilrsAxisButton(gilrs::GamepadId, gilrs::ev::Code, bool),
    /// No input at all
    NoInput,
}

impl UserInput {
    /// Conver the user input to a string, suitable for the user to see.
    pub fn to_string(self) -> String {
        match self {
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
                    if dir { " Positive" } else { " Negative" }
                )
            }
            UserInput::NoInput => "None".to_string(),
        }
    }
}
