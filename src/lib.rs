//! **Plotting for [`rapl`] arrays.**
//!
//! A plot is built from *marks*: [`line()`], [`scatter`] and friends take
//! arrays and return plain values. Marks layer with `+` into a [`Plot`],
//! which owns the axes, and anything renders to SVG with `to_svg` or `save`,
//! or opens in a viewer with `show`. With the `png` feature, `save` also
//! writes PNG files.
//!
//! ```
//! use rapl::*;
//! use rapl_plot::*;
//!
//! let t = Ndarr::from_fn([200], |ix| ix[0] as f64 * 0.05);
//! // [200, 3]: one curve per column.
//! let waves = Ndarr::from_fn([200, 3], |ix| {
//!     (ix[0] as f64 * 0.05 * (ix[1] + 1) as f64).sin()
//! });
//!
//! let svg = line(&t, &waves)
//!     .label(["1 Hz", "2 Hz", "3 Hz"])
//!     .title("Harmonics")
//!     .x(axis().label("time (s)"))
//!     .to_svg()?;
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), rapl_plot::Error>(())
//! ```
//!
//! # Shape decides what is drawn
//!
//! For a mark whose `y` has shape `[n, ...]`, axis 0 runs along the points
//! and every remaining axis is part of the *series frame*: `[n, k]` draws
//! `k` series and `[n, a, b]` draws `a * b`. Options such as color, width
//! and label are [`Channel`]s: a single value for every series, or an array
//! that broadcasts against the series frame. Views and slices are accepted
//! everywhere an array is.
//!
//! # Errors
//!
//! Building marks and plots never fails. Shapes and axis settings are
//! checked when a figure is rendered, and the [`Error`] names the mark or
//! axis at fault.

mod axis;
mod channel;
mod color;
mod error;
mod figure;
mod marks;
mod plot;
#[cfg(feature = "png")]
mod png;
mod render;
mod scene;
mod simplify;
mod svg;
mod theme;
pub mod ticks;
mod viewer;

pub use axis::{Axis, Ticks, axis};
pub use channel::{Channel, IntoChannel, Number};
pub use color::{Color, Dash};
pub use error::Error;
pub use figure::Figure;
pub use marks::{Line, Scatter, line, line_y, scatter, scatter_y};
pub use plot::{Legend, Plot};
pub use theme::Theme;
