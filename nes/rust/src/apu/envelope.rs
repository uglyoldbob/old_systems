//! Envelope module for the nes apu

/// An envelope sequencer for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ApuEnvelope {
    /// Initiates reload of the timers
    startflag: bool,
    /// The divider for feeding the decay timer
    divider: u8,
    /// The counter for envelope output
    decay: u8,
}

impl ApuEnvelope {
    /// Create a new envelope
    pub fn new() -> Self {
        Self {
            startflag: false,
            divider: 0,
            decay: 0,
        }
    }

    pub fn restart(&mut self) {
        self.startflag = true;
    }

    /// Returns the audio level of the envelope
    pub fn audio_output(&self, regs: &[u8]) -> u8 {
        let cv_flag = (regs[0] & 0x10) != 0;
        if cv_flag {
            regs[0] & 0xF
        } else {
            self.decay
        }
    }

    /// Clock the envelope
    pub fn clock(&mut self, regs: &[u8]) {
        let cv = regs[0] & 0xF;
        // True when the envelope should loop
        let eloop = (regs[0] & 0x20) != 0;

        if !self.startflag {
            if self.divider == 0 {
                self.divider = cv;
                if self.decay > 0 {
                    self.decay -= 1;
                } else if eloop {
                    self.decay = 15;
                }
            }
        } else {
            self.decay = 15;
            self.divider = cv;
        }
    }
}