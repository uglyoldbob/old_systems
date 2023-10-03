//! The module for the nes apu triangle channel

use super::ApuLength;

/// A triangle channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ApuTriangleChannel {
    /// The channel registers
    pub registers: [u8; 4],
    /// The length of the channel for playback
    pub length: ApuLength,
    /// Set when length loading should be active
    pub length_enabled: bool,
    /// The main counter for the channel
    counter: u16,
    /// The index into the sequencer
    sequence_index: u8,
}

/// Sequence used for audio output
const SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

impl ApuTriangleChannel {
    /// Create a new triangle channel
    pub fn new() -> Self {
        Self {
            registers: [0; 4],
            length: ApuLength::new(),
            length_enabled: false,
            counter: 0,
            sequence_index: 0,
        }
    }

    /// Clock the channel
    pub fn cycle(&mut self) {
        let timer = (self.registers[2] as u16) | ((self.registers[3] & 7) as u16) << 8;
        if self.length.running() && timer != 0 {
            if self.counter > 0 {
                self.counter -= 1;
            } else {
                self.counter = timer;
                self.sequence_index = (self.sequence_index + 1) & 0x1f;
            }
        }
    }

    /// Return the audio sample for this channel
    pub fn audio(&self) -> f32 {
        SEQUENCE[self.sequence_index as usize] as f32 / 255.0
    }
}
