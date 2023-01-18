pub struct NesApu {
    registers: [u8; 24],
}

impl NesApu {
    pub fn new() -> Self {
        Self { registers: [0; 24] }
    }

    pub fn reset(&mut self) {
        self.registers[0x15] = 0;
    }

    pub fn clock_fast(&mut self) {}

    pub fn clock_slow(&mut self) {}

    pub fn write(&mut self, addr: u16, data: u8) {
        let addr2 = addr % 24;
        self.registers[addr2 as usize] = data;
        println!("WRITE APU REGISTER {:x} with {:x}", addr2, data);
    }

    //it is assumed that the only readable address is filtered before making it to this function
    pub fn read(&mut self, _addr: u16) -> u8 {
        self.registers[0x15]
    }
}
