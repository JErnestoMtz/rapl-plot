use crate::{Color, Dash};
use rapl::{Buffer, Dim, Dyn, Ndarr, Rank};
use std::fmt::Display;

/// A primitive number that can be plotted.
pub trait Number: Copy {
    /// The value as an `f64`, rounding where it has to.
    fn to_f64(self) -> f64;
}

macro_rules! impl_number {
    ($($t:ty),*) => {$(
        impl Number for $t {
            fn to_f64(self) -> f64 {
                self as f64
            }
        }
        impl IntoChannel<f64> for $t {
            fn into_channel(self) -> Channel<f64> {
                Channel(Inner::Const(self as f64))
            }
        }
    )*};
}
impl_number!(f64, f32, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

/// A mark option that is either one value or an array of them.
///
/// For a mark whose `y` has shape `[n, ...]`, axis 0 is the point axis and
/// the remaining axes are the series frame:
///
/// - a single value applies to every series;
/// - an array of lower rank than `y` broadcasts against the series frame,
///   giving one value per series;
/// - an array of the same rank as `y` broadcasts against all of `y`, giving
///   one value per point, on the marks that can vary along a series.
#[derive(Debug, Clone)]
pub struct Channel<T>(pub(crate) Inner<T>);

#[derive(Debug, Clone)]
pub(crate) enum Inner<T> {
    Default,
    Const(T),
    Array(Ndarr<T, Dyn>),
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Channel(Inner::Default)
    }
}

/// Values accepted where a mark option takes a [`Channel`]: a single value,
/// an `Ndarr` (owned or borrowed, any rank and buffer), or a plain array
/// standing in for a rank-one `Ndarr`.
pub trait IntoChannel<T> {
    /// Wrap the value or copy the array.
    fn into_channel(self) -> Channel<T>;
}

macro_rules! impl_array_channel {
    ($target:ty, [$($bound:tt)*], $convert:expr) => {
        impl<U: $($bound)*, R: Rank, B: Buffer<U>> IntoChannel<$target> for &Ndarr<U, R, B> {
            fn into_channel(self) -> Channel<$target> {
                Channel(Inner::Array(self.map($convert).into_dyn()))
            }
        }
        impl<U: $($bound)*, R: Rank, B: Buffer<U>> IntoChannel<$target> for Ndarr<U, R, B> {
            fn into_channel(self) -> Channel<$target> {
                (&self).into_channel()
            }
        }
        impl<U: $($bound)*, const N: usize> IntoChannel<$target> for [U; N] {
            fn into_channel(self) -> Channel<$target> {
                Ndarr::from(Vec::from(self)).into_channel()
            }
        }
    };
}
impl_array_channel!(f64, [Number], |v: &U| v.to_f64());
impl_array_channel!(String, [Display], |v: &U| v.to_string());
impl_array_channel!(Color, [Into<Color> + Copy], |v: &U| (*v).into());
impl_array_channel!(Dash, [Into<Dash> + Copy], |v: &U| (*v).into());

impl IntoChannel<String> for &str {
    fn into_channel(self) -> Channel<String> {
        Channel(Inner::Const(self.to_string()))
    }
}

impl IntoChannel<String> for String {
    fn into_channel(self) -> Channel<String> {
        Channel(Inner::Const(self))
    }
}

impl IntoChannel<Color> for Color {
    fn into_channel(self) -> Channel<Color> {
        Channel(Inner::Const(self))
    }
}

impl IntoChannel<Dash> for Dash {
    fn into_channel(self) -> Channel<Dash> {
        Channel(Inner::Const(self))
    }
}

/// A channel laid out against a mark's series and points.
pub(crate) enum Resolved<T> {
    Default,
    /// One value per series, in C-order over the series frame.
    Series(Vec<T>),
    /// One value per point of each series.
    Points(Vec<Vec<T>>),
}

/// One value for a whole series, or one per point.
#[derive(Debug, Clone)]
pub(crate) enum Per<T> {
    One(T),
    Each(Vec<T>),
}

impl<T: Clone> Per<T> {
    pub(crate) fn get(&self, point: usize) -> T {
        match self {
            Per::One(value) => value.clone(),
            Per::Each(values) => values[point].clone(),
        }
    }
}

impl<T: Clone> Resolved<T> {
    /// The value for one series, when the channel was set per series.
    pub(crate) fn series(&self, series: usize) -> Option<T> {
        match self {
            Resolved::Series(values) => Some(values[series].clone()),
            _ => None,
        }
    }

    /// The values for one series, falling back to `default` when unset.
    pub(crate) fn per(&self, series: usize, default: T) -> Per<T> {
        match self {
            Resolved::Default => Per::One(default),
            Resolved::Series(values) => Per::One(values[series].clone()),
            Resolved::Points(lanes) => Per::Each(lanes[series].clone()),
        }
    }
}

impl<T: Clone> Channel<T> {
    /// Lay the channel out against a mark whose `y` has `shape`. `per_point`
    /// says whether the mark can vary this option along a series.
    pub(crate) fn resolve(
        &self,
        name: &str,
        shape: &[usize],
        per_point: bool,
    ) -> Result<Resolved<T>, String> {
        let frame = &shape[1..];
        let values = match &self.0 {
            Inner::Default => return Ok(Resolved::Default),
            Inner::Const(value) => {
                return Ok(Resolved::Series(vec![
                    value.clone();
                    frame.iter().product()
                ]));
            }
            Inner::Array(values) => values,
        };
        if values.rank() < shape.len() {
            let view = values
                .broadcast_view_to(&Dim::<Dyn>::from(frame))
                .map_err(|_| {
                    format!(
                        "{name} has shape {:?}, which does not broadcast against the series frame {frame:?}",
                        values.shape()
                    )
                })?;
            Ok(Resolved::Series(view.iter_elems().cloned().collect()))
        } else if values.rank() > shape.len() {
            Err(format!(
                "{name} has shape {:?}, which has more axes than y ({shape:?})",
                values.shape()
            ))
        } else if !per_point {
            Err(format!(
                "{name} has shape {:?}, the same rank as y, which means one value per point; this mark takes one value per series (shape {frame:?} or a single value)",
                values.shape()
            ))
        } else {
            let view = values
                .broadcast_view_to(&Dim::<Dyn>::from(shape))
                .map_err(|_| {
                    format!(
                        "{name} has shape {:?}, which does not broadcast against y ({shape:?})",
                        values.shape()
                    )
                })?;
            let lanes = view
                .lanes(0)
                .map_err(|e| e.to_string())?
                .map(|lane| lane.iter_elems().cloned().collect())
                .collect();
            Ok(Resolved::Points(lanes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_rank_is_per_series() {
        let width: Channel<f64> = Ndarr::from([1.0, 2.0]).into_channel();
        let Resolved::Series(values) = width.resolve("width", &[5, 3, 2], false).unwrap() else {
            panic!("expected one value per series");
        };
        assert_eq!(values, [1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
    }

    #[test]
    fn same_rank_is_per_point() {
        let size: Channel<f64> = Ndarr::from([[1], [2], [3]]).into_channel();
        let Resolved::Points(lanes) = size.resolve("size", &[3, 2], true).unwrap() else {
            panic!("expected one value per point");
        };
        assert_eq!(lanes, [[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]]);
        assert!(size.resolve("size", &[3, 2], false).is_err());
    }

    #[test]
    fn single_values_reach_every_series() {
        let label = "all".into_channel();
        assert_eq!(
            label
                .resolve("label", &[4, 3], false)
                .unwrap()
                .series(2)
                .unwrap(),
            "all"
        );
        assert_eq!(
            label
                .resolve("label", &[4], false)
                .unwrap()
                .series(0)
                .unwrap(),
            "all"
        );
    }

    #[test]
    fn mismatched_frames_are_reported() {
        let label = ["a", "b", "c"].into_channel();
        let message = label.resolve("label", &[10, 2], false).err().unwrap();
        assert!(message.contains("label has shape [3]"), "{message}");
    }
}
