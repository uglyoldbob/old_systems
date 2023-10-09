//! Responsible for emulating the details of the audio processing (apu) of the nes console.

use biquad::Biquad;

///The modes that the sweep can operate in
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ApuSweepAddition {
    /// The math uses ones complement numbers
    OnesComplement,
    /// The math uses twos complement numbers
    TwosComplement,
}

/// An sweep unit for the square channels of the apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ApuSweep {
    /// The math mode for the sweep unit
    mode: ApuSweepAddition,
    /// The counter for division
    counter: u8,
    /// The reload flag
    reload: bool,
    /// The calculated mute output of the unit
    mute: bool,
}

impl ApuSweep {
    /// Create a new apu sweep
    fn new(math: ApuSweepAddition) -> Self {
        Self {
            mode: math,
            counter: 0,
            reload: false,
            mute: false,
        }
    }

    /// Returns the mute flag
    pub fn mute(&self) -> bool {
        self.mute
    }

    /// Reloads the sweep unit later
    pub fn reload(&mut self) {
        self.reload = true;
    }

    /// Clock the unit
    fn clock(&mut self, data: &[u8], permod: &mut u16) {
        let enabled = (data[1] & 0x80) != 0;
        let negative = (data[1] & 8) != 0;
        let shift = data[1] & 7;
        let period = (data[1] >> 4) & 7;
        let square_period = data[2] as u16 | (data[3] as u16 & 0x7) << 8;

        let mut new_mute = false;
        if square_period < 8 {
            new_mute = true;
        }

        *permod = if self.reload {
            self.counter = period;
            self.reload = false;
            *permod
        } else if self.counter > 0 {
            self.counter -= 1;
            *permod
        } else {
            self.counter = period;
            if enabled && shift != 0 {
                let delta = (square_period >> shift) as u16;
                if negative {
                    match self.mode {
                        ApuSweepAddition::OnesComplement => *permod + (delta ^ 0xFFFF),
                        ApuSweepAddition::TwosComplement => *permod - delta,
                    }
                } else {
                    *permod + delta
                }
            } else {
                *permod
            }
        };

        if *permod > 0x7ff {
            new_mute = true;
        }

        self.mute = new_mute;
    }
}

mod length;
use length::ApuLength;

mod envelope;
use envelope::ApuEnvelope;

mod square;
use square::ApuSquareChannel;

mod noise;
use noise::ApuNoiseChannel;

mod triangle;
use triangle::ApuTriangleChannel;

mod dmc;
use dmc::ApuDmcChannel;

/// The nes apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct NesApu {
    /// Used to divide the input clock by 2
    clock: bool,
    /// Status register
    status: u8,
    /// Frame clock
    fclock: u8,
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
    /// Inhibits clocking the length counters when set
    inhibit_length_clock: bool,
    /// The audio buffer
    buffer: Vec<f32>,
    /// The index into the audio buffer
    buffer_index: usize,
    /// A clock that always runs
    always_clock: usize,
    /// Halt holders for the 4 channels
    pend_halt: [Option<bool>; 4],
}

impl NesApu {
    /// Build a new apu
    pub fn new() -> Self {
        Self {
            clock: false,
            status: 0,
            fclock: 0,
            squares: [
                ApuSquareChannel::new(ApuSweepAddition::OnesComplement),
                ApuSquareChannel::new(ApuSweepAddition::TwosComplement),
            ],
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
            inhibit_length_clock: false,
            buffer: vec![],
            buffer_index: 0,
            always_clock: 0,
            pend_halt: [None; 4],
        }
    }

    /// Returns the length of the audio buffer
    pub fn get_audio_buffer_length(&self) -> usize {
        self.buffer.len()
    }

    /// Initialize the audio buffer
    pub fn set_audio_buffer(&mut self, size: usize) {
        self.buffer = vec![0.0; size];
    }

    /// Reset the apu
    pub fn reset(&mut self) {
        self.status = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
        self.frame_sequencer_reset = 2;
        self.always_clock = 0;
    }

    /// Get the irq line for the apu
    pub fn irq(&self) -> bool {
        (self.status & 0xc0) != 0 && (self.fclock & 0x40) == 0 || self.dmc.interrupt_flag
    }

    /// Get the dmc dma request
    pub fn dma(&self) -> Option<u16> {
        self.dmc.dma_request
    }

    /// Get the clock phase
    pub fn get_clock(&self) -> bool {
        self.clock
    }

    /// Used by the cpu to provide the dma response from the cpu
    /// Used by the dmc channel
    pub fn provide_dma_response(&mut self, data: u8) {
        println!("Provide dmc dma response");
        self.dmc.dma_request = None;
        self.dmc.sample_buffer = Some(data);
        self.dmc.dma_result = Some(data);

        self.dmc.dma_address = self.dmc.dma_address.wrapping_add(1);
        if self.dmc.dma_address == 0 {
            self.dmc.dma_address = 0x8000;
        }

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
        if (self.fclock & 0x40) == 0 {
            self.status |= 0x40;
        }
    }

    /// Operate the frame sequencer
    fn frame_sequencer_clock(&mut self) {
        if !self.clock && self.frame_sequencer_reset > 0 {
            self.frame_sequencer_reset -= 1;
            if self.frame_sequencer_reset == 0 {
                self.frame_sequencer_clock = 0;
            }
        }
        if (self.fclock & 0x80) == 0 {
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
        //TODO clock the triangle linear counter
        self.squares[0].envelope_clock();
        self.squares[1].envelope_clock();
        self.noise.envelope_clock();
    }

    /// The half frame, as determined by the frame sequencer
    fn half_frame(&mut self) {
        //first square length counter
        if !self.inhibit_length_clock {
            self.squares[0].length.clock();
        }
        self.squares[0].clock_sweep();
        //second square length counter
        if !self.inhibit_length_clock {
            self.squares[1].length.clock();
        }
        self.squares[1].clock_sweep();
        //triangle channel length counter
        if !self.inhibit_length_clock {
            self.triangle.length.clock();
        }
        //noise channel length counter
        if !self.inhibit_length_clock {
            self.noise.length.clock();
        }
    }

    /// Get the interval between audio samples
    pub fn get_audio_interval(&self) -> f32 {
        self.sample_interval
    }

    /// Set the interval between audio samples
    pub fn set_audio_interval(&mut self, interval: f32) {
        self.sample_interval = interval;
    }

    /// Fill the local audio buffer with data, returning a Some when it is full
    fn fill_audio_buffer(&mut self, sample: f32) -> Option<&[f32]> {
        self.buffer[self.buffer_index] = sample;
        if self.buffer_index < (self.buffer.len() - 1) {
            self.buffer_index += 1;
            None
        } else {
            self.buffer_index = 0;
            Some(&self.buffer)
        }
    }

    /// Build an audio sample and run the audio filter
    fn build_audio_sample(
        &mut self,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) -> Option<f32> {
        let audio = self.squares[0].audio()
            + self.squares[1].audio()
            + self.triangle.audio()
            + self.noise.audio()
            + self.dmc.audio();
        if let Some(filter) = filter {
            let e = filter.run(audio / 2.5 - 1.0);
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

    /// Clock the apu
    pub fn clock_slow(
        &mut self,
        sound: &mut Option<crossbeam_channel::Sender<f32>>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.always_clock = self.always_clock.wrapping_add(1);
        self.frame_sequencer_clock();
        if let Some(h) = self.pend_halt[0].take() {
            self.squares[0].length.set_halt(h);
        }
        if let Some(h) = self.pend_halt[1].take() {
            self.squares[1].length.set_halt(h);
        }
        if let Some(h) = self.pend_halt[2].take() {
            self.triangle.length.set_halt(h);
        }
        if let Some(h) = self.pend_halt[3].take() {
            self.noise.length.set_halt(h);
        }
        self.inhibit_length_clock = false;
        if self.clock {
            self.timing_clock = self.timing_clock.wrapping_add(1);
            self.squares[0].cycle();
            self.squares[1].cycle();
            self.noise.cycle();
            self.dmc.cycle(self.timing_clock);
        }
        self.clock ^= true;
        self.triangle.cycle();

        if self.sound_disabled_clock < 2048 {
            self.sound_disabled_clock += 1;
        } else if self.sound_disabled_clock == 2048 {
            self.sound_disabled = false;
        }
        if let Some(sample) = self.build_audio_sample(filter) {
            if let Some(sender) = sound {
                sender.send(sample);
            }
        }
    }

    /// A lookup table for setting the dmc rates
    const DMC_RATE_TABLE: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];

    /// Write to an apu register
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.pend_halt[0] = Some((data & 0x20) != 0);
            }
            4 => {
                self.pend_halt[1] = Some((data & 0x20) != 0);
            }
            8 => {
                self.pend_halt[2] = Some((data & 0x80) != 0);
            }
            12 => {
                self.pend_halt[3] = Some((data & 0x20) != 0);
            }
            _ => {}
        }
        match addr {
            3 => {
                let length = data >> 3;
                if (self.status & (1 << 0)) != 0
                    && self.squares[0].length_enabled
                    && (!self.clock || !self.squares[0].length.running())
                {
                    self.squares[0].length.set_length(length);
                    self.inhibit_length_clock = true;
                }
                self.squares[0].envelope.restart();
            }
            7 => {
                let length = data >> 3;
                if (self.status & (1 << 1)) != 0
                    && self.squares[1].length_enabled
                    && (!self.clock || !self.squares[1].length.running())
                {
                    self.squares[1].length.set_length(length);
                    self.inhibit_length_clock = true;
                }
                self.squares[1].envelope.restart();
            }
            0xb => {
                let length = data >> 3;
                if (self.status & (1 << 2)) != 0
                    && self.triangle.length_enabled
                    && (!self.clock || !self.triangle.length.running())
                {
                    self.triangle.length.set_length(length);
                    self.inhibit_length_clock = true;
                }
            }
            0xf => {
                let length = data >> 3;
                if (self.status & (1 << 3)) != 0
                    && self.noise.length_enabled
                    && (!self.clock || !self.noise.length.running())
                {
                    self.noise.length.set_length(length);
                    self.inhibit_length_clock = true;
                }
                self.noise.envelope.restart();
            }
            0x10 => {
                self.dmc.interrupt_flag = false;
                self.dmc.rate = NesApu::DMC_RATE_TABLE[(data & 0xF) as usize] / 2 - 1;
                self.dmc.interrupt_enable = (data & 0x80) != 0;
                self.dmc.loop_flag = (data & 0x40) != 0;
            }
            0x11 => {
                self.dmc.output = data & 0x7f;
            }
            0x12 => {
                self.dmc.dma_address = 0xC000 + (data as u16 * 64);
            }
            0x13 => {
                self.dmc.programmed_length = (data as u16) * 16 + 1;
                self.dmc.registers[3] = data;
            }
            0x15 => {
                let data2 = (self.status & 0x60) | (data & 0x1f);
                self.squares[0].length_enabled = (data2 & 1) != 0;
                if (data2 & 1) == 0 {
                    self.squares[0].length.stop();
                }
                self.squares[1].length_enabled = (data2 & 2) != 0;
                if (data2 & 2) == 0 {
                    self.squares[1].length.stop();
                }
                self.triangle.length_enabled = (data2 & 4) != 0;
                if (data2 & 4) == 0 {
                    self.triangle.length.stop();
                }
                self.noise.length_enabled = (data2 & 8) != 0;
                if (data2 & 8) == 0 {
                    self.noise.length.stop();
                }
                if (data2 & 0x10) == 0 {
                    self.dmc.length = 0;
                } else if self.dmc.length == 0 {
                    self.dmc.programmed_length = (self.dmc.registers[3] as u16) * 16 + 1;
                    self.dmc.length = self.dmc.programmed_length;
                    println!("Set dmc length");
                    self.dmc.playing = true;
                }
                self.dmc.interrupt_flag = false;
                self.status = data2;
            }
            0x17 => {
                self.frame_sequencer_reset = 2;
                if (data & 0x80) != 0 {
                    self.half_frame();
                }
                self.fclock = data;
                if (data & 0x40) != 0 {
                    self.status &= !0x40;
                }
            }
            _ => {}
        }
        match addr {
            1 => {
                self.squares[0].sweep_reload();
            }
            5 => {
                self.squares[1].sweep_reload();
            }
            _ => {}
        }
        match addr {
            0..=3 => self.squares[0].registers[addr as usize] = data,
            4..=7 => self.squares[1].registers[(addr & 3) as usize] = data,
            8..=11 => self.triangle.registers[(addr & 3) as usize] = data,
            12..=15 => self.noise.registers[(addr & 3) as usize] = data,
            16..=19 => self.dmc.registers[(addr & 3) as usize] = data,
            _ => {}
        }
    }

    /// Dump, like read, but without side effects
    pub fn dump(&self, _addr: u16) -> u8 {
        let mut data = self.status & 0x40;
        if self.dmc.interrupt_flag {
            data |= 0x80;
        }
        if self.squares[0].length.running() {
            data |= 1;
        }
        if self.squares[1].length.running() {
            data |= 1 << 1;
        }
        if self.triangle.length.running() {
            data |= 1 << 2;
        }
        if self.noise.length.running() {
            data |= 1 << 3;
        }
        if self.dmc.length > 0 {
            data |= 1 << 4;
        }
        data
    }

    /// Read the apu register, it is assumed that the only readable address is filtered before making it to this function
    pub fn read(&mut self, addr: u16) -> u8 {
        let data = self.dump(addr);
        self.status &= !0x40;
        data
    }
}
