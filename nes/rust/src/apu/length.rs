//! Module for the length counter of the apu units

/// The length counter
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ApuLength {
    /// The length of the counter
    length: u8,
    /// The halt flag for the counter
    halt: bool,
    /// Indicates a pending change in the halt flag
    pend_halt: Option<bool>,
}

/// A lookup table for setting the length of the audio channels
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

impl ApuLength {
    /// Create a new length counter
    pub fn new() -> Self {
        Self {
            length: 0,
            halt: false,
            pend_halt: None,
        }
    }

    /// Clock the length counter
    pub fn clock(&mut self) {
        if self.length > 0 && !self.halt {
            self.length -= 1;
            if self.length == 0 {
                println!("Length ended");
            }
        }
    }

    /// Set the length of the counter with a lookup
    pub fn set_length(&mut self, index: u8) {
        self.length = LENGTH_TABLE[index as usize];
        println!("Load length with {}", self.length);
    }

    /// Sets the halt flag
    pub fn set_halt(&mut self, h: bool) {
        self.pend_halt = Some(h);
        self.halt = h;
    }

    /// Returns true if the length counter is running
    pub fn running(&self) -> bool {
        self.length != 0
    }

    /// Stop the counter immediately
    pub fn stop(&mut self) {
        self.length = 0;
    }
}
