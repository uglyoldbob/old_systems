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
    /// The polled irq enable flag
    polled_irq_flag: bool,
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
    /// The current cycle for the dma access (true indicates write)
    dma_cycle: bool,
    /// Indicates that the dma is running
    dma_running: bool,
    /// The memory access of the last few cycles, true indicates write)
    last_accesses: [bool; 3],
    /// The total number of cycles for dma
    dma_count: u16,
    /// The last read address
    last_read: u16,
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
            polled_irq_flag: false,
            interrupt_type: false,
            interrupting: false,
            oamdma: None,
            dma_counter: 0,
            dma_running: false,
            outs: [false; 3],
            dmc_dma: None,
            dmc_dma_counter: 0,
            dma_cycle: false,
            last_accesses: [false; 3],
            dma_count: 0,
            last_read: 0,
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
        //println!("I: {}", s);
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
        self.polled_irq_flag = (self.p & CPU_FLAG_INT_DISABLE) == 0;
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
        c: impl FnOnce(&mut Self, u8) -> (),
        addr: u16,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) -> u8 {
        self.last_accesses[2] = self.last_accesses[1];
        self.last_accesses[1] = self.last_accesses[0];
        self.last_accesses[0] = false;
        self.last_read = addr;
        let a = bus.memory_cycle_read(addr, self.outs, self.calc_oe(addr), cpu_peripherals);
        c(self, a);
        a
    }

    /// Convenience function for running a write cycle on the bus
    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
    ) {
        self.last_accesses[2] = self.last_accesses[1];
        self.last_accesses[1] = self.last_accesses[0];
        self.last_accesses[0] = true;
        if addr == 0x4014 {
            self.oamdma = Some(data);
        } else if addr == 0x4016 {
            self.outs[0] = (data & 1) != 0;
            self.outs[1] = (data & 2) != 0;
            self.outs[2] = (data & 4) != 0;
        }
        bus.memory_cycle_write(addr, data, self.outs, [true; 2], cpu_peripherals);
    }

    /// Returns true when a breakpoint is active
    pub fn breakpoint(&self) -> bool {
        let mut b = false;
        if self.done_fetching {
            for v in &self.breakpoints {
                if self.debugger.pc == *v {
                    println!("subcycle for breakpoint is {}", self.subcycle);
                    b = true;
                }
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

    ///Poll the interrupt line
    fn poll_interrupt_line(&mut self, irq: bool, nmi: bool) {
        if !self.prev_nmi && nmi {
            self.nmi_detected = true;
        }
        self.prev_nmi = nmi;
        self.interrupt_shift[0] = self.interrupt_shift[1];
        self.interrupt_shift[1] = (irq, self.nmi_detected);
    }

    /// Run a single cycle of the cpu
    pub fn cycle(
        &mut self,
        bus: &mut NesMotherboard,
        cpu_peripherals: &mut NesCpuPeripherals,
        nmi: bool,
        irq: bool,
    ) {
        let s = self;
        s.dma_cycle = !s.dma_cycle;

        #[cfg(feature = "debugger")]
        {
            s.done_fetching = false;
        }

        s.poll_interrupt_line(irq, nmi);
        if s.reset {
            match s.subcycle {
                0 => {
                    s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                    s.subcycle += 1;
                }
                1 => {
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.subcycle += 1;
                }
                2 => {
                    s.memory_cycle_read(|s,v| {}, s.s as u16 + 0x100, bus, cpu_peripherals);
                    s.subcycle += 1;
                }
                3 => {
                    s.memory_cycle_read(|s,v| {}, s.s as u16 + 0xff, bus, cpu_peripherals);
                    s.subcycle += 1;
                }
                4 => {
                    s.memory_cycle_read(|s,v| {}, s.s as u16 + 0xfe, bus, cpu_peripherals);
                    s.subcycle += 1;
                }
                5 => {
                    let pcl = s.memory_cycle_read(|s,v| {}, 0xfffc, bus, cpu_peripherals);
                    let mut pc = s.pc.to_le_bytes();
                    pc[0] = pcl;
                    s.pc = u16::from_le_bytes(pc);
                    s.subcycle += 1;
                }
                _ => {
                    let pch = s.memory_cycle_read(|s,v| {}, 0xfffd, bus, cpu_peripherals);
                    let mut pc = s.pc.to_le_bytes();
                    pc[1] = pch;
                    s.pc = u16::from_le_bytes(pc);
                    s.subcycle = 0;
                    s.reset = false;
                }
            }
        } else if s.opcode.is_none() {
            if (s.interrupt_shift[0].0 && s.polled_irq_flag)
                || s.interrupt_shift[0].1
                || s.interrupting
            {
                s.polled_irq_flag = false;
                match s.subcycle {
                    0 => {
                        s.interrupting = true;
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.subcycle += 1;
                    }
                    1 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle += 1;
                    }
                    2 => {
                        s.memory_cycle_write(
                            s.s as u16 + 0x100,
                            s.pc.to_le_bytes()[1],
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle += 1;
                    }
                    3 => {
                        s.memory_cycle_write(
                            s.s as u16 + 0x100,
                            s.pc.to_le_bytes()[0],
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.interrupt_type = s.nmi_detected;
                        s.nmi_detected = false;
                        s.subcycle += 1;
                    }
                    4 => {
                        s.p &= !(CPU_FLAG_B1 | CPU_FLAG_B2);
                        s.memory_cycle_write(
                            s.s as u16 + 0x100,
                            s.p,
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle += 1;
                    }
                    5 => {
                        let addr = if !s.interrupt_type {
                            //IRQ
                            0xfffe
                        } else {
                            //NMI
                            0xfffa
                        };
                        #[cfg(feature = "debugger")]
                        {
                            if !s.interrupt_type {
                                s.copy_debugger("IRQ".to_string());
                            } else {
                                s.copy_debugger("NMI".to_string());
                            }
                            s.done_fetching = true;
                        }
                        let pcl = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        let mut pc = s.pc.to_le_bytes();
                        pc[0] = pcl;
                        s.pc = u16::from_le_bytes(pc);
                        s.p |= CPU_FLAG_INT_DISABLE;
                        s.subcycle += 1;
                    }
                    _ => {
                        let addr = if !s.interrupt_type {
                            //IRQ
                            0xffff
                        } else {
                            //NMI
                            0xfffb
                        };
                        let pch = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        let mut pc = s.pc.to_le_bytes();
                        pc[1] = pch;
                        s.pc = u16::from_le_bytes(pc);
                        s.subcycle = 0;
                        s.interrupting = false;
                    }
                }
            } else if s.dma_running {
                if let Some(a) = s.dmc_dma {
                    match s.dmc_dma_counter {
                        0 => {
                            s.memory_cycle_read(|s,v| {}, s.last_read, bus, cpu_peripherals);
                            s.dmc_dma_counter += 1;
                        }
                        _ => {
                            if !cpu_peripherals.apu.get_clock() {
                                s.memory_cycle_read(
                                    |s,v| {},
                                    s.last_read,
                                    bus,
                                    cpu_peripherals,
                                );
                            } else {
                                let t = s.memory_cycle_read(|s,v| {}, a, bus, cpu_peripherals);
                                cpu_peripherals.apu.provide_dma_response(t);
                                s.dmc_dma = None;
                                s.dma_running = false;
                                s.dmc_dma_counter = 0;
                            }
                        }
                    }
                } else if let Some(addr) = s.oamdma {
                    s.dma_count += 1;
                    if (s.dma_counter & 1) == 0 && !s.dma_cycle {
                        let addr = (addr as u16) << 8 | (s.dma_counter >> 1);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.dma_counter += 1;
                    } else if (s.dma_counter & 1) != 0 && s.dma_cycle {
                        s.memory_cycle_write(0x2004, s.temp, bus, cpu_peripherals);
                        s.dma_counter += 1;
                    } else {
                        s.memory_cycle_read(|s,v| {}, s.last_read, bus, cpu_peripherals);
                    }
                    if s.dma_counter == 512 {
                        s.dma_running = false;
                        println!("DMA took {} cycles", s.dma_count);
                        s.dma_count = 0;
                        s.oamdma = None;
                        s.dma_counter = 0;
                    }
                }
            } else if s.dmc_dma.is_some()
                && !s.dma_running
                && !cpu_peripherals.apu.get_clock()
                && !s.last_accesses[0]
            {
                s.dma_running = true;
                s.memory_cycle_read(|s,v| {}, s.last_read, bus, cpu_peripherals);
                s.dma_count += 1;
            } else if s.oamdma.is_some() && !s.dma_running && !s.last_accesses[0] {
                s.dma_running = true;
                s.memory_cycle_read(|s,v| {}, s.last_read, bus, cpu_peripherals);
                s.dma_count += 1;
            } else {
                s.opcode = Some(s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals));
                //TODO set done fetching and call copy_debugger for single byte opcodes
                s.subcycle = 1;
            }
        } else if let Some(o) = s.opcode {
            match o {
                //brk instruction
                0 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("BRK".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        let temp = s.pc.wrapping_add(2);
                        let pc = temp.to_le_bytes();
                        s.memory_cycle_write(0x100 + s.s as u16, pc[1], bus, cpu_peripherals);
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle = 3;
                    }
                    3 => {
                        let temp = s.pc.wrapping_add(2);
                        let pc = temp.to_le_bytes();
                        s.memory_cycle_write(0x100 + s.s as u16, pc[0], bus, cpu_peripherals);
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle = 4;
                    }
                    4 => {
                        //s.p |= CPU_FLAG_B1;
                        s.memory_cycle_write(
                            0x100 + s.s as u16,
                            s.p | CPU_FLAG_B1,
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.temp = s.memory_cycle_read(|s,v| {}, 0xfffe, bus, cpu_peripherals);
                        s.p |= CPU_FLAG_INT_DISABLE;
                        s.subcycle = 6;
                    }
                    _ => {
                        s.temp2 = s.memory_cycle_read(|s,v| {}, 0xffff, bus, cpu_peripherals);
                        let addr: u16 = (s.temp as u16) | (s.temp2 as u16) << 8;
                        s.pc = addr;
                        s.end_instruction();
                    }
                },
                //and immediate
                0x29 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("AND #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a &= s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & s.temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //and zero page
                0x25 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("AND ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.a &= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //and zero page x
                0x35 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("AND ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.a &= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //and absolute
                0x2d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("AND ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.a &= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //and absolute x
                0x3d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("AND ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //and absolute y
                0x39 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("AND ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //and indirect x
                0x21 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("AND (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //and indirect y
                0x31 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("AND (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a &= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ora or immediate
                0x09 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("ORA #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a |= s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //ora zero page
                0x05 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ORA ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.a |= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ora zero page x
                0x15 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ORA ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.a |= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ora absolute
                0x0d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ORA ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.a |= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ora absolute x
                0x1d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ORA ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ora absolute y
                0x19 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ORA ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ora indirect x
                0x01 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ORA (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ora indirect y
                0x11 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ORA (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a |= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //eor xor immediate
                0x49 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("EOR #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a ^= s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //eor zero page
                0x45 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("EOR ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //eor zero page x
                0x55 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("EOR ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.a ^= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //eor absolute
                0x4d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("EOR ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //eor absolute x
                0x5d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("EOR ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //eor absolute y
                0x59 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("EOR ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //eor xor indirect x
                0x41 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("EOR (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //eor indirect y
                0x51 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("EOR (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a ^= s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //adc immediate, add with carry
                0x69 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("ADC #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.cpu_adc(s.temp);
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //adc zero page
                0x65 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ADC ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //adc zero page x
                0x75 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ADC ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //adc absolute
                0x6d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ADC ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //adc absolute x
                0x7d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ADC ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_adc(s.temp);
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //adc absolute y
                0x79 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ADC ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_adc(s.temp);
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //adc adc indirect x
                0x61 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ADC (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //adc indirect y
                0x71 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ADC (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_adc(s.temp);
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sbc immediate, subtract with carry
                0xe9 | 0xeb => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("SBC #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.cpu_sbc(s.temp);
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //sbc zero page
                0xe5 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("SBC ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sbc zero page x
                0xf5 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("SBC ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sbc absolute
                0xed => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("SBC ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sbc absolute x
                0xfd => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("SBC ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_sbc(s.temp);
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sbc absolute y
                0xf9 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("SBC ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_sbc(s.temp);
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sbc indirect x
                0xe1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("SBC (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sbc indirect y
                0xf1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("SBC (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.cpu_sbc(s.temp);
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //inc increment zero page
                0xe6 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("INC ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.temp2 = s.temp2.wrapping_add(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //inc increment zero page x
                0xf6 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("INC ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }

                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.temp2 = s.temp2.wrapping_add(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //inc absolute
                0xee => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("INC ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //inc absolute x
                0xfe => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("INC ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //iny, increment y
                0xc8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("INY".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.y = s.y.wrapping_add(1);
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.y & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.y == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //inx, increment x
                0xe8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("INX".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.x = s.x.wrapping_add(1);
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //dec decrement zero page
                0xc6 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("DEC ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.temp2 = s.temp2.wrapping_sub(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dec decrement zero page x
                0xd6 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("DEC ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.temp2 = s.temp2.wrapping_sub(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dec absolute
                0xce => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("DEC ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_sub(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //dec absolute x
                0xde => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("DEC ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_sub(1);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //dey, decrement y
                0x88 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("DEY".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.y = s.y.wrapping_sub(1);
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.y & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.y == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //dex, decrement x
                0xca => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("DEX".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.x = s.x.wrapping_sub(1);
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //tay, transfer accumulator to y
                0xa8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TAY".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.y = s.a;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.y & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.y == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //tax, transfer accumulator to x
                0xaa => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TAX".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.x = s.a;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //tya, transfer y to accumulator
                0x98 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TYA".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.a = s.y;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //txa, transfer x to accumulator
                0x8a => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TXA".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.a = s.x;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //tsx, transfer stack pointer to x
                0xba => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TSX".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.x = s.s;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //txs, transfer x to stack pointer
                0x9a => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("TXS".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.s = s.x;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //bit zero page
                0x24 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("BIT ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        s.p |= s.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        s.temp &= s.a;
                        s.p &= !CPU_FLAG_ZERO;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //bit absolute
                0x2c => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("BIT ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        s.p |= s.temp & (CPU_FLAG_OVERFLOW | CPU_FLAG_NEGATIVE);
                        s.temp &= s.a;
                        s.p &= !CPU_FLAG_ZERO;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //cmp, compare immediate
                0xc9 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("CMP #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                    if s.a == s.temp {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if s.a >= s.temp {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //cmp zero page
                0xc5 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CMP ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cmp zero page x
                0xd5 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CMP ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cmp absolute
                0xcd => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("CMP ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //cmp absolute x
                0xdd => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("CMP ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if s.a == s.temp {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if s.a >= s.temp {
                                s.p |= CPU_FLAG_CARRY;
                            }
                            if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //cmp absolute y
                0xd9 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("CMP ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if s.a == s.temp {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if s.a >= s.temp {
                                s.p |= CPU_FLAG_CARRY;
                            }
                            if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //cmp indirect x
                0xc1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CMP (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cmp indirect y
                0xd1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CMP (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                            if s.a == s.temp {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if s.a >= s.temp {
                                s.p |= CPU_FLAG_CARRY;
                            }
                            if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }

                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cpy, compare y immediate
                0xc0 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("CPY #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                    if s.y == s.temp {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if s.y >= s.temp {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    if ((s.y.wrapping_sub(s.temp)) & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //cpy zero page
                0xc4 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CPY ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.y == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.y >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.y.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cpy absolute
                0xcc => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("CPY ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.y == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.y >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.y.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //cpx, compare x immediate
                0xe0 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("CPX #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                    if s.x == s.temp {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if s.x >= s.temp {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    if ((s.x.wrapping_sub(s.temp)) & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //cpx zero page
                0xe4 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("CPX ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.x == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.x >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.x.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //cpx absolute
                0xec => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("CPX ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.x == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.x >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.x.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //jmp absolute
                0x4c => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    _ => {
                        let t2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let newpc: u16 = (s.temp as u16) | (t2 as u16) << 8;
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("JMP ${:04x}", newpc));
                            s.done_fetching = true;
                        }
                        s.pc = newpc;
                        s.end_instruction();
                    }
                },
                //jmp indirect
                0x6c => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("JMP (${:04x})", temp));
                            s.done_fetching = true;
                        }
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.subcycle = 3;
                    }
                    3 => {
                        let temp = s.temp;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.tempaddr = (s.temp2 as u16) << 8 | (temp.wrapping_add(1) as u16);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.pc = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.end_instruction();
                    }
                },
                //sta, store a zero page
                0x85 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STA ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.memory_cycle_write(s.temp as u16, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sta, store a zero page x
                0x95 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STA ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.memory_cycle_write(
                            s.temp.wrapping_add(s.x) as u16,
                            s.a,
                            bus,
                            cpu_peripherals,
                        );
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sta absolute
                0x8d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("STA ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.memory_cycle_write(temp, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sta absolute x
                0x9d => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("STA ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let temp = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | temp as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.memory_cycle_write(addr, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sta absolute y
                0x99 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("STA ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.y);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.memory_cycle_write(addr, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sta indirect x
                0x81 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STA (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.memory_cycle_write(addr, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sta indirect y
                0x91 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STA (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let temp2 = s.temp2.wrapping_add(s.y);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp as u16) << 8 | temp2 as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.memory_cycle_write(addr, s.a, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ldx immediate
                0xa2 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("LDX #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.x = s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //ldx zero page
                0xa6 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDX ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.x =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ldx zero page y
                0xb6 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDX ${:02x},Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.temp = s.temp.wrapping_add(s.y);
                    }
                    2 => {
                        s.subcycle = 3;
                    }
                    _ => {
                        s.x =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ldx absolute
                0xae => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDX ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.x = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ldx absolute y
                0xbe => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDX ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                            s.x =
                                s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.x == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.x & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.x =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sty store y zero page
                0x84 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STY ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    _ => {
                        s.memory_cycle_write(s.temp as u16, s.y, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sty zero page x
                0x94 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STY ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.subcycle = 3;
                    }
                    _ => {
                        s.memory_cycle_write(
                            s.temp.wrapping_add(s.x) as u16,
                            s.y,
                            bus,
                            cpu_peripherals,
                        );
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sty absolute
                0x8c => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("STY ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.memory_cycle_write(temp, s.y, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ldy load y immediate
                0xa0 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("LDY #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.y = s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.y == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.y & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //ldy zero page
                0xa4 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDY ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.y =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.y == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.y & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ldy zero page x
                0xb4 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDY ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.y = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.y == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.y & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ldy absolute
                0xac => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDY ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.y = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.y == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.y & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ldy absolute x
                0xbc => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDY ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let addr =
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            s.y = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.y == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.y & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.y = s.memory_cycle_read(
                            |s,v| {},
                            addr.wrapping_add(s.x as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.y == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.y & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lda immediate
                0xa9 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("LDA #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a = s.temp;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //lda zero page
                0xa5 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDA ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.a =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lda zero page x
                0xb5 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDA ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.a = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(s.x) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lda absolute
                0xad => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDA ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.a = s.memory_cycle_read(|s,v| {}, temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lda indirect x
                0xa1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDA (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lda absolute x
                0xbd => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDA ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.x);
                        if !overflow {
                            addr = addr.wrapping_add(s.x as u16);
                            s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (addr & 0xFF00) | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.x as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lda absolute y
                0xb9 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LDA ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp2 as u16) << 8 | val as u16,
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lda indirect y
                0xb1 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LDA (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.memory_cycle_read(
                                |s,v| {},
                                (s.temp as u16) << 8 | (val as u16),
                                bus,
                                cpu_peripherals,
                            );
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //stx zero page
                0x86 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STX ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.memory_cycle_write(s.temp as u16, s.x, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //stx zero page y
                0x96 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("STX ${:02x},Y", s.temp));
                            s.done_fetching = true;
                        }

                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_add(s.y);
                        s.memory_cycle_write(s.temp as u16, s.x, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //stx, store x absolute
                0x8e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("STX ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.memory_cycle_write(temp, s.x, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lsr logical shift right, accumulator
                0x4a => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("LSR A".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 1) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    s.a >>= 1;
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //lsr zero page
                0x46 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LSR ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 >>= 1;
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lsr zero page x
                0x56 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("LSR ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 >>= 1;
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lsr absolute
                0x4e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LSR ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lsr absolute x
                0x5e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("LSR ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //asl, arithmetic shift left accumulator
                0x0a => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("ASL A".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    s.a <<= 1;
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //asl zero page
                0x06 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ASL ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 <<= 1;
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //asl zero page x
                0x16 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ASL ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 <<= 1;
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //asl absolute
                0x0e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ASL ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //asl absolute x
                0x1e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ASL ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ror rotate right accumulator
                0x6a => {
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("ROR A".to_string());
                        s.done_fetching = true;
                    }
                    let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 1) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    s.a >>= 1;
                    if old_carry {
                        s.a |= 0x80;
                    }
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //ror zero page
                0x66 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ROR ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 >>= 1;
                        if old_carry {
                            s.temp2 |= 0x80;
                        }
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ror zero page x
                0x76 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ROR ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 >>= 1;
                        if old_carry {
                            s.temp2 |= 0x80;
                        }
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //ror absolute
                0x6e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ROR ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ror absolute x
                0x7e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ROR ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let val = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | val as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rol accumulator
                0x2a => {
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("ROL A".to_string());
                        s.done_fetching = true;
                    }
                    let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    s.a <<= 1;
                    if old_carry {
                        s.a |= 0x1;
                    }
                    if s.a == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (s.a & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //rol zero page
                0x26 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ROL ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 <<= 1;
                        if old_carry {
                            s.temp2 |= 1;
                        }
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rol zero page x
                0x36 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("ROL ${:02x},X", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp2 <<= 1;
                        if old_carry {
                            s.temp2 |= 1;
                        }
                        if s.temp2 == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp2 & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rol absolute
                0x2e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ROL ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 1;
                        }
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rol absolute x
                0x3e => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("ROL ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        let temp = s.temp.wrapping_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | temp as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 1;
                        }
                        if s.temp == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rti, return from interrupt
                0x40 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("RTI".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );

                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, 0x100 + s.s as u16, bus, cpu_peripherals);
                        s.s = s.s.wrapping_add(1);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.p = s.memory_cycle_read(
                            |s,v| {},
                            0x100 + s.s as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !CPU_FLAG_B1;
                        s.p |= CPU_FLAG_B2;
                        s.s = s.s.wrapping_add(1);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            0x100 + s.s as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_add(1);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            0x100 + s.s as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.pc = (s.temp2 as u16) << 8 | s.temp as u16;
                        s.end_instruction();
                    }
                },
                //jsr absolute
                0x20 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, 0x100 + s.s as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        let pc = (s.pc.wrapping_add(2)).to_le_bytes();
                        s.memory_cycle_write(0x100 + s.s as u16, pc[1], bus, cpu_peripherals);
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle = 4;
                    }
                    4 => {
                        let pc = (s.pc.wrapping_add(2)).to_le_bytes();
                        s.memory_cycle_write(0x100 + s.s as u16, pc[0], bus, cpu_peripherals);
                        s.s = s.s.wrapping_sub(1);
                        s.subcycle = 5;
                    }
                    _ => {
                        let t2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let newpc: u16 = (s.temp as u16) | (t2 as u16) << 8;
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("JSR ${:04x}", newpc));
                            s.done_fetching = true;
                        }
                        s.pc = newpc;
                        s.end_instruction();
                    }
                },
                //nop
                0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xea | 0xfa => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("NOP".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //special nop
                0x82 | 0xc2 | 0xe2 => {
                    let temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("NOP* #${:02x}", temp));
                        s.done_fetching = true;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //special nop
                0x89 => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("NOP* #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //extra nop
                0x04 | 0x44 | 0x64 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("NOP".to_string());
                            s.done_fetching = true;
                        }
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //extra nop
                0x0c => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("NOP".to_string());
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //extra nop
                0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("NOP".to_string());
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        //technically needs to have a register added to it
                        //but since it is zero page, the read has no side effects
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //extra nop
                0x1c | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("NOP".to_string());
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        let (_val, overflow) = s.temp.overflowing_add(s.x);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        if overflow {
                            s.subcycle = 4;
                        } else {
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            s.tempaddr.wrapping_add(s.x as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //extra nop
                0x80 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("NOP".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //clv, clear overflow flag
                0xb8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("CLV".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p &= !CPU_FLAG_OVERFLOW;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //sec set carry flag
                0x38 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("SEC".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p |= CPU_FLAG_CARRY;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //sei set interrupt disable flag
                0x78 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("SEI".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                    s.p |= CPU_FLAG_INT_DISABLE;
                }
                //sed set decimal flag
                0xf8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("SED".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p |= CPU_FLAG_DECIMAL;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //cld, clear decimal flag
                0xd8 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("CLD".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p &= !CPU_FLAG_DECIMAL;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //clc clear carry flag
                0x18 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("CLC".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.p &= !CPU_FLAG_CARRY;
                    s.pc = s.pc.wrapping_add(1);
                    s.end_instruction();
                }
                //cli clear interrupt disable
                0x58 => {
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger("CLI".to_string());
                        s.done_fetching = true;
                    }
                    s.memory_cycle_read(|s,v| {}, s.pc.wrapping_add(1), bus, cpu_peripherals);
                    s.end_instruction();
                    s.p &= !CPU_FLAG_INT_DISABLE;
                    s.pc = s.pc.wrapping_add(1);
                }
                //beq, branch if equal (zero flag set)
                0xf0 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BEQ ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_ZERO) != 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bne, branch if not equal (zero flag not set)
                0xd0 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BNE ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_ZERO) == 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bvs, branch if overflow set
                0x70 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BVS ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_OVERFLOW) != 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bvc branch if overflow clear
                0x50 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BVC ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_OVERFLOW) == 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bpl, branch if negative clear
                0x10 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BPL ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_NEGATIVE) == 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bmi branch if negative flag set
                0x30 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BMI ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_NEGATIVE) != 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bcs, branch if carry set
                0xb0 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BCS ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_CARRY) != 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //bcc branch if carry flag clear
                0x90 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            let tempaddr = s.pc.wrapping_add(2).wrapping_add(adjust);
                            s.copy_debugger(format!("BCC ${:04X}", tempaddr));
                            s.done_fetching = true;
                        }
                        if (s.p & CPU_FLAG_CARRY) == 0 {
                            s.pc = s.pc.wrapping_add(2);
                            let mut adjust = s.temp as u16;
                            if (s.temp & 0x80) != 0 {
                                adjust |= 0xff00;
                            }
                            s.tempaddr = s.pc.wrapping_add(adjust);
                            s.subcycle = 2;
                        } else {
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        }
                    }
                    2 => {
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        let pc = s.pc.to_le_bytes();
                        let pc2 = s.tempaddr.to_le_bytes();
                        s.pc = s.tempaddr;
                        if pc[1] != pc2[1] {
                            s.subcycle = 3;
                        } else {
                            s.end_instruction();
                        }
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.end_instruction();
                    }
                },
                //pha push accumulator
                0x48 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("PHA".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    _ => {
                        s.memory_cycle_write(
                            0x100 + s.s as u16,
                            s.a,
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.pc = s.pc.wrapping_add(1);
                        s.end_instruction();
                    }
                },
                //php push processor status
                0x08 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("PHP".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    _ => {
                        s.memory_cycle_write(
                            0x100 + s.s as u16,
                            s.p | CPU_FLAG_B1,
                            bus,
                            cpu_peripherals,
                        );
                        s.s = s.s.wrapping_sub(1);
                        s.pc = s.pc.wrapping_add(1);
                        s.end_instruction();
                    }
                },
                //plp, pull processor status
                0x28 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("PLP".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );

                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, 0x100 + s.s as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.s = s.s.wrapping_add(1);
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            0x100 + s.s as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.pc = s.pc.wrapping_add(1);
                        s.end_instruction();
                        s.p = s.temp;
                        s.p &= !CPU_FLAG_B1;
                        s.p |= CPU_FLAG_B2;
                    }
                },
                //pla, pull accumulator
                0x68 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("PLA".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, 0x100 + s.s as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.s = s.s.wrapping_add(1);
                        s.a = s.memory_cycle_read(
                            |s,v| {},
                            0x100 + s.s as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(1);
                        s.end_instruction();
                    }
                },
                //rts, return from subroutine
                0x60 => match s.subcycle {
                    1 => {
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger("RTS".to_string());
                            s.done_fetching = true;
                        }
                        s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.s as u16 + 0x100, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.s = s.s.wrapping_add(1);
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.s as u16 + 0x100,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.pc = s.temp as u16;
                        s.s = s.s.wrapping_add(1);
                        s.pc |= (s.memory_cycle_read(
                            |s,v| {},
                            s.s as u16 + 0x100,
                            bus,
                            cpu_peripherals,
                        ) as u16)
                            << 8;
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_read(|s,v| {}, s.pc, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(1);
                        s.end_instruction();
                    }
                },
                //lax (indirect x)?, undocumented
                0xa3 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*LAX (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    _ => {
                        let addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.x = s.a;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lax zero page?, undocumented
                0xa7 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*LAX ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.a = s.temp;
                        s.x = s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lax absolute, undocumented
                0xaf => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*LAX ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        let addr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.a = s.temp;
                        s.x = s.temp;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //lax indirect y, undocumented
                0xb3 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*LAX (${:02x}),Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        let (_val, overflow) = s.temp2.overflowing_add(s.y);
                        if !overflow {
                            addr = addr.wrapping_add(s.y as u16);
                            s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                            s.x = s.a;
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.a == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.a & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(2);
                            s.end_instruction();
                        } else {
                            s.subcycle = 5;
                        }
                    }
                    _ => {
                        let mut addr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        addr = addr.wrapping_add(s.y as u16);
                        s.a = s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.x = s.a;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }

                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lax zero page y, undocumented
                0xb7 => match s.subcycle {
                    1 => {
                        s.subcycle = 2;
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*LAX ${:02x},Y", s.temp));
                            s.done_fetching = true;
                        }
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_add(s.y);
                        s.x =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.a = s.x;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //lax absolute y, undocumented
                0xbf => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*LAX ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | s.temp as u16;
                        let (_val, overflow) = s.temp.overflowing_add(s.y);
                        if !overflow {
                            s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                            s.x =
                                s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                            s.a = s.x;
                            s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                            if s.x == 0 {
                                s.p |= CPU_FLAG_ZERO;
                            }
                            if (s.x & 0x80) != 0 {
                                s.p |= CPU_FLAG_NEGATIVE;
                            }
                            s.pc = s.pc.wrapping_add(3);
                            s.end_instruction();
                        } else {
                            s.subcycle = 4;
                        }
                    }
                    _ => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.x =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.a = s.x;
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if s.x == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.x & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sax indirect x
                0x83 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SAX (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.tempaddr = s.temp.wrapping_add(s.x) as u16;
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.temp.wrapping_add(s.x).wrapping_add(1) as u16;
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp = s.x & s.a;
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sax zero page
                0x87 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SAX ${:02x}", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    _ => {
                        s.temp2 = s.a & s.x;
                        s.memory_cycle_write(s.temp as u16, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sax absolute
                0x8f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SAX ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    _ => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp2 = s.a & s.x;
                        s.memory_cycle_write(s.tempaddr, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sax absolute y
                0x97 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SAX ${:02x},Y", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.subcycle = 3;
                    }
                    _ => {
                        s.tempaddr = s.temp.wrapping_add(s.y) as u16;
                        s.temp2 = s.a & s.x;
                        s.memory_cycle_write(s.tempaddr, s.temp2, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dcp, undocumented, decrement and compare indirect x
                0xc3 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*DCP (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_sub(1);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dcp zero page, undocumented
                0xc7 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*DCP ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_sub(1);
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dcp absolute, undocumented
                0xcf => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*DCP ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_sub(1);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //dcp indirect y
                0xd3 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*DCP (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_sub(1);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dcp zero page x, undocumented
                0xd7 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*DCP ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_sub(1);
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //dcp absolute y, undocumented
                0xdb => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*DCP ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_sub(1);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //dcp absolute x, undocumented
                0xdf => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*DCP ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_sub(1);
                        s.subcycle = 6;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_CARRY | CPU_FLAG_NEGATIVE);
                        if s.a == s.temp {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if s.a >= s.temp {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        if ((s.a.wrapping_sub(s.temp)) & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //isb indirect x, increment memory, sub memory from accumulator, undocumented
                0xe3 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*ISB (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.cpu_sbc(s.temp);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //isb zero page, undocumented
                0xe7 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*ISB ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(1);
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //isb absolute, undocumented
                0xef => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*ISB ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.cpu_sbc(s.temp);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //isb indirect y, undocumented
                0xf3 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*ISB (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_add(1);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //isb zero page x, undocumented
                0xf7 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*ISB ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.temp = s.temp.wrapping_add(1);
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //isb absolute y, undocumented
                0xfb => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*ISB ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.cpu_sbc(s.temp);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //isb absolute x, undocumented
                0xff => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*ISB ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.temp = s.temp.wrapping_add(1);
                        s.subcycle = 6;
                    }
                    _ => {
                        s.cpu_sbc(s.temp);
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //slo shift left, then or with accumulator, undocumented
                //indirect x
                0x03 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SLO (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //slo zero page, undocumented
                0x07 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SLO ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //slo absolute, undocumented
                0x0f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SLO ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.subcycle = 5;
                    }
                    _ => {
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //slo indirect y, undocumented
                0x13 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SLO (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //slo zero page x, undocumented
                0x17 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SLO ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //slo absolute y, undocumented
                0x1b => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SLO ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //slo absolute x, undocumented
                0x1f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SLO ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_NEGATIVE | CPU_FLAG_ZERO | CPU_FLAG_CARRY);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        s.subcycle = 6;
                    }
                    _ => {
                        s.a |= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rla, rotate left, then and with accumulator, undocumented
                //indirect x
                0x23 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RLA (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 7;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rla zero page, undocumented
                0x27 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RLA ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 4;
                    }
                    _ => {
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rla absolute, undocumented
                0x2f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RLA ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rla indirect y
                0x33 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RLA (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rla zero page x, undocumented
                0x37 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RLA ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rla absolute y, undocumented
                0x3b => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RLA ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rla absolute x, undocumented
                0x3f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RLA ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x80) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp <<= 1;
                        if old_carry {
                            s.temp |= 0x1;
                        }
                        s.a &= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sre, shift right, then xor with accumulator, undocumented
                //indirect x
                0x43 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SRE (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 7;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sre zero page, undocumented
                0x47 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SRE ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 7;
                    }
                    _ => {
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sre absolute, undocumented
                0x4f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SRE ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sre indirect y, undocumented
                0x53 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SRE (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sre zero page x, undocumented
                0x57 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*SRE ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //sre absolute y, undocumented
                0x5b => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SRE ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //sre absolute x, undocumented
                0x5f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SRE ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;

                        s.a ^= s.temp;
                        if s.a == 0 {
                            s.p |= CPU_FLAG_ZERO;
                        }
                        if (s.a & 0x80) != 0 {
                            s.p |= CPU_FLAG_NEGATIVE;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rra, rotate right, then and with accumulator, undocumented
                //indirect x
                0x63 => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RRA (${:02x},X)", s.temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp = s.temp.wrapping_add(s.x);
                        s.temp2 =
                            s.memory_cycle_read(|s,v| {}, s.temp as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.temp.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = (s.temp as u16) << 8 | (s.temp2 as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.cpu_adc(s.temp);
                        s.subcycle = 7;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rra zero page, undocumented
                0x67 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RRA ${:02x}", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rra absolute, undocumented
                0x6f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RRA ${:04x}", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.cpu_adc(s.temp);
                        s.subcycle = 5;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rra indirect y
                0x73 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RRA (${:02x}),Y", s.temp2));
                            s.done_fetching = true;
                        }
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.temp2.wrapping_add(1) as u16,
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 5;
                    }
                    5 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 6;
                    }
                    6 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.subcycle = 7;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rra zero page x, undocumented
                0x77 => match s.subcycle {
                    1 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            s.copy_debugger(format!("*RRA ${:02x},X", s.temp2));
                            s.done_fetching = true;
                        }

                        s.subcycle = 2;
                    }
                    2 => {
                        s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 3;
                    }
                    3 => {
                        s.temp2 = s.temp2.wrapping_add(s.x);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.temp2 as u16, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    4 => {
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    _ => {
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.memory_cycle_write(s.temp2 as u16, s.temp, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(2);
                        s.end_instruction();
                    }
                },
                //rra absolute y, undocumented
                0x7b => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RRA ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.y as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.cpu_adc(s.temp);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //rra absolute x, undocumented
                0x7f => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*RRA ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        s.tempaddr = (s.temp2 as u16) << 8 | (s.temp as u16);
                        s.memory_cycle_read(
                            |s,v| {},
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 4;
                    }
                    4 => {
                        s.tempaddr = s.tempaddr.wrapping_add(s.x as u16);
                        s.temp =
                            s.memory_cycle_read(|s,v| {}, s.tempaddr, bus, cpu_peripherals);
                        s.subcycle = 5;
                    }
                    5 => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        let old_carry = (s.p & CPU_FLAG_CARRY) != 0;
                        s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                        if (s.temp & 0x1) != 0 {
                            s.p |= CPU_FLAG_CARRY;
                        }
                        s.temp >>= 1;
                        if old_carry {
                            s.temp |= 0x80;
                        }
                        s.cpu_adc(s.temp);
                        s.subcycle = 6;
                    }
                    _ => {
                        s.memory_cycle_write(s.tempaddr, s.temp, bus, cpu_peripherals);
                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //ANC immediate, undocumented
                //performs AND and ROL
                0x0b | 0x2b => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("*ANC #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a &= s.temp;
                    let temp = s.a;

                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    if temp == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }

                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //ALR, and immediate with lsr, undocumented
                0x4b => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("*ALR #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a &= s.temp;

                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.a & 1) != 0 {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    let temp = s.a >> 1;
                    if temp == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    s.a = temp;
                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //ARR, undocumented AND immediate and ROR
                0x6b => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("*ARR #${:02x}", s.temp));
                        s.done_fetching = true;
                    }
                    s.a &= s.temp;

                    s.p &= !CPU_FLAG_OVERFLOW;
                    if ((s.a ^ (s.a >> 1)) & 0x40) == 0x40 {
                        s.p |= CPU_FLAG_OVERFLOW;
                    }
                    let carry = (s.a & 0x80) != 0;
                    s.a >>= 1;
                    if (s.p & CPU_FLAG_CARRY) != 0 {
                        s.a |= 0x80;
                    }

                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if carry {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    let temp = s.a;
                    if temp == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    if (temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }

                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //lax undocumented, lda immediate, then tax
                0xab => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("*LAX #${:02x}", s.temp));
                        s.done_fetching = true;
                    }

                    s.a = s.temp;

                    s.x = s.a;
                    s.p &= !(CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if (s.x & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if s.x == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }

                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //axs undocumented
                0xcb => {
                    s.temp = s.memory_cycle_read(
                        |s,v| {},
                        s.pc.wrapping_add(1),
                        bus,
                        cpu_peripherals,
                    );
                    #[cfg(feature = "debugger")]
                    {
                        s.copy_debugger(format!("*AXS #${:02x}", s.temp));
                        s.done_fetching = true;
                    }

                    let anding = s.a & s.x;
                    let temp = (anding as u16).wrapping_sub(s.temp as u16);
                    let carry = (((temp >> 8) & 0x01) ^ 0x01) == 0x01;
                    s.p &= !(CPU_FLAG_CARRY | CPU_FLAG_ZERO | CPU_FLAG_NEGATIVE);
                    if carry {
                        s.p |= CPU_FLAG_CARRY;
                    }
                    if (temp & 0x80) != 0 {
                        s.p |= CPU_FLAG_NEGATIVE;
                    }
                    if (temp & 0xFF) == 0 {
                        s.p |= CPU_FLAG_ZERO;
                    }
                    s.x = (temp & 0xFF) as u8;

                    s.pc = s.pc.wrapping_add(2);
                    s.end_instruction();
                }
                //SHY, undocumented
                0x9C => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SHY ${:04x},X", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let addr =
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.x) as u16);
                        s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        let addr = ((s.temp2 as u16).wrapping_add(1) << 8 | (s.temp as u16))
                            .wrapping_add(s.x as u16);
                        let mask = (s.y as u16) << 8 | 0xFF;
                        let val = s.temp2.wrapping_add(1) & s.y;
                        s.memory_cycle_write(addr & mask, val, bus, cpu_peripherals);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                //SHX, undocumented
                0x9E => match s.subcycle {
                    1 => {
                        s.temp = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(1),
                            bus,
                            cpu_peripherals,
                        );
                        s.subcycle = 2;
                    }
                    2 => {
                        s.temp2 = s.memory_cycle_read(
                            |s,v| {},
                            s.pc.wrapping_add(2),
                            bus,
                            cpu_peripherals,
                        );
                        #[cfg(feature = "debugger")]
                        {
                            let temp = (s.temp2 as u16) << 8 | s.temp as u16;
                            s.copy_debugger(format!("*SHX ${:04x},Y", temp));
                            s.done_fetching = true;
                        }
                        s.subcycle = 3;
                    }
                    3 => {
                        let addr =
                            (s.temp2 as u16) << 8 | (s.temp.wrapping_add(s.y) as u16);
                        s.memory_cycle_read(|s,v| {}, addr, bus, cpu_peripherals);
                        s.subcycle = 4;
                    }
                    _ => {
                        let addr = ((s.temp2 as u16).wrapping_add(1) << 8 | (s.temp as u16))
                            .wrapping_add(s.y as u16);
                        let mask = (s.x as u16) << 8 | 0xFF;
                        let val = s.temp2.wrapping_add(1) & s.x;
                        s.memory_cycle_write(addr & mask, val, bus, cpu_peripherals);

                        s.pc = s.pc.wrapping_add(3);
                        s.end_instruction();
                    }
                },
                _ => match s.subcycle {
                    1 => {
                        s.copy_debugger(format!("*JAM ${:02x}", o));
                        s.subcycle = 2;
                    }
                    _ => {}
                },
            }
        }
    }
}
