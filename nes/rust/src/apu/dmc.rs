//! The dmc module for the nes apu

/// A dmc channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ApuDmcChannel {
    /// The channel registers
    pub registers: [u8; 4],
    /// The interrupt flag
    pub interrupt_flag: bool,
    /// The interrupt is enabled
    pub interrupt_enable: bool,
    /// Used for addressinng the individual bits of the audio sample
    pub bit_counter: u8,
    /// The programmed rate for the channel
    pub rate: u16,
    /// The counter for the divider used in the channel
    rate_counter: u16,
    /// The programmed length for the channel
    pub programmed_length: u16,
    /// Length parameter for playback
    pub length: u16,
    /// The sample buffer to play from
    pub sample_buffer: Option<u8>,
    /// The contents of the shift register
    shift_register: u8,
    /// The potential address for a dma request
    pub dma_request: Option<u16>,
    /// The result of a dma operation
    pub dma_result: Option<u8>,
    /// The address to use for dma
    pub dma_address: u16,
    /// True when the channel is looping
    pub loop_flag: bool,
    /// True when the channel is playing
    pub playing: bool,
    /// True when the channel is silent
    silence: bool,
    /// The stored output for the channel
    pub output: u8,
}

impl ApuDmcChannel {
    /// Create a new dmc channel
    pub fn new() -> Self {
        Self {
            registers: [0; 4],
            interrupt_flag: false,
            interrupt_enable: false,
            bit_counter: 0,
            rate: 0,
            rate_counter: 0,
            programmed_length: 0,
            length: 0,
            sample_buffer: None,
            shift_register: 0,
            dma_request: None,
            dma_result: None,
            dma_address: 0,
            loop_flag: false,
            playing: false,
            silence: true,
            output: 0,
        }
    }

    ///Check the dma stuff
    pub fn dma_cycle(&mut self) {
        if self.sample_buffer.is_none() && self.dma_request.is_none() && self.length > 0 {
            self.dma_request = Some(self.dma_address | 0x8000);
            self.length -= 1;
        }
    }

    ///Clock the dmc channel
    pub fn cycle(&mut self, _timing: u32) {
        if self.rate_counter > 0 {
            self.rate_counter -= 1;
        } else {
            self.rate_counter = self.rate;

            if !self.silence && self.playing {
                if (self.shift_register & 1) != 0 {
                    if self.output <= 125 {
                        self.output += 2;
                    }
                } else if self.output >= 2 {
                    self.output -= 2;
                }
                self.shift_register >>= 1;
            }

            if self.bit_counter < 7 {
                self.bit_counter += 1;
            } else {
                self.silence = self.sample_buffer.is_none();
                if let Some(b) = self.sample_buffer {
                    self.shift_register = b;
                    self.sample_buffer = None;
                } else {
                    self.playing = false;
                }
                if self.length == 0 {
                    self.playing = false;
                }
                self.bit_counter = 0;
            }
        }
        //println!("DMC CYCLE {} {}", self.rate_counter, self.bit_counter);
    }

    /// Return the audio sample for this channel
    pub fn audio(&self) -> f32 {
        (self.output as f32) / 255.0
    }
}
