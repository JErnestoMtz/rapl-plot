use crate::Color;

/// Colors and type used to draw a figure. A plain value: change the fields
/// you care about and pass it to [`Figure::theme`](crate::Figure::theme).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Fill behind the whole figure. Its alpha is honoured, so
    /// [`Color::TRANSPARENT`] gives a figure with no background.
    pub background: Color,
    /// Text, axis lines and ticks.
    pub foreground: Color,
    /// Grid lines.
    pub grid: Color,
    /// Colors given in turn to series that do not set their own.
    pub cycle: Vec<Color>,
    /// CSS font family.
    pub font_family: String,
    /// Size of tick and axis labels in pixels; the title is a little larger.
    pub font_size: f64,
}

impl Theme {
    /// Dark text on white.
    pub fn light() -> Theme {
        Theme {
            background: Color::WHITE,
            foreground: Color::BLACK,
            grid: Color::rgb(0xe4, 0xe6, 0xea),
            cycle: vec![
                Color::BLUE,
                Color::YELLOW,
                Color::RED,
                Color::TEAL,
                Color::GREEN,
                Color::PINK,
                Color::PURPLE,
                Color::BROWN,
                Color::GRAY,
            ],
            font_family: "system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif"
                .into(),
            font_size: 12.0,
        }
    }

    /// Light text on a dark background.
    pub fn dark() -> Theme {
        Theme {
            background: Color::rgb(0x16, 0x18, 0x1d),
            foreground: Color::rgb(0xe6, 0xe8, 0xec),
            grid: Color::rgb(0x2e, 0x32, 0x3a),
            ..Theme::light()
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::light()
    }
}
