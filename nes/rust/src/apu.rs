use biquad::Biquad;
use rb::RbProducer;

struct ApuSquareChannel {
    length: u8,
    counter: u8,
}

impl ApuSquareChannel {
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
        }
    }

    fn cycle(&mut self) {}

    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

struct ApuNoiseChannel {
    length: u8,
    counter: u8,
}

impl ApuNoiseChannel {
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
        }
    }

    fn cycle(&mut self) {}

    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

struct ApuTriangleChannel {
    length: u8,
    counter: u8,
}

impl ApuTriangleChannel {
    fn new() -> Self {
        Self {
            length: 0,
            counter: 0,
        }
    }

    fn cycle(&mut self) {}

    fn audio(&self) -> f32 {
        self.counter as f32
    }
}

struct ApuDmcChannel {
    interrupt_flag: bool,
    interrupt_enable: bool,
    bit_counter: u8,
    rate: u16,
    rate_counter: u16,
    programmed_length: u16,
    length: u16,
    sample_buffer: Option<u8>,
    shift_register: u8,
    dma_request: Option<u16>,
    dma_result: Option<u8>,
    dma_address: u16,
    loop_flag: bool,
    playing: bool,
    silence: bool,
    output: u8,
}

impl ApuDmcChannel {
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

    fn cycle(&mut self, timing: u32) {
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

    fn audio(&self) -> f32 {
        self.output as f32
    }
}

pub struct NesApu {
    clock: bool,
    registers: [u8; 24],
    squares: [ApuSquareChannel; 2],
    noise: ApuNoiseChannel,
    triangle: ApuTriangleChannel,
    dmc: ApuDmcChannel,
    frame_sequencer_clock: u32,
    frame_sequencer_reset: u8,
    sound_disabled: bool,
    sound_disabled_clock: u16,
    read_count: u8,
    cycles: u32,
    timing_clock: u32,
    filter: Option<biquad::DirectForm1<f32>>,
    last_sample_rate: f32,
    output_index: f32,
    sample_interval: f32,
}

impl NesApu {
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
            read_count: 0,
            cycles: 0,
            timing_clock: 0,
            filter: None,
            last_sample_rate: 0.0,
            output_index: 0.0,
            sample_interval: 100.0,
        }
    }

    pub fn reset(&mut self) {
        self.registers[0x15] = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
        self.frame_sequencer_reset = 2;
    }

    pub fn irq(&self) -> bool {
        (self.registers[0x15] & 0xc0) != 0 && (self.registers[0x17] & 0x40) == 0
    }

    pub fn dma(&self) -> Option<u16> {
        self.dmc.dma_request
    }

    pub fn provide_dma_response(&mut self, data: u8) {
        self.dmc.dma_request = None;
        self.dmc.sample_buffer = Some(data);
        self.dmc.dma_result = Some(data);

        if self.dmc.length == 0 {
            if self.dmc.loop_flag {
                self.dmc.length = self.dmc.programmed_length;
            } else {
                if self.dmc.interrupt_enable {
                    self.dmc.interrupt_flag = true;
                }
            }
        }
    }

    fn set_interrupt_flag(&mut self) {
        if (self.registers[0x17] & 0x40) == 0 {
            self.registers[0x15] |= 0x40;
        }
    }

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
            } else if self.frame_sequencer_clock == 3728 {
                if self.clock {
                    self.quarter_frame();
                }
            } else if self.frame_sequencer_clock == 7456 {
                if self.clock {
                    self.quarter_frame();
                    self.half_frame();
                }
            } else if self.frame_sequencer_clock == 11185 {
                if self.clock {
                    self.quarter_frame();
                }
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
            } else if self.frame_sequencer_clock == 3728 {
                if self.clock {
                    self.quarter_frame();
                }
            } else if self.frame_sequencer_clock == 7456 {
                if self.clock {
                    self.quarter_frame();
                    self.half_frame();
                }
            } else if self.frame_sequencer_clock == 11185 {
                if self.clock {
                    self.quarter_frame();
                }
            } else if self.frame_sequencer_clock == 18640 {
                if self.clock {
                    self.quarter_frame();
                    self.half_frame();
                }
            }
        }
        if self.clock {
            self.frame_sequencer_clock += 1;
        }
    }

    fn quarter_frame(&mut self) {
        //TODO clock the envelopes, and triangle linear counter
    }

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

    fn build_audio_sample(&mut self, rate: u32) -> Option<f32> {
        //TODO Make this a variable, depending on actual cpu speed
        let rf = rate as f32;
        if self.last_sample_rate != rf {
            let sampling_frequency = 21.47727e6 / 12.0;
            let filter_coeff = biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                biquad::Hertz::<f32>::from_hz(sampling_frequency).unwrap(),
                biquad::Hertz::<f32>::from_hz(rf / 2.2).unwrap(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap();
            self.filter = Some(biquad::DirectForm1::<f32>::new(filter_coeff));
            self.last_sample_rate = rf;
            self.sample_interval = sampling_frequency / rf;
        }

        let audio = self.squares[0].audio()
            + self.squares[1].audio()
            + self.triangle.audio()
            + self.noise.audio()
            + self.dmc.audio();
        if let Some(filter) = &mut self.filter {
            let e = filter.run(audio / 5.0);
            self.output_index += 1.0;
            if self.output_index >= self.sample_interval {
                self.output_index -= self.sample_interval;
                Some(e)
            } else {
                None
            }
        }
        else {
            None
        }
    }

    pub fn clock_slow_pre(&mut self) {}

    pub fn clock_slow(&mut self, rate: u32, sound: &mut Option<rb::Producer<f32>>) {
        self.cycles = self.cycles.wrapping_add(1);
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
        if let Some(sample) = self.build_audio_sample(rate) {
            if let Some(p) = sound {
                let data: [f32; 1] = [sample];
                let _e = p.write(&data);
            }
        }
    }

    const LENGTH_TABLE: [u8; 32] = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96,
        22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];

    const DMC_RATE_TABLE: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];

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
                } else {
                    if self.dmc.length == 0 {
                        self.dmc.programmed_length = (self.registers[0x13] as u16) * 16 + 1;
                        self.dmc.length = self.dmc.programmed_length;
                        self.dmc.playing = true;
                    }
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

    //it is assumed that the only readable address is filtered before making it to this function
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
        self.read_count = self.read_count.wrapping_add(1);
        data
    }
}
