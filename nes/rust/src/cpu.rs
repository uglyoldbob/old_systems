//! This module is responsible for emulating the cpu of the nes.

use crate::apu::NesApu;
use crate::motherboard::NesMotherboard;
use crate::ppu::NesPpu;

/// The peripherals for the cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCpuPeripherals {
    /// The ppu for the nes system
    pub ppu: NesPpu,
    /// The apu for the nes system
    pub apu: NesApu,
}

impl NesCpuPeripherals {
    /// Create a new sett of cpu peripherals
    pub fn new(ppu: NesPpu, apu: NesApu) -> Self {
        Self { ppu, apu }
    }

    /// Run a ppu address cycle
    pub fn ppu_cycle(&mut self, bus: &mut NesMotherboard) {
        self.ppu.cycle(bus);
    }

    /// A ppu dump cycle, no side effects
    pub fn ppu_dump(&self, addr: u16) -> Option<u8> {
        self.ppu.dump(addr)
    }

    /// Run a ppu read cycle
    pub fn ppu_read(&mut self, addr: u16, palette: &[u8; 32]) -> Option<u8> {
        self.ppu.read(addr, palette)
    }

    /// Run a ppu write cycle
    pub fn ppu_write(&mut self, addr: u16, data: u8, palette: &mut [u8; 32]) {
        self.ppu.write(addr, data, palette);
    }

    /// Returns true when the frame has ended. USed for synchronizing the emulator to the appropriate frame rate
    pub fn ppu_frame_end(&mut self) -> bool {
        self.ppu.get_frame_end()
    }

    /// Returns a reference to the frame data for the ppu
    pub fn ppu_get_frame(&mut self) -> &[u8; 256 * 240 * 3] {
        self.ppu.get_frame()
    }

    /// Used for automated testing, to determine how many frames have passed.
    #[cfg(any(test, feature = "debugger"))]
    pub fn ppu_frame_number(&self) -> u64 {
        self.ppu.frame_number()
    }

    /// Returns the ppu irq line
    pub fn ppu_irq(&self) -> bool {
        self.ppu.irq()
    }

    /// Reset the ppu
    pub fn ppu_reset(&mut self) {
        self.ppu.reset();
    }
}

#[cfg(feature = "debugger")]
#[derive(serde::Serialize, serde::Deserialize)]
/// Stores the state of the cpu at the debugger point.
/// Single byte instructions make debugging without this weird, because the instruction has already taken effect
/// by the time the debugger is presenting the information
pub struct NesCpuDebuggerPoint {
    /// The a register
    pub a: u8,
    /// The x register
    pub x: u8,
    /// The y register
    pub y: u8,
    /// The stack register
    pub s: u8,
    /// The flags register
    pub p: u8,
    /// The program counter
    pub pc: u16,
    /// The string that corresponds to the disassembly for the most recently fetched instruction
    pub disassembly: String,
}

/// A struct for implementing the nes cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NesCpu {
    /// The a register
    a: u8,
    /// The x register
    x: u8,
    /// The y register
    y: u8,
    /// The stack register
    s: u8,
    /// The flags register
    p: u8,
    /// The program counter
    pc: u16,
    /// The portion of an instruction currently being executed
    subcycle: u8,
    /// Indicates that the reset routine of the cpu should execute
    reset: bool,
    /// The current opcode being executed
    opcode: Option<u8>,
    /// A temporary variable used inn proccessing instructions
    temp: u8,
    /// A temporary variable used inn proccessing instructions
    temp2: u8,
    /// A temporary address used in processing instructions
    tempaddr: u16,
    /// A list of breakpoints for the cpu
    #[cfg(feature = "debugger")]
    pub breakpoints: Vec<u16>,
    /// True when the last byte of an instruction has been fetched
    #[cfg(feature = "debugger")]
    done_fetching: bool,
    /// The debugger information
    #[cfg(feature = "debugger")]
    pub debugger: NesCpuDebuggerPoint,
    /// The status of nmi_detection from last cpu cycle
    prev_nmi: bool,
    /// True when an nmi has been detected
    nmi_detected: bool,
    /// Shift register for the interrupt detection routine
    interrupt_shift: [(bool, bool); 2],
    /// Indicates the type of interrupt, true for nmi, false for irq
    interrupt_type: bool,
    /// Indicates that the cpu is currently interrupting with an interrupt
    interrupting: bool,
    /// The address to use for oam dam
    oamdma: Option<u8>,
    /// The dma counter for oam dma
    dma_counter: u16,
    /// The three outputs used for controller driving
    outs: [bool; 3],
    /// The address for dmc dma
    dmc_dma: Option<u16>,
    /// Counter for doing dmc dma operations, since it takes more than one cycle
    dmc_dma_counter: u8,
}

/// The carry flag for the cpu flags register
const CPU_FLAG_CARRY: u8 = 1;
/// The zero flag for the cpu flags register
const CPU_FLAG_ZERO: u8 = 2;
/// The interrupt disable flag for the cpu flags register
const CPU_FLAG_INT_DISABLE: u8 = 4;
/// The decimal flag for the cpu flags register
const CPU_FLAG_DECIMAL: u8 = 8;
/// The b1 flag for the cpu flags register
const CPU_FLAG_B1: u8 = 0x10;
/// The b2 flag for the cpu flags register
const CPU_FLAG_B2: u8 = 0x20;
/// The overflow flag for the cpu flags register
const CPU_FLAG_OVERFLOW: u8 = 0x40;
/// The negative flag for the cpu flags register
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
            reset: true,
            opcode: None,
            temp: 0,
            temp2: 0,
            tempaddr: 0,
            #[cfg(feature = "debugger")]
            breakpoints: vec![],
            #[cfg(feature = "debugger")]
            debugger: NesCpuDebuggerPoint {
                a: 0,
                x: 0,
                y: 0,
                s: 0xfd,
                p: CPU_FLAG_B2 | CPU_FLAG_INT_DISABLE,
                pc: 0xfffc,
                disassembly: "RESET".to_string(),
            },
            #[cfg(feature = "debugger")]
            done_fetching: false,
            prev_nmi: false,
            nmi_detected: false,
            interrupt_shift: [(false, false); 2],
            interrupt_type: false,
            interrupting: false,
            oamdma: None,
            dma_counter: 0,
            outs: [false; 3],
            dmc_dma: None,
            dmc_dma_counter: 0,
        }
    }

    /// Copies current cpu state to the debugger state
    #[cfg(feature = "debugger")]
    fn copy_debugger(&mut self, s: String) {
        self.debugger.a = self.a;
        self.debugger.x = self.x;
        self.debugger.y = self.y;
        self.debugger.s = self.s;
        self.debugger.p = self.p;
        self.debugger.pc = self.pc;
        self.debugger.disassembly = s;
    }

    /// Returns true at the very start of an instruction
    #[cfg(test)]
    pub fn instruction_start(&self) -> bool {
        self.subcycle == 0
    }

    /// Returns true when done fetching all bytes for an instruction.
    #[cfg(feature = "debugger")]
    pub fn breakpoint_option(&self) -> bool {
        self.done_fetching
    }

    /// Returns the pc value
    #[cfg(test)]
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    /// Returns the a value
    #[cfg(test)]
    pub fn get_a(&self) -> u8 {
        self.a
    }

    /// Returns the x value
    #[cfg(test)]
    pub fn get_x(&self) -> u8 {
        self.x
    }

    /// Returns the y value
    #[cfg(test)]
    pub fn get_y(&self) -> u8 {
        self.y
    }

    /// Returns the p value
    #[cfg(test)]
    pub fn get_p(&self) -> u8 {
        self.p
    }

    /// Returns the sp value
    #[cfg(test)]
    pub fn get_sp(&self) -> u8 {
        self.s
    }

    /// Reset the cpu
    pub fn reset(&mut self) {
        self.s = self.s.wrapping_sub(3);
        self.p |= CPU_FLAG_INT_DISABLE; //set IRQ disable flag
        self.pc = 0xfffc;
        self.reset = true;
        self.subcycle = 0;
        self.opcode = None;
        #[cfg(feature = "debugger")]
        {
            self.copy_debugger("RESET".to_string());
        }
    }

    /// signal the end of a cpu instruction
    fn end_instruction(&mut self) {
        self.subcycle = 0;
        self.opcode = None;
    }

    /// run the sbc calculation
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

    /// Run the adc calculation
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

    /// Calculate the two output enable outputs
    fn calc_oe(&mut self, addr: u16) -> [bool; 2] {
        [addr != 0x4016, addr != 0x4017]
    }

    /// convenience function for running a read cycle on the bus
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) -> u8 {
        bus.memory_cycle_read(addr, self.outs, self.calc_oe(addr), cpu_peripherals)
    }

    /// Convenience function for running a write cycle on the bus
    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) {
        if addr == 0x4014 {
            self.oamdma = Some(data);
        } else if addr == 0x4016 {
            self.outs[0] = (data & 1) != 0;
            self.outs[1] = (data & 2) != 0;
            self.outs[2] = (data & 4) != 0;
        }
        bus.memory_cycle_write(addr, data, self.outs, [true; 2], cpu_peripherals);
    }

    /// Check all breakpoints to see if a break needs to occur
    #[cfg(feature = "debugger")]
    fn check_breakpoints(&mut self) {
        for b in &self.breakpoints {
            if self.pc == *b {
                self.subcycle = 1;
            }
        }
    }

    /// Returns true when a breakpoint is active
    pub fn breakpoint(&self) -> bool {
        let mut b = false;
        for v in &self.breakpoints {
            if self.pc == *v {
                b = true;
            }
        }
        b
    }

    /// Show the disassembly of the current instruction
    #[cfg(feature = "debugger")]
    pub fn disassemble(&self) -> Option<String> {
        Some(self.debugger.disassembly.to_owned())
    }

    /// Set the dma input for dmc dma
    pub fn set_dma_input(&mut self, data: Option<u16>) {
        if data.is_some() && self.dmc_dma.is_none() {
            self.dmc_dma = data;
            self.dmc_dma_counter = 0;
        }
    }

    /// Run a single cycle of the cpu
    pub fn cycle(
        &mut self,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
        nmi: bool,
        irq: bool,
    ) {
        if !self.prev_nmi && nmi {
            self.nmi_detected = true;
        }
        #[cfg(feature = "debugger")]
        {
            self.done_fetching = false;
        }
        self.prev_nmi = nmi;
        self.interrupt_shift[0] = self.interrupt_shift[1];
        self.interrupt_shift[1] = (irq, self.nmi_detected);
        if self.reset {
            match self.subcycle {
                0 => {
                    self.memory_cycle_read(self.pc, bus, cpu_peripherals);
                    self.subcycle += 1;
                }
                1 => {
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                    self.reset = false;
                }
            }
        } else if let Some(a) = self.dmc_dma {
            match self.dmc_dma_counter {
                0 => {
                    self.dmc_dma_counter += 1;
                }
                1 => {
                    self.dmc_dma_counter += 1;
                }
                2 => {
                    self.dmc_dma_counter += 1;
                }
                _ => {
                    let t = self.memory_cycle_read(a, bus, cpu_peripherals);
                    cpu_peripherals.apu.provide_dma_response(t);
                    self.dmc_dma = None;
                    self.dmc_dma_counter = 0;
                }
            }
        } else if self.opcode.is_none() {
            if (self.interrupt_shift[0].0 && ((self.p & CPU_FLAG_INT_DISABLE) == 0))
                || self.interrupt_shift[0].1
                || self.interrupting
            {
                match self.subcycle {
                    0 => {
                        self.interrupting = true;
                        self.memory_cycle_read(self.pc, bus, cpu_peripherals);
                        self.subcycle += 1;
                    }
                    1 => {
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle += 1;
                    }
                    2 => {
                        self.memory_cycle_write(
                            self.s as u16 + 0x100,
                            self.pc.to_le_bytes()[1],
                            bus,
                            cpu_peripherals,
                        );
                        self.s = self.s.wrapping_sub(1);
                        self.subcycle += 1;
                    }
                    3 => {
                        self.memory_cycle_write(
                            self.s as u16 + 0x100,
                            self.pc.to_le_bytes()[0],
                            bus,
                            cpu_peripherals,
                        );
                        self.s = self.s.wrapping_sub(1);
                        self.interrupt_type = self.nmi_detected;
                        self.nmi_detected = false;
                        self.subcycle += 1;
                    }
                    4 => {
                        self.p &= !(CPU_FLAG_B1 | CPU_FLAG_B2);
                        self.memory_cycle_write(
                            self.s as u16 + 0x100,
                            self.p,
                            bus,
                            cpu_peripherals,
                        );
                        self.s = self.s.wrapping_sub(1);
                        self.subcycle += 1;
                    }
                    5 => {
                        let addr = if !self.interrupt_type {
                            //IRQ
                            0xfffe
                        } else {
                            //NMI
                            0xfffa
                        };
                        #[cfg(feature = "debugger")]
                        {
                            if !self.interrupt_type {
                                self.copy_debugger("IRQ".to_string());
                            } else {
                                self.copy_debugger("NMI".to_string());
                            }
                            self.done_fetching = true;
                        }
                        let pcl = self.memory_cycle_read(addr, bus, cpu_peripherals);
                        let mut pc = self.pc.to_le_bytes();
                        pc[0] = pcl;
                        self.pc = u16::from_le_bytes(pc);
                        self.p |= CPU_FLAG_INT_DISABLE;
                        self.subcycle += 1;
                    }
                    _ => {
                        let addr = if !self.interrupt_type {
                            //IRQ
                            0xffff
                        } else {
                            //NMI
                            0xfffb
                        };
                        let pch = self.memory_cycle_read(addr, bus, cpu_peripherals);
                        let mut pc = self.pc.to_le_bytes();
                        pc[1] = pch;
                        self.pc = u16::from_le_bytes(pc);
                        self.subcycle = 0;
                        self.interrupting = false;
                    }
                }
            } else if let Some(addr) = self.oamdma {
                if self.dma_counter == 512 {
                    self.oamdma = None;
                    self.dma_counter = 0;
                } else if (self.dma_counter & 1) == 0 {
                    let addr = (addr as u16) << 8 | (self.dma_counter >> 1);
                    self.temp = self.memory_cycle_read(addr, bus, cpu_peripherals);
                    self.dma_counter += 1;
                } else {
                    self.memory_cycle_write(0x2004, self.temp, bus, cpu_peripherals);
                    self.dma_counter += 1;
                }
            } else {
                self.opcode = Some(self.memory_cycle_read(self.pc, bus, cpu_peripherals));
                #[cfg(feature = "debugger")]
                self.check_breakpoints();
                self.subcycle = 1;
            }
        } else if let Some(o) = self.opcode {
            match o {
                //brk instruction
                0 => match self.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("BRK".to_string());
                            self.done_fetching = true;
                        }
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        let pc = self.pc.to_le_bytes();
                        self.memory_cycle_write(0x100 + self.s as u16, pc[1], bus, cpu_peripherals);
                        self.s = self.s.wrapping_sub(1);
                        self.subcycle = 3;
                    }
                    3 => {
                        let pc = self.pc.to_le_bytes();
                        self.memory_cycle_write(0x100 + self.s as u16, pc[0], bus, cpu_peripherals);
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
                        self.p |= CPU_FLAG_INT_DISABLE;
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
                0x29 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("AND #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.a &= self.temp;
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
                //and zero page
                0x25 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("AND ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("AND ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("AND ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    _ => {
                        let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("AND ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.x);
                        if !overflow {
                            addr = addr.wrapping_add(self.x as u16);
                            self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("AND ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.y);
                        if !overflow {
                            addr = addr.wrapping_add(self.y as u16);
                            self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("AND (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("AND (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a &= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                0x09 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("ORA #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
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
                //ora zero page
                0x05 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ORA ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ORA ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ORA ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ORA ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.x);
                        if !overflow {
                            addr = addr.wrapping_add(self.x as u16);
                            self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ORA ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.y);
                        if !overflow {
                            addr = addr.wrapping_add(self.y as u16);
                            self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ORA (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ORA (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a |= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                0x49 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("EOR #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
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
                //eor zero page
                0x45 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("EOR ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                //eor zero page x
                0x55 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("EOR ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("EOR ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    _ => {
                        let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
                        self.a ^= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("EOR ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.x);
                        if !overflow {
                            addr = addr.wrapping_add(self.x as u16);
                            self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("EOR ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        let (_val, overflow) = self.temp.overflowing_add(self.y);
                        if !overflow {
                            addr = addr.wrapping_add(self.y as u16);
                            self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("EOR (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("EOR (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                            self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                        self.a ^= self.memory_cycle_read(addr, bus, cpu_peripherals);
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
                //adc immediate, add with carry
                0x69 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("ADC #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.cpu_adc(self.temp);
                    self.pc = self.pc.wrapping_add(2);
                    self.end_instruction();
                }
                //adc zero page
                0x65 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ADC ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.cpu_adc(self.temp);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //adc zero page x
                0x75 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ADC ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ADC ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ADC ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ADC ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ADC (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ADC (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                //sbc immediate, subtract with carry
                0xe9 | 0xeb => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("SBC #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.cpu_sbc(self.temp);
                    self.pc = self.pc.wrapping_add(2);
                    self.end_instruction();
                }
                //sbc zero page
                0xe5 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("SBC ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.cpu_sbc(self.temp);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //sbc zero page x
                0xf5 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("SBC ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("SBC ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("SBC ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("SBC ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("SBC (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("SBC (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("INC ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("INC ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("INC ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //inc absolute x
                0xfe => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("INC ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //iny, increment y
                0xc8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("INY".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //inx, increment x
                0xe8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("INX".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //dec decrement zero page
                0xc6 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("DEC ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("DEC ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("DEC ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_sub(1);
                        self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //dec absolute x
                0xde => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("DEC ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_sub(1);
                        self.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //dey, decrement y
                0x88 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("DEY".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //dex, decrement x
                0xca => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("DEX".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //tay, transfer accumulator to y
                0xa8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TAY".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //tax, transfer accumulator to x
                0xaa => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TAX".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //tya, transfer y to accumulator
                0x98 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TYA".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //txa, transfer x to accumulator
                0x8a => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TXA".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //tsx, transfer stack pointer to x
                0xba => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TSX".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                //txs, transfer x to stack pointer
                0x9a => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("TXS".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.s = self.x;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //bit zero page
                0x24 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("BIT ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        self.p |= self.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        self.temp &= self.a;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("BIT ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    _ => {
                        let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        self.p |= self.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        self.temp &= self.a;
                        self.p &= !CPU_FLAG_ZERO;
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //cmp, compare immediate
                0xc9 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("CMP #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
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
                //cmp zero page
                0xc5 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CMP ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CMP ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("CMP ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("CMP ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("CMP ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CMP (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CMP (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                0xc0 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("CPY #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
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
                //cpy zero page
                0xc4 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CPY ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("CPY ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                0xe0 => {
                    self.temp =
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("CPX #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
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
                //cpx zero page
                0xe4 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("CPX ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("CPX ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    _ => {
                        let t2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("JMP ${:04x}", newpc));
                            self.done_fetching = true;
                        }
                        self.pc = newpc;
                        self.end_instruction();
                    }
                },
                //jmp indirect
                0x6c => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("JMP (${:04x})", temp));
                            self.done_fetching = true;
                        }
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.subcycle = 3;
                    }
                    3 => {
                        let temp = self.temp;
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.tempaddr = (self.temp2 as u16) << 8 | (temp.wrapping_add(1) as u16);
                        self.subcycle = 4;
                    }
                    _ => {
                        self.temp2 = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.pc = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.end_instruction();
                    }
                },
                //sta, store a zero page
                0x85 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STA ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STA ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("STA ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("STA ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("STA ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STA (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STA (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                0xa2 => {
                    self.temp = self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("LDX #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.x = self.temp;
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
                //ldx zero page
                0xa6 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDX ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDX ${:02x},Y", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDX ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDX ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        let (_val, overflow) = self.temp.overflowing_add(self.y);
                        if !overflow {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STY ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STY ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("STY ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                0xa0 => {
                    self.temp = self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("LDY #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.y = self.temp;
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
                //ldy zero page
                0xa4 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDY ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDY ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDY ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDY ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                0xa9 => {
                    self.temp = self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger(format!("LDA #${:02x}", self.temp));
                        self.done_fetching = true;
                    }
                    self.a = self.temp;
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
                //lda zero page
                0xa5 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDA ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDA ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDA ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDA (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDA ${:04x},X", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LDA ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LDA (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STX ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("STX ${:02x},Y", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("STX ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                0x4a => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("LSR A".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (self.a & 1) != 0 {
                        self.p |= CPU_FLAG_CARRY;
                    }
                    self.a >>= 1;
                    if self.a == 0 {
                        self.p |= CPU_FLAG_ZERO;
                    }
                    if (self.a & 0x80) != 0 {
                        self.p |= CPU_FLAG_NEGATIVE;
                    }
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //lsr zero page
                0x46 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LSR ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 >>= 1;
                        if self.temp2 == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("LSR ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 >>= 1;
                        if self.temp2 == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LSR ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //lsr absolute x
                0x5e => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("LSR ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //asl, arithmetic shift left accumulator
                0x0a => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("ASL A".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (self.a & 0x80) != 0 {
                        self.p |= CPU_FLAG_CARRY;
                    }
                    self.a <<= 1;
                    if self.a == 0 {
                        self.p |= CPU_FLAG_ZERO;
                    }
                    if (self.a & 0x80) != 0 {
                        self.p |= CPU_FLAG_NEGATIVE;
                    }
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //asl zero page
                0x06 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ASL ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 <<= 1;
                        if self.temp2 == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ASL ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 <<= 1;
                        if self.temp2 == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ASL ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //asl absolute x
                0x1e => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ASL ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //ror rotate right accumulator
                0x6a => {
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("ROR A".to_string());
                        self.done_fetching = true;
                    }
                    let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                    self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (self.a & 1) != 0 {
                        self.p |= CPU_FLAG_CARRY;
                    }
                    self.a >>= 1;
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
                //ror zero page
                0x66 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ROR ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 >>= 1;
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ROR ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 >>= 1;
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ROR ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //ror absolute x
                0x7e => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ROR ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rol accumulator
                0x2a => {
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("ROL A".to_string());
                        self.done_fetching = true;
                    }
                    let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                    self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (self.a & 0x80) != 0 {
                        self.p |= CPU_FLAG_CARRY;
                    }
                    self.a <<= 1;
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
                //rol zero page
                0x26 => match self.subcycle {
                    1 => {
                        self.subcycle = 2;
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ROL ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 <<= 1;
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("ROL ${:02x},X", self.temp));
                            self.done_fetching = true;
                        }
                        self.temp = self.temp.wrapping_add(self.x);
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp2 & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp2 <<= 1;
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
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ROL ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 1;
                        }
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rol absolute x
                0x3e => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("ROL ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 1;
                        }
                        if self.temp == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("RTI".to_string());
                            self.done_fetching = true;
                        }
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.s = self.s.wrapping_add(1);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.p =
                            self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                        self.p &= !CPU_FLAG_B1;
                        self.p |= CPU_FLAG_B2;
                        self.s = self.s.wrapping_add(1);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.temp =
                            self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                        self.s = self.s.wrapping_add(1);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.temp2 =
                            self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        let pc = (self.pc.wrapping_add(2)).to_le_bytes();
                        self.memory_cycle_write(0x100 + self.s as u16, pc[1], bus, cpu_peripherals);
                        self.s = self.s.wrapping_sub(1);
                        self.subcycle = 4;
                    }
                    4 => {
                        let pc = (self.pc.wrapping_add(2)).to_le_bytes();
                        self.memory_cycle_write(0x100 + self.s as u16, pc[0], bus, cpu_peripherals);
                        self.s = self.s.wrapping_sub(1);
                        self.subcycle = 5;
                    }
                    _ => {
                        let t2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let newpc: u16 = (self.temp as u16) | (t2 as u16) << 8;
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("JSR ${:04x}", newpc));
                            self.done_fetching = true;
                        }
                        self.pc = newpc;
                        self.end_instruction();
                    }
                },
                //nop
                0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xea | 0xfa => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("NOP".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.pc = self.pc.wrapping_add(1);
                    self.subcycle = 0;
                    self.opcode = None;
                }
                //extra nop
                0x04 | 0x44 | 0x64 => match self.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("NOP".to_string());
                            self.done_fetching = true;
                        }
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("NOP".to_string());
                            self.done_fetching = true;
                        }
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
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("NOP".to_string());
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("NOP".to_string());
                            self.done_fetching = true;
                        }
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
                0x80 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("NOP".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.pc = self.pc.wrapping_add(2);
                    self.subcycle = 0;
                    self.opcode = None;
                }
                //clv, clear overflow flag
                0xb8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("CLV".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !CPU_FLAG_OVERFLOW;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //sec set carry flag
                0x38 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("SEC".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p |= CPU_FLAG_CARRY;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //sei set interrupt disable flag
                0x78 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("SEI".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p |= CPU_FLAG_INT_DISABLE;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //sed set decimal flag
                0xf8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("SED".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p |= CPU_FLAG_DECIMAL;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //cld, clear decimal flag
                0xd8 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("CLD".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !CPU_FLAG_DECIMAL;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //clc clear carry flag
                0x18 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("CLC".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !CPU_FLAG_CARRY;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //cli clear interrupt disable
                0x58 => {
                    #[cfg(feature = "debugger")]
                    {
                        self.copy_debugger("CLI".to_string());
                        self.done_fetching = true;
                    }
                    self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                    self.p &= !CPU_FLAG_INT_DISABLE;
                    self.pc = self.pc.wrapping_add(1);
                    self.end_instruction();
                }
                //beq, branch if equal (zero flag set)
                0xf0 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BEQ ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_ZERO) != 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BNE ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_ZERO) == 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
                            self.subcycle = 3;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BVS ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_OVERFLOW) != 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BVC ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_OVERFLOW) == 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BPL ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_NEGATIVE) == 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BMI ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_NEGATIVE) != 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BCS ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_CARRY) != 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = self.pc.wrapping_add(2).wrapping_add(adjust);
                            self.copy_debugger(format!("BCC ${:04X}", tempaddr));
                            self.done_fetching = true;
                        }
                        if (self.p & CPU_FLAG_CARRY) == 0 {
                            self.pc = self.pc.wrapping_add(2);
                            let mut adjust = self.temp as u16;
                            if (self.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            self.tempaddr = self.pc.wrapping_add(adjust);
                            self.subcycle = 2;
                        } else {
                            self.pc = self.pc.wrapping_add(2);
                            self.end_instruction();
                        }
                    }
                    2 => {
                        self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        let pc = self.pc.to_le_bytes();
                        let pc2 = self.tempaddr.to_le_bytes();
                        self.pc = self.tempaddr;
                        if pc[1] != pc2[1] {
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("PHA".to_string());
                            self.done_fetching = true;
                        }
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("PHP".to_string());
                            self.done_fetching = true;
                        }
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("PLP".to_string());
                            self.done_fetching = true;
                        }
                        self.s = self.s.wrapping_add(1);
                        self.p =
                            self.memory_cycle_read(0x100 + self.s as u16, bus, cpu_peripherals);
                        self.p &= !CPU_FLAG_B1;
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("PLA".to_string());
                            self.done_fetching = true;
                        }
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
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger("RTS".to_string());
                            self.done_fetching = true;
                        }
                        self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
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
                        self.pc |=
                            (self.memory_cycle_read(self.s as u16 + 0x100, bus, cpu_peripherals)
                                as u16)
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*LAX (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*LAX ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    _ => {
                        self.temp = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*LAX ${:04x}", temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*LAX (${:02x}),Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*LAX ${:02x},Y", self.temp));
                            self.done_fetching = true;
                        }
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*LAX ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | self.temp as u16;
                        let (_val, overflow) = self.temp.overflowing_add(self.y);
                        if !overflow {
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SAX (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.tempaddr = self.temp.wrapping_add(self.x) as u16;
                        self.temp2 = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SAX ${:02x}", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    _ => {
                        self.temp2 = self.a & self.x;
                        self.memory_cycle_write(self.temp as u16, self.temp2, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //sax absolute
                0x8f => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SAX ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    _ => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp2 = self.a & self.x;
                        self.memory_cycle_write(self.tempaddr, self.temp2, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //sax absolute y
                0x97 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SAX ${:02x},Y", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.subcycle = 3;
                    }
                    _ => {
                        self.tempaddr = self.temp.wrapping_add(self.y) as u16;
                        self.temp2 = self.a & self.x;
                        self.memory_cycle_write(self.tempaddr, self.temp2, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //dcp, undocumented, decrement and compare indirect x
                0xc3 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*DCP (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_sub(1);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //dcp zero page, undocumented
                0xc7 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*DCP ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.temp = self.temp.wrapping_sub(1);
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*DCP ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_sub(1);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //dcp indirect y
                0xd3 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*DCP (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*DCP ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.temp = self.temp.wrapping_sub(1);
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*DCP ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 6;
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
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //dcp absolute x, undocumented
                0xdf => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*DCP ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_sub(1);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //isb indirect x, increment memory, sub memory from accumulator, undocumented
                0xe3 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*ISB (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.subcycle = 7;
                    }
                    _ => {
                        self.cpu_sbc(self.temp);
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //isb zero page, undocumented
                0xe7 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*ISB ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.temp = self.temp.wrapping_add(1);
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*ISB ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.subcycle = 5;
                    }
                    _ => {
                        self.cpu_sbc(self.temp);
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //isb indirect y, undocumented
                0xf3 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*ISB (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*ISB ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.temp = self.temp.wrapping_add(1);
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*ISB ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.cpu_sbc(self.temp);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //isb absolute x, undocumented
                0xff => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*ISB ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.temp = self.temp.wrapping_add(1);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.cpu_sbc(self.temp);
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //slo shift left, then or with accumulator, undocumented
                //indirect x
                0x03 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SLO (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 7;
                    }
                    _ => {
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.a |= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //slo zero page, undocumented
                0x07 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SLO ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    _ => {
                        self.a |= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SLO ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.subcycle = 5;
                    }
                    _ => {
                        self.a |= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //slo indirect y, undocumented
                0x13 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SLO (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 7;
                    }
                    _ => {
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a |= self.temp;
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SLO ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    _ => {
                        self.a |= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SLO ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a |= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SLO ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        self.subcycle = 6;
                    }
                    _ => {
                        self.a |= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rla, rotate left, then and with accumulator, undocumented
                //indirect x
                0x23 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RLA (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.a &= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 7;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //rla zero page, undocumented
                0x27 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RLA ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RLA ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.a &= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rla indirect y
                0x33 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RLA (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 7;
                    }
                    _ => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RLA ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RLA ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a &= self.temp;
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
                //rla absolute x, undocumented
                0x3f => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RLA ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x80) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp <<= 1;
                        if old_carry {
                            self.temp |= 0x1;
                        }
                        self.a &= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //sre, shift right, then xor with accumulator, undocumented
                //indirect x
                0x43 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SRE (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.a ^= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 7;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //sre zero page, undocumented
                0x47 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SRE ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.a ^= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SRE ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.a ^= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //sre indirect y, undocumented
                0x53 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SRE (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 7;
                    }
                    _ => {
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a ^= self.temp;
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*SRE ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
                        self.a ^= self.temp;
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SRE ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.a ^= self.temp;
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
                //sre absolute x, undocumented
                0x5f => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*SRE ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;

                        self.a ^= self.temp;
                        if self.a == 0 {
                            self.p |= CPU_FLAG_ZERO;
                        }
                        if (self.a & 0x80) != 0 {
                            self.p |= CPU_FLAG_NEGATIVE;
                        }
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rra, rotate right, then and with accumulator, undocumented
                //indirect x
                0x63 => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RRA (${:02x},X)", self.temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.temp.wrapping_add(self.x);
                        self.temp2 = self.memory_cycle_read(self.temp as u16, bus, cpu_peripherals);
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
                        self.tempaddr = (self.temp as u16) << 8 | (self.temp2 as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 5;
                    }
                    5 => {
                        self.subcycle = 6;
                    }
                    6 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.cpu_adc(self.temp);
                        self.subcycle = 7;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(2);
                        self.end_instruction();
                    }
                },
                //rra zero page, undocumented
                0x67 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RRA ${:02x}", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RRA ${:04x}", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.cpu_adc(self.temp);
                        self.subcycle = 5;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rra indirect y
                0x73 => match self.subcycle {
                    1 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RRA (${:02x}),Y", self.temp2));
                            self.done_fetching = true;
                        }
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
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
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.subcycle = 7;
                    }
                    _ => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
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
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            self.copy_debugger(format!("*RRA ${:02x},X", self.temp2));
                            self.done_fetching = true;
                        }
                        self.temp2 = self.temp2.wrapping_add(self.x);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp = self.memory_cycle_read(self.temp2 as u16, bus, cpu_peripherals);
                        self.subcycle = 3;
                    }
                    3 => {
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.memory_cycle_write(self.temp2 as u16, self.temp, bus, cpu_peripherals);
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
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RRA ${:04x},Y", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.y as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.cpu_adc(self.temp);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                //rra absolute x, undocumented
                0x7f => match self.subcycle {
                    1 => {
                        self.temp =
                            self.memory_cycle_read(self.pc.wrapping_add(1), bus, cpu_peripherals);
                        self.subcycle = 2;
                    }
                    2 => {
                        self.temp2 =
                            self.memory_cycle_read(self.pc.wrapping_add(2), bus, cpu_peripherals);
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (self.temp2 as u16) << 8 | self.temp as u16;
                            self.copy_debugger(format!("*RRA ${:04x},X", temp));
                            self.done_fetching = true;
                        }
                        self.subcycle = 3;
                    }
                    3 => {
                        self.tempaddr = (self.temp2 as u16) << 8 | (self.temp as u16);
                        self.tempaddr = self.tempaddr.wrapping_add(self.x as u16);
                        self.temp = self.memory_cycle_read(self.tempaddr, bus, cpu_peripherals);
                        self.subcycle = 4;
                    }
                    4 => {
                        self.subcycle = 5;
                    }
                    5 => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        let old_carry = (self.p & CPU_FLAG_CARRY) != 0;
                        self.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (self.temp & 0x1) != 0 {
                            self.p |= CPU_FLAG_CARRY;
                        }
                        self.temp >>= 1;
                        if old_carry {
                            self.temp |= 0x80;
                        }
                        self.cpu_adc(self.temp);
                        self.subcycle = 6;
                    }
                    _ => {
                        self.memory_cycle_write(self.tempaddr, self.temp, bus, cpu_peripherals);
                        self.pc = self.pc.wrapping_add(3);
                        self.end_instruction();
                    }
                },
                _ => {}
            }
        }
    }
}
