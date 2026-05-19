use bluetooth_rust::{BluetoothAdapterTrait, BluetoothDeviceTrait};
use eframe::{
    NativeOptions,
    egui::{
        self, Align2, Color32, Event, FontId, Pos2, Rect, Rounding, Sense, TouchPhase, Vec2, vec2,
    },
};
use std::collections::HashMap;

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

pub struct DemoApp {
    active_touches: HashMap<u64, egui::Pos2>,
    rects: ButtonRects,
    page: Page,
    // Config state — add your own fields here
    show_debug_strip: bool,
    button_opacity: f32,
    bluetooth_adapter: bluetooth_rust::BluetoothAdapter,
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

        let up_p = self.touch_hits(d.up) || hit_ul || hit_ur;
        let down_p = self.touch_hits(d.down) || hit_dl || hit_dr;
        let left_p = self.touch_hits(d.left) || hit_ul || hit_dl;
        let right_p = self.touch_hits(d.right) || hit_ur || hit_dr;

        let select_p = self.touch_hits(self.rects.select);
        let start_p = self.touch_hits(self.rects.start);
        let b_p = self.touch_hits(self.rects.b);
        let a_p = self.touch_hits(self.rects.a);

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

        self.paint_cardinal(ui, up_rect, up_p, 4.0, Dir::Up);
        self.paint_cardinal(ui, down_rect, down_p, 4.0, Dir::Down);
        self.paint_cardinal(ui, left_rect, left_p, 4.0, Dir::Left);
        self.paint_cardinal(ui, right_rect, right_p, 4.0, Dir::Right);

        // Centre fill
        let centre = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy - cell * 0.5), sz);
        ui.painter()
            .rect_filled(centre, Rounding::ZERO, Color32::from_rgb(60, 60, 70));

        self.paint_btn(ui, sel_rect, "SELECT", select_p, pill_h * 0.5);
        self.paint_btn(ui, sta_rect, "START", start_p, pill_h * 0.5);

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

        self.paint_btn(ui, b_rect, "B", b_p, ab_r);
        self.paint_btn(ui, a_rect, "A", a_p, ab_r);

        // ── Debug strip ────────────────────────────────────────────────────
        if self.show_debug_strip {
            let pressed: Vec<&str> = [
                (up_p, "Up"),
                (down_p, "Down"),
                (left_p, "Left"),
                (right_p, "Right"),
                (select_p, "Select"),
                (start_p, "Start"),
                (b_p, "B"),
                (a_p, "A"),
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
            if let Some(devs) = self.bluetooth_adapter.get_paired_devices() {
                for mut dev in devs {
                    if let Ok(uuids) = dev.get_uuids() {
                        if uuids.contains(&bluetooth_rust::BluetoothUuid::Custom("76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string(), 0)) {
                            child.label(format!("{:?}", dev.get_address()));
                            log::error!("Found bluetooth emulator {:?}", dev.get_address());
                        } else {
                            child.label(format!("NOT {:?}", dev.get_address()));
                            log::error!("No bluetooth emulator {:?}", dev.get_address());
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

        egui::CentralPanel::default().show_inside(ui, |ui| match self.page {
            Page::Controller => self.show_controller(ui),
            Page::Config => self.show_config(ui),
        });

        if !self.active_touches.is_empty() {
            ui.ctx().request_repaint();
        }
    }
}
