//! Responsible for emulating the details of the audio processing (apu) of the nes console.

use biquad::Biquad;

/// An audio producer of several different kinds of data
pub enum AudioProducer {
    U8(
        ringbuf::Producer<
            u8,
            std::sync::Arc<ringbuf::SharedRb<u8, Vec<std::mem::MaybeUninit<u8>>>>,
        >,
    ),
    U16(
        ringbuf::Producer<
            u16,
            std::sync::Arc<ringbuf::SharedRb<u16, Vec<std::mem::MaybeUninit<u16>>>>,
        >,
    ),
    U32(
        ringbuf::Producer<
            u32,
            std::sync::Arc<ringbuf::SharedRb<u32, Vec<std::mem::MaybeUninit<u32>>>>,
        >,
    ),
    F32(
        ringbuf::Producer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    ),
}

/// An audio consumer of several different kinds of data
pub enum AudioConsumer {
    U8(
        ringbuf::Consumer<
            u8,
            std::sync::Arc<ringbuf::SharedRb<u8, Vec<std::mem::MaybeUninit<u8>>>>,
        >,
    ),
    U16(
        ringbuf::Consumer<
            u16,
            std::sync::Arc<ringbuf::SharedRb<u16, Vec<std::mem::MaybeUninit<u16>>>>,
        >,
    ),
    U32(
        ringbuf::Consumer<
            u32,
            std::sync::Arc<ringbuf::SharedRb<u32, Vec<std::mem::MaybeUninit<u32>>>>,
        >,
    ),
    F32(
        ringbuf::Consumer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    ),
}

impl AudioProducer {
    pub fn push_slice(&mut self, slice: &AudioBuffer) {
        match self {
            AudioProducer::U8(d) => match slice {
                AudioBuffer::U8(s) => {
                    d.push_slice(&s);
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        d.push((t >> 8) as u8);
                    }
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        d.push((t >> 24) as u8);
                    }
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        d.push((t / 255.0) as u8);
                    }
                }
            },
            AudioProducer::U16(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        d.push((*t as u16) * 0x101);
                    }
                }
                AudioBuffer::U16(s) => {
                    d.push_slice(&s);
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        d.push((*t >> 16) as u16);
                    }
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        d.push((t * 65535.0) as u16);
                    }
                }
            },
            AudioProducer::U32(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        d.push((*t as u32) * 0x1010101);
                    }
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        d.push((*t as u32) * 0x10001);
                    }
                }
                AudioBuffer::U32(s) => {
                    d.push_slice(&s);
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        d.push((t * 4294967295.0) as u32);
                    }
                }
            },
            AudioProducer::F32(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        d.push((*t as f32) / 255.0);
                    }
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        d.push((*t as f32) / 65535.0);
                    }
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        d.push((*t as f32) / 4294967295.0);
                    }
                }
                AudioBuffer::F32(s) => {
                    d.push_slice(&s);
                }
            },
        }
    }

    pub fn make_buffer(&self, size: usize) -> AudioBuffer {
        println!("Make audio buffer size {}", size);
        match self {
            AudioProducer::U8(_) => AudioBuffer::U8(vec![0; size]),
            AudioProducer::U16(_) => AudioBuffer::U16(vec![0; size]),
            AudioProducer::U32(_) => AudioBuffer::U32(vec![0; size]),
            AudioProducer::F32(_) => AudioBuffer::F32(vec![0.0; size]),
        }
    }
}

pub struct AudioBufferIterator<'a> {
    data: &'a AudioBuffer,
    index: usize,
}

impl<'a> Iterator for AudioBufferIterator<'a> {
    type Item = AudioSample;
    fn next(&mut self) -> Option<Self::Item> {
        let result = if self.index < self.data.len() {
            Some(match self.data {
                AudioBuffer::U8(d) => AudioSample::U8(d[self.index]),
                AudioBuffer::U16(d) => AudioSample::U16(d[self.index]),
                AudioBuffer::U32(d) => AudioSample::U32(d[self.index]),
                AudioBuffer::F32(d) => AudioSample::F32(d[self.index]),
            })
        } else {
            None
        };
        self.index += 1;
        result
    }
}

pub enum AudioBuffer {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    F32(Vec<f32>),
}

impl AudioBuffer {
    pub fn iter(&self) -> AudioBufferIterator {
        AudioBufferIterator {
            data: &self,
            index: 0,
        }
    }

    pub fn new_f32(size: usize) -> Self {
        println!("Make audio bufferf size {}", size);
        Self::F32(vec![0.0; size])
    }

    pub fn gstreamer_slice(&self) -> Vec<u8> {
        match self {
            AudioBuffer::U8(d) => d.to_vec(),
            AudioBuffer::U16(d) => d.iter().map(|a| a.to_le_bytes()).flatten().collect(),
            AudioBuffer::U32(d) => d.iter().map(|a| a.to_le_bytes()).flatten().collect(),
            AudioBuffer::F32(d) => d
                .iter()
                .map(|a| a.to_bits().to_le_bytes())
                .flatten()
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            AudioBuffer::U8(d) => d.len(),
            AudioBuffer::U16(d) => d.len(),
            AudioBuffer::U32(d) => d.len(),
            AudioBuffer::F32(d) => d.len(),
        }
    }

    pub fn place(&mut self, index: usize, elem: AudioSample) {
        match self {
            AudioBuffer::U8(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s;
                }
                AudioSample::U16(s) => {
                    d[index] = (s >> 8) as u8;
                }
                AudioSample::U32(s) => {
                    d[index] = (s >> 24) as u8;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 255.0) as u8;
                }
            },
            AudioBuffer::U16(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s as u16 * 0x101;
                }
                AudioSample::U16(s) => {
                    d[index] = s;
                }
                AudioSample::U32(s) => {
                    d[index] = (s >> 16) as u16;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 65535.0) as u16;
                }
            },
            AudioBuffer::U32(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s as u32 * 0x1010101;
                }
                AudioSample::U16(s) => {
                    d[index] = s as u32 * 0x10001;
                }
                AudioSample::U32(s) => {
                    d[index] = s;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 4294967295.0) as u32;
                }
            },
            AudioBuffer::F32(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = (s as f32) / 255.0;
                }
                AudioSample::U16(s) => {
                    d[index] = (s as f32) / 65535.0;
                }
                AudioSample::U32(s) => {
                    d[index] = (s as f32) / 4294967295.0;
                }
                AudioSample::F32(s) => {
                    d[index] = s;
                }
            },
        }
    }
}

#[derive(Copy, Clone)]
pub enum AudioSample {
    U8(u8),
    U16(u16),
    U32(u32),
    F32(f32),
}

/// The various ways of producing samples of data
enum AudioProducerMethod {
    /// A ring buffer is used to produce the audio
    RingBuffer(AudioProducer),
    /// The audio is pushed directly to gstreamer for recordings
    GStreamer(gstreamer_app::AppSrc),
}

impl AudioProducerMethod {
    /// Push a slice of data to the audio producer
    fn push_slice(&mut self, slice: &AudioBuffer) {
        match self {
            AudioProducerMethod::RingBuffer(rb) => {
                rb.push_slice(slice);
            }
            AudioProducerMethod::GStreamer(appsrc) => {
                let b: Vec<u8> = slice.gstreamer_slice();
                let buf = gstreamer::Buffer::from_slice(b);
                appsrc.do_timestamp();
                let _e = appsrc.push_buffer(buf).is_err();
            }
        }
    }
}

/// A struct that allows the producer and the rate of sample production to be linked together
pub struct AudioProducerWithRate {
    /// The number of clocks between generated samples
    interval: f32,
    /// The counter for generating samples at the right times
    counter: f32,
    /// The ringbuffer to put samples into
    producer: AudioProducerMethod,
    /// The buffer to put samples into
    buffer: AudioBuffer,
    /// The index of where to store samples
    buffer_index: usize,
}

impl AudioProducerWithRate {
    /// Build a new object
    pub fn new(producer: AudioProducer, size: usize) -> Self {
        let buf = producer.make_buffer(size);
        Self {
            interval: 1.0,
            counter: 0.0,
            producer: AudioProducerMethod::RingBuffer(producer),
            buffer: buf,
            buffer_index: 0,
        }
    }

    /// Create a new object and a new ringbuffer based on size
    pub fn new_gstreamer(size: usize, interval: f32, src: gstreamer_app::AppSrc) -> Self {
        Self {
            interval,
            counter: 0.0,
            producer: AudioProducerMethod::GStreamer(src),
            buffer: AudioBuffer::new_f32(size),
            buffer_index: 0,
        }
    }

    /// Set the interval for generating audio on this stream
    pub fn set_audio_interval(&mut self, i: f32) {
        self.interval = i;
    }

    /// Fill the local buffer from another audiobuffer
    pub fn fill_with_buffer(&mut self, buf: &AudioBuffer) {
        self.producer.push_slice(buf);
    }

    pub fn make_buffer(&self, t: AudioSample, d: &Vec<u8>) -> AudioBuffer {
        match t {
            AudioSample::U8(_) => AudioBuffer::U8(d.to_vec()),
            AudioSample::U16(_) => {
                let a: Vec<u16> = d
                    .chunks_exact(2)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .collect();
                AudioBuffer::U16(a)
            }
            AudioSample::U32(_) => {
                let a: Vec<u32> = d
                    .chunks_exact(4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .collect();
                AudioBuffer::U32(a)
            }
            AudioSample::F32(_) => {
                let a: Vec<f32> = d
                    .chunks_exact(4)
                    .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .collect();
                AudioBuffer::F32(a)
            }
        }
    }

    /// Fill the local audio buffer with data, returning a Some when it is full
    fn fill_audio_buffer(&mut self, sample: AudioSample) {
        self.counter += 1.0;
        if self.counter >= self.interval {
            self.counter -= self.interval;
            self.buffer.place(self.buffer_index, sample);
            let out = if self.buffer_index < (self.buffer.len() - 1) {
                self.buffer_index += 1;
                None
            } else {
                self.buffer_index = 0;
                Some(&self.buffer)
            };
            if let Some(out) = out {
                self.producer.push_slice(out);
            }
        }
    }
}

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
                let delta = square_period >> shift;
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
    /// Inhibits clocking the length counters when set
    inhibit_length_clock: bool,
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
            inhibit_length_clock: false,
            always_clock: 0,
            pend_halt: [None; 4],
        }
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

    /// Build an audio sample and run the audio filter
    fn build_audio_sample(
        &mut self,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) -> Option<AudioSample> {
        let audio = self.squares[0].audio()
            + self.squares[1].audio()
            + self.triangle.audio()
            + self.noise.audio()
            + self.dmc.audio();
        if let Some(filter) = filter {
            let e = filter.run(audio / 5.0);
            self.output_index += 1.0;
            Some(AudioSample::F32(e.min(1.0).max(0.0)))
        } else {
            None
        }
    }

    /// Clock the apu
    pub fn clock_slow(
        &mut self,
        sound: &mut Vec<&mut AudioProducerWithRate>,
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
        self.dmc.dma_cycle();
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
            for p in sound {
                p.fill_audio_buffer(sample);
                p.fill_audio_buffer(sample);
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
