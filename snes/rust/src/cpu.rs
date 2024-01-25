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
    /// Program bank register
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

    /// Run a single cycle of the cpu
    pub fn cycle(
        &mut self,
        bus: &mut SnesMotherboard,
        cpu_peripherals: &mut SnesCpuPeripherals,
        nmi: bool,
        irq: bool,
    ) {
        if self.reset {

        }
    }
}
