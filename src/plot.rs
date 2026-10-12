use crate::marks::{Line, Scatter, Series};
use crate::{Axis, Color, Error, Figure, Theme};
use std::ops::Add;
use std::path::Path;

/// Where the legend sits inside the axes. It is only drawn when at least
/// one series has a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum Legend {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    /// Never draw a legend.
    Hidden,
}

#[derive(Debug, Clone)]
pub(crate) enum Mark {
    Line(Line),
    Scatter(Scatter),
}

impl Mark {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Mark::Line(_) => "line",
            Mark::Scatter(_) => "scatter",
        }
    }

    pub(crate) fn resolve(
        &self,
        cycle: &mut impl Iterator<Item = Color>,
    ) -> Result<Vec<Series>, String> {
        match self {
            Mark::Line(mark) => mark.resolve(cycle),
            Mark::Scatter(mark) => mark.resolve(cycle),
        }
    }
}

/// One set of axes: layered marks that share an x and a y scale.
///
/// Every mark converts into a `Plot`, and `+` layers marks and plots, so a
/// plot is usually built by adding marks and then setting what the defaults
/// got wrong:
///
/// ```
/// use rapl::*;
/// use rapl_plot::*;
///
/// let x = Ndarr::from_fn([50], |ix| ix[0] as f64 * 0.2);
/// let model = x.map(|v| v.sin());
/// let measured = x.map(|v| v.sin() + 0.1 * (v * 7.0).cos());
///
/// let plot = (line(&x, &model).label("model") + scatter(&x, &measured).label("measured"))
///     .title("Fit")
///     .x(axis().label("time (s)"))
///     .y(axis().lim(-1.5, 1.5));
///
/// let svg = plot.to_svg()?;
/// assert!(svg.contains("time (s)"));
/// # Ok::<(), rapl_plot::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Plot {
    pub(crate) marks: Vec<Mark>,
    pub(crate) title: Option<String>,
    pub(crate) x: Option<Axis>,
    pub(crate) y: Option<Axis>,
    pub(crate) legend: Option<Legend>,
}

impl Plot {
    /// Empty axes.
    pub fn new() -> Plot {
        Plot::default()
    }

    /// Text above the axes.
    pub fn title(mut self, title: impl Into<String>) -> Plot {
        self.title = Some(title.into());
        self
    }

    /// Configure the horizontal axis.
    pub fn x(mut self, axis: Axis) -> Plot {
        self.x = Some(axis);
        self
    }

    /// Configure the vertical axis.
    pub fn y(mut self, axis: Axis) -> Plot {
        self.y = Some(axis);
        self
    }

    /// Move or hide the legend.
    pub fn legend(mut self, legend: Legend) -> Plot {
        self.legend = Some(legend);
        self
    }

    /// Make a figure of this plot with the given size in pixels.
    pub fn size(self, width: f64, height: f64) -> Figure {
        Figure::from(self).size(width, height)
    }

    /// Make a figure of this plot drawn with `theme`.
    pub fn theme(self, theme: Theme) -> Figure {
        Figure::from(self).theme(theme)
    }

    /// Render to a standalone SVG document at the default size.
    pub fn to_svg(&self) -> Result<String, Error> {
        Figure::from(self.clone()).to_svg()
    }

    /// Render and write an `.svg` or `.png` file; see [`Figure::save`].
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        Figure::from(self.clone()).save(path)
    }

    /// Open the plot in the system's viewer; see [`Figure::show`].
    pub fn show(&self) -> Result<(), Error> {
        Figure::from(self.clone()).show()
    }

    /// Render to PNG bytes at the default size and scale; see
    /// [`Figure::to_png`].
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        Figure::from(self.clone()).to_png()
    }
}

/// Layer the right-hand marks over the left-hand ones. Settings made on
/// the left win; the right fills in what the left did not set.
impl<R: Into<Plot>> Add<R> for Plot {
    type Output = Plot;

    fn add(mut self, rhs: R) -> Plot {
        let rhs = rhs.into();
        self.marks.extend(rhs.marks);
        self.title = self.title.or(rhs.title);
        self.x = self.x.or(rhs.x);
        self.y = self.y.or(rhs.y);
        self.legend = self.legend.or(rhs.legend);
        self
    }
}

/// What every mark shares: it converts into a [`Plot`], layers with `+`,
/// and forwards the plot-level methods so a single mark can be configured
/// and rendered without naming `Plot`. Figure-level `size` and `theme` are
/// not forwarded: `size` already means marker size on some marks.
macro_rules! mark {
    ($mark:ident) => {
        impl From<$mark> for Plot {
            fn from(mark: $mark) -> Plot {
                Plot {
                    marks: vec![Mark::$mark(mark)],
                    ..Plot::default()
                }
            }
        }

        /// Layer another mark or plot over this mark.
        impl<R: Into<Plot>> Add<R> for $mark {
            type Output = Plot;

            fn add(self, rhs: R) -> Plot {
                Plot::from(self) + rhs
            }
        }

        impl $mark {
            /// A plot of this mark with a title; see [`Plot::title`].
            pub fn title(self, title: impl Into<String>) -> Plot {
                Plot::from(self).title(title)
            }

            /// A plot of this mark with a configured horizontal axis.
            pub fn x(self, axis: Axis) -> Plot {
                Plot::from(self).x(axis)
            }

            /// A plot of this mark with a configured vertical axis.
            pub fn y(self, axis: Axis) -> Plot {
                Plot::from(self).y(axis)
            }

            /// A plot of this mark with the legend moved or hidden.
            pub fn legend(self, legend: Legend) -> Plot {
                Plot::from(self).legend(legend)
            }

            /// Render this mark alone to an SVG document.
            pub fn to_svg(&self) -> Result<String, Error> {
                Plot::from(self.clone()).to_svg()
            }

            /// Render this mark alone and write a file; see [`Figure::save`].
            pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
                Plot::from(self.clone()).save(path)
            }

            /// Open this mark alone in the system's viewer; see
            /// [`Figure::show`].
            pub fn show(&self) -> Result<(), Error> {
                Plot::from(self.clone()).show()
            }

            /// Render this mark alone to PNG bytes; see [`Figure::to_png`].
            #[cfg(feature = "png")]
            pub fn to_png(&self) -> Result<Vec<u8>, Error> {
                Plot::from(self.clone()).to_png()
            }
        }
    };
}
mark!(Line);
mark!(Scatter);

#[cfg(test)]
mod tests {
    use crate::{axis, line_y, scatter_y};
    use rapl::Ndarr;

    #[test]
    fn adding_layers_marks_and_keeps_the_left_settings() {
        let y = Ndarr::from([1.0, 2.0]);
        let left = line_y(&y).title("left").x(axis().label("left x"));
        let right = scatter_y(&y).title("right").y(axis().label("right y"));
        let plot = left + right + line_y(&y);
        assert_eq!(plot.marks.len(), 3);
        assert_eq!(plot.title.as_deref(), Some("left"));
        assert_eq!(plot.x.unwrap().label.as_deref(), Some("left x"));
        assert_eq!(plot.y.unwrap().label.as_deref(), Some("right y"));
    }
}
