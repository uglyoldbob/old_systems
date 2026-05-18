use eframe::{NativeOptions, egui::{self, Event, TouchPhase, Sense, Align2, FontId, vec2, Rect}};
use std::collections::HashMap;

#[cfg(target_os = "android")]
use egui_winit::winit;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::Renderer;
    unsafe { std::env::set_var("RUST_BACKTRACE", "full"); }
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

#[derive(Default)]
pub struct DemoApp {
    active_touches: HashMap<u64, egui::Pos2>,
    rect_a: Option<Rect>,
    rect_b: Option<Rect>,
}

impl DemoApp {
    pub fn run(options: NativeOptions) -> Result<(), eframe::Error> {
        eframe::run_native(
            "egui-android-demo",
            options,
            Box::new(|_cc| Ok(Box::<DemoApp>::default())),
        )
    }

    fn touch_hits(&self, rect: Rect) -> bool {
        self.active_touches.values().any(|&pos| rect.contains(pos))
    }

    fn paint_button(&self, ui: &egui::Ui, rect: Rect, label: &str, pressed: bool) {
        let visuals = ui.visuals();
        let (fill, stroke) = if pressed {
            (visuals.selection.bg_fill, visuals.widgets.active.bg_stroke)
        } else {
            (visuals.widgets.inactive.bg_fill, visuals.widgets.inactive.bg_stroke)
        };
        ui.painter().rect(rect, 12.0, fill, stroke, egui::StrokeKind::Inside);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(32.0),
            visuals.widgets.inactive.fg_stroke.color,
        );
    }
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Update touch state
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

        // Determine pressed state using rects from the previous frame
        let a_pressed = self.rect_a.map_or(false, |r| self.touch_hits(r));
        let b_pressed = self.rect_b.map_or(false, |r| self.touch_hits(r));

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("NES Controller test app");
            ui.add_space(16.0);

            ui.horizontal(|ui| {
                let (rect_a, _) = ui.allocate_exact_size(vec2(70.0, 70.0), Sense::hover());
                ui.add_space(12.0);
                let (rect_b, _) = ui.allocate_exact_size(vec2(70.0, 70.0), Sense::hover());

                // Store rects for next frame's hit testing
                self.rect_a = Some(rect_a);
                self.rect_b = Some(rect_b);

                self.paint_button(ui, rect_a, "A", a_pressed);
                self.paint_button(ui, rect_b, "B", b_pressed);
            });

            ui.add_space(8.0);

            if a_pressed { ui.label("A is held"); }
            if b_pressed { ui.label("B is held"); }
        });

        if !self.active_touches.is_empty() {
            ui.ctx().request_repaint();
        }
    }
}