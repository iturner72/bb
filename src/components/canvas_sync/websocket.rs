use super::types::CanvasMessage;
use leptos::prelude::*;

/// WebSocket interface that can be passed down to canvas components
/// Uses signals and callbacks instead of storing WebSocket directly to ensure Send + Sync
#[derive(Clone, Copy)]
pub struct CanvasWebSocket {
    pub connected: ReadSignal<bool>,
    pub send_message: Callback<CanvasMessage>,
    pub connection_status: ReadSignal<String>,
}

impl CanvasWebSocket {
    pub fn new(
        connected: ReadSignal<bool>,
        send_message: Callback<CanvasMessage>,
        connection_status: ReadSignal<String>,
    ) -> Self {
        Self {
            connected,
            send_message,
            connection_status,
        }
    }

    pub fn send(&self, message: CanvasMessage) {
        self.send_message.run(message);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.get()
    }

    pub fn status(&self) -> String {
        self.connection_status.get()
    }
}

/// Context key for the WebSocket interface
#[derive(Clone, Copy)]
pub struct CanvasWebSocketContext;

impl CanvasWebSocketContext {
    pub fn provide(websocket: CanvasWebSocket) {
        leptos::context::provide_context(websocket);
    }

    pub fn use_context() -> Option<CanvasWebSocket> {
        leptos::context::use_context::<CanvasWebSocket>()
    }

    pub fn expect_context() -> CanvasWebSocket {
        leptos::prelude::expect_context::<CanvasWebSocket>()
    }
}
