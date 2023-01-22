struct ApuSquareChannel {
    length: u8,
}

impl ApuSquareChannel {
    fn new() -> Self {
        Self { length: 0 }
    }

    fn cycle(&mut self) {}
}

struct ApuNoiseChannel {
    length: u8,
}

impl ApuNoiseChannel {
    fn new() -> Self {
        Self { length: 0 }
    }

    fn cycle(&mut self) {}
}

struct ApuTriangleChannel {
    length: u8,
}

impl ApuTriangleChannel {
    fn new() -> Self {
        Self { length: 0 }
    }

    fn cycle(&mut self) {}
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
    dma_request: Option<u16>,
    dma_result: Option<u8>,
    dma_address: u16,
    loop_flag: bool,
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
            dma_request: None,
            dma_result: None,
            dma_address: 0,
            loop_flag: false,
        }
    }

    fn cycle(&mut self) {
        if self.dma_result.is_some() {
            self.sample_buffer = self.dma_result;
            self.dma_result = None;
        } else if self.sample_buffer.is_none() && self.dma_request.is_none() && self.length > 0 {
            self.dma_request = Some(self.dma_address | 0x8000);
        }
        if self.rate_counter <= self.rate {
            self.rate_counter += 1;
        } else {
            self.rate_counter = 0;
            if self.bit_counter < 6 {
                self.bit_counter += 1;
            } else {
                self.sample_buffer = None;
                self.bit_counter = 0;
            }
        }
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
        }
    }

    pub fn reset(&mut self) {
        self.registers[0x15] = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
    }

    pub fn irq(&self) -> bool {
        (self.registers[0x15] & 0xc0) != 0 && (self.registers[0x17] & 0x40) == 0
    }

    pub fn dma(&self) -> Option<u16> {
        self.dmc.dma_request
    }

    pub fn provide_dma_response(&mut self, data: u8) {
        self.dmc.dma_request = None;
        self.dmc.dma_result = Some(data);
        if self.dmc.length > 0 {
            self.dmc.length -= 1;
            if self.dmc.length == 0 {
                if self.dmc.loop_flag {
                    self.dmc.length = self.dmc.programmed_length;
                }
                else if self.dmc.interrupt_enable {
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
            println!(
                "Clock square 0 {} {} {}",
                self.squares[0].length, self.frame_sequencer_clock, self.clock
            );
        }
        //second square length counter
        let halt = (self.registers[4] & 0x20) != 0;
        if !halt && self.squares[1].length > 0 {
            self.squares[1].length -= 1;
            println!(
                "Clock square 1 {} {} {}",
                self.squares[1].length, self.frame_sequencer_clock, self.clock
            );
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

    pub fn clock_slow_pre(&mut self) {}

    pub fn clock_slow(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);
        self.frame_sequencer_clock();
        if self.clock {
            self.squares[0].cycle();
            self.squares[1].cycle();
            self.triangle.cycle();
            self.noise.cycle();
            self.dmc.cycle();
        }
        self.clock ^= true;

        if self.sound_disabled_clock < 2048 {
            self.sound_disabled_clock += 1;
        } else if self.sound_disabled_clock == 2048 {
            self.sound_disabled = false;
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
                self.dmc.rate = 1 + NesApu::DMC_RATE_TABLE[(data & 0xF) as usize] / 2;
                self.dmc.interrupt_enable = (data & 0x80) != 0;
                self.dmc.loop_flag = (data & 0x40) != 0;
            }
            0x13 => {
                self.dmc.programmed_length = (data as u16) * 16 + 1;
                println!("Update next length to {}", self.dmc.programmed_length);
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
                        println!("Start length {} {}", self.dmc.length, self.dmc.loop_flag);
                    }
                    else {
                        println!("Do not start dmc");
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
            _ => {
            }
        }
        match addr {
            0x15 | 0x17 => {}
            _ => {
                self.registers[addr2 as usize] = data;
            }
        }
        //println!("WRITE APU REGISTER {:x} with {:x}", addr2, data);
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

        let looping = if self.dmc.loop_flag {
            "LOOP"
        }
        else {
            "NOLOOP"
        };
        println!("READ APU REGISTER AS {:x} {} {} {} {}", data, self.frame_sequencer_clock, (data & 0x10) != 0, self.dmc.length, looping);
        self.registers[0x15] &= !0x40;
        self.read_count = self.read_count.wrapping_add(1);
        data
    }
}
