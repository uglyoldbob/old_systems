//! The square channel module for the nes apu

use super::ApuEnvelope;
use super::ApuSweep;
use super::ApuSweepAddition;

/// A square channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ApuSquareChannel {
    /// The channel registers
    pub registers: [u8; 4],
    /// The length of the channel for playback
    pub length: u8,
    /// The counter for the channel
    counter: u8,
    /// The envelope for sound generation
    pub envelope: ApuEnvelope,
    /// The sweep module
    sweep: ApuSweep,
    /// The counter for duty cycle
    duty_counter: u8,
    /// The counter based on the timer register (registers 2 and 3)
    freq_counter: u16,
}

/// A lookup table to determine the duty cycle of the square wave.
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

impl ApuSquareChannel {
    /// Create a new square channel for the apu
    pub fn new(math: ApuSweepAddition) -> Self {
        Self {
            registers: [0; 4],
            sweep: ApuSweep::new(math),
            length: 0,
            counter: 0,
            envelope: ApuEnvelope::new(),
            duty_counter: 0,
            freq_counter: 0,
        }
    }

    /// Reloads the sweep
    pub fn sweep_reload(&mut self) {
        self.sweep.reload();
    }

    /// Retrieves the duty cycle mode.
    pub fn get_duty_mode(&self) -> u8 {
        self.registers[0] >> 6
    }

    /// Calculates the timer period from the registers.
    pub fn get_freq_timer(&self) -> u16 {
        (self.registers[2] as u16) | (((self.registers[3] & 7) as u16) << 8)
    }

    /// Clock the channel
    pub fn cycle(&mut self) {
        if self.freq_counter > 0 {
            self.freq_counter -= 1;
        } else {
            self.freq_counter = self.get_freq_timer();
            self.duty_counter = (self.duty_counter + 1) & 0x07;
        }
    }

    /// Clock the envelope
    pub fn envelope_clock(&mut self) {
        self.envelope.clock(&self.registers);
    }

    /// Clock the sweep
    pub fn clock_sweep(&mut self) {
        let delta = self.sweep.clock(&self.registers) as i32;
        let mut period =
            (self.registers[2] as u16 | ((self.registers[3] & 0x7) as u16) << 8) as i32;
        period = period + delta;
        let new_period = period as u16;
        self.registers[2] = (new_period & 0xFF) as u8;
        self.registers[3] = (self.registers[3] & 0xF8) | ((new_period >> 8) & 7) as u8;
    }

    /// Operates the sweep mechanism
    fn sweep(&mut self) -> bool {
        false
    }

    /// Return the audio sample for this channel
    pub fn audio(&mut self) -> f32 {
        if self.length != 0
            && DUTY_TABLE[self.get_duty_mode() as usize][self.duty_counter as usize] != 0
            && !self.sweep()
        {
            self.envelope.audio_output(&self.registers[..]) as f32 / 255.0
        } else {
            0.0
        }
    }
}
