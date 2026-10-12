use crate::channel::{Channel, IntoChannel, Number, Per};
use crate::{Color, Dash};
use rapl::{Buffer, Dyn, Ndarr, Rank};

fn to_f64<T: Number, R: Rank, B: Buffer<T>>(values: &Ndarr<T, R, B>) -> Ndarr<f64, Dyn> {
    values.map(|v| v.to_f64()).into_dyn()
}

/// The point index along axis 0 of `y`, as an x coordinate.
fn index_of<T, R: Rank, B: Buffer<T>>(y: &Ndarr<T, R, B>) -> Ndarr<f64, Dyn> {
    let n = y.shape().first().copied().unwrap_or(0);
    Ndarr::from_fn([n], |ix| ix[0] as f64).into_dyn()
}

/// Lines through the points `(x, y)`; see [`line()`].
#[derive(Debug, Clone)]
pub struct Line {
    x: Ndarr<f64, Dyn>,
    y: Ndarr<f64, Dyn>,
    color: Channel<Color>,
    width: Channel<f64>,
    opacity: Channel<f64>,
    dash: Channel<Dash>,
    label: Channel<String>,
}

/// One line per lane of `y` along axis 0.
///
/// `y` has shape `[n, ...]`: a `[n]` array is one line, `[n, k]` is `k`
/// lines, and `[n, a, b]` is one line per `[a, b]` entry. `x` is either
/// `[n]`, shared by every line, or the same shape as `y`. Points that are
/// not finite break the line.
///
/// Options take one value per series or a single value for all of them; see
/// [`Channel`].
///
/// ```
/// use rapl::*;
/// use rapl_plot::*;
///
/// let x = Ndarr::from_fn([100], |ix| ix[0] as f64 * 0.1);
/// let y = Ndarr::from_fn([100, 2], |ix| (ix[0] as f64 * 0.1 + ix[1] as f64).sin());
///
/// let svg = line(&x, &y).label(["sin(x)", "sin(x + 1)"]).width(2).to_svg()?;
/// assert!(svg.contains("sin(x + 1)"));
/// # Ok::<(), rapl_plot::Error>(())
/// ```
pub fn line<TX, TY, RX, RY, BX, BY>(x: &Ndarr<TX, RX, BX>, y: &Ndarr<TY, RY, BY>) -> Line
where
    TX: Number,
    TY: Number,
    RX: Rank,
    RY: Rank,
    BX: Buffer<TX>,
    BY: Buffer<TY>,
{
    Line {
        x: to_f64(x),
        y: to_f64(y),
        color: Channel::default(),
        width: Channel::default(),
        opacity: Channel::default(),
        dash: Channel::default(),
        label: Channel::default(),
    }
}

/// [`line()`] against the point index `0, 1, 2, ...`.
pub fn line_y<T: Number, R: Rank, B: Buffer<T>>(y: &Ndarr<T, R, B>) -> Line {
    let mut mark = line(y, y);
    mark.x = index_of(y);
    mark
}

impl Line {
    /// Stroke color. Unset series take the next color of the theme's cycle.
    pub fn color(mut self, color: impl IntoChannel<Color>) -> Self {
        self.color = color.into_channel();
        self
    }

    /// Stroke width in pixels.
    pub fn width(mut self, width: impl IntoChannel<f64>) -> Self {
        self.width = width.into_channel();
        self
    }

    /// Opacity from 0 to 1.
    pub fn opacity(mut self, opacity: impl IntoChannel<f64>) -> Self {
        self.opacity = opacity.into_channel();
        self
    }

    /// Stroke pattern.
    pub fn dash(mut self, dash: impl IntoChannel<Dash>) -> Self {
        self.dash = dash.into_channel();
        self
    }

    /// Legend entry. Any array of `Display` values works.
    pub fn label(mut self, label: impl IntoChannel<String>) -> Self {
        self.label = label.into_channel();
        self
    }

    pub(crate) fn resolve(
        &self,
        cycle: &mut impl Iterator<Item = Color>,
    ) -> Result<Vec<Series>, String> {
        let lanes = lanes_xy(&self.x, &self.y)?;
        let shape = self.y.shape();
        let color = self.color.resolve("color", shape, false)?;
        let width = self.width.resolve("width", shape, false)?;
        let opacity = self.opacity.resolve("opacity", shape, false)?;
        let dash = self.dash.resolve("dash", shape, false)?;
        let label = self.label.resolve("label", shape, false)?;
        Ok(lanes
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| Series {
                x,
                y,
                label: label.series(i),
                geom: Geom::Line {
                    color: color.series(i).unwrap_or_else(|| next(cycle)),
                    width: width.series(i).unwrap_or(1.5),
                    opacity: opacity.series(i).unwrap_or(1.0),
                    dash: dash.series(i).unwrap_or_default(),
                },
            })
            .collect())
    }
}

/// Markers at the points `(x, y)`; see [`scatter`].
#[derive(Debug, Clone)]
pub struct Scatter {
    x: Ndarr<f64, Dyn>,
    y: Ndarr<f64, Dyn>,
    color: Channel<Color>,
    size: Channel<f64>,
    opacity: Channel<f64>,
    label: Channel<String>,
}

/// One set of markers per lane of `y` along axis 0, with the same shape
/// rules as [`line()`]. Points that are not finite are skipped.
///
/// `color`, `size` and `opacity` can also vary per point: pass an array of
/// the same rank as `y`.
///
/// ```
/// use rapl::*;
/// use rapl_plot::*;
///
/// let x = Ndarr::from([1.0, 2.0, 3.0, 4.0]);
/// let y = Ndarr::from([2.0, 1.0, 4.0, 3.0]);
/// let weight = Ndarr::from([4, 8, 12, 16]);
///
/// let svg = scatter(&x, &y).size(&weight).to_svg()?;
/// assert_eq!(svg.matches("<circle").count(), 4);
/// # Ok::<(), rapl_plot::Error>(())
/// ```
pub fn scatter<TX, TY, RX, RY, BX, BY>(x: &Ndarr<TX, RX, BX>, y: &Ndarr<TY, RY, BY>) -> Scatter
where
    TX: Number,
    TY: Number,
    RX: Rank,
    RY: Rank,
    BX: Buffer<TX>,
    BY: Buffer<TY>,
{
    Scatter {
        x: to_f64(x),
        y: to_f64(y),
        color: Channel::default(),
        size: Channel::default(),
        opacity: Channel::default(),
        label: Channel::default(),
    }
}

/// [`scatter`] against the point index `0, 1, 2, ...`.
pub fn scatter_y<T: Number, R: Rank, B: Buffer<T>>(y: &Ndarr<T, R, B>) -> Scatter {
    let mut mark = scatter(y, y);
    mark.x = index_of(y);
    mark
}

impl Scatter {
    /// Fill color. Unset series take the next color of the theme's cycle.
    pub fn color(mut self, color: impl IntoChannel<Color>) -> Self {
        self.color = color.into_channel();
        self
    }

    /// Marker diameter in pixels.
    pub fn size(mut self, size: impl IntoChannel<f64>) -> Self {
        self.size = size.into_channel();
        self
    }

    /// Opacity from 0 to 1.
    pub fn opacity(mut self, opacity: impl IntoChannel<f64>) -> Self {
        self.opacity = opacity.into_channel();
        self
    }

    /// Legend entry. Any array of `Display` values works.
    pub fn label(mut self, label: impl IntoChannel<String>) -> Self {
        self.label = label.into_channel();
        self
    }

    pub(crate) fn resolve(
        &self,
        cycle: &mut impl Iterator<Item = Color>,
    ) -> Result<Vec<Series>, String> {
        let lanes = lanes_xy(&self.x, &self.y)?;
        let shape = self.y.shape();
        let color = self.color.resolve("color", shape, true)?;
        let size = self.size.resolve("size", shape, true)?;
        let opacity = self.opacity.resolve("opacity", shape, true)?;
        let label = self.label.resolve("label", shape, false)?;
        Ok(lanes
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| {
                let unset = matches!(color, crate::channel::Resolved::Default);
                let fallback = if unset { next(cycle) } else { Color::BLACK };
                Series {
                    x,
                    y,
                    label: label.series(i),
                    geom: Geom::Scatter {
                        color: color.per(i, fallback),
                        size: size.per(i, 6.0),
                        opacity: opacity.per(i, 1.0),
                    },
                }
            })
            .collect())
    }
}

/// One resolved series, ready to be scaled and drawn.
#[derive(Debug, Clone)]
pub(crate) struct Series {
    pub(crate) x: Vec<f64>,
    pub(crate) y: Vec<f64>,
    pub(crate) label: Option<String>,
    pub(crate) geom: Geom,
}

#[derive(Debug, Clone)]
pub(crate) enum Geom {
    Line {
        color: Color,
        width: f64,
        opacity: f64,
        dash: Dash,
    },
    Scatter {
        color: Per<Color>,
        size: Per<f64>,
        opacity: Per<f64>,
    },
}

fn next(cycle: &mut impl Iterator<Item = Color>) -> Color {
    cycle.next().unwrap_or(Color::BLACK)
}

/// The x and y coordinates of one series.
type Xy = (Vec<f64>, Vec<f64>);

/// The `(x, y)` coordinates of every series: one per lane of `y` along
/// axis 0, with `x` shared when it has rank one.
fn lanes_xy(x: &Ndarr<f64, Dyn>, y: &Ndarr<f64, Dyn>) -> Result<Vec<Xy>, String> {
    if y.rank() == 0 {
        return Err("y is a scalar; it needs at least one axis".into());
    }
    let collect = |lane: rapl::NdView<'_, f64, rapl::U1>| lane.iter_elems().copied().collect();
    let ys = y.lanes(0).map_err(|e| e.to_string())?.map(collect);
    if x.shape() == y.shape() {
        let xs = x.lanes(0).map_err(|e| e.to_string())?.map(collect);
        Ok(xs.zip(ys).collect())
    } else if x.rank() == 1 && x.len() == y.shape()[0] {
        let shared: Vec<f64> = x.iter_elems().copied().collect();
        Ok(ys.map(|y| (shared.clone(), y)).collect())
    } else {
        Err(format!(
            "x has shape {:?} but y has shape {:?}; x must be [{}] or match y",
            x.shape(),
            y.shape(),
            y.shape()[0]
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme;

    fn cycle() -> impl Iterator<Item = Color> {
        Theme::light().cycle.into_iter().cycle()
    }

    #[test]
    fn every_lane_along_axis_zero_is_a_series() {
        let x = Ndarr::from([0.0, 1.0, 2.0]);
        let y = Ndarr::from_fn([3, 2, 2], |ix| (ix[0] * 100 + ix[1] * 10 + ix[2]) as f64);
        let series = line(&x, &y).resolve(&mut cycle()).unwrap();
        assert_eq!(series.len(), 4);
        assert_eq!(series[3].y, [11.0, 111.0, 211.0]);
        assert_eq!(series[3].x, [0.0, 1.0, 2.0]);
    }

    #[test]
    fn views_and_integer_arrays_are_accepted() {
        let x = Ndarr::from([0u8, 1, 2, 3]);
        let y = Ndarr::from([[1i32, 2, 3, 4], [5, 6, 7, 8]]);
        let series = scatter(&x, &y.t_view()).resolve(&mut cycle()).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[1].y, [5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn x_can_match_the_shape_of_y() {
        let x = Ndarr::from([[0.0, 10.0], [1.0, 11.0]]);
        let y = Ndarr::from([[5.0, 6.0], [7.0, 8.0]]);
        let series = line(&x, &y).resolve(&mut cycle()).unwrap();
        assert_eq!(series[1].x, [10.0, 11.0]);
        assert_eq!(series[1].y, [6.0, 8.0]);
    }

    #[test]
    fn index_is_the_default_x() {
        let series = line_y(&Ndarr::from([5, 6, 7]))
            .resolve(&mut cycle())
            .unwrap();
        assert_eq!(series[0].x, [0.0, 1.0, 2.0]);
    }

    #[test]
    fn mismatched_shapes_are_reported() {
        let x = Ndarr::from([0.0, 1.0, 2.0]);
        let y = Ndarr::from([0.0, 1.0]);
        let message = line(&x, &y).resolve(&mut cycle()).err().unwrap();
        assert!(
            message.contains("x has shape [3] but y has shape [2]"),
            "{message}"
        );
    }

    #[test]
    fn unset_colors_follow_the_cycle_and_set_ones_do_not_consume_it() {
        let x = Ndarr::from([0.0, 1.0]);
        let mut cycle = cycle();
        let fixed = line(&x, &x)
            .color(Color::BLACK)
            .resolve(&mut cycle)
            .unwrap();
        let auto = line(&x, &x).resolve(&mut cycle).unwrap();
        assert!(matches!(
            fixed[0].geom,
            Geom::Line {
                color: Color::BLACK,
                ..
            }
        ));
        assert!(matches!(
            auto[0].geom,
            Geom::Line {
                color: Color::BLUE,
                ..
            }
        ));
    }
}
