//! This module is responsible for emulating the cpu of the snes.

use crate::apu::SnesApu;
use crate::motherboard::SnesMotherboard;
use crate::ppu::SnesPpu;

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
/// The index register width flag
const CPU_FLAG_INDEX_WIDTH: u8 = 0x10;
/// The b2 flag for the cpu flags register
const CPU_FLAG_B2: u8 = 0x20;
/// The memory width flag for native mode
const CPU_FLAG_MEMORY: u8 = 0x20;
/// The overflow flag for the cpu flags register
const CPU_FLAG_OVERFLOW: u8 = 0x40;
/// The negative flag for the cpu flags register
const CPU_FLAG_NEGATIVE: u8 = 0x80;

/// Describes how many cycles it takes per step of the cpu
#[derive(serde::Serialize, serde::Deserialize)]
enum CpuCycleLength {
    /// The step takes six cycles
    ShortCycle,
    /// The step takes eight cycles
    MediumCycle,
    /// The step takes twelve cycles
    LongCycle,
}

impl CpuCycleLength {
    fn count(&self) -> u8 {
        match self {
            Self::ShortCycle => 6,
            Self::MediumCycle => 8,
            Self::LongCycle => 12,
        }
    }
}

/// The peripherals for the cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpuPeripherals {
    /// The ppu for the nes system
    pub ppu: SnesPpu,
    /// The apu for the nes system
    pub apu: SnesApu,
}

impl SnesCpuPeripherals {
    /// Build a set of cpu peripherals
    pub fn new(ppu: SnesPpu, apu: SnesApu) -> Self {
        Self { ppu, apu }
    }

    /// reset the ppu
    pub fn ppu_reset(&mut self) {}

    /// Run a ppu cycle
    pub fn ppu_cycle(&mut self, mb: &mut SnesMotherboard) {
        self.ppu.cycle(mb);
    }

    /// Return the ppu frame number
    pub fn ppu_frame_number(&self) -> u64 {
        self.ppu.frame_number()
    }

    /// Has the current ppu frame ended?
    pub fn ppu_frame_end(&mut self) -> bool {
        self.ppu.get_frame_end()
    }

    /// Returns a reference to the frame data for the ppu
    pub fn ppu_get_frame(&mut self) -> &crate::ppu::RgbImage {
        self.ppu.get_frame()
    }
}

#[cfg(feature = "debugger")]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
/// Stores the state of the cpu at the debugger point.
/// Single byte instructions make debugging without this weird, because the instruction has already taken effect
/// by the time the debugger is presenting the information
pub struct SnesCpuDebuggerPoint {
    /// The registers
    pub registers: CpuRegisters,
    /// The string that corresponds to the disassembly for the most recently fetched instruction
    pub disassembly: String,
}

impl SnesCpuDebuggerPoint {
    /// Build a new point
    pub fn new() -> Self {
        Self {
            registers: CpuRegisters::new(),
            disassembly: "RESET".to_string(),
        }
    }
}

/// The registers for the cpu
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CpuRegisters {
    /// The accumulator
    a: u16,
    /// index register x
    x: u16,
    /// index register y
    y: u16,
    /// Stack pointer
    sp: u16,
    /// Data bank register 1
    dbr: u8,
    /// Direct register
    db: u16,
    /// Program bank
    pb: u8,
    /// Program bank register (the k register)
    pbr: u8,
    /// Status register
    p: u8,
    /// program counter
    pub pc: u16,
}

impl CpuRegisters {
    /// Construct a new set of registers
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            dbr: 0,
            db: 0,
            pb: 0,
            pbr: 0,
            p: 0,
            pc: 0xfffc,
        }
    }
}

/// A struct for implementing the snes cpu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesCpu {
    /// The registers for the cpu
    registers: CpuRegisters,
    /// A list of breakpoints for the cpu
    #[cfg(feature = "debugger")]
    pub breakpoints: Vec<u16>,
    /// The debugger information
    #[cfg(feature = "debugger")]
    pub debugger: SnesCpuDebuggerPoint,
    /// True when the last byte of an instruction has been fetched
    #[cfg(feature = "debugger")]
    done_fetching: bool,
    /// The portion of an instruction currently being executed
    subcycle: u8,
    /// The reset indicator for the cpu
    reset: bool,
    /// The length of the current tick
    length: CpuCycleLength,
    /// The counter for the length of the current tick
    length_ctr: u8,
    /// The emulation mode flag
    emulation: bool,
    /// The current opcode being executed
    opcode: Option<u8>,
}

impl SnesCpu {
    /// Build a new cpu
    pub fn new() -> Self {
        Self {
            registers: CpuRegisters::new(),
            #[cfg(feature = "debugger")]
            breakpoints: Vec::new(),
            #[cfg(feature = "debugger")]
            debugger: SnesCpuDebuggerPoint::new(),
            #[cfg(feature = "debugger")]
            done_fetching: false,
            subcycle: 0,
            reset: true,
            length: CpuCycleLength::ShortCycle,
            length_ctr: 0,
            emulation: false,
            opcode: None,
        }
    }

    /// Reset the cpu
    pub fn reset(&mut self) {
        self.reset = true;
    }

    /// Returns true when done fetching all bytes for an instruction.
    #[cfg(feature = "debugger")]
    pub fn breakpoint_option(&self) -> bool {
        self.done_fetching
    }

    /// Returns true when a breakpoint is active
    pub fn breakpoint(&self) -> bool {
        let mut b = false;
        if self.done_fetching {
            for v in &self.breakpoints {
                if self.debugger.registers.pc == *v {
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

    /// Return the cycle length for the given address
    fn eval_address(addr: u32) -> CpuCycleLength {
        let bank = (addr>>16) as u8;
        let low_addr = (addr & 0xFFFF) as u16;
        match (bank, low_addr) {
            ((0..=0x3f), (0..=0x1fff)) => CpuCycleLength::MediumCycle,
            ((0..=0x3f), (0x2000..=0x3fff)) => CpuCycleLength::ShortCycle,
            ((0..=0x3f), (0x4000..=0x41ff)) => CpuCycleLength::LongCycle,
            ((0..=0x3f), (0x4200..=0x5fff)) => CpuCycleLength::ShortCycle,
            ((0..=0x3f), (0x6000..=0xffff)) => CpuCycleLength::MediumCycle,
            ((0x40..=0x7f), _) => CpuCycleLength::MediumCycle,
            ((0x80..=0xbf), (0..=0x1fff)) => CpuCycleLength::MediumCycle,
            ((0x80..=0xbf), (0x2000..=0x3fff)) => CpuCycleLength::ShortCycle,
            ((0x80..=0xbf), (0x4000..=0x41ff)) => CpuCycleLength::LongCycle,
            ((0x80..=0xbf), (0x4200..=0x5fff)) => CpuCycleLength::ShortCycle,
            ((0x80..=0xbf), (0x6000..=0x7fff)) => CpuCycleLength::MediumCycle,
            ((0x80..=0xbf), (0x8000..=0xffff)) => CpuCycleLength::ShortCycle, //todo variable
            ((0xc0..=0xff), _) => CpuCycleLength::ShortCycle, //todo variable
        }
    }

    /// Retrieve the full 24 bits of address for pc
    fn get_full_pc(&self) -> u32 {
        let pcl = self.registers.pc as u32;
        let pch = (self.registers.p as u32) << 16;
        pcl | pch
    }

    /// Run a single cycle of the cpu
    pub fn cycle(
        &mut self,
        bus: &mut SnesMotherboard,
        cpu_peripherals: &mut SnesCpuPeripherals,
        nmi: bool,
        irq: bool,
    ) {
        self.length_ctr += 1;
        if self.length_ctr < self.length.count() {
            return;
        }
        self.length_ctr = 0;
        
        if self.reset {
            match self.subcycle {
                0 => {
                    self.emulation = true;
                    self.subcycle += 1;
                }
                _ => {
                    self.reset = false;
                    self.subcycle = 0;
                }
            }
        }
        else if self.opcode.is_none() {
            if self.length_ctr == 0 {
                self.length = Self::eval_address(self.get_full_pc());
            }
        }
        else {

        }
    }
}
