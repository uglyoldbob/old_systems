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
            bus.memory_cycle_read(0, [false; 3], [true; 2]);
        }
    }
}
