use crate::cartridge::NesCartridgeData;
use crate::cartridge::NesMapper;

pub struct Mapper {
    shift_register: u8,
    shift_counter: u8,
    shift_locked: bool,
    /// Control, chr bank 0, chr bank 1, prg bank
    registers: [u8; 4],
}

impl Mapper {
    pub fn new(d: &NesCartridgeData) -> Box<dyn NesMapper> {
        Box::new(Self {
            shift_register: 0,
            shift_counter: 0,
            shift_locked: false,
            registers: [0x0c0, 0, 0, 0],
        })
    }

    fn update_register(&mut self, adr: u8, data: u8) {
        println!("Update mmc1 register {} to {:x}", adr, data);
        self.registers[adr as usize] = data;
    }
}

impl NesMapper for Mapper {
    fn memory_cycle_read(&mut self, cart: &mut NesCartridgeData, addr: u16) -> Option<u8> {
        self.shift_locked = false;
        match addr {
            0x6000..=0x7fff => {
                let mut addr2 = addr & 0x1fff;
                if cart.prg_ram.len() != 0 {
                    addr2 = addr2 % cart.prg_ram.len() as u16;
                    println!("Read address {:x} {:x} from cart mapper 1", addr, addr2);
                    Some(cart.prg_ram[addr2 as usize])
                }
                else {
                    None
                }
            }
            0x8000..=0xffff => {
                match (self.registers[0] & 0xC0) >> 2 {
                    0 | 1 => {
                        //32kb bankswitch
                        let addr2 = addr & 0x7fff;
                        let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                        addr3 |= (self.registers[3] as u32 & 0xE) << 16;
                        println!("Read address {:x} {:x} from cart mapper 1", addr, addr3);
                        Some(cart.prg_rom[addr3 as usize])
                    }
                    2 => {
                        //first half fixed, second half switched
                        if addr < 0xc000 {
                            //fixed to first bank
                            let addr2 = addr & 0x3fff;
                            let addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            println!("Read address {:x} {:x} from cart mapper 1", addr, addr3);
                            Some(cart.prg_rom[addr3 as usize])
                        } else {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            addr3 |= (self.registers[3] as u32 & 0xF) << 15;
                            println!("Read address {:x} {:x} from cart mapper 1", addr, addr3);
                            Some(cart.prg_rom[addr3 as usize])
                        }
                    }
                    _ => {
                        //first half switched, second half fixed
                        if addr < 0xc000 {
                            //switched
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32 % cart.prg_rom.len() as u32;
                            addr3 |= (self.registers[3] as u32 & 0xF) << 15;
                            println!("Read address {:x} {:x} from cart mapper 1", addr, addr3);
                            Some(cart.prg_rom[addr3 as usize])
                        } else {
                            //fixed to last bank
                            let addr2 = addr & 0x3fff;
                            let mut addr3 = addr2 as u32;
                            addr3 |= ((cart.prg_rom.len() - 1) & !0x3fff) as u32;
                            println!("Read address {:x} {:x} from cart mapper 1", addr, addr3);
                            Some(cart.prg_rom[addr3 as usize])
                        }
                    }
                }
            }
            _ => None,
        }
    }

    fn memory_cycle_nop(&mut self) {
        self.shift_locked = false;
    }

    fn memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, addr: u16, data: u8) {
        println!("Writing to address {:x} with {:x}", addr, data);
        if addr >= 0x8000 {
            if !self.shift_locked {
                self.shift_locked = true;
                if (data & 0x80) != 0 {
                    self.shift_counter = 0;
                    self.shift_register = 0;
                    self.registers[0] |= 0xC0;
                } else {
                    if self.shift_counter < 4 {
                        self.shift_counter += 1;
                        self.shift_register >>= 1;
                        if (data & 1) != 0 {
                            self.shift_register |= 0x10;
                        }
                    } else {
                        let adr_select = (addr & 0x6000) >> 13;
                        self.update_register(adr_select as u8, self.shift_register);
                        self.shift_counter = 0;
                        self.shift_register = 0;
                    }
                }
            }
        }
    }

    fn ppu_memory_cycle_address(&mut self, addr: u16) {}

    fn ppu_memory_cycle_read(&mut self, _cart: &mut NesCartridgeData) -> Option<u8> {
        None
    }

    fn ppu_memory_cycle_write(&mut self, _cart: &mut NesCartridgeData, _data: u8) {}

    fn rom_byte_hack(&mut self, _cart: &mut NesCartridgeData, _addr: u32, _new_byte: u8) {}
}
