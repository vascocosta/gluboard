pub struct AnsiStyle {
    bg: Option<AnsiColor>,
    fg: Option<AnsiColor>,
}

impl AnsiStyle {
    pub fn new(fg: Option<AnsiColor>, bg: Option<AnsiColor>) -> Self {
        Self { bg, fg }
    }
    pub fn apply(&self, text: &str) -> String {
        let fg = match &self.fg {
            Some(fg) => match fg {
                AnsiColor::Black => 30,
                AnsiColor::Red => 31,
                AnsiColor::Green => 32,
                AnsiColor::Yellow => 33,
                AnsiColor::Blue => 34,
                AnsiColor::Magenta => 35,
                AnsiColor::Cyan => 36,
                AnsiColor::White => 37,
                AnsiColor::Default => 39,
            },
            None => 39,
        };

        let bg = match &self.bg {
            Some(bg) => match bg {
                AnsiColor::Black => 40,
                AnsiColor::Red => 41,
                AnsiColor::Green => 42,
                AnsiColor::Yellow => 43,
                AnsiColor::Blue => 44,
                AnsiColor::Magenta => 45,
                AnsiColor::Cyan => 46,
                AnsiColor::White => 47,
                AnsiColor::Default => 49,
            },
            None => 49,
        };

        format!("\u{001b}[{};{}m{}\u{001b}[{};{}m", fg, bg, text, 37, 40)
    }
}

pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}
