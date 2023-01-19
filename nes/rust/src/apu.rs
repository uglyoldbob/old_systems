struct ApuSquareChannel {
    length: u8,
}

impl ApuSquareChannel {
    fn new() -> Self {
        Self { length: 0 }
    }
}

struct ApuNoiseChannel {
    length: u8,
}

impl ApuNoiseChannel {
    fn new() -> Self {
        Self { length: 0 }
    }
}

struct ApuTriangleChannel {
    length: u8,
}

impl ApuTriangleChannel {
    fn new() -> Self {
        Self { length: 0 }
    }
}

struct ApuDmcChannel {}

impl ApuDmcChannel {
    fn new() -> Self {
        Self {}
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
    frame_sequencer_step: u8,
    frame_sequencer_reset: u8,
    sound_disabled: bool,
    sound_disabled_clock: u16,
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
            frame_sequencer_step: 0,
            frame_sequencer_reset: 0,
            sound_disabled: true,
            sound_disabled_clock: 0,
        }
    }

    pub fn reset(&mut self) {
        self.registers[0x15] = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
    }

    pub fn clock_fast(&mut self) {}

    pub fn irq(&self) -> bool {
        (self.registers[0x15] & 0xc0) != 0 && (self.registers[0x17] & 0x40) == 0
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
            if self.clock {
                self.frame_sequencer_clock += 1;
            }
        } else {
            //5 step sequence
            if self.frame_sequencer_clock == 3728 {
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
            if self.clock {
                self.frame_sequencer_clock += 1;
                if self.frame_sequencer_clock == 18641 {
                    self.frame_sequencer_clock = 0;
                }
            }
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

    pub fn clock_slow(&mut self) {
        self.frame_sequencer_clock();
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

    pub fn write(&mut self, addr: u16, data: u8) {
        let addr2 = addr % 24;
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
                    //dmc bit clear
                    //TODO set dmc bytes remaining to 0
                } else {
                    //TODO restart dmc sample if bytes remaining is 0
                }
                self.registers[addr2 as usize] = data2;
            }
            0x17 => {
                self.frame_sequencer_reset = 2;
                self.registers[addr2 as usize] = data;
                if (data & 0x80) != 0 {
                    self.half_frame();
                }
                if (data & 0x40) != 0 {
                    self.registers[0x15] &= !0x40;
                }
            }
            _ => {
                self.registers[addr2 as usize] = data;
            }
        }
        //println!("WRITE APU REGISTER {:x} with {:x}", addr2, data);
    }

    //it is assumed that the only readable address is filtered before making it to this function
    pub fn read(&mut self, _addr: u16) -> u8 {
        let mut data = self.registers[0x15] & 0xF0;

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

        //println!("READ APU REGISTER AS {:x}", data);
        self.registers[0x15] &= !0x40;
        data
    }
}
