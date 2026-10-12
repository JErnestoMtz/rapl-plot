use crate::channel::{Channel, Inner, IntoChannel, Number};
use crate::{Error, ticks};
use rapl::{Buffer, Ndarr, U1};
use std::fmt::Debug;
use std::sync::Arc;

/// Where an axis puts its ticks. Built from a count or from an array of
/// positions by [`Axis::ticks`].
#[derive(Debug, Clone)]
pub enum Ticks {
    /// As many nicely placed ticks as fit.
    Auto,
    /// Nicely placed ticks, at most this many steps apart end to end.
    About(usize),
    /// Exactly these positions; the ones outside the limits are not drawn.
    At(Ndarr<f64, U1>),
}

impl From<usize> for Ticks {
    fn from(count: usize) -> Self {
        Ticks::About(count)
    }
}

impl<T: Number, B: Buffer<T>> From<&Ndarr<T, U1, B>> for Ticks {
    fn from(positions: &Ndarr<T, U1, B>) -> Self {
        Ticks::At(positions.map(|v| v.to_f64()))
    }
}

impl<T: Number, const N: usize> From<[T; N]> for Ticks {
    fn from(positions: [T; N]) -> Self {
        Ticks::from(&Ndarr::from(Vec::from(positions)))
    }
}

type Formatter = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// One axis of a plot: its limits, ticks, labels and grid. Start from
/// [`axis`] and hand the result to [`Plot::x`](crate::Plot::x) or
/// [`Plot::y`](crate::Plot::y).
///
/// ```
/// use rapl::*;
/// use rapl_plot::*;
///
/// let at = Ndarr::from([0.0, 0.5, 1.0]);
/// let names = Ndarr::from(vec!["low", "mid", "high"]);
/// let x = axis().label("level").ticks(&at).tick_labels(&names).lim(0.0, 1.0);
///
/// let svg = line_y(&Ndarr::from([0.0, 1.0])).x(x).to_svg()?;
/// assert!(svg.contains("mid"));
/// # Ok::<(), rapl_plot::Error>(())
/// ```
#[derive(Clone)]
pub struct Axis {
    pub(crate) label: Option<String>,
    lim: Option<(f64, f64)>,
    ticks: Ticks,
    tick_labels: Channel<String>,
    format: Option<Formatter>,
    pub(crate) grid: bool,
    pub(crate) reversed: bool,
}

/// An axis with automatic limits and ticks, a grid and no label.
pub fn axis() -> Axis {
    Axis::default()
}

impl Default for Axis {
    fn default() -> Self {
        Axis {
            label: None,
            lim: None,
            ticks: Ticks::Auto,
            tick_labels: Channel::default(),
            format: None,
            grid: true,
            reversed: false,
        }
    }
}

impl Debug for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Axis")
            .field("label", &self.label)
            .field("lim", &self.lim)
            .field("ticks", &self.ticks)
            .field("tick_labels", &self.tick_labels)
            .field("format", &self.format.as_ref().map(|_| "<fn>"))
            .field("grid", &self.grid)
            .field("reversed", &self.reversed)
            .finish()
    }
}

impl Axis {
    /// Text along the axis.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Fix the range instead of fitting it to the data. `lo` must be below
    /// `hi`; use [`reversed`](Self::reversed) to flip the direction.
    pub fn lim(mut self, lo: f64, hi: f64) -> Self {
        self.lim = Some((lo, hi));
        self
    }

    /// A tick count (`8`), or exact positions as a rank-one array.
    pub fn ticks(mut self, ticks: impl Into<Ticks>) -> Self {
        self.ticks = ticks.into();
        self
    }

    /// Text for each tick set with [`ticks`](Self::ticks): a rank-one
    /// array of anything `Display`, the same length as the positions.
    pub fn tick_labels(mut self, labels: impl IntoChannel<String>) -> Self {
        self.tick_labels = labels.into_channel();
        self
    }

    /// Turn tick values into text, for ticks without explicit labels.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(format));
        self
    }

    /// Show or hide the grid lines at this axis' ticks.
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }

    /// Run the axis from high to low.
    pub fn reversed(mut self) -> Self {
        self.reversed = true;
        self
    }

    /// Settle limits and ticks for data spanning `extent`. `name` is `"x"`
    /// or `"y"`, for error messages; `room` is how many tick steps fit.
    pub(crate) fn resolve(
        &self,
        name: &str,
        extent: Option<(f64, f64)>,
        room: usize,
    ) -> Result<Scale, Error> {
        let (lo, hi) = match self.lim {
            Some((lo, hi)) if lo.is_finite() && hi.is_finite() && lo < hi => (lo, hi),
            Some((lo, hi)) => {
                return Err(Error::Axis(format!(
                    "{name} axis: limits ({lo}, {hi}) must be finite and increasing"
                )));
            }
            None => fit(extent),
        };
        let format =
            |positions: &Ndarr<f64, U1>, fallback: &dyn Fn() -> Ndarr<String, U1>| match &self
                .format
            {
                Some(format) => positions.map(|&v| format(v)),
                None => fallback(),
            };
        let (positions, labels) = match &self.ticks {
            Ticks::At(positions) => {
                let labels = match &self.tick_labels.0 {
                    Inner::Default => format(positions, &|| positions.map(|v| v.to_string())),
                    Inner::Array(labels) if labels.shape() == positions.shape() => {
                        Ndarr::from(labels.iter_elems().cloned().collect::<Vec<_>>())
                    }
                    Inner::Array(labels) => {
                        return Err(Error::Axis(format!(
                            "{name} axis: tick labels of shape {:?} for {} ticks",
                            labels.shape(),
                            positions.len()
                        )));
                    }
                    Inner::Const(_) => {
                        return Err(Error::Axis(format!(
                            "{name} axis: tick_labels needs an array, one label per tick"
                        )));
                    }
                };
                (positions.clone(), labels)
            }
            _ if !matches!(self.tick_labels.0, Inner::Default) => {
                return Err(Error::Axis(format!(
                    "{name} axis: tick_labels needs explicit positions from ticks(&positions)"
                )));
            }
            automatic => {
                let intervals = match automatic {
                    Ticks::About(count) => *count,
                    _ => room,
                };
                let positions = ticks::nice(lo, hi, intervals);
                let step = ticks::step(lo, hi, intervals).unwrap_or(1.0);
                let labels = format(&positions, &|| ticks::labels(&positions, step));
                (positions, labels)
            }
        };
        let ticks = positions
            .iter_elems()
            .copied()
            .zip(labels.into_data())
            .filter(|(position, _)| (lo..=hi).contains(position))
            .collect();
        Ok(Scale { lo, hi, ticks })
    }
}

/// Limits that fit the data with a little room, or `[0, 1]` without data.
fn fit(extent: Option<(f64, f64)>) -> (f64, f64) {
    let (lo, hi) = extent.unwrap_or((0.0, 1.0));
    let pad = match hi - lo {
        span if span > 0.0 => span * 0.05,
        _ if lo == 0.0 => 1.0,
        _ => lo.abs() * 0.05,
    };
    (lo - pad, hi + pad)
}

/// A settled linear axis: its limits and the ticks inside them.
#[derive(Debug, Clone)]
pub(crate) struct Scale {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
    pub(crate) ticks: Vec<(f64, String)>,
}

impl Scale {
    /// Where `value` falls between the limits, 0 at `lo` and 1 at `hi`.
    pub(crate) fn fraction(&self, value: f64) -> f64 {
        (value - self.lo) / (self.hi - self.lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(scale: &Scale) -> Vec<&str> {
        scale
            .ticks
            .iter()
            .map(|(_, label)| label.as_str())
            .collect()
    }

    #[test]
    fn automatic_limits_pad_the_data() {
        let scale = axis().resolve("x", Some((0.0, 10.0)), 6).unwrap();
        assert_eq!((scale.lo, scale.hi), (-0.5, 10.5));
        assert_eq!(labels(&scale), ["0", "2", "4", "6", "8", "10"]);
    }

    #[test]
    fn constant_data_still_has_a_range() {
        let scale = axis().resolve("y", Some((3.0, 3.0)), 4).unwrap();
        assert!(scale.lo < 3.0 && scale.hi > 3.0);
        assert!(!scale.ticks.is_empty());
    }

    #[test]
    fn explicit_ticks_take_any_display_labels() {
        let x = axis()
            .lim(0.0, 2.0)
            .ticks([0, 1, 2, 3])
            .tick_labels(Ndarr::from(['a', 'b', 'c', 'd']));
        let scale = x.resolve("x", None, 5).unwrap();
        assert_eq!(labels(&scale), ["a", "b", "c"]);
    }

    #[test]
    fn formatter_applies_to_automatic_ticks() {
        let y = axis()
            .lim(0.0, 1.0)
            .ticks(2)
            .format(|v| format!("{:.0}%", v * 100.0));
        assert_eq!(
            labels(&y.resolve("y", None, 9).unwrap()),
            ["0%", "50%", "100%"]
        );
    }

    #[test]
    fn inconsistent_settings_are_reported() {
        let missing = axis().tick_labels(["a"]);
        assert!(matches!(missing.resolve("x", None, 5), Err(Error::Axis(_))));
        let short = axis().ticks([0.0, 1.0]).tick_labels(["a"]);
        assert!(matches!(short.resolve("x", None, 5), Err(Error::Axis(_))));
        assert!(matches!(
            axis().lim(2.0, 1.0).resolve("x", None, 5),
            Err(Error::Axis(_))
        ));
    }
}
