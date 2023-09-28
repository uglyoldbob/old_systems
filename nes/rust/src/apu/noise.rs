//! The noise module for the nes apu

use super::ApuEnvelope;

/// A noise channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ApuNoiseChannel {
    /// The channel registers
    pub registers: [u8; 4],
    /// The length of the channel
    pub length: u8,
    /// Set when length loading should be active
    pub length_enabled: bool,
    /// The counter for the channel
    counter: u16,
    /// The envelope for sound generation
    pub envelope: ApuEnvelope,
    /// The shift counter for random noise generation
    shift_ctr: u16,
}
/// The periods for the various noise channel settings. Units are clock cycles.
const FREQ_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

impl ApuNoiseChannel {
    /// Create a new channel
    pub fn new() -> Self {
        Self {
            registers: [0; 4],
            length: 0,
            length_enabled: false,
            counter: 0,
            envelope: ApuEnvelope::new(),
            shift_ctr: 1,
        }
    }

    /// clock the channel
    pub fn cycle(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = FREQ_TABLE[(self.registers[2] & 0xF) as usize] - 1;

            let shift = if (self.registers[3] & 0x80) != 0 {
                6
            } else {
                1
            };
            let bit1 = self.shift_ctr & 1; // Bit 0
            let bit2 = (self.shift_ctr >> shift) & 1; // Bit 1 or 6 from above
            self.shift_ctr = (self.shift_ctr & 0x7fff) | ((bit1 ^ bit2) << 14);
            self.shift_ctr >>= 1;
        }
    }

    /// Clock the envelope
    pub fn envelope_clock(&mut self) {
        self.envelope.clock(&self.registers);
    }

    /// Return the audio sample for this channel
    pub fn audio(&self) -> f32 {
        if ((self.shift_ctr & 1) == 0) && self.length != 0 {
            self.envelope.audio_output(&self.registers[..]) as f32 / 255.0
        } else {
            0.0
        }
    }
}
