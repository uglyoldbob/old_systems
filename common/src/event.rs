//! This module covers funcality for custom events withing the emulator on the main event loop

/// The kinds of events that can occur
pub enum EventType {
    /// The network data should be checked
    CheckNetwork,
}

/// The custom event struct for the application
pub struct Event {
    /// The message sent
    pub message: EventType,
    #[cfg(feature = "egui-multiwin")]
    /// The optional window id for the message
    id: Option<egui_multiwin::winit::window::WindowId>,
}

impl Event {
    #[cfg(feature = "egui-multiwin")]
    /// Return the window id for the custom event
    pub fn window_id(&self) -> Option<egui_multiwin::winit::window::WindowId> {
        self.id
    }

    /// Create a non-window specific event
    pub fn new_general(message: EventType) -> Self {
        Self { message, 
            #[cfg(feature = "egui-multiwin")]
            id: None
        }
    }
}
