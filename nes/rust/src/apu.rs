//! Responsible for emulating the details of the audio processing (apu) of the nes console.

use biquad::Biquad;
use rb::RbProducer;

/// An envelope sequencer for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
struct ApuEnvelope {
    /// Initiates reload of the timers
    startflag: bool,
    /// The divider for feeding the decay timer
    divider: u8,
    /// The counter for envelope output
    decay: u8,
}

impl ApuEnvelope {
    /// Create a new envelope
    fn new() -> Self {
        Self {
            startflag: false,
            divider: 0,
            decay: 0,
        }
    }

    /// Returns the audio level of the envelope
    fn audio_output(&self, regs: &[u8]) -> u8 {
        let cv_flag = (regs[0] & 0x10) != 0;
        if cv_flag {
            regs[0] & 0xF
        } else {
            self.decay
        }
    }

    /// Clock the envelope
    fn clock(&mut self, regs: &[u8]) {
        let cv = regs[0] & 0xF;
        // True when the envelope should loop
        let eloop = (regs[0] & 0x20) != 0;

        if !self.startflag {
            if self.divider == 0 {
                self.divider = cv;
                if self.decay > 0 {
                    self.decay -= 1;
                }
                else if eloop {
                    self.decay = 15;
                }
            }
        }
        else {
            self.decay = 15;
            self.divider = cv;
        }
    }
}

/// A square channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
struct ApuSquareChannel {
    /// The length of the channel for playback
    length: u8,
    /// The counter for the channel
    counter: u8,
    /// The envelope for sound generation
    envelope: ApuEnvelope,
}

impl ApuSquareChannel {
    /// Create a new square channel for the apu
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
            envelope: ApuEnvelope::new(),
        }
    }

    /// Clock the channel
    fn cycle(&mut self) {}

    /// Return the audio sample for this channel
    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

/// A noise channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
struct ApuNoiseChannel {
    /// The length of the channel
    length: u8,
    /// The counter for the channel
    counter: u8,
    /// The envelope for sound generation
    envelope: ApuEnvelope,
}

impl ApuNoiseChannel {
    /// Create a new channel
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
            envelope: ApuEnvelope::new(),
        }
    }

    /// clock the channel
    fn cycle(&mut self) {}

    /// Return the audio sample for this channel
    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

/// A triangle channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
struct ApuTriangleChannel {
    /// The length of the channel for playback
    length: u8,
    /// The counter for the channel
    counter: u8,
}

impl ApuTriangleChannel {
    /// Create a new triangle channel
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
        }
    }

    /// Clock the channel
    fn cycle(&mut self) {}

    /// Return the audio sample for this channel
    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

/// A dmc channel for the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
struct ApuDmcChannel {
    /// The interrupt flag
    interrupt_flag: bool,
    /// The interrupt is enabled
    interrupt_enable: bool,
    /// Used for addressinng the individual bits of the audio sample
    bit_counter: u8,
    /// The programmed rate for the channel
    rate: u16,
    /// The counter for the divider used in the channel
    rate_counter: u16,
    /// The programmed length for the channel
    programmed_length: u16,
    /// Length parameter for playback
    length: u16,
    /// The sample buffer to play from
    sample_buffer: Option<u8>,
    /// The contents of the shift register
    shift_register: u8,
    /// The potential address for a dma request
    dma_request: Option<u16>,
    /// The result of a dma operation
    dma_result: Option<u8>,
    /// The address to use for dma
    dma_address: u16,
    /// True when the channel is looping
    loop_flag: bool,
    /// True when the channel is playing
    playing: bool,
    /// True when the channel is silent
    silence: bool,
    /// The stored output for the channel
    output: u8,
}

impl ApuDmcChannel {
    /// Create a new dmc channel
    fn new() -> Self {
        Self {
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

    ///Clock the dmc channel
    fn cycle(&mut self, _timing: u32) {
        if self.sample_buffer.is_none() && self.dma_request.is_none() && self.length > 0 {
            self.dma_request = Some(self.dma_address | 0x8000);
            self.length -= 1;
        }
        if self.rate_counter > 0 {
            self.rate_counter -= 1;
        } else {
            self.rate_counter = self.rate;
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
    fn audio(&self) -> f32 {
        self.output as f32
    }
}

/// The nes apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesApu {
    /// Used to divide the input clock by 2
    clock: bool,
    /// The registers for the apu
    registers: [u8; 24],
    /// The two square audio channels
    squares: [ApuSquareChannel; 2],
    /// The noise audio channel
    noise: ApuNoiseChannel,
    /// The triangle audio channel
    triangle: ApuTriangleChannel,
    /// The dmc audio channel
    dmc: ApuDmcChannel,
    /// The counter for the frame sequencer
    frame_sequencer_clock: u32,
    /// Variable used for the operation of the frame sequencer
    frame_sequencer_reset: u8,
    /// This flag disables sound on startup
    sound_disabled: bool,
    /// The timer for disabling sound on startup
    sound_disabled_clock: u16,
    /// The timing clock used by the dmc channel
    timing_clock: u32,
    /// The index for generating audio samples
    output_index: f32,
    /// The number of slow clock cycles between audio samples
    sample_interval: f32,
}

impl NesApu {
    /// Build a new apu
    pub fn new() -> Self {
        Self {
            clock: false,
            registers: [0; 24],
            squares: [ApuSquareChannel::new(), ApuSquareChannel::new()],
            noise: ApuNoiseChannel::new(),
            triangle: ApuTriangleChannel::new(),
            dmc: ApuDmcChannel::new(),
            frame_sequencer_clock: 0,
            frame_sequencer_reset: 0,
            sound_disabled: true,
            sound_disabled_clock: 0,
            timing_clock: 0,
            output_index: 0.0,
            sample_interval: 100.0,
        }
    }

    /// Reset the apu
    pub fn reset(&mut self) {
        self.registers[0x15] = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
        self.frame_sequencer_reset = 2;
    }

    /// Get the irq line for the apu
    pub fn irq(&self) -> bool {
        (self.registers[0x15] & 0xc0) != 0 && (self.registers[0x17] & 0x40) == 0
    }

    /// Get the dmc dma request
    pub fn dma(&self) -> Option<u16> {
        self.dmc.dma_request
    }

    /// Used by the cpu to provide the dma response from the cpu
    /// Used by the dmc channel
    pub fn provide_dma_response(&mut self, data: u8) {
        self.dmc.dma_request = None;
        self.dmc.sample_buffer = Some(data);
        self.dmc.dma_result = Some(data);

        if self.dmc.length == 0 {
            if self.dmc.loop_flag {
                self.dmc.length = self.dmc.programmed_length;
            } else if self.dmc.interrupt_enable {
                self.dmc.interrupt_flag = true;
            }
        }
    }

    /// Set the interrupt flag from the frame sequencer
    fn set_interrupt_flag(&mut self) {
        if (self.registers[0x17] & 0x40) == 0 {
            self.registers[0x15] |= 0x40;
        }
    }

    /// Operate the frame sequencer
    fn frame_sequencer_clock(&mut self) {
        if !self.clock {
            if self.frame_sequencer_reset > 0 {
                self.frame_sequencer_reset -= 1;
                if self.frame_sequencer_reset == 0 {
                    self.frame_sequencer_clock = 0;
                }
            }
        }
        if (self.registers[0x17] & 0x80) == 0 {
            //4 step sequence
            if self.frame_sequencer_clock == 14915 {
                self.set_interrupt_flag();
                self.frame_sequencer_clock = 0;
            } else if self.frame_sequencer_clock == 3728 && self.clock {
                self.quarter_frame();
            } else if self.frame_sequencer_clock == 7456 && self.clock {
                self.quarter_frame();
                self.half_frame();
            } else if self.frame_sequencer_clock == 11185 && self.clock {
                self.quarter_frame();
            } else if self.frame_sequencer_clock == 14914 {
                self.set_interrupt_flag();
                if self.clock {
                    self.quarter_frame();
                    self.half_frame();
                }
            }
        } else {
            //5 step sequence
            if self.frame_sequencer_clock == 18641 {
                self.frame_sequencer_clock = 0;
            } else if self.frame_sequencer_clock == 3728 && self.clock {
                self.quarter_frame();
            } else if self.frame_sequencer_clock == 7456 && self.clock {
                self.quarter_frame();
                self.half_frame();
            } else if self.frame_sequencer_clock == 11185 && self.clock {
                self.quarter_frame();
            } else if self.frame_sequencer_clock == 18640 && self.clock {
                self.quarter_frame();
                self.half_frame();
            }
        }
        if self.clock {
            self.frame_sequencer_clock += 1;
        }
    }

    /// The quarter frame, as determined by the frame sequencer
    fn quarter_frame(&mut self) {
        //TODO clock the envelopes, and triangle linear counter
        self.squares[0].envelope.clock(&self.registers[0..4]);
        self.squares[1].envelope.clock(&self.registers[4..8]);
        self.noise.envelope.clock(&self.registers[12..16]);
    }

    /// The half frame, as determined by the frame sequencer
    fn half_frame(&mut self) {
        //TODO clock the length counters and sweep units
        //first square length counter
        let halt = (self.registers[0] & 0x20) != 0;
        if !halt && self.squares[0].length > 0 {
            self.squares[0].length -= 1;
        }
        //second square length counter
        let halt = (self.registers[4] & 0x20) != 0;
        if !halt && self.squares[1].length > 0 {
            self.squares[1].length -= 1;
        }
        //triangle channel length counter
        let halt = (self.registers[8] & 0x80) != 0;
        if !halt && self.triangle.length > 0 {
            self.triangle.length -= 1;
        }
        //noise channel length counter
        let halt = (self.registers[12] & 0x20) != 0;
        if !halt && self.noise.length > 0 {
            self.noise.length -= 1;
        }
    }

    /// Set the interval between audio samples
    pub fn set_audio_interval(&mut self, interval: f32) {
        self.sample_interval = interval;
    }

    /// Build an audio sample and run the audio filter
    fn build_audio_sample(&mut self, filter: &mut Option<biquad::DirectForm1<f32>>) -> Option<f32> {
        let audio = self.squares[0].audio()
            + self.squares[1].audio()
            + self.triangle.audio()
            + self.noise.audio()
            + self.dmc.audio();
        if let Some(filter) = filter {
            //let audio = rand::Rng::gen::<f32>(&mut rand::thread_rng());
            let e = filter.run(audio / 5.0);
            self.output_index += 1.0;
            if self.output_index >= self.sample_interval {
                self.output_index -= self.sample_interval;
                Some(e)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Clock the apu, this used to do something, now it doesn't
    pub fn clock_slow_pre(&mut self) {}

    /// Clock the apu
    pub fn clock_slow(
        &mut self,
        sound: &mut Option<rb::Producer<f32>>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.frame_sequencer_clock();
        if self.clock {
            self.timing_clock = self.timing_clock.wrapping_add(1);
            self.squares[0].cycle();
            self.squares[1].cycle();
            self.triangle.cycle();
            self.noise.cycle();
            self.dmc.cycle(self.timing_clock);
        }
        self.clock ^= true;

        if self.sound_disabled_clock < 2048 {
            self.sound_disabled_clock += 1;
        } else if self.sound_disabled_clock == 2048 {
            self.sound_disabled = false;
        }
        if let Some(sample) = self.build_audio_sample(filter) {
            if let Some(p) = sound {
                let data: [f32; 1] = [sample];
                let _e = p.write(&data);
            }
        }
    }

    /// A lookup table for setting the length of the audio channels
    const LENGTH_TABLE: [u8; 32] = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96,
        22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];

    /// A lookup table for setting the dmc rates
    const DMC_RATE_TABLE: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];

    /// Write to an apu register
    pub fn write(&mut self, addr: u16, data: u8) {
        let addr2 = addr % 24;
        let mut halt = false;
        match addr {
            0 | 4 | 12 => {
                if (data & 0x20) != 0 {
                    //println!("Halt channel");
                    halt = true;
                }
            }
            8 => {
                if (data & 0x80) != 0 {
                    //println!("Halt triangle channel");
                    halt = true;
                }
            }
            0x10..=0x13 => {
                //println!("DMC write {:x} with {:x}", addr, data);
            }
            _ => {}
        }
        if halt {
            //println!("Halted at {} {}", self.frame_sequencer_clock, self.clock);
        }
        match addr {
            3 => {
                let length = data >> 3;
                if (self.registers[0x15] & 1 << 0) != 0 {
                    self.squares[0].length = NesApu::LENGTH_TABLE[length as usize];
                }
            }
            7 => {
                let length = data >> 3;
                if (self.registers[0x15] & 1 << 1) != 0 {
                    self.squares[1].length = NesApu::LENGTH_TABLE[length as usize];
                }
            }
            0xb => {
                let length = data >> 3;
                if (self.registers[0x15] & 1 << 2) != 0 {
                    self.triangle.length = NesApu::LENGTH_TABLE[length as usize];
                }
            }
            0xf => {
                let length = data >> 3;
                if (self.registers[0x15] & 1 << 3) != 0 {
                    self.noise.length = NesApu::LENGTH_TABLE[length as usize];
                }
            }
            0x10 => {
                self.dmc.interrupt_flag = false;
                self.dmc.rate = NesApu::DMC_RATE_TABLE[(data & 0xF) as usize] / 2 - 1;
                self.dmc.interrupt_enable = (data & 0x80) != 0;
                self.dmc.loop_flag = (data & 0x40) != 0;
            }
            0x13 => {
                self.dmc.programmed_length = (data as u16) * 16 + 1;
                self.registers[addr2 as usize] = data;
            }
            0x15 => {
                let data2 = (self.registers[0x15] & 0x60) | (data & 0x1f);
                if (data2 & 1) == 0 {
                    self.squares[0].length = 0;
                }
                if (data2 & 2) == 0 {
                    self.squares[1].length = 0;
                }
                if (data2 & 4) == 0 {
                    self.triangle.length = 0;
                }
                if (data2 & 8) == 0 {
                    self.noise.length = 0;
                }
                if (data2 & 0x10) == 0 {
                    self.dmc.length = 0;
                } else if self.dmc.length == 0 {
                    self.dmc.programmed_length = (self.registers[0x13] as u16) * 16 + 1;
                    self.dmc.length = self.dmc.programmed_length;
                    self.dmc.playing = true;
                }
                self.dmc.interrupt_flag = false;
                self.registers[addr2 as usize] = data2;
            }
            0x17 => {
                self.frame_sequencer_reset = 2;
                if (data & 0x80) != 0 {
                    self.half_frame();
                }
                self.registers[addr2 as usize] = data;
                if (data & 0x40) != 0 {
                    self.registers[0x15] &= !0x40;
                }
            }
            _ => {}
        }
        match addr {
            0x15 | 0x17 => {}
            _ => {
                self.registers[addr2 as usize] = data;
            }
        }
    }

    /// Read the apu register, it is assumed that the only readable address is filtered before making it to this function
    pub fn read(&mut self, _addr: u16) -> u8 {
        let mut data = self.registers[0x15] & 0x40;
        if self.dmc.interrupt_flag {
            data |= 0x80;
        }

        if self.squares[0].length > 0 {
            data |= 1;
        }
        if self.squares[1].length > 0 {
            data |= 1 << 1;
        }
        if self.triangle.length > 0 {
            data |= 1 << 2;
        }
        if self.noise.length > 0 {
            data |= 1 << 3;
        }
        if self.dmc.length > 0 {
            data |= 1 << 4;
        }
        self.registers[0x15] &= !0x40;
        data
    }
}
