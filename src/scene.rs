//! The drawing primitives every mark is lowered to. A backend only has to
//! know how to draw these.

use crate::{Color, Dash};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Paint {
    pub(crate) color: Color,
    pub(crate) opacity: f64,
}

impl Paint {
    pub(crate) fn solid(color: Color) -> Paint {
        Paint {
            color,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Stroke {
    pub(crate) paint: Paint,
    pub(crate) width: f64,
    pub(crate) dash: Dash,
}

impl Stroke {
    pub(crate) fn solid(color: Color, width: f64) -> Stroke {
        Stroke {
            paint: Paint::solid(color),
            width,
            dash: Dash::Solid,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone)]
pub(crate) enum Prim {
    /// Open polylines sharing one stroke.
    Path {
        lines: Vec<Vec<(f64, f64)>>,
        stroke: Stroke,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<Paint>,
        stroke: Option<Stroke>,
    },
    Circle {
        x: f64,
        y: f64,
        r: f64,
        fill: Paint,
    },
    /// Text whose baseline starts, is centred or ends at `(x, y)`.
    Text {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        color: Color,
        anchor: Anchor,
        /// Read bottom to top instead of left to right.
        vertical: bool,
        bold: bool,
    },
    /// Primitives clipped to a rectangle.
    Clip {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        items: Vec<Prim>,
    },
}

/// Rough width of `text` at `size`; there is no font to measure against.
pub(crate) fn text_width(text: &str, size: f64) -> f64 {
    text.chars().count() as f64 * size * 0.58
}
