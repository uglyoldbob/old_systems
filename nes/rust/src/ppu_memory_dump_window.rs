//! The module that allows dumping ppu memory

use crate::NesEmulatorData;
use egui_multiwin::{
    egui_glow::EguiGlow,
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

/// The window for dumping ppu data
#[cfg(feature = "egui-multiwin")]
pub struct PpuMemoryDumpWindow {}

impl PpuMemoryDumpWindow {
    /// Create a request to create a new window of self.
    pub fn new_request() -> NewWindowRequest<NesEmulatorData> {
        NewWindowRequest {
            window_state: Box::new(PpuMemoryDumpWindow {}),
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
        }
    }
}

#[cfg(feature = "egui-multiwin")]
impl TrackedWindow<NesEmulatorData> for PpuMemoryDumpWindow {
    fn is_root(&self) -> bool {
        false
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut NesEmulatorData,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<NesEmulatorData> {
        egui.egui_ctx.request_repaint();
        let quit = false;
        let windows_to_create = vec![];

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.label("PPU Dump Window");
            egui_multiwin::egui::ScrollArea::vertical().show(ui, |ui| {
                #[cfg(feature = "debugger")]
                {
                    for i in (0..=0xFFFF).step_by(8) {
                        let a1 = format!("{:02X}", c.mb.ppu_peek(i));
                        let a2 = format!("{:02X}", c.mb.ppu_peek(i+1));
                        let a3 = format!("{:02X}", c.mb.ppu_peek(i+2));
                        let a4 = format!("{:02X}", c.mb.ppu_peek(i+3));
                        let a5 = format!("{:02X}", c.mb.ppu_peek(i+4));
                        let a6 = format!("{:02X}", c.mb.ppu_peek(i+5));
                        let a7 = format!("{:02X}", c.mb.ppu_peek(i+6));
                        let a8 = format!("{:02X}", c.mb.ppu_peek(i+7));
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