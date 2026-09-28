//! for user input related code

use eframe::egui;

/// The types of user input that can be accepted
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum UserInput {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    /// Direct controller button input, used by the touchscreen buttons
    DirectInput(u8),
    /// User input provided by egui input layer
    Egui(egui::Key),
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    /// User input provided by a button from gilrs
    GilrsButton(gilrs::GamepadId, gilrs::ev::Code),
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    /// User input button provided by an axis from gilrs, true means positive direction
    GilrsAxisButton(gilrs::GamepadId, gilrs::ev::Code, bool),
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
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
            #[cfg(any(target_os = "android", target_os = "ios"))]
            UserInput::DirectInput(_) => "NA".to_string(),
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            UserInput::BluetoothButton(address, index) => {
                format!("{:x?} {}", address, index)
            }
            UserInput::Egui(k) => {
                format!("{:?}", k)
            }
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            UserInput::GilrsButton(id, b) => {
                format!("Gamepad {} {:?}", id, b)
            }
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
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
