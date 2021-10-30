use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    /// Ping
    Ping,

    /// Close the active window.
    CloseWindow,

    MoveWindow {
        x: u32,
        y: u32,
    },
}
