use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    /// Quit `ravenwm`.
    Quit,

    /// Ping
    Ping,

    /// Close the active window.
    CloseWindow,

    MoveWindow {
        x: u32,
        y: u32,
    },
}
