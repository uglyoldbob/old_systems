#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod cartridge;
mod cpu;
mod ppu;

#[cfg(test)]
use std::io::BufRead;

use crate::cartridge::NesCartridge;
use crate::cpu::NesCpu;
use crate::cpu::NesCpuPeripherals;
use crate::cpu::NesMemoryBus;
use crate::ppu::NesPpu;

use egui_glow::EguiGlow;
use egui_multiwin::multi_window::MultiWindow;
use egui_multiwin::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

struct NesMotherboard {
    cart: Option<NesCartridge>,
    ram: [u8; 2048],
}

impl NesMotherboard {
    fn new() -> Self {
        //board ram is random on startup
        let mut main_ram: [u8; 2048] = [0; 2048];
        for i in main_ram.iter_mut() {
            *i = rand::random();
        }
        Self {
            cart: None,
            ram: main_ram,
        }
    }

    fn insert_cartridge(&mut self, c: NesCartridge) {
        if let None = self.cart {
            self.cart = Some(c);
        }
    }
}

impl NesMemoryBus for NesMotherboard {
    fn memory_cycle_read(
        &mut self,
        addr: u16,
        _out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) -> u8 {
        let mut response: u8 = 0;
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                response = self.ram[addr as usize];
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                response = per.ppu_read(addr);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    let resp = cart.memory_read(addr);
                    if let Some(v) = resp {
                        response = v;
                    }
                }
            }
        }
        response
    }
    fn memory_cycle_write(
        &mut self,
        addr: u16,
        data: u8,
        _out: [bool; 3],
        _controllers: [bool; 2],
        per: &mut NesCpuPeripherals,
    ) {
        match addr {
            0..=0x1fff => {
                let addr = addr & 0x7ff;
                self.ram[addr as usize] = data;
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x2000..=0x3fff => {
                let addr = addr & 7;
                //ppu registers
                per.ppu_write(addr, data);
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4000..=0x4017 => {
                //apu and io
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            0x4018..=0x401f => {
                //disabled apu and oi functionality
                //test mode
                if let Some(cart) = &mut self.cart {
                    cart.memory_nop();
                }
            }
            _ => {
                if let Some(cart) = &mut self.cart {
                    cart.memory_write(addr, data);
                }
            }
        }
    }

    fn ppu_cycle_1(&mut self, addr: u16) {
        if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_1(addr);
        }
    }
    fn ppu_cycle_2_write(&mut self, data: u8) {
        if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_write(data);
        }
    }
    fn ppu_cycle_2_read(&mut self) -> u8 {
        if let Some(cart) = &mut self.cart {
            cart.ppu_cycle_read()
        } else {
            42
        }
    }
}

#[test]
fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn check_nes_roms() {
    let mut roms = Vec::new();
    let pb = std::path::PathBuf::from("./");
    let entries = std::fs::read_dir(&pb).unwrap();
    for e in entries.into_iter() {
        if let Ok(e) = e {
            let path = e.path();
            let meta = std::fs::metadata(&path).unwrap();
            if meta.is_file() {
                println!("Element {}", path.display());
                if path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".nes")
                {
                    roms.push(path);
                }
            }
        }
    }
    println!("Checking roms in {}", pb.display());
    for r in roms {
        println!("Testing rom {}", r.display());
        let nc = NesCartridge::load_cartridge(r.into_os_string().into_string().unwrap());
        assert!(
            nc.is_ok(),
            "Unable to load rom because {:?}",
            nc.err().unwrap()
        );
    }
}

#[cfg(test)]
fn convert_hex_to_decimal(d: char) -> u8 {
    match d {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' | 'a' => 10,
        'B' | 'b' => 11,
        'C' | 'c' => 12,
        'D' | 'd' => 13,
        'E' | 'e' => 14,
        'F' | 'f' => 15,
        _ => 0,
    }
}

#[test]
fn basic_cpu_test() {
    let mut cpu: NesCpu = NesCpu::new();
    let ppu: NesPpu = NesPpu::new();
    let mut cpu_peripherals: NesCpuPeripherals = NesCpuPeripherals::new(ppu);
    let mut mb: NesMotherboard = NesMotherboard::new();
    let nc = NesCartridge::load_cartridge("./nestest.nes".to_string());
    let goldenlog = std::fs::File::open("./nestest.log").unwrap();
    let mut goldenlog = std::io::BufReader::new(goldenlog).lines();
    let mut log_line = 0;

    let mut nc = nc.unwrap();
    nc.rom_byte_hack(0xfffc, 0x00);
    mb.insert_cartridge(nc);

    let mut t: String;
    let mut b;
    for i in 0..26554 {
        cpu.cycle(&mut mb, &mut cpu_peripherals);
        if cpu.instruction_start() {
            log_line += 1;
            t = goldenlog.next().unwrap().unwrap();
            println!("Instruction end at cycle {}", i + 1);
            println!("NESTEST LOG LINE {}: {}", log_line, t);
            b = t.as_bytes();
            let d = convert_hex_to_decimal(b[0] as char) as u16;
            let d2 = convert_hex_to_decimal(b[1] as char) as u16;
            let d3 = convert_hex_to_decimal(b[2] as char) as u16;
            let d4 = convert_hex_to_decimal(b[3] as char) as u16;
            let address = d << 12 | d2 << 8 | d3 << 4 | d4;

            let reg_a: u8 = (convert_hex_to_decimal(b[50] as char) as u8) << 4
                | convert_hex_to_decimal(b[51] as char) as u8;
            assert_eq!(cpu.get_a(), reg_a);

            let reg_x: u8 = (convert_hex_to_decimal(b[55] as char) as u8) << 4
                | convert_hex_to_decimal(b[56] as char) as u8;
            assert_eq!(cpu.get_x(), reg_x);

            let reg_y: u8 = (convert_hex_to_decimal(b[60] as char) as u8) << 4
                | convert_hex_to_decimal(b[61] as char) as u8;
            assert_eq!(cpu.get_y(), reg_y);

            let reg_p: u8 = (convert_hex_to_decimal(b[65] as char) as u8) << 4
                | convert_hex_to_decimal(b[66] as char) as u8;
            assert_eq!(cpu.get_p(), reg_p);

            let reg_sp: u8 = (convert_hex_to_decimal(b[71] as char) as u8) << 4
                | convert_hex_to_decimal(b[72] as char) as u8;
            assert_eq!(cpu.get_sp(), reg_sp);

            println!("Address is {:x} {:x}", address, cpu.get_pc());
            assert_eq!(cpu.get_pc(), address);
            println!("");

            let mut logcycle: u32 = 0;
            for i in 90..95 {
                if i < b.len() {
                    logcycle *= 10;
                    logcycle += convert_hex_to_decimal(b[i] as char) as u32;
                }
            }
            assert_eq!(i + 1, logcycle);
        }
    }
    assert_eq!(cpu.get_pc(), 0xc66e);
}

struct NesEmulatorData {
    cpu: NesCpu,
    cpu_peripherals: NesCpuPeripherals,
    mb: NesMotherboard,
    cpu_clock_counter: u8,
    ppu_clock_counter: u8,
    #[cfg(debug_assertions)]
    paused: bool,
    #[cfg(debug_assertions)]
    single_step: bool,
}

impl NesEmulatorData {
    fn new() -> Self {
        let mut mb: NesMotherboard = NesMotherboard::new();
        let nc = NesCartridge::load_cartridge("./nes/rust/cpu.nes".to_string());

        let nc = nc.unwrap();
        mb.insert_cartridge(nc);
        let ppu = NesPpu::new();
        Self {
            cpu: NesCpu::new(),
            cpu_peripherals: NesCpuPeripherals::new(ppu),
            mb: mb,
            #[cfg(debug_assertions)]
            paused: true,
            #[cfg(debug_assertions)]
            single_step: false,
            cpu_clock_counter: rand::random::<u8>() % 16,
            ppu_clock_counter: rand::random::<u8>() % 4,
        }
    }

    pub fn cycle_step(&mut self) {
        self.cpu_clock_counter += 1;
        if self.cpu_clock_counter >= 12 {
            self.cpu_clock_counter = 0;
            self.cpu.cycle(&mut self.mb, &mut self.cpu_peripherals);
        }

        self.ppu_clock_counter += 1;
        if self.ppu_clock_counter >= 4 {
            self.ppu_clock_counter = 0;
            self.cpu_peripherals.ppu_cycle(&mut self.mb);
        }
    }
}

struct MainNesWindow {}

impl MainNesWindow {
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(MainNesWindow {}),
            builder: glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(glutin::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Emulator"),
        }
    }
}

impl TrackedWindow for MainNesWindow {
    type Data = NesEmulatorData;

    fn is_root(&self) -> bool {
        true
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
    ) -> RedrawResponse<Self::Data> {
        egui.egui_ctx.request_repaint();
        let mut quit = false;
        let mut windows_to_create = vec![];

        let mut counter = 0;
        'emulator_loop: loop {
            #[cfg(debug_assertions)]
            {
                counter += 1;
                if !c.paused {
                    c.cycle_step();
                    if c.single_step && c.cpu_clock_counter == 0 && c.cpu.breakpoint_option() {
                        c.paused = true;
                        break 'emulator_loop;
                    }
                }
                else {
                    break 'emulator_loop;
                }
                if counter == 1000 {
                    counter = 0;
                    break 'emulator_loop;
                }
            }
            #[cfg(not(debug_assertions))]
            {
                c.cycle_step();
                break 'emulator_loop;
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(&egui.egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let button = egui::Button::new("Open rom?");
                    if ui.add_enabled(true, button).clicked() {}
                });
            });
        });

        egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {});
        RedrawResponse {
            quit: quit,
            new_windows: windows_to_create,
        }
    }
}

struct DebugNesWindow {}

impl DebugNesWindow {
    fn new() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(DebugNesWindow {}),
            builder: glutin::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(glutin::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES Debug"),
        }
    }
}

impl TrackedWindow for DebugNesWindow {
    type Data = NesEmulatorData;

    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
    ) -> RedrawResponse<Self::Data> {
        egui.egui_ctx.request_repaint();
        let mut quit = false;
        let mut windows_to_create = vec![];

        egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("Debug window");
            #[cfg(debug_assertions)]
            {
                if c.paused {
                    if ui.button("Unpause").clicked() {
                        c.paused = false;
                        c.single_step = false;
                    }
                    if ui.button("Single step").clicked() {
                        c.single_step = true;
                        c.paused = false;
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(format!("Address: 0x{:x}", c.cpu.get_pc()));
                    if let Some(t) = c.cpu.disassemble() {
                        ui.label(t);
                    }
                });
            }
        });
        RedrawResponse {
            quit: quit,
            new_windows: windows_to_create,
        }
    }
}

fn main() {
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let mut multi_window = MultiWindow::new();
    let root_window = MainNesWindow::new();
    let nes_data = NesEmulatorData::new();
    let _e = multi_window.add(root_window, &event_loop);
    if cfg!(debug_assertions) {
        let debug_win = DebugNesWindow::new();
        let _e = multi_window.add(debug_win, &event_loop);
    }
    multi_window.run(event_loop, nes_data);
}
