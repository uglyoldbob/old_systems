use eframe::{
    NativeOptions,
    egui::{self, Align2, Color32, Event, FontId, Rect, Rounding, Sense, TouchPhase, vec2},
};
use std::collections::HashMap;

#[cfg(target_os = "android")]
use egui_winit::winit;

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
    b: Option<Rect>,
    a: Option<Rect>,
}

#[derive(Default)]
pub struct DemoApp {
    active_touches: HashMap<u64, egui::Pos2>,
    rects: ButtonRects,
}

impl DemoApp {
    pub fn run(options: NativeOptions) -> Result<(), eframe::Error> {
        eframe::run_native(
            "NES Controller",
            options,
            Box::new(|_cc| Ok(Box::<DemoApp>::default())),
        )
    }

    fn touch_hits(&self, rect: Option<Rect>) -> bool {
        rect.map_or(false, |r| {
            self.active_touches.values().any(|&pos| r.contains(pos))
        })
    }

    fn paint_btn(&self, ui: &egui::Ui, rect: Rect, label: &str, pressed: bool, round: f32) {
        let fill = if pressed {
            Color32::from_rgb(220, 80, 80)
        } else {
            Color32::from_rgb(60, 60, 70)
        };
        let stroke_color = if pressed {
            Color32::WHITE
        } else {
            Color32::from_rgb(100, 100, 120)
        };
        ui.painter().rect(
            rect,
            Rounding::same(round as u8),
            fill,
            egui::Stroke::new(2.0, stroke_color),
            egui::StrokeKind::Inside,
        );
        if !label.is_empty() {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(rect.height() * 0.38),
                Color32::WHITE,
            );
        }
    }
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // --- Update touch tracking ---
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

        // --- Derive pressed state from last frame's rects ---
        let d = self.rects.dpad;
        let hit_up = self.touch_hits(d.up);
        let hit_down = self.touch_hits(d.down);
        let hit_left = self.touch_hits(d.left);
        let hit_right = self.touch_hits(d.right);
        let hit_up_left = self.touch_hits(d.up_left);
        let hit_up_right = self.touch_hits(d.up_right);
        let hit_down_left = self.touch_hits(d.down_left);
        let hit_down_right = self.touch_hits(d.down_right);

        // A diagonal activates the two cardinal directions it spans
        let up_p = hit_up || hit_up_left || hit_up_right;
        let down_p = hit_down || hit_down_left || hit_down_right;
        let left_p = hit_left || hit_up_left || hit_down_left;
        let right_p = hit_right || hit_up_right || hit_down_right;

        let select_p = self.touch_hits(self.rects.select);
        let start_p = self.touch_hits(self.rects.start);
        let b_p = self.touch_hits(self.rects.b);
        let a_p = self.touch_hits(self.rects.a);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let panel_rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                panel_rect,
                Rounding::same(24),
                Color32::from_rgb(30, 30, 38),
            );

            let origin = panel_rect.min;
            let w = panel_rect.width();
            let h = panel_rect.height();

            // ── D-pad geometry ─────────────────────────────────────────────
            //
            //   Each cell is one "unit". The full d-pad occupies a 3×3 grid:
            //
            //   [UL] [ U] [UR]
            //   [ L] [  ] [ R]
            //   [DL] [ D] [DR]
            //
            //   The four corner cells are diagonal buttons (smaller, rounded).
            //   The four edge cells are cardinal buttons.
            //   The centre cell is a cosmetic fill only.

            let cell = (w * 0.10).min(h * 0.16);
            let sz = vec2(cell, cell);

            // Centre of the entire 3×3 grid
            let cx = origin.x + w * 0.22;
            let cy = origin.y + h * 0.55;

            // Cardinal rects (edge cells)
            let up_rect = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy - cell * 1.5), sz);
            let down_rect = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy + cell * 0.5), sz);
            let left_rect = Rect::from_min_size(egui::pos2(cx - cell * 1.5, cy - cell * 0.5), sz);
            let right_rect = Rect::from_min_size(egui::pos2(cx + cell * 0.5, cy - cell * 0.5), sz);

            // Diagonal rects (corner cells) — slightly inset so they don't
            // overlap the cardinal arms visually
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

            // ── Select / Start ─────────────────────────────────────────────
            let pill_w = w * 0.10;
            let pill_h = h * 0.055;
            let pill_y = origin.y + h * 0.52;
            let sel_rect = Rect::from_min_size(
                egui::pos2(origin.x + w * 0.36, pill_y),
                vec2(pill_w, pill_h),
            );
            let sta_rect = Rect::from_min_size(
                egui::pos2(origin.x + w * 0.52, pill_y),
                vec2(pill_w, pill_h),
            );

            // ── A / B ──────────────────────────────────────────────────────
            let ab_r = (w * 0.08).min(h * 0.13);
            let ab_sz = vec2(ab_r * 2.0, ab_r * 2.0);
            let ab_cx = origin.x + w * 0.78;
            let ab_cy = origin.y + h * 0.55;
            let b_rect = Rect::from_min_size(egui::pos2(ab_cx - ab_r * 2.4, ab_cy - ab_r), ab_sz);
            let a_rect = Rect::from_min_size(egui::pos2(ab_cx + ab_r * 0.4, ab_cy - ab_r), ab_sz);

            // ── Store rects for next frame ─────────────────────────────────
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
                b: Some(b_rect),
                a: Some(a_rect),
            };

            // ── Paint ──────────────────────────────────────────────────────

            // Diagonal corners (paint first so cardinals draw on top at edges)
            self.paint_btn(ui, ul_rect, "\\", hit_up_left, cell * 0.45);
            self.paint_btn(ui, ur_rect, "/", hit_up_right, cell * 0.45);
            self.paint_btn(ui, dl_rect, "/", hit_down_left, cell * 0.45);
            self.paint_btn(ui, dr_rect, "\\", hit_down_right, cell * 0.45);

            // Cardinal arms
            self.paint_btn(ui, up_rect, "^", up_p, 4.0);
            self.paint_btn(ui, down_rect, "V", down_p, 4.0);
            self.paint_btn(ui, left_rect, "◀", left_p, 4.0);
            self.paint_btn(ui, right_rect, "▶", right_p, 4.0);

            // Centre cosmetic fill
            let centre_rect = Rect::from_min_size(egui::pos2(cx - cell * 0.5, cy - cell * 0.5), sz);
            ui.painter()
                .rect_filled(centre_rect, Rounding::ZERO, Color32::from_rgb(60, 60, 70));

            // Select / Start
            self.paint_btn(ui, sel_rect, "SELECT", select_p, pill_h * 0.5);
            self.paint_btn(ui, sta_rect, "START", start_p, pill_h * 0.5);

            // A / B
            self.paint_btn(ui, b_rect, "B", b_p, ab_r);
            self.paint_btn(ui, a_rect, "A", a_p, ab_r);

            // ── Debug strip ────────────────────────────────────────────────
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

            ui.allocate_rect(panel_rect, Sense::hover());
        });

        if !self.active_touches.is_empty() {
            ui.ctx().request_repaint();
        }
    }
}
