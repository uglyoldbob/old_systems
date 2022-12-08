pub trait NesMemoryBus {
    fn memory_cycle_read(&mut self, addr: u16, out: [bool; 3], controllers: [bool; 2]) -> u8;
    fn memory_cycle_write(&mut self, addr: u16, data: u8, out: [bool; 3], controllers: [bool; 2]);
}

pub struct NesCpu {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    p: u8,
    pc: u16,
    subcycle: u8,
    interrupts: [bool; 3],
    opcode: Option<u8>,
    temp: u8,
}

impl NesCpu {
    /// Construct a new cpu instance.
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0xfd,
            p: 4, //interrupt disable flag
            subcycle: 0,
            pc: 0xfffc,
            interrupts: [false, true, false],
            opcode: None,
            temp: 0,
        }
    }

    pub fn reset(&mut self) {
        self.s -= 3;
        self.p |= 4; //set IRQ disable flag
    }

    pub fn cycle(&mut self, bus: &mut dyn NesMemoryBus) {
        if self.interrupts[1] {
            match self.subcycle {
                0 => {
                    bus.memory_cycle_read(self.pc, [false; 3], [true; 2]);
                    self.subcycle += 1;
                }
                1 => {
                    bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                    self.subcycle += 1;
                }
                2 => {
                    bus.memory_cycle_read(self.s as u16 + 0x100, [false; 3], [true; 2]);
                    self.subcycle += 1;
                }
                3 => {
                    bus.memory_cycle_read(self.s as u16 + 0xff, [false; 3], [true; 2]);
                    self.subcycle += 1;
                }
                4 => {
                    bus.memory_cycle_read(self.s as u16 + 0xfe, [false; 3], [true; 2]);
                    self.subcycle += 1;
                }
                5 => {
                    let pcl = bus.memory_cycle_read(0xfffc, [false; 3], [true; 2]);
                    let mut pc = self.pc.to_le_bytes();
                    pc[0] = pcl;
                    self.pc = u16::from_le_bytes(pc);
                    self.subcycle += 1;
                }
                _ => {
                    let pch = bus.memory_cycle_read(0xfffd, [false; 3], [true; 2]);
                    let mut pc = self.pc.to_le_bytes();
                    pc[1] = pch;
                    self.pc = u16::from_le_bytes(pc);
                    self.subcycle += 1;
                    self.subcycle = 0;
                    self.interrupts[1] = false;
                }
            }
        } else {
            if let None = self.opcode {
                self.opcode = Some(bus.memory_cycle_read(self.pc, [false; 3], [true; 2]));
            }
            if let Some(o) = self.opcode {
                match o {
                    //jmp absolute
                    0x4c => match self.subcycle {
                        0 => {
                            self.subcycle = 1;
                        }
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            let t2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                            self.pc = newpc;
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //ldx immediate
                    0xa2 => match self.subcycle {
                        0 => {
                            self.subcycle = 1;
                        }
                        _ => {
                            self.x = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //stx zero page
                    0x86 => match self.subcycle {
                        0 => {
                            self.subcycle = 1;
                        }
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            bus.memory_cycle_write(self.temp as u16, self.x, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //jsr absolute
                    0x20 => match self.subcycle {
                        0 => {
                            self.subcycle = 1;
                        }
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let pc = self.pc.to_le_bytes();
                            bus.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[1],
                                [false; 3],
                                [true; 2],
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 4;
                        }
                        4 => {
                            let pc = self.pc.to_le_bytes();
                            bus.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[0],
                                [false; 3],
                                [true; 2],
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 5;
                        }
                        _ => {
                            let t2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                            self.pc = newpc;
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //nop
                    0xea => {
                        self.pc = self.pc.wrapping_add(1);
                        self.opcode = None;
                    }
                    //sec set carry flag
                    0x38 => match self.subcycle {
                        0 => {
                            self.subcycle = 1;
                        }
                        _ => {
                            self.p |= 1;
                            self.pc = self.pc.wrapping_add(1);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    _ => {
                        unimplemented!();
                    }
                }
            }
        }
    }
}
