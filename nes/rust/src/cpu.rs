use crate::ppu::NesPpu;

pub struct NesCpuPeripherals {
    ppu: NesPpu,
}

impl NesCpuPeripherals {
    pub fn new(ppu: NesPpu) -> Self {
        Self { ppu: ppu }
    }

    pub fn ppu_cycle(&mut self, bus: &mut dyn NesMemoryBus) {
        self.ppu.cycle(bus);
    }

    pub fn ppu_read(&mut self, addr: u16) -> Option<u8> {
        self.ppu.read(addr)
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        self.ppu.write(addr, data);
    }

    pub fn ppu_frame_end(&mut self) -> bool {
        self.ppu.get_frame_end()
    }

    pub fn ppu_get_frame(&mut self) -> &[u8] {
        self.ppu.get_frame()
    }

    pub fn ppu_frame_number(&self) -> u64 {
        self.ppu.frame_number()
    }
}

pub trait NesMemoryBus {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        out: [bool; 3],
        controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) -> u8;
    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        out: [bool; 3],
        controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    );
    fn ppu_cycle_1(&mut self, addr: u16);
    fn ppu_cycle_2_write(&mut self, data: u8);
    fn ppu_cycle_2_read(&mut self) -> u8;
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
    #[cfg(debug_assertions)]
    breakpoints: [Option<u16>; 10],
    #[cfg(debug_assertions)]
    old_pc: [u16; 2],
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
            #[cfg(debug_assertions)]
            breakpoints: [None; 10],
            #[cfg(debug_assertions)]
            old_pc: [0; 2],
        }
    }

    #[cfg(test)]
    pub fn instruction_start(&self) -> bool {
        self.subcycle == 0
    }

    #[cfg(debug_assertions)]
    pub fn breakpoint_option(&self) -> bool {
        self.subcycle == 1
    }

    #[cfg(any(test, debug_assertions))]
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    #[cfg(test)]
    pub fn get_a(&self) -> u8 {
        self.a
    }

    #[cfg(test)]
    pub fn get_x(&self) -> u8 {
        self.x
    }

    #[cfg(test)]
    pub fn get_y(&self) -> u8 {
        self.y
    }

    #[cfg(test)]
    pub fn get_p(&self) -> u8 {
        self.p
    }

    #[cfg(test)]
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

    fn calc_out(&mut self, _addr: u16) -> [bool; 3] {
        [false; 3]
    }

    fn calc_oe(&mut self, addr: u16) -> [bool; 2] {
        [addr == 0x4016, addr == 0x4017]
    }

    fn memory_cycle_read(
        &mut self,
        addr: u16,
        bus: &mut dyn NesMemoryBus,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) -> u8 {
        bus.memory_cycle_read(
            addr,
            self.calc_out(addr),
            self.calc_oe(addr),
            cpu_peripherals,
        )
    }

    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        bus: &mut dyn NesMemoryBus,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) {
        bus.memory_cycle_write(
            addr,
            data,
            self.calc_out(addr),
            self.calc_oe(addr),
            cpu_peripherals,
        );
    }

    #[cfg(debug_assertions)]
    fn check_breakpoints(&mut self) {
        self.old_pc[1] = self.old_pc[0];
        self.old_pc[0] = self.pc;
        for b in self.breakpoints {
            if let Some(br) = b {
                if self.pc == br || self.pc == 0xffff {
                    self.subcycle = 1;
                }
            }
        }
        if self.pc == 58552 {
            self.pc = self.pc;
        }
    }

    #[cfg(debug_assertions)]
    pub fn disassemble(&self) -> Option<String> {
        if let Some(o) = self.opcode {
            match o {
                0x01 | 0x05 | 0x09 | 0x0d | 0x11 | 0x15 | 0x19 | 0x1d => Some("ORA".to_string()),
                0x21 | 0x25 | 0x29 | 0x2d | 0x31 | 0x35 | 0x39 | 0x3d => Some("AND".to_string()),
                0x41 | 0x45 | 0x49 | 0x4d | 0x51 | 0x55 | 0x59 | 0x5d => Some("EOR".to_string()),
                0x61 | 0x65 | 0x69 | 0x6d | 0x71 | 0x75 | 0x79 | 0x7d => Some("ADC".to_string()),
                0x81 | 0x85 | 0x89 | 0x8d | 0x91 | 0x95 | 0x99 | 0x9d => Some("STA".to_string()),
                0xa1 | 0xa5 | 0xa9 | 0xad | 0xb1 | 0xb5 | 0xb9 | 0xbd => Some("LDA".to_string()),
                0xc1 | 0xc5 | 0xc9 | 0xcd | 0xd1 | 0xd5 | 0xd9 | 0xdd => Some("CMP".to_string()),
                0xe1 | 0xe5 | 0xe9 | 0xed | 0xf1 | 0xf5 | 0xf9 | 0xfd => Some("SBC".to_string()),
                0xa0 | 0xa4 | 0xac | 0xb4 | 0xbc => Some("LDY".to_string()),
                0xa2 | 0xa6 | 0xae | 0xb6 | 0xbe => Some("LDX".to_string()),
                0x06 | 0x0a | 0x0e | 0x16 | 0x1e => Some("ASL".to_string()),
                0x26 | 0x2a | 0x2e | 0x36 | 0x3e => Some("ROL".to_string()),
                0x46 | 0x4a | 0x4e | 0x56 | 0x5e => Some("LSR".to_string()),
                0x66 | 0x6a | 0x6e | 0x76 | 0x7e => Some("ROR".to_string()),
                0xc6 | 0xce | 0xd6 | 0xde => Some("DEC".to_string()),
                0xe6 | 0xee | 0xf6 | 0xfe => Some("INC".to_string()),
                0x86 | 0x8e | 0x96 => Some("STX".to_string()),
                0x84 | 0x8c | 0x94 => Some("STY".to_string()),
                0xc0 | 0xc4 | 0xcc => Some("CPY".to_string()),
                0xe0 | 0xe4 | 0xec => Some("CPX".to_string()),
                0x4c | 0x6c => Some("JMP".to_string()),
                0x24 | 0x2c => Some("BIT".to_string()),
                0x00 => Some("BRK".to_string()),
                0x20 => Some("JSR".to_string()),
                0x40 => Some("RTI".to_string()),
                0x60 => Some("RTS".to_string()),
                0x08 => Some("PHP".to_string()),
                0x28 => Some("PLP".to_string()),
                0x48 => Some("PHA".to_string()),
                0x68 => Some("PLA".to_string()),
                0x88 => Some("DEY".to_string()),
                0xa8 => Some("TAY".to_string()),
                0xc8 => Some("INY".to_string()),
                0xe8 => Some("INX".to_string()),
                0x8a => Some("TXA".to_string()),
                0xaa => Some("TAX".to_string()),
                0xca => Some("DEX".to_string()),
                0xea => Some("NOP".to_string()),
                0x10 => Some("BPL".to_string()),
                0x30 => Some("BMI".to_string()),
                0x50 => Some("BVC".to_string()),
                0x70 => Some("BVS".to_string()),
                0x90 => Some("BCC".to_string()),
                0xb0 => Some("BCS".to_string()),
                0xd0 => Some("BNE".to_string()),
                0xf0 => Some("BEQ".to_string()),
                0x18 => Some("CLC".to_string()),
                0x38 => Some("SEC".to_string()),
                0x58 => Some("CLI".to_string()),
                0x78 => Some("SEI".to_string()),
                0x98 => Some("TYA".to_string()),
                0xb8 => Some("CLV".to_string()),
                0xd8 => Some("CLD".to_string()),
                0xf8 => Some("SED".to_string()),
                _ => Some(format!("Invalid {:x}", o)),
            }
        } else {
            None
        }
    }

    pub fn cycle(&mut self, bus: &mut dyn NesMemoryBus, cpu_peripherals: &mut NesCpuPeripherals) {
        if self.interrupts[1] {
            match self.subcycle {
                0 => {
                    self.memory_cycle_read(self.pc, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                1 => {
                    self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                2 => {
                    self.memory_cycle_read(self.s as u16 + 0x100, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                3 => {
                    self.memory_cycle_read(self.s as u16 + 0xff, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                4 => {
                    self.memory_cycle_read(self.s as u16 + 0xfe, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                5 => {
                    let pcl = self.memory_cycle_read(0xfffc, bus, cpu_peripherals);
                    let mut pc = self.pc.to_le_bytes();
                    pc[0] = pcl;
                    self.pc = u16::from_le_bytes(pc);
                    self.subcycle += 1;
                }
                _ => {
                    let pch = self.memory_cycle_read(0xfffd, bus, cpu_peripherals);
                    let mut pc = self.pc.to_le_bytes();
                    pc[1] = pch;
                    self.pc = u16::from_le_bytes(pc);
                    self.subcycle = 0;
                    self.interrupts[1] = false;
                }
            }
        } else {
            if let None = self.opcode {
                self.opcode = Some(self.memory_cycle_read(self.pc, bus, cpu_peripherals));
                #[cfg(debug_assertions)]
                self.check_breakpoints();
                self.subcycle = 1;
            } else if let Some(o) = self.opcode {
                match o {
                    //brk instruction
                    0 => match self.subcycle {
                        1 => {
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            let mut pc = self.pc.to_le_bytes();
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[1],
                                bus,
                                cpu_peripherals,
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut pc = self.pc.to_le_bytes();
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[0],
                                bus,
                                cpu_peripherals,
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p |= CPU_FLAG_B1;
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                self.p,
                                bus,
                                cpu_peripherals,
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.temp = self.memory_cycle_read(0xfffe, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        _ => {
                            self.temp2 = self.memory_cycle_read(0xffff, bus, cpu_peripherals);
                            let addr: u16 = (self.temp as u16) | (self.temp2 as u16) << 8;
                            self.pc = addr;
                            self.end_instruction();
                        }
                    },
                    //and immediate
                    0x29 => match self.subcycle {
                        _ => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a & self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a | self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a =
                                    self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a =
                                    self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.a ^ self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc zero page
                    0x65 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    0x75 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //adc absolute
                    0x6d => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc absolute x
                    0x7d => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc absolute y
                    0x79 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //adc adc indirect x
                    0x61 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc, subtract with carry
                    0xe9 | 0xeb => match self.subcycle {
                        _ => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc zero page
                    0xe5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc zero page x
                    0xf5 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute
                    0xed => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute x
                    0xfd => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc absolute y
                    0xf9 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);

                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sbc indirect x
                    0xe1 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //inc increment zero page
                    0xe6 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            let t2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                            self.pc = newpc;
                            self.end_instruction();
                        }
                    },
                    //jmp indirect
                    0x6c => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.subcycle = 3;
                        }
                        3 => {
                            let temp = self.temp;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.tempaddr =
                                (self.temp2 as u16) << 8 | (temp.wrapping_add(1) as u16);
                            self.subcycle = 4;
                        }
                        _ => {
                            self.temp2 =
                                self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.pc = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.end_instruction();
                        }
                    },
                    //sta, store a zero page
                    0x85 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.memory_cycle_write(self.temp as u16, self.a, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sta, store a zero page x
                    0x95 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.memory_cycle_write(
                                self.temp.wrapping_add(self.x) as u16,
                                self.a,
                                bus,
                                cpu_peripherals,
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sta absolute
                    0x8d => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.memory_cycle_write(temp, self.a, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta absolute x
                    0x9d => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.subcycle = 4;
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.x as u16);
                            self.memory_cycle_write(addr, self.a, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta absolute y
                    0x99 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.subcycle = 4;
                        }
                        _ => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.memory_cycle_write(addr, self.a, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sta indirect x
                    0x81 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.memory_cycle_write(addr, self.a, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            addr = addr.wrapping_add(self.y as u16);
                            self.memory_cycle_write(addr, self.a, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //ldx immediate
                    0xa2 => match self.subcycle {
                        _ => {
                            self.x = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.x = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.y);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.x = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.x = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                                self.x =
                                    self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.x = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        _ => {
                            self.memory_cycle_write(self.temp as u16, self.y, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sty zero page x
                    0x94 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.memory_cycle_write(
                                self.temp.wrapping_add(self.x) as u16,
                                self.y,
                                bus,
                                cpu_peripherals,
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sty absolute
                    0x8c => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.memory_cycle_write(temp, self.y, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //ldy load y immediate
                    0xa0 => match self.subcycle {
                        _ => {
                            self.y = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.y = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.y = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.y = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let addr =
                                (self.temp2 as u16) << 8 | (self.temp.wrapping_add(self.x) as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                self.y = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.y = self.memory_cycle_read(
                                addr.wrapping_add(self.x as u16),
                                bus,
                                cpu_peripherals,
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
                            self.a = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.a = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.a = self.memory_cycle_read(
                                self.temp.wrapping_add(self.x) as u16,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.a = self.memory_cycle_read(temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if !overflow {
                                addr = addr.wrapping_add(self.x as u16);
                                self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.memory_cycle_write(self.temp as u16, self.x, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //stx zero page y
                    0x96 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.y);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.memory_cycle_write(self.temp as u16, self.x, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //stx, store x absolute
                    0x8e => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.memory_cycle_write(temp, self.x, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.x);
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                                self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                            self.p = self.p & !CPU_FLAG_B1;
                            self.p |= CPU_FLAG_B2;
                            self.subcycle = 2;
                        }
                        2 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp =
                                self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp2 =
                                self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let pc = (self.pc + 2).to_le_bytes();
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[1],
                                bus,
                                cpu_peripherals,
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 4;
                        }
                        4 => {
                            let pc = (self.pc + 2).to_le_bytes();
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                pc[0],
                                bus,
                                cpu_peripherals,
                            );
                            self.s = self.s.wrapping_sub(1);
                            self.subcycle = 5;
                        }
                        _ => {
                            let t2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            let (_val, overflow) = self.temp.overflowing_add(self.x);
                            if overflow {
                                self.subcycle = 4;
                            } else {
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
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            self.subcycle = 0;
                            self.opcode = None;
                        }
                    },
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
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bne, branch if not equal (zero flag not set)
                    0xd0 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.pc = self.pc.wrapping_add(2);
                            if (self.p & CPU_FLAG_ZERO) == 0 {
                                self.tempaddr = self.pc;
                                self.subcycle = 2;
                            } else {
                                self.end_instruction();
                            }
                        }
                        2 => {
                            self.memory_cycle_read(self.tempaddr + 2, bus, cpu_peripherals);
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
                            self.end_instruction();
                        }
                    },
                    //bvs, branch if overflow set
                    0x70 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bvc branch if overflow clear
                    0x50 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bpl, branch if negative clear
                    0x10 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bmi branch if negative flag set
                    0x30 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bcs, branch if carry set
                    0xb0 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //bcc branch if carry flag clear
                    0x90 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            let pc = self.pc.to_le_bytes();
                            if pc[0] < self.temp {
                                self.pc = self.pc.wrapping_add(256);
                                self.subcycle = 3;
                            } else {
                                self.end_instruction();
                            }
                        }
                        _ => {
                            self.end_instruction();
                        }
                    },
                    //pha push accumulator
                    0x48 => match self.subcycle {
                        1 => {
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                self.a,
                                bus,
                                cpu_peripherals,
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
                            self.memory_cycle_write(
                                0x100 + self.s as u16,
                                self.p | CPU_FLAG_B1,
                                bus,
                                cpu_peripherals,
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
                                self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
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
                                self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
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
                            self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.memory_cycle_read(self.s as u16 + 0x100, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.s = self.s.wrapping_add(1);
                            self.temp =
                                self.memory_cycle_read(self.s as u16 + 0x100, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.pc = self.temp as u16;
                            self.s = self.s.wrapping_add(1);
                            self.pc |= (self.memory_cycle_read(
                                self.s as u16 + 0x100,
                                bus,
                                cpu_peripherals,
                            ) as u16)
                                << 8;
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(1);
                            self.end_instruction();
                        }
                    },
                    //lax (indirect x)?, undocumented
                    0xa3 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.tempaddr = self.temp.wrapping_add(self.x) as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.tempaddr.wrapping_add(1),
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.a = self.temp;
                            self.x = self.temp;
                            self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //lax zero page?, undocumented
                    0xa7 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.a = self.temp;
                            self.x = self.temp;
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
                    //lax absolute, undocumented
                    0xaf => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            let addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.a = self.temp;
                            self.x = self.temp;
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
                    //lax indirect y, undocumented
                    0xb3 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            let mut addr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            let (_val, overflow) = self.temp2.overflowing_add(self.y);
                            if !overflow {
                                addr = addr.wrapping_add(self.y as u16);
                                self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
                                self.x = self.a;
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
                            self.a = self.memory_cycle_read(addr, bus, cpu_peripherals);
                            self.x = self.a;
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
                    //lax zero page y, undocumented
                    0xb7 => match self.subcycle {
                        1 => {
                            self.subcycle = 2;
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_add(self.y);
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.x = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.a = self.x;
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
                    //lax absolute y, undocumented
                    0xbf => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                            let (_val, overflow) = self.temp.overflowing_add(self.y);
                            if !overflow {
                                self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                                self.x =
                                    self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                                self.a = self.x;
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
                            self.x = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.a = self.x;
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
                    //sax indirect x
                    0x83 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.tempaddr = self.temp.wrapping_add(self.x) as u16;
                            self.temp2 =
                                self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = self.temp.wrapping_add(self.x).wrapping_add(1) as u16;
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.x & self.a;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sax zero page
                    0x87 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        _ => {
                            self.temp2 = self.a & self.x;
                            self.memory_cycle_write(
                                self.temp as u16,
                                self.temp2,
                                bus,
                                cpu_peripherals,
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sax absolute
                    0x8f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        _ => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp2 = self.a & self.x;
                            self.memory_cycle_write(
                                self.tempaddr,
                                self.temp2,
                                bus,
                                cpu_peripherals,
                            );
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sax absolute y
                    0x97 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.subcycle = 3;
                        }
                        _ => {
                            self.tempaddr = self.temp.wrapping_add(self.y) as u16;
                            self.temp2 = self.a & self.x;
                            self.memory_cycle_write(
                                self.tempaddr,
                                self.temp2,
                                bus,
                                cpu_peripherals,
                            );
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //dcp, undocumented, decrement and compare indirect x
                    0xc3 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.temp = self.temp.wrapping_sub(1);
                            self.subcycle = 6;
                        }
                        6 => {
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 7;
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
                    //dcp zero page, undocumented
                    0xc7 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
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
                    //dcp absolute, undocumented
                    0xcf => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //dcp indirect y
                    0xd3 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                    //dcp zero page x, undocumented
                    0xd7 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.temp2 = self.temp2.wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
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
                    //dcp absolute y, undocumented
                    0xdb => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
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
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //dcp absolute x, undocumented
                    0xdf => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_sub(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
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
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //isb indirect x, increment memory, sub memory from accumulator, undocumented
                    0xe3 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        6 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 7;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //isb zero page, undocumented
                    0xe7 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //isb absolute, undocumented
                    0xef => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //isb indirect y, undocumented
                    0xf3 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //isb zero page x, undocumented
                    0xf7 => match self.subcycle {
                        1 => {
                            self.temp2 = self
                                .memory_cycle_read(self.pc + 1, bus, cpu_peripherals)
                                .wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //isb absolute y, undocumented
                    0xfb => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //isb absolute x, undocumented
                    0xff => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.temp.wrapping_add(1);
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.cpu_sbc(self.temp);
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //slo shift left, then or with accumulator, undocumented
                    //indirect x
                    0x03 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        6 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 7;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //slo zero page, undocumented
                    0x07 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //slo absolute, undocumented
                    0x0f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //slo indirect y, undocumented
                    0x13 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a | self.temp;
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
                    //slo zero page x, undocumented
                    0x17 => match self.subcycle {
                        1 => {
                            self.temp2 = self
                                .memory_cycle_read(self.pc + 1, bus, cpu_peripherals)
                                .wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.subcycle = 5;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //slo absolute y, undocumented
                    0x1b => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //slo absolute x, undocumented
                    0x1f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        _ => {
                            self.a = self.a | self.temp;
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
                    //rla, rotate left, then and with accumulator, undocumented
                    //indirect x
                    0x23 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        6 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 7;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rla zero page, undocumented
                    0x27 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rla absolute, undocumented
                    0x2f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //rla indirect y
                    0x33 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a & self.temp;
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
                    //rla zero page x, undocumented
                    0x37 => match self.subcycle {
                        1 => {
                            self.temp2 = self
                                .memory_cycle_read(self.pc + 1, bus, cpu_peripherals)
                                .wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x80) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp << 1;
                            if old_carry {
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //rla absolute y, undocumented
                    0x3b => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //rla absolute x, undocumented
                    0x3f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                                self.temp |= 0x1;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a & self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //sre, shift right, then xor with accumulator, undocumented
                    //indirect x
                    0x43 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        6 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 7;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sre zero page, undocumented
                    0x47 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 7;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //sre absolute, undocumented
                    0x4f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //sre indirect y, undocumented
                    0x53 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a ^ self.temp;
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
                    //sre zero page x, undocumented
                    0x57 => match self.subcycle {
                        1 => {
                            self.temp2 = self
                                .memory_cycle_read(self.pc + 1, bus, cpu_peripherals)
                                .wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //sre absolute y, undocumented
                    0x5b => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //sre absolute x, undocumented
                    0x5f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.a = self.a ^ self.temp;
                            if self.a == 0 {
                                self.p |= CPU_FLAG_ZERO;
                            }
                            if (self.a & 0x80) != 0 {
                                self.p |= CPU_FLAG_NEGATIVE;
                            }
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
                    //rra, rotate right, then and with accumulator, undocumented
                    //indirect x
                    0x63 => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp = self.temp.wrapping_add(self.x);
                            self.temp =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 =
                                self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            self.temp = self.memory_cycle_read(
                                self.temp.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 5;
                        }
                        5 => {
                            self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 6;
                        }
                        6 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.subcycle = 7;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rra zero page, undocumented
                    0x67 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.cpu_adc(self.temp);
                            self.subcycle = 4;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rra absolute, undocumented
                    0x6f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.subcycle = 5;
                        }
                        _ => {
                            self.pc = self.pc.wrapping_add(3);
                            self.end_instruction();
                        }
                    },
                    //rra indirect y
                    0x73 => match self.subcycle {
                        1 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.temp2 = self.memory_cycle_read(
                                self.temp2.wrapping_add(1) as u16,
                                bus,
                                cpu_peripherals,
                            );
                            self.subcycle = 4;
                        }
                        4 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 5;
                        }
                        5 => {
                            self.subcycle = 6;
                        }
                        6 => {
                            self.subcycle = 7;
                        }
                        _ => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    },
                    //rra zero page x, undocumented
                    0x77 => match self.subcycle {
                        1 => {
                            self.temp2 = self
                                .memory_cycle_read(self.pc + 1, bus, cpu_peripherals)
                                .wrapping_add(self.x);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp =
                                self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(
                                self.temp2 as u16,
                                self.temp,
                                bus,
                                cpu_peripherals,
                            );
                            self.cpu_adc(self.temp);
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
                    //rra absolute y, undocumented
                    0x7b => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
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
                    //rra absolute x, undocumented
                    0x7f => match self.subcycle {
                        1 => {
                            self.temp = self.memory_cycle_read(self.pc + 1, bus, cpu_peripherals);
                            self.subcycle = 2;
                        }
                        2 => {
                            self.temp2 = self.memory_cycle_read(self.pc + 2, bus, cpu_peripherals);
                            self.subcycle = 3;
                        }
                        3 => {
                            self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                            self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                            self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                            self.subcycle = 4;
                        }
                        4 => {
                            let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                            self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if (self.temp & 0x1) != 0 {
                                self.p |= CPU_FLAG_CARRY;
                            }
                            self.temp = self.temp >> 1;
                            if old_carry {
                                self.temp |= 0x80;
                            }
                            self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                            self.cpu_adc(self.temp);
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
                    _ => {
                        println!("{}", format!("CPU OPCODE {:x} not implemented", o));
                        unimplemented!();
                    }
                }
            }
        }
    }
}
