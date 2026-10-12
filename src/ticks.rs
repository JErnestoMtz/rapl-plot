//! Tick placement and tick labels for linear axes.

use rapl::{Ndarr, U1};

/// Tick positions inside `[lo, hi]` on multiples of a "nice" step
/// (1, 2 or 5 times a power of ten), with at most `max_intervals` steps
/// between the first and the last.
///
/// Returns an empty array when the range is empty, reversed or not finite.
pub fn nice(lo: f64, hi: f64, max_intervals: usize) -> Ndarr<f64, U1> {
    let Some(step) = step(lo, hi, max_intervals) else {
        return Ndarr::from(Vec::new());
    };
    // Nudged so limits that sit on a multiple of the step keep their tick.
    let first = (lo / step - 1e-9).ceil();
    let last = (hi / step + 1e-9).floor();
    let n = (last - first) as usize + 1;
    // The nudge lets a tick land a rounding error outside the range, hence
    // the clamp; `+ 0.0` turns the `-0.0` a negative `first` gives into `0.0`.
    Ndarr::from_fn([n], |ix| {
        ((first + ix[0] as f64) * step).clamp(lo, hi) + 0.0
    })
}

/// The spacing [`nice`] uses for `[lo, hi]`, or `None` for a range it
/// cannot tick.
pub fn step(lo: f64, hi: f64, max_intervals: usize) -> Option<f64> {
    let span = hi - lo;
    if !span.is_finite() || span <= 0.0 || max_intervals == 0 {
        return None;
    }
    let raw = span / max_intervals as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let nice = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .find(|m| m * magnitude >= raw * (1.0 - 1e-9))
        .unwrap_or(10.0);
    Some(nice * magnitude)
}

/// Labels for ticks spaced `step` apart: just enough decimals to tell
/// neighbours apart, switching to scientific notation for very large or very
/// small values.
pub fn labels(ticks: &Ndarr<f64, U1>, step: f64) -> Ndarr<String, U1> {
    let largest = ticks.iter_elems().fold(0.0, |m: f64, t| m.max(t.abs()));
    let step_exponent = step.log10().floor();
    if largest >= 1e6 || (largest > 0.0 && largest < 1e-4) {
        let digits = (largest.log10().floor() - step_exponent).max(0.0) as usize;
        return ticks.map(|t| format!("{t:.digits$e}"));
    }
    let decimals = (-step_exponent).max(0.0) as usize;
    ticks.map(|t| format!("{t:.decimals$}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_interval() {
        let t = nice(0.0, 1.0, 5);
        assert_eq!(t.len(), 6);
        assert_eq!(
            labels(&t, step(0.0, 1.0, 5).unwrap()).data(),
            ["0.0", "0.2", "0.4", "0.6", "0.8", "1.0"]
        );
    }

    #[test]
    fn ticks_stay_inside_the_range() {
        for (lo, hi) in [(-1.3, 7.9), (0.011, 0.013), (-5e7, 3e8), (99.0, 101.0)] {
            let t = nice(lo, hi, 6);
            assert!(t.len() >= 3 && t.len() <= 7, "{lo}..{hi}: {t:?}");
            assert!(t.iter_elems().all(|&v| v >= lo && v <= hi));
        }
    }

    #[test]
    fn symmetric_range_has_a_plain_zero() {
        let t = nice(-1.0, 1.0, 4);
        assert_eq!(
            labels(&t, 0.5).data(),
            ["-1.0", "-0.5", "0.0", "0.5", "1.0"]
        );
    }

    #[test]
    fn degenerate_ranges_have_no_ticks() {
        assert!(nice(1.0, 1.0, 5).is_empty());
        assert!(nice(2.0, 1.0, 5).is_empty());
        assert!(nice(0.0, f64::NAN, 5).is_empty());
    }

    #[test]
    fn extreme_values_use_scientific_notation() {
        let big = nice(0.0, 4e9, 4);
        assert_eq!(
            labels(&big, 1e9).data(),
            ["0e0", "1e9", "2e9", "3e9", "4e9"]
        );
        let small = nice(0.0, 3e-5, 3);
        assert_eq!(labels(&small, 1e-5).data()[3], "3e-5");
    }
}
