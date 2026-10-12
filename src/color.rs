/// An sRGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red, 0 to 255.
    pub r: u8,
    /// Green, 0 to 255.
    pub g: u8,
    /// Blue, 0 to 255.
    pub b: u8,
    /// Alpha, 0 (transparent) to 255 (opaque).
    pub a: u8,
}

#[allow(missing_docs)]
impl Color {
    /// Fully transparent; as a theme background it leaves the figure unfilled.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    pub const BLACK: Color = Color::rgb(0x1b, 0x1e, 0x23);
    pub const WHITE: Color = Color::rgb(0xff, 0xff, 0xff);
    pub const GRAY: Color = Color::rgb(0x94, 0x98, 0xa0);
    pub const BLUE: Color = Color::rgb(0x42, 0x69, 0xd0);
    pub const YELLOW: Color = Color::rgb(0xef, 0xb1, 0x18);
    pub const RED: Color = Color::rgb(0xff, 0x72, 0x5c);
    pub const TEAL: Color = Color::rgb(0x6c, 0xc5, 0xb0);
    pub const GREEN: Color = Color::rgb(0x3c, 0xa9, 0x51);
    pub const PINK: Color = Color::rgb(0xff, 0x8a, 0xb7);
    pub const PURPLE: Color = Color::rgb(0xa4, 0x63, 0xf2);
    pub const BROWN: Color = Color::rgb(0x9c, 0x6b, 0x4e);
}

impl Color {
    /// An opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    /// A color with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` is optional).
    pub fn hex(code: &str) -> Option<Color> {
        let code = code.strip_prefix('#').unwrap_or(code);
        if !code.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&code[i..i + 2], 16).ok();
        let nibble = |i: usize| u8::from_str_radix(&code[i..i + 1], 16).ok().map(|v| v * 17);
        match code.len() {
            3 => Some(Color::rgb(nibble(0)?, nibble(1)?, nibble(2)?)),
            6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// `#rrggbb`, without alpha.
    pub(crate) fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Stroke pattern of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum Dash {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

impl Dash {
    pub(crate) fn pattern(self) -> Option<&'static str> {
        match self {
            Dash::Solid => None,
            Dash::Dashed => Some("6 4"),
            Dash::Dotted => Some("1.5 3.5"),
            Dash::DashDot => Some("6 3 1.5 3"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_codes() {
        assert_eq!(Color::hex("#4269d0"), Some(Color::BLUE));
        assert_eq!(Color::hex("fff"), Some(Color::WHITE));
        assert_eq!(Color::hex("#00000080"), Some(Color::rgba(0, 0, 0, 128)));
        assert_eq!(Color::hex("#12345"), None);
        assert_eq!(Color::hex("#zzzzzz"), None);
        assert_eq!(Color::BLUE.css(), "#4269d0");
    }
}
