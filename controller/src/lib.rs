/// Commands sent from the controller to the emulator
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
pub enum ControllerSend {
    /// The buttons actively held down for the controller, 1 means pressed, 0 means not pressed
    ButtonData(u16),
    /// Get player number
    GetPlayerNumber,
}

/// commands sent from the emulator to the controller
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
pub enum ControllerReceive {
    /// Indicate to the controller which player number the controller is for
    PlayerNumber(Option<u8>),
    /// Acknowledge button presses
    AcknowledgeButtonData,
}