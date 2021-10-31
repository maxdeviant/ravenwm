/// A rectangle.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rectangle {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Rectangle {
    /// Creates a new [`Rectangle`].
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn left(&self) -> i16 {
        self.x
    }

    pub fn top(&self) -> i16 {
        self.y
    }

    pub fn right(&self) -> i16 {
        self.x + self.width as i16
    }

    pub fn bottom(&self) -> i16 {
        self.y + self.height as i16
    }

    /// Inflates this [`Rectangle`] by the specified amount.
    pub fn inflate(&mut self, width: i16, height: i16) {
        self.x -= width;
        self.y -= height;
        self.width += 2 * width as u16;
        self.height += 2 * height as u16;
    }

    /// Deflates this [`Rectangle`] by the specified amount.
    pub fn deflate(&mut self, width: i16, height: i16) {
        self.inflate(-width, -height);
    }
}
