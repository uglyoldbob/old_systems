//! The module for dumping all of the cpu address space

use crate::NesEmulatorData;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::{arboard, egui_glow::EguiGlow};

#[cfg(feature = "egui-multiwin")]
use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping cpu data
pub struct CpuMemoryDumpWindow {}

#[cfg(feature = "egui-multiwin")]
impl CpuMemoryDumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::Windows::CpuMemoryDumpWindow(CpuMemoryDumpWindow {}),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 320.0,
                    height: 240.0,
                })
                .with_title("UglyOldBob NES CPU Dump"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow for CpuMemoryDumpWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("CPU Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    for i in (0..=0xFFFF).step_by(8) {
                        let a1 = if let Some(a) = c.mb.memory_dump(i, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a2 = if let Some(a) = c.mb.memory_dump(i + 1, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a3 = if let Some(a) = c.mb.memory_dump(i + 2, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a4 = if let Some(a) = c.mb.memory_dump(i + 3, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a5 = if let Some(a) = c.mb.memory_dump(i + 4, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a6 = if let Some(a) = c.mb.memory_dump(i + 5, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a7 = if let Some(a) = c.mb.memory_dump(i + 6, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        let a8 = if let Some(a) = c.mb.memory_dump(i + 7, &c.cpu_peripherals) {
                            format!("{:02X}", a)
                        } else {
                            "**".to_string()
                        };
                        ui.label(format!(
                            "{:04X}: {} {} {} {}\t{} {} {} {}",
                            i, a1, a2, a3, a4, a5, a6, a7, a8,
                        ));
                    }
                }
            });
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
