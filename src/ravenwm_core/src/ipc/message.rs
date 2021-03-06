use hex_color::HexColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    /// Quit `ravenwm`.
    Quit,

    /// Close the active window.
    CloseWindow,

    MoveWindow {
        x: u32,
        y: u32,
    },

    SetBorderWidth {
        width: u32,
    },

    SetBorderColor {
        color: HexColor,
    },
}
