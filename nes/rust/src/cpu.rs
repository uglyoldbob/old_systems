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
    temp2: u8,
    tempaddr: u16,
}

const CPU_FLAG_CARRY: u8 = 1;
const CPU_FLAG_ZERO: u8 = 2;
const CPU_FLAG_INT_DISABLE: u8 = 4;
const CPU_FLAG_DECIMAL: u8 = 8;
const CPU_FLAG_B1: u8 = 0x10;
const CPU_FLAG_B2: u8 = 0x20;
const CPU_FLAG_OVERFLOW: u8 = 0x40;
const CPU_FLAG_NEGATIVE: u8 = 0x80;

impl NesCpu {
    /// Construct a new cpu instance.
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0xfd,
            p: CPU_FLAG_B2 | CPU_FLAG_INT_DISABLE,
            subcycle: 0,
            pc: 0xfffc,
            interrupts: [false, true, false],
            opcode: None,
            temp: 0,
            temp2: 0,
            tempaddr: 0,
        }
    }

    pub fn instruction_start(&self) -> bool {
        self.subcycle == 0
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn get_a(&self) -> u8 {
        self.a
    }

    pub fn get_x(&self) -> u8 {
        self.x
    }

    pub fn get_y(&self) -> u8 {
        self.y
    }

    pub fn get_p(&self) -> u8 {
        self.p
    }

    pub fn get_sp(&self) -> u8 {
        self.s
    }

    pub fn reset(&mut self) {
        self.s -= 3;
        self.p |= 4; //set IRQ disable flag
    }

    fn end_instruction(&mut self) {
        self.subcycle = 0;
        self.opcode = None;
    }

    fn cpu_sbc(&mut self, temp: u8) {
        let overflow;
        let olda = self.a;
        (self.a, overflow) = self.a.overflowing_sub(temp);
        let mut overflow2 = false;
        if (self.p & CPU_FLAG_CARRY) == 0 {
            (self.a, overflow2) = self.a.overflowing_sub(1);
        }
        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
        if self.a == 0 {
            self.p |= CPU_FLAG_ZERO;
        }
        if (self.a & 0x80) != 0 {
            self.p |= CPU_FLAG_NEGATIVE;
        }
        if !(overflow | overflow2) {
            self.p |= CPU_FLAG_CARRY;
        }
        if ((olda ^ self.a) & (self.temp ^ self.a ^ 0x80) & 0x80) != 0 {
            self.p |= CPU_FLAG_OVERFLOW;
        }
    }

    fn cpu_adc(&mut self, temp: u8) {
        let overflow;
        let olda = self.a;
        (self.a, overflow) = self.a.overflowing_add(temp);
        let mut overflow2 = false;
        if (self.p & CPU_FLAG_CARRY) != 0 {
            (self.a, overflow2) = self.a.overflowing_add(1);
        }
        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
        if self.a == 0 {
            self.p |= CPU_FLAG_ZERO;
        }
        if (self.a & 0x80) != 0 {
            self.p |= CPU_FLAG_NEGATIVE;
        }
        if overflow | overflow2 {
            self.p |= CPU_FLAG_CARRY;
        }
        if ((olda ^ self.a) & (self.temp ^ self.a) & 0x80) != 0 {
            self.p |= CPU_FLAG_OVERFLOW;
        }
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
                    self.subcycle = 0;
                    self.interrupts[1] = false;
                }
            }
        } else {
            if let None = self.opcode {
                self.opcode = Some(bus.memory_cycle_read(self.pc, [false; 3], [true; 2]));
                self.subcycle = 1;
            } else if let Some(o) = self.opcode {
                match o {
                    //and immediate
                    0x29 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.a = self.a & self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //and zero page
                    0x25 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.a = self.a & self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //and zero page x
                    0x35 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.a &= self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //and absolute
                    0x2d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.a = self.a & self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //and absolute x
                    0x3d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.a = self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //and absolute y
                    0x39 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //and indirect x
                    0x21 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //and indirect y
                    0x31 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a & bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ora or immediate
                    0x09 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.a = self.a | self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ora zero page
                    0x05 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.a |= self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ora zero page x
                    0x15 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.a |= self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ora absolute
                    0x0d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.a |= self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ora absolute x
                    0x1d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.a = self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ora absolute y
                    0x19 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ora indirect x
                    0x01 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ora indirect y
                    0x11 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a | bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //eor xor immediate
                    0x49 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.a = self.a ^ self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //eor zero page
                    0x45 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.a = self.a ^ self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //eor zero page x
                    0x55 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.a ^= self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //eor absolute
                    0x4d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.a = self.a ^ self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //eor absolute x
                    0x5d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.a = self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //eor absolute y
                    0x59 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //eor xor indirect x
                    0x41 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //eor indirect y
                    0x51 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = self.a ^ bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc, add with carry
                    0x69 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc zero page
                    0x65 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    0x75 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc absolute
                    0x6d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc absolute x
                    0x7d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_adc(self.temp);
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc absolute y
                    0x79 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_adc(self.temp);
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc adc indirect x
                    0x61 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc indirect y
                    0x71 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_adc(self.temp);
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc, subtract with carry
                    0xe9 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc zero page
                    0xe5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc zero page x
                    0xf5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute
                    0xed => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute x
                    0xfd => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_sbc(self.temp);
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute y
                    0xf9 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_sbc(self.temp);
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc indirect x
                    0xe1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc indirect y
                    0xf1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.cpu_sbc(self.temp);
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //inc increment zero page
                    0xe6 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.temp2.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //inc increment zero page x
                    0xf6 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.temp2.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //inc absolute
                    0xee => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //inc absolute x
                    0xfe => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //iny, increment y
                    0xc8 => match self.subcycle {
                        _ => {
                            self.y = self.y.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //inx, increment x
                    0xe8 => match self.subcycle {
                        _ => {
                            self.x = self.x.wrapping_add(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //dec decrement zero page
                    0xc6 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.temp2.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //dec decrement zero page x
                    0xd6 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.temp2.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //dec absolute
                    0xce => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //dec absolute x
                    0xde => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //dey, decrement y
                    0x88 => match self.subcycle {
                        _ => {
                            self.y = self.y.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //dex, decrement x
                    0xca => match self.subcycle {
                        _ => {
                            self.x = self.x.wrapping_sub(1);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //tay, transfer accumulator to y
                    0xa8 => match self.subcycle {
                        _ => {
                            self.y = self.a;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //tax, transfer accumulator to x
                    0xaa => match self.subcycle {
                        _ => {
                            self.x = self.a;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //tya, transfer y to accumulator
                    0x98 => match self.subcycle {
                        _ => {
                            self.a = self.y;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //txa, transfer x to accumulator
                    0x8a => match self.subcycle {
                        _ => {
                            self.a = self.x;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //tsx, transfer stack pointer to x
                    0xba => match self.subcycle {
                        _ => {
                            self.x = self.s;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //txs, transfer x to stack pointer
                    0x9a => match self.subcycle {
                        _ => {
                            self.s = self.x;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bit
                    0x24 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                            self.p |= self.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                            self.temp = self.a & self.temp;
                            self.p &= !CPU_FLAG_ZERO;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //bit absolute
                    0x2c => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                            self.p |= self.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                            self.temp = self.a & self.temp;
                            self.p &= !CPU_FLAG_ZERO;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //cmp, compare immediate
                    0xc9 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cmp zero page
                    0xc5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cmp zero page x
                    0xd5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cmp absolute
                    0xcd => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //cmp absolute x
                    0xdd => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                                if self.a == self.temp {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if self.a >= self.temp {
                                    self.p |= CPU_FLAG_CARRY;
                                }
                                if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //cmp absolute y
                    0xd9 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                                if self.a == self.temp {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if self.a >= self.temp {
                                    self.p |= CPU_FLAG_CARRY;
                                }
                                if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //cmp indirect x
                    0xc1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cmp indirect y
                    0xd1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                                if self.a == self.temp {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if self.a >= self.temp {
                                    self.p |= CPU_FLAG_CARRY;
                                }
                                if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }

                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.temp = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.a == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.a >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.a.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cpy, compare y immediate
                    0xc0 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.y == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.y >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.y.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cpy zero page
                    0xc4 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.y == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.y >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.y.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cpy absolute
                    0xcc => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.y == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.y >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.y.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //cpx, compare x immediate
                    0xe0 => match self.subcycle {
                        _ => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.x == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.x >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.x.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cpx zero page
                    0xe4 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.temp =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.x == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.x >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.x.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //cpx absolute
                    0xec => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if self.x == self.temp {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if self.x >= self.temp {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            if ((self.x.wrapping_sub(self.temp)) & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //jmp absolute
                    0x4c => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            let t2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                            self.pc = newpc;
                            self.end_instruction();
                        }
                    },
                    //jmp indirect
                    0x6c => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.subcycle = 3;
                        }
                        3 => {
                            let temp = self.temp;
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.tempaddr =
                                (self.temp2 as u16) << 8 | (temp.wrapping_add(1) as u16);
                            self.subcycle = 4;
                        }
                        _ => {
                            self.temp2 =
                                bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.pc = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.end_instruction();
                        }
                    },
                    //sta, store a zero page
                    0x85 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            bus.memory_cycle_write(self.temp as u16, self.a, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sta, store a zero page x
                    0x95 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            bus.memory_cycle_write(
                                self.temp.wrapping_add(self.x) as u16,
                                self.a,
                                [false; 3],
                                [true; 2],
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sta absolute
                    0x8d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            bus.memory_cycle_write(temp, self.a, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta absolute x
                    0x9d => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.subcycle = 4;
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            bus.memory_cycle_write(addr, self.a, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta absolute y
                    0x99 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.subcycle = 4;
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            bus.memory_cycle_write(addr, self.a, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta indirect x
                    0x81 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            bus.memory_cycle_write(addr, self.a, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sta indirect y
                    0x91 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.subcycle = 5;
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            bus.memory_cycle_write(addr, self.a, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldx immediate
                    0xa2 => match self.subcycle {
                        _ => {
                            self.x = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldx zero page
                    0xa6 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            self.x = bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldx zero page y
                    0xb6 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.y);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.x = bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldx absolute
                    0xae => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.x = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ldx absolute y
                    0xbe => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                                self.x =
                                    bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.x == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.x & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.x = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.x == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.x & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sty store y zero page
                    0x84 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        _ => {
                            bus.memory_cycle_write(self.temp as u16, self.y, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sty zero page x
                    0x94 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            bus.memory_cycle_write(
                                self.temp.wrapping_add(self.x) as u16,
                                self.y,
                                [false; 3],
                                [true; 2],
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sty absolute
                    0x8c => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            bus.memory_cycle_write(temp, self.y, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ldy load y immediate
                    0xa0 => match self.subcycle {
                        _ => {
                            self.y = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldy zero page
                    0xa4 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.y = bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldy zero page x
                    0xb4 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.y = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldy absolute
                    0xac => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.y = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ldy absolute x
                    0xbc => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let addr =
                                (self.temp2 as u16) << 8 | (self.temp.wrapping_add(self.x) as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                self.y = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.y == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.y & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.y = bus.memory_cycle_read(
                                addr.wrapping_add(self.x as u16),
                                [false; 3],
                                [true; 2],
                            );
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.y == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.y & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lda immediate
                    0xa9 => match self.subcycle {
                        _ => {
                            self.a = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lda zero page
                    0xa5 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.a = bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lda zero page x
                    0xb5 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.a = bus.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lda absolute
                    0xad => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.a = bus.memory_cycle_read(temp, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lda indirect x
                    0xa1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lda absolute x
                    0xbd => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lda absolute y
                    0xb9 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            } else {
                                self.subcycle = 4;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lda indirect y
                    0xb1 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = bus.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                                self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                                if self.a == 0 {
                                    self.p |= CPU_FLAG_ZERO;
                                }
                                if (self.a & 0x80) != 0 {
                                    self.p |= CPU_FLAG_NEGATIVE;
                                }
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            } else {
                                self.subcycle = 5;
                            }
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.a = bus.memory_cycle_read(addr, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }

                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //stx zero page
                    0x86 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            bus.memory_cycle_write(self.temp as u16, self.x, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //stx zero page y
                    0x96 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.y);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            bus.memory_cycle_write(self.temp as u16, self.x, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //stx, store x absolute
                    0x8e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            bus.memory_cycle_write(temp, self.x, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lsr logical shift right, accumulator
                    0x4a => match self.subcycle {
                        _ => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.a = self.a >> 1;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //lsr zero page
                    0x46 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 >> 1;
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lsr zero page x
                    0x56 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 >> 1;
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lsr absolute
                    0x4e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //lsr absolute x
                    0x5e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //asl, arithmetic shift left accumulator
                    0x0a => match self.subcycle {
                        _ => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.a = self.a << 1;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //asl zero page
                    0x06 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 << 1;
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //asl zero page x
                    0x16 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 << 1;
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //asl absolute
                    0x0e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //asl absolute x
                    0x1e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ror rotate right accumulator
                    0x6a => match self.subcycle {
                        _ => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.a = self.a >> 1;
                            if old_carry {
                                self.a |= 0x80;
                            }
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //ror zero page
                    0x66 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 >> 1;
                            if old_carry {
                                self.temp2 |= 0x80;
                            }
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ror zero page x
                    0x76 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 >> 1;
                            if old_carry {
                                self.temp2 |= 0x80;
                            }
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ror absolute
                    0x6e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ror absolute x
                    0x7e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //rol accumulator
                    0x2a => match self.subcycle {
                        _ => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.a = self.a << 1;
                            if old_carry {
                                self.a |= 0x1;
                            }
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //rol zero page
                    0x26 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 << 1;
                            if old_carry {
                                self.temp2 |= 1;
                            }
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rol zero page x
                    0x36 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                bus.memory_cycle_read(self.temp as u16, [false; 3], [true; 2]);
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp2 = self.temp2 << 1;
                            if old_carry {
                                self.temp2 |= 1;
                            }
                            if self.temp2 == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp2 & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 3;
                        }
                        3 => {
                            bus.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rol absolute
                    0x2e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 1;
                            }
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //rol absolute x
                    0x3e => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = bus.memory_cycle_read(self.tempaddr, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 1;
                            }
                            if self.temp == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            bus.memory_cycle_write(self.tempaddr, self.temp, [false; 3], [true; 2]);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //rti, return from interrupt
                    0x40 => match self.subcycle {
                        1 => {
                            self.s = self.s.wrapping_add(1);
                            self.p =
                                bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.p = self.p & !CPU_FLAG_B1;
                            self.p |= CPU_FLAG_B2;
                            self.subcycle = 2;
                        }
                        2 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp =
                                bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp2 =
                                bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.pc = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.subcycle = 5;
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //jsr absolute
                    0x20 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            let pc = (self.pc + 2).to_le_bytes();
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
                            let pc = (self.pc + 2).to_le_bytes();
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
                            self.end_instruction();
                        }
                    },
                    //nop
                    0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xea | 0xfa => match self.subcycle {
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //extra nop
                    0x04 | 0x44 | 0x64 => match self.subcycle {
                        1 => {
                            bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //extra nop
                    0x0c => match self.subcycle {
                        1 => {
                            bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //extra nop
                    0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 => match self.subcycle {
                        1 => {
                            bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
                    //extra nop
                    0x1c | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16)<<8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if overflow {
                                self.subcycle = 4;
                            }
                            else {
                                self.pc = self.pc.wrapping_add(3);
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //extra nop
                    0x80 => match self.subcycle {
                        _ => {
                            bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    }
                    //clv, clear overflow flag
                    0xb8 => match self.subcycle {
                        _ => {
                            self.p &= !CPU_FLAG_OVERFLOW;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //sec set carry flag
                    0x38 => match self.subcycle {
                        _ => {
                            self.p |= CPU_FLAG_CARRY;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //sei set interrupt disable flag
                    0x78 => match self.subcycle {
                        _ => {
                            self.p |= CPU_FLAG_INT_DISABLE;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //sed set decimal flag
                    0xf8 => match self.subcycle {
                        _ => {
                            self.p |= CPU_FLAG_DECIMAL;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //cld, clear decimal flag
                    0xd8 => match self.subcycle {
                        _ => {
                            self.p &= !CPU_FLAG_DECIMAL;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //clc clear carry flag
                    0x18 => match self.subcycle {
                        _ => {
                            self.p &= !CPU_FLAG_CARRY;
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //beq, branch if equal (zero flag set)
                    0xf0 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_ZERO) != 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bne, branch if not equal (zero flag not set)
                    0xd0 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.pc = self.pc.wrapping_add(2);
                            if (self.p & CPU_FLAG_ZERO) == 0 {
                                self.tempaddr = self.pc;
                                self.subcycle = 2;
                            } else {
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.tempaddr + 2, [false; 3], [true; 2]);
                            self.tempaddr = self.pc.wrapping_add(self.temp as u16);
                            let pc = self.tempaddr.to_le_bytes();
                            let pc2 = self.pc.to_le_bytes();
                            self.pc = self.tempaddr;
                            if pc[1] != pc2[1] {
                                self.pc = self.pc.wrapping_sub(256);
                                self.end_instruction();
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bvs, branch if overflow set
                    0x70 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_OVERFLOW) != 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bvc branch if overflow clear
                    0x50 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_OVERFLOW) == 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bpl, branch if negative clear
                    0x10 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_NEGATIVE) == 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bmi branch if negative flag set
                    0x30 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_NEGATIVE) != 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bcs, branch if carry set
                    0xb0 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_CARRY) != 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //bcc branch if carry flag clear
                    0x90 => match self.subcycle {
                        1 => {
                            self.temp = bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            if (self.p & CPU_FLAG_CARRY) == 0 {
                                self.pc = self.pc.wrapping_add(2);
                                let mut pc = self.pc.to_le_bytes();
                                pc[0] = pc[0].wrapping_add(self.temp);
                                self.pc = u16::from_le_bytes(pc);
                                self.subcycle = 2;
                            } else {
                                self.pc = self.pc.wrapping_add(2);
                                self.end_instruction();
                            }
                        }
                        2 => {
                            bus.memory_cycle_read(self.pc + 2, [false; 3], [true; 2]);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //pha push accumulator
                    0x48 => match self.subcycle {
                        1 => {
                            bus.memory_cycle_write(
                                0x100 + self.s as u16,
                                self.a,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 2;
                        }
                        _ => {
                            self.s = self.s.wrapping_sub(1);
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //php push processor status
                    0x08 => match self.subcycle {
                        1 => {
                            bus.memory_cycle_write(
                                0x100 + self.s as u16,
                                self.p | CPU_FLAG_B1,
                                [false; 3],
                                [true; 2],
                            );
                            self.subcycle = 2;
                        }
                        _ => {
                            self.s = self.s.wrapping_sub(1);
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //plp, pull processor status
                    0x28 => match self.subcycle {
                        1 => {
                            self.s = self.s.wrapping_add(1);
                            self.p =
                                bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.p = self.p & !CPU_FLAG_B1;
                            self.p |= CPU_FLAG_B2;
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //pla, pull accumulator
                    0x68 => match self.subcycle {
                        1 => {
                            self.s = self.s.wrapping_add(1);
                            self.a =
                                bus.memory_cycle_read(0x100 + self.s as u16, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //rts, return from subroutine
                    0x60 => match self.subcycle {
                        1 => {
                            bus.memory_cycle_read(self.pc + 1, [false; 3], [true; 2]);
                            self.subcycle = 2;
                        }
                        2 => {
                            bus.memory_cycle_read(self.s as u16 + 0x100, [false; 3], [true; 2]);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp =
                                bus.memory_cycle_read(self.s as u16 + 0x100, [false; 3], [true; 2]);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.pc = self.temp as u16;
                            self.s = self.s.wrapping_add(1);
                            self.pc |= (bus.memory_cycle_read(
                                self.s as u16 + 0x100,
                                [false; 3],
                                [true; 2],
                            ) as u16)
                                << 8;
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
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
