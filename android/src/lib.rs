use byteorder::NetworkEndian;
use eframe::{
    NativeOptions,
    egui::{
        self, Align2, Color32, Event, FontId, Pos2, Rect, Rounding, Sense, TouchPhase, Vec2, vec2,
    },
};
use std::{collections::HashMap, io::Write, sync::atomic::AtomicU16, thread::JoinHandle};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg(target_os = "android")]
use egui_winit::winit;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::Renderer;
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "full");
    }
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    let options = NativeOptions {
        android_app: Some(app),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };
    DemoApp::run(options).unwrap();
}

#[derive(Default, Clone, Copy)]
struct DpadRects {
    up: Option<Rect>,
    down: Option<Rect>,
    left: Option<Rect>,
    right: Option<Rect>,
    up_left: Option<Rect>,
    up_right: Option<Rect>,
    down_left: Option<Rect>,
    down_right: Option<Rect>,
}

#[derive(Default, Clone, Copy)]
struct ButtonRects {
    dpad: DpadRects,
    select: Option<Rect>,
    start: Option<Rect>,
    config: Option<Rect>,
    b: Option<Rect>,
    a: Option<Rect>,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Page {
    #[default]
    Controller,
    Config,
}

struct EmulatorHandlerRunner {
    a: JoinHandle<()>,
    data: Arc<AtomicU16>,
    done: Arc<AtomicBool>,
    player: Arc<AtomicU16>,
}

impl Drop for EmulatorHandlerRunner {
    fn drop(&mut self) {
        self.set_done();
    }
}

impl EmulatorHandlerRunner {
    pub fn run(mut e: EmulatorHandler) -> Self {
        let data = e.data.clone();
        let done = e.done.clone();
        let player = e.player.clone();
        let a = std::thread::spawn(move || {
            loop {
                if e.done.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                if e.send().is_err() {
                    e.done.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
            }
            log::error!("Controller handler ended");
        });
        Self {
            a,
            data,
            done,
            player,
        }
    }

    fn set_done(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_done(&self) -> bool {
        self.done.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn send_controller_data(&mut self, data: u16) -> Result<(), std::io::Error> {
        self.data.store(data, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn get_player_num(&self) -> Option<u8> {
        let v = self.player.load(std::sync::atomic::Ordering::Relaxed);
        if v == 0xffff {
            None
        } else {
            Some(v as u8)
        }
    }
}

struct EmulatorHandler {
    stream: bluetooth_rust::BluetoothSocket,
    data: Arc<AtomicU16>,
    done: Arc<AtomicBool>,
    player: Arc<AtomicU16>,
}

impl EmulatorHandler {
    fn new(mut stream: bluetooth_rust::BluetoothSocket) -> Self {
        let mut s = Self {
            stream,
            data: Arc::new(AtomicU16::new(0)),
            done: Arc::new(AtomicBool::new(false)),
            player: Arc::new(AtomicU16::new(0xffff)),
        };
        s
    }

    fn receive_packet(&mut self) -> Result<(), std::io::Error> {
        use byteorder::BigEndian;
        use byteorder::ReadBytesExt;
        use byteorder::WriteBytesExt;
        use std::io::Read;
        use std::io::Write;
        log::error!("Reading packet length");
        let mut packet_buf = [0u8; 256];
        log::error!("About to receive a packet");
        let packet_len = self.stream.read_u16::<BigEndian>()?;
        if packet_len as usize > packet_buf.len() {
            log::error!("Received a bad packet of length {packet_len:x}");
            return Err(std::io::Error::other(format!(
                "Received a packet that was too long {packet_len:x}"
            )));
        }
        log::error!("Got packet length 0x{:x}", packet_len);

        self.stream
            .read_exact(&mut packet_buf[..packet_len as usize])?;

        log::error!(
            "Pakcet contents {:02x?}",
            &packet_buf[..packet_len as usize]
        );
        let packet: controller::ControllerReceive =
            bincode::deserialize(&packet_buf[..packet_len as usize])
                .map_err(|e| std::io::Error::other(e))?;
        log::error!("Got packet {:x?}", packet);
        match packet {
            controller::ControllerReceive::PlayerNumber(i) => {
                log::error!("I am player {:?}", i);
                if let Some(a) = i {
                    self.player.store(a as u16, std::sync::atomic::Ordering::Relaxed);
                } else {
                    self.player.store(0xffff, std::sync::atomic::Ordering::Relaxed);
                }
            }
            controller::ControllerReceive::AcknowledgeButtonData => {
                log::error!("Button presses were received");
            }
        }
        Ok(())
    }

    fn get_player_num(&mut self) -> Result<(), std::io::Error> {
        use byteorder::BigEndian;
        use byteorder::ReadBytesExt;
        use byteorder::WriteBytesExt;
        use std::io::Read;
        use std::io::Write;
        if self.player.load(std::sync::atomic::Ordering::Relaxed) == 0xffff {
            let d = bincode::serialize(&controller::ControllerSend::GetPlayerNumber)
                .map_err(|e| std::io::Error::other(e))?;
            log::error!("About to write request to get player number");
            self.stream.write_u16::<BigEndian>(d.len() as u16)?;
            self.stream.write_all(&d)?;
            self.stream.flush()?;
            log::error!("Done with write request to get player number");
            self.receive_packet()?;
        }
        Ok(())
    }

    fn send(&mut self) -> Result<(), std::io::Error> {
        use byteorder::BigEndian;
        use byteorder::ReadBytesExt;
        use byteorder::WriteBytesExt;
        use std::io::Read;
        use std::io::Write;
        self.get_player_num();
        if self.player.load(std::sync::atomic::Ordering::Relaxed) != 0xffff {
            let d = bincode::serialize(&controller::ControllerSend::ButtonData(
                self.data.load(std::sync::atomic::Ordering::Relaxed),
            ))
            .map_err(|e| std::io::Error::other(e))?;
            self.stream.write_u16::<BigEndian>(d.len() as u16)?;
            self.stream.write_all(&d)?;
            self.stream.flush()?;
            self.receive_packet()?;
        }
        Ok(())
    }
}

pub struct DemoApp {
    active_touches: HashMap<u64, egui::Pos2>,
    rects: ButtonRects,
    page: Page,
    // Config state — add your own fields here
    show_debug_strip: bool,
    button_opacity: f32,
    bluetooth_adapter: bluetooth_rust::BluetoothAdapter,
    bluetooth_emulators: Vec<bluetooth_rust::BluetoothDevice>,
    emulator_socket: Option<EmulatorHandlerRunner>,

    up_p: bool,
    down_p: bool,
    left_p: bool,
    right_p: bool,
    select_p: bool,
    start_p: bool,
    b_p: bool,
    a_p: bool,
}

impl DemoApp {
    fn new(bluetooth_adapter: bluetooth_rust::BluetoothAdapter) -> Self {
        Self {
            active_touches: HashMap::new(),
            rects: ButtonRects::default(),
            page: Page::default(),
            show_debug_strip: true,
            button_opacity: 1.0,
            bluetooth_adapter,
            bluetooth_emulators: Vec::new(),
            emulator_socket: None,
            up_p: false,
            down_p: false,
            left_p: false,
            right_p: false,
            select_p: false,
            start_p: false,
            b_p: false,
            a_p: false,
        }
    }
}

impl DemoApp {
    pub fn run(options: NativeOptions) -> Result<(), eframe::Error> {
        let mut b = bluetooth_rust::BluetoothAdapterBuilder::new();
        #[cfg(target_os = "android")]
        b.with_android_app(options.android_app.as_ref().unwrap().clone());
        #[cfg(target_os = "android")]
        android::request_bluetooth_connect(options.android_app.as_ref().unwrap());
        let b = b.build().expect("Failed to connect to bluetooth");
        let da = DemoApp::new(b);
        eframe::run_native(
            "NES Controller",
            options,
            Box::new(|_cc| Ok(Box::<DemoApp>::new(da))),
        )
    }

    fn touch_hits(&self, rect: Option<Rect>) -> bool {
        rect.map_or(false, |r| {
            self.active_touches.values().any(|&pos| r.contains(pos))
        })
    }

    fn btn_colors(&self, pressed: bool) -> (Color32, egui::Stroke) {
        let alpha = (self.button_opacity * 255.0) as u8;
        if pressed {
            (
                Color32::from_rgba_unmultiplied(220, 80, 80, alpha),
                egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(60, 60, 70, alpha),
                egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 100, 120, alpha)),
            )
        }
    }

    fn paint_btn(&self, ui: &egui::Ui, rect: Rect, label: &str, pressed: bool, round: f32) {
        let (fill, stroke) = self.btn_colors(pressed);
        ui.painter().rect(
            rect,
            Rounding::same(round as u8),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        if !label.is_empty() {
            let alpha = (self.button_opacity * 255.0) as u8;
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(rect.height() * 0.38),
                Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
            );
        }
    }

    fn paint_cardinal(&self, ui: &egui::Ui, rect: Rect, pressed: bool, round: f32, dir: Dir) {
        let (fill, stroke) = self.btn_colors(pressed);
        ui.painter().rect(
            rect,
            Rounding::same(round as u8),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );

        let c = rect.center();
        let s = rect.size().min_elem() * 0.28;
        let alpha = (self.button_opacity * 255.0) as u8;
        let arrow_color = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);

        let tri: [Pos2; 3] = match dir {
            Dir::Up => [c + vec2(-s, s), c + vec2(s, s), c + vec2(0.0, -s)],
            Dir::Down => [c + vec2(-s, -s), c + vec2(s, -s), c + vec2(0.0, s)],
            Dir::Left => [c + vec2(s, -s), c + vec2(s, s), c + vec2(-s, 0.0)],
            Dir::Right => [c + vec2(-s, -s), c + vec2(-s, s), c + vec2(s, 0.0)],
        };

        ui.painter().add(egui::Shape::convex_polygon(
            tri.to_vec(),
            arrow_color,
            egui::Stroke::NONE,
        ));
    }

    fn paint_diagonal(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        pressed: bool,
        round: f32,
        vdir: Dir,
        hdir: Dir,
    ) {
        let (fill, stroke) = self.btn_colors(pressed);
        ui.painter().rect(
            rect,
            Rounding::same(round as u8),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );

        let c = rect.center();
        let s = rect.size().min_elem() * 0.22;
        let alpha = (self.button_opacity * 255.0) as u8;
        let arrow_color = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);

        let v_offset = match vdir {
            Dir::Up => vec2(0.0, -s * 0.6),
            Dir::Down => vec2(0.0, s * 0.6),
            _ => vec2(0.0, 0.0),
        };
        let h_offset = match hdir {
            Dir::Left => vec2(-s * 0.6, 0.0),
            Dir::Right => vec2(s * 0.6, 0.0),
            _ => vec2(0.0, 0.0),
        };

        let vc = c + v_offset + h_offset * 0.3;
        let vt: [Pos2; 3] = match vdir {
            Dir::Up => [vc + vec2(-s, s), vc + vec2(s, s), vc + vec2(0.0, -s)],
            _ => [vc + vec2(-s, -s), vc + vec2(s, -s), vc + vec2(0.0, s)],
        };

        let hc = c + h_offset + v_offset * 0.3;
        let ht: [Pos2; 3] = match hdir {
            Dir::Left => [hc + vec2(s, -s), hc + vec2(s, s), hc + vec2(-s, 0.0)],
            _ => [hc + vec2(-s, -s), hc + vec2(-s, s), hc + vec2(s, 0.0)],
        };

        for tri in [vt, ht] {
            ui.painter().add(egui::Shape::convex_polygon(
                tri.to_vec(),
                arrow_color,
                egui::Stroke::NONE,
            ));
        }
    }

    // ── Pages ──────────────────────────────────────────────────────────────

    fn show_controller(&mut self, ui: &mut egui::Ui) {
        let panel_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            panel_rect,
            Rounding::same(24),
            Color32::from_rgb(30, 30, 38),
        );

        let origin = panel_rect.min;
        let w = panel_rect.width();
        let h = panel_rect.height();

        // ── D-pad ──────────────────────────────────────────────────────────
        let cell = (w * 0.10).min(h * 0.16);
        let sz = vec2(cell, cell);
        let cx = origin.x + w * 0.22;
        let cy = origin.y + h * 0.55;

        let up_rect = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy - cell * 1.5), sz);
        let down_rect = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy + cell * 0.5), sz);
        let left_rect = Rect::from_min_size(egui::pos2(cx - cell * 1.5, cy - cell * 0.5), sz);
        let right_rect = Rect::from_min_size(egui::pos2(cx + cell * 0.5, cy - cell * 0.5), sz);

        let diag_inset = cell * 0.08;
        let diag_sz = vec2(cell - diag_inset * 2.0, cell - diag_inset * 2.0);

        let ul_rect = Rect::from_min_size(
            egui::pos2(cx - cell * 1.5 + diag_inset, cy - cell * 1.5 + diag_inset),
            diag_sz,
        );
        let ur_rect = Rect::from_min_size(
            egui::pos2(cx + cell * 0.5 + diag_inset, cy - cell * 1.5 + diag_inset),
            diag_sz,
        );
        let dl_rect = Rect::from_min_size(
            egui::pos2(cx - cell * 1.5 + diag_inset, cy + cell * 0.5 + diag_inset),
            diag_sz,
        );
        let dr_rect = Rect::from_min_size(
            egui::pos2(cx + cell * 0.5 + diag_inset, cy + cell * 0.5 + diag_inset),
            diag_sz,
        );

        // ── Select / Start / Config ────────────────────────────────────────
        let pill_w = w * 0.10;
        let pill_h = h * 0.055;
        let pill_y = origin.y + h * 0.55; // vertically centred
        let cfg_y = origin.y + h * 0.38; // config button sits above

        let sel_rect = Rect::from_min_size(
            egui::pos2(origin.x + w * 0.36, pill_y),
            vec2(pill_w, pill_h),
        );
        let sta_rect = Rect::from_min_size(
            egui::pos2(origin.x + w * 0.52, pill_y),
            vec2(pill_w, pill_h),
        );

        // Config button — small gear icon pill centred between select and start
        let cfg_w = pill_w * 0.7;
        let cfg_rect = Rect::from_min_size(
            egui::pos2(origin.x + w * 0.44 + pill_w * 0.15, cfg_y),
            vec2(cfg_w, pill_h),
        );

        // ── A / B ──────────────────────────────────────────────────────────
        let ab_r = (w * 0.08).min(h * 0.13);
        let ab_sz = vec2(ab_r * 2.0, ab_r * 2.0);
        let ab_cx = origin.x + w * 0.78;
        let ab_cy = origin.y + h * 0.55;
        let b_rect = Rect::from_min_size(egui::pos2(ab_cx - ab_r * 2.4, ab_cy - ab_r), ab_sz);
        let a_rect = Rect::from_min_size(egui::pos2(ab_cx + ab_r * 0.4, ab_cy - ab_r), ab_sz);

        // ── Read pressed state (previous frame rects) ──────────────────────
        let d = self.rects.dpad;
        let hit_ul = self.touch_hits(d.up_left);
        let hit_ur = self.touch_hits(d.up_right);
        let hit_dl = self.touch_hits(d.down_left);
        let hit_dr = self.touch_hits(d.down_right);

        self.up_p = self.touch_hits(d.up) || hit_ul || hit_ur;
        self.down_p = self.touch_hits(d.down) || hit_dl || hit_dr;
        self.left_p = self.touch_hits(d.left) || hit_ul || hit_dl;
        self.right_p = self.touch_hits(d.right) || hit_ur || hit_dr;

        self.select_p = self.touch_hits(self.rects.select);
        self.start_p = self.touch_hits(self.rects.start);
        self.b_p = self.touch_hits(self.rects.b);
        self.a_p = self.touch_hits(self.rects.a);

        // Config button tap — navigate on release (touch ended this frame)
        // We detect this by checking if the rect was hit last frame but has
        // no active touch this frame. Simplest: just treat it as a tap on
        // any active touch inside the rect right now, gated so it only fires once.
        let config_hit = self.touch_hits(self.rects.config);
        if config_hit {
            self.page = Page::Config;
        }

        // ── Store rects ────────────────────────────────────────────────────
        self.rects = ButtonRects {
            dpad: DpadRects {
                up: Some(up_rect),
                down: Some(down_rect),
                left: Some(left_rect),
                right: Some(right_rect),
                up_left: Some(ul_rect),
                up_right: Some(ur_rect),
                down_left: Some(dl_rect),
                down_right: Some(dr_rect),
            },
            select: Some(sel_rect),
            start: Some(sta_rect),
            config: Some(cfg_rect),
            b: Some(b_rect),
            a: Some(a_rect),
        };

        // ── Paint ──────────────────────────────────────────────────────────
        self.paint_diagonal(ui, ul_rect, hit_ul, cell * 0.45, Dir::Up, Dir::Left);
        self.paint_diagonal(ui, ur_rect, hit_ur, cell * 0.45, Dir::Up, Dir::Right);
        self.paint_diagonal(ui, dl_rect, hit_dl, cell * 0.45, Dir::Down, Dir::Left);
        self.paint_diagonal(ui, dr_rect, hit_dr, cell * 0.45, Dir::Down, Dir::Right);

        self.paint_cardinal(ui, up_rect, self.up_p, 4.0, Dir::Up);
        self.paint_cardinal(ui, down_rect, self.down_p, 4.0, Dir::Down);
        self.paint_cardinal(ui, left_rect, self.left_p, 4.0, Dir::Left);
        self.paint_cardinal(ui, right_rect, self.right_p, 4.0, Dir::Right);

        // Centre fill
        let centre = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy - cell * 0.5), sz);
        ui.painter()
            .rect_filled(centre, Rounding::ZERO, Color32::from_rgb(60, 60, 70));

        self.paint_btn(ui, sel_rect, "SELECT", self.select_p, pill_h * 0.5);
        self.paint_btn(ui, sta_rect, "START", self.start_p, pill_h * 0.5);

        // Config button — gear symbol, dimmer than action buttons
        let cfg_alpha = (self.button_opacity * 180.0) as u8;
        ui.painter().rect(
            cfg_rect,
            Rounding::same((pill_h * 0.5) as u8),
            Color32::from_rgba_unmultiplied(50, 50, 60, cfg_alpha),
            egui::Stroke::new(
                1.5,
                Color32::from_rgba_unmultiplied(120, 120, 140, cfg_alpha),
            ),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            cfg_rect.center(),
            Align2::CENTER_CENTER,
            "CFG",
            FontId::proportional(cfg_rect.height() * 0.45),
            Color32::from_rgba_unmultiplied(160, 160, 180, cfg_alpha),
        );

        let player_rect = Rect::from_min_size(
            egui::pos2(origin.x + w * 0.44 + pill_w * 0.15, cfg_y - cfg_rect.height() * 1.5),
            vec2(cfg_w, pill_h),
        );

        ui.painter().text(
            player_rect.center(),
            Align2::CENTER_CENTER,
            if let Some(es) = &self.emulator_socket {
                if let Some(pn) = es.get_player_num() {
                    format!("Connected as player {}", pn + 1)
                } else {
                    "Waiting on emulator".to_string()
                }
            } else {
                "Not connected".to_string()
            },
            FontId::proportional(14.0),
            Color32::from_rgb(180, 180, 200),
        );

        self.paint_btn(ui, b_rect, "B", self.b_p, ab_r);
        self.paint_btn(ui, a_rect, "A", self.a_p, ab_r);

        // ── Debug strip ────────────────────────────────────────────────────
        if self.show_debug_strip {
            let pressed: Vec<&str> = [
                (self.up_p, "Up"),
                (self.down_p, "Down"),
                (self.left_p, "Left"),
                (self.right_p, "Right"),
                (self.select_p, "Select"),
                (self.start_p, "Start"),
                (self.b_p, "B"),
                (self.a_p, "A"),
            ]
            .iter()
            .filter_map(|&(p, n)| if p { Some(n) } else { None })
            .collect();

            ui.painter().text(
                egui::pos2(origin.x + w * 0.5, origin.y + h * 0.92),
                Align2::CENTER_CENTER,
                if pressed.is_empty() {
                    "—".into()
                } else {
                    pressed.join(" + ")
                },
                FontId::proportional(14.0),
                Color32::from_rgb(180, 180, 200),
            );
        }

        ui.allocate_rect(panel_rect, Sense::hover());
    }

    fn show_config(&mut self, ui: &mut egui::Ui) {
        self.up_p = false;
        self.down_p = false;
        self.left_p = false;
        self.right_p = false;
        self.select_p = false;
        self.start_p = false;
        self.a_p = false;
        self.b_p = false;
        let panel_rect = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(panel_rect, Rounding::ZERO, Color32::from_rgb(22, 22, 30));

        // Use a normal egui layout inside a padded inner rect
        let padding = 24.0;
        let inner = panel_rect.shrink(padding);
        ui.allocate_rect(panel_rect, Sense::hover());

        // Child UI pinned to the inner rect
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );

        child.heading("Settings");
        child.add_space(16.0);

        // ── Back button ───────────────────────────────────────────────────
        if child.button("  Back to Controller  ").clicked() {
            self.page = Page::Controller;
        }

        child.add_space(12.0);
        child.separator();
        child.add_space(8.0);

        // ── Settings ──────────────────────────────────────────────────────
        child.checkbox(&mut self.show_debug_strip, "Show debug strip");

        child.add_space(6.0);
        child.label("Button opacity");
        child.add(egui::Slider::new(&mut self.button_opacity, 0.2..=1.0).show_value(true));

        if child
            .add(egui::Button::new("Find emulator").min_size([70.0, 70.0].into()))
            .clicked()
        {
            self.bluetooth_emulators.clear();
            if let Some(devs) = self.bluetooth_adapter.get_paired_devices() {
                for mut dev in devs {
                    let wanted_uuid = bluetooth_rust::BluetoothUuid::Custom(
                        "76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string(),
                        0,
                    );
                    dev.run_sdp(wanted_uuid);
                    let wanted_uuid = bluetooth_rust::BluetoothUuid::Unknown(
                        "76ecef8b-24d4-4f7c-9de0-706864b6bc14".to_string(),
                    );
                    if let Ok(uuids) = dev.get_uuids() {
                        log::error!("UUIDS ARE {:?}", uuids);
                        if uuids.contains(&wanted_uuid) {
                            child.label(format!("{:?}", dev.get_address()));
                            log::error!("Found bluetooth emulator {:?}", dev.get_address());
                            self.bluetooth_emulators.push(dev);
                        } else {
                            child.label(format!("NOT {:?}", dev.get_address()));
                            log::error!("No bluetooth emulator {:?}", dev.get_address());
                        }
                    }
                }
            }
        }
        for d in &mut self.bluetooth_emulators {
            if let Ok(bluetooth_rust::PairingStatus::Paired) = d.get_pair_state() {
                if let Ok(a) = d.get_address() {
                    if self.emulator_socket.is_none() {
                        let btn = egui::Button::new(&format!("Connect to {}", a))
                            .min_size([70.0, 70.0].into());
                        if child.add(btn).clicked() {
                            log::error!("Need to connect to {}", a);

                            match d.get_rfcomm_socket(
                                23,
                                bluetooth_rust::BluetoothUuid::Custom(
                                    "76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string(),
                                    0,
                                ),
                                false,
                            ) {
                                Ok(mut socket) => {
                                    if socket.sync_connect().is_ok() {
                                        log::error!("Got connection to emulator");
                                        let eh = EmulatorHandler::new(socket);
                                        let eh = EmulatorHandlerRunner::run(eh);
                                        self.emulator_socket = Some(eh);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Error connecting to emulator {}", e);
                                }
                            }
                        }
                    } else {
                        let btn = egui::Button::new("Disconnect from emulator")
                            .min_size([70.0, 70.0].into());
                        if child.add(btn).clicked() {
                            self.emulator_socket.take();
                        }
                    }
                }
            }
        }
    }
}

enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Track touches on the controller page only; clear on config page
        // so no ghost hits fire while the user is in settings.
        if self.page == Page::Controller {
            ui.input(|i| {
                for event in &i.events {
                    if let Event::Touch { id, phase, pos, .. } = event {
                        match phase {
                            TouchPhase::Start | TouchPhase::Move => {
                                self.active_touches.insert(id.0, *pos);
                            }
                            TouchPhase::End | TouchPhase::Cancel => {
                                self.active_touches.remove(&id.0);
                            }
                        }
                    }
                }
            });
        } else {
            self.active_touches.clear();
        }

        let mut end_socket = false;
        if let Some(es) = &mut self.emulator_socket {
            ui.ctx().request_repaint();
            if es.is_done() {
                end_socket = true;
            }
            /// The index into the button combination array for button A
            pub const BUTTON_COMBO_A: usize = 0;
            /// The index into the button combination array for turbo A
            pub const BUTTON_COMBO_TURBOA: usize = 1;
            /// The index into the button combination array for turbo B
            pub const BUTTON_COMBO_TURBOB: usize = 2;
            /// The index into the button combination array for button b
            pub const BUTTON_COMBO_B: usize = 3;
            /// The index into the button combination array for button start
            pub const BUTTON_COMBO_START: usize = 4;
            /// The index into the button combination array for button slow
            pub const BUTTON_COMBO_SLOW: usize = 5;
            /// The index into the button combination array for button select
            pub const BUTTON_COMBO_SELECT: usize = 6;
            /// The index into the button combination array for button up
            pub const BUTTON_COMBO_UP: usize = 7;
            /// The index into the button combination array for button down
            pub const BUTTON_COMBO_DOWN: usize = 8;
            /// The index into the button combination array for button left
            pub const BUTTON_COMBO_LEFT: usize = 9;
            /// The index into the button combination array for button right
            pub const BUTTON_COMBO_RIGHT: usize = 10;
            /// The index into the button combination array for fire/trigger
            pub const BUTTON_COMBO_FIRE: usize = 11;
            /// The index into the button combination array for a light sensor
            pub const BUTTON_COMBO_LIGHT: usize = 12;
            /// The index into the button combination array for a potentiometer
            pub const BUTTON_COMBO_POTENTIOMETER: usize = 13;
            /// The extra button for the power pad
            pub const BUTTON_COMBO_POWERPAD: usize = 14;

            let mut data = 0u16;

            if self.up_p {
                data |= 1 << BUTTON_COMBO_UP;
            }
            if self.down_p {
                data |= 1 << BUTTON_COMBO_DOWN;
            }
            if self.left_p {
                data |= 1 << BUTTON_COMBO_LEFT;
            }
            if self.right_p {
                data |= 1 << BUTTON_COMBO_RIGHT;
            }
            if self.select_p {
                data |= 1 << BUTTON_COMBO_SELECT;
            }
            if self.start_p {
                data |= 1 << BUTTON_COMBO_START;
            }
            if self.a_p {
                data |= 1 << BUTTON_COMBO_A;
            }
            if self.b_p {
                data |= 1 << BUTTON_COMBO_B;
            }

            if let Err(e) = es.send_controller_data(data) {
                log::error!("Error sending controller data: {:?}", e);
            }
        }
        if end_socket {
            self.emulator_socket.take();
        }

        egui::CentralPanel::default().show_inside(ui, |ui| match self.page {
            Page::Controller => self.show_controller(ui),
            Page::Config => self.show_config(ui),
        });

        if !self.active_touches.is_empty() {
            ui.ctx().request_repaint();
        }
    }
}
