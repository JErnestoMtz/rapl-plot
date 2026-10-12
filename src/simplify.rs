//! Thins out long polylines without visibly changing what they draw.
//!
//! A line with far more points than the figure has pixels is mostly
//! redundant: many points land in the same pixel column and only their
//! extremes show. Everything here works on runs of pixel coordinates.

use crate::render::Rect;

/// Width of a reduction column in pixels. Half a pixel, so the result still
/// holds at twice the size it was made for.
const COLUMN: f64 = 0.5;

/// Points nearer than this to the last one kept are dropped from a curve
/// that doubles back in x, in pixels.
const TOLERANCE: f64 = 0.25;

pub(crate) type Run = Vec<(f64, f64)>;

/// Reduce one run of connected points to the runs worth drawing inside
/// `frame`, which is already grown by the stroke width.
///
/// Runs with at most four points per column are returned untouched. Longer
/// ones lose the stretches that lie outside the frame, and then, per
/// stretch:
///
/// - if x only increases or only decreases, each column keeps its first,
///   lowest, highest and last point (the M4 reduction), which covers the
///   same pixels as the full line, give or take anti-aliasing;
/// - otherwise only points within [`TOLERANCE`] of the last kept point are
///   dropped.
pub(crate) fn simplify(run: Run, frame: Rect) -> Vec<Run> {
    let budget = 4.0 * (frame.w / COLUMN).ceil();
    if run.len() as f64 <= budget {
        return vec![run];
    }
    visible(&run, frame)
        .into_iter()
        .map(|piece| {
            if monotonic(&piece) {
                extremes_per_column(&piece, frame.x)
            } else {
                drop_near(&piece)
            }
        })
        .collect()
}

/// The stretches of `run` whose segments can touch the frame. A segment is
/// kept when its bounding box overlaps the frame, so the line still enters
/// and leaves at the right angle.
fn visible(run: &[(f64, f64)], frame: Rect) -> Vec<Run> {
    let (right, bottom) = (frame.x + frame.w, frame.y + frame.h);
    let touches = |a: (f64, f64), b: (f64, f64)| {
        a.0.max(b.0) >= frame.x
            && a.0.min(b.0) <= right
            && a.1.max(b.1) >= frame.y
            && a.1.min(b.1) <= bottom
    };
    let mut pieces: Vec<Run> = Vec::new();
    let mut open = false;
    for pair in run.windows(2) {
        if touches(pair[0], pair[1]) {
            if !open {
                pieces.push(vec![pair[0]]);
            }
            pieces.last_mut().expect("just pushed").push(pair[1]);
        }
        open = touches(pair[0], pair[1]);
    }
    pieces
}

fn monotonic(run: &[(f64, f64)]) -> bool {
    run.windows(2).all(|p| p[0].0 <= p[1].0) || run.windows(2).all(|p| p[0].0 >= p[1].0)
}

/// M4: the first, lowest, highest and last point of every column, in their
/// original order. Needs x to be monotonic.
fn extremes_per_column(run: &[(f64, f64)], origin: f64) -> Run {
    let column = |point: &(f64, f64)| ((point.0 - origin) / COLUMN).floor();
    let mut kept = Vec::new();
    for group in run.chunk_by(|a, b| column(a) == column(b)) {
        if group.len() <= 4 {
            kept.extend_from_slice(group);
            continue;
        }
        let by_y = |a: &usize, b: &usize| group[*a].1.total_cmp(&group[*b].1);
        let lowest = (0..group.len()).min_by(by_y).expect("not empty");
        let highest = (0..group.len()).max_by(by_y).expect("not empty");
        let mut picks = [0, lowest, highest, group.len() - 1];
        picks.sort_unstable();
        let mut last = None;
        for pick in picks {
            if last != Some(pick) {
                kept.push(group[pick]);
            }
            last = Some(pick);
        }
    }
    kept
}

/// Drop points that sit on top of the last one kept; the end points stay.
fn drop_near(run: &[(f64, f64)]) -> Run {
    let mut kept = vec![run[0]];
    for &point in &run[1..run.len() - 1] {
        let last = kept[kept.len() - 1];
        if (point.0 - last.0).hypot(point.1 - last.1) >= TOLERANCE {
            kept.push(point);
        }
    }
    kept.push(run[run.len() - 1]);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const FRAME: Rect = Rect {
        x: 50.0,
        y: 20.0,
        w: 500.0,
        h: 300.0,
    };

    /// Repeatable noise in `[0, 1)`.
    fn noise(i: usize) -> f64 {
        let x = (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 11;
        x as f64 / (1u64 << 53) as f64
    }

    /// `n` points crossing the frame left to right with noisy heights.
    fn noisy(n: usize) -> Run {
        (0..n)
            .map(|i| {
                (
                    50.0 + 500.0 * i as f64 / (n - 1) as f64,
                    20.0 + 300.0 * noise(i),
                )
            })
            .collect()
    }

    /// Lowest and highest y in every reduction column.
    fn column_extremes(run: &[(f64, f64)]) -> BTreeMap<i64, (f64, f64)> {
        let mut columns = BTreeMap::new();
        for &(x, y) in run {
            let column = ((x - FRAME.x) / COLUMN).floor() as i64;
            let entry = columns.entry(column).or_insert((y, y));
            *entry = (entry.0.min(y), entry.1.max(y));
        }
        columns
    }

    #[test]
    fn short_runs_are_left_alone() {
        let run = noisy(4000);
        assert_eq!(simplify(run.clone(), FRAME), [run]);
    }

    #[test]
    fn long_runs_keep_every_column_extreme() {
        let run = noisy(300_000);
        let reduced = simplify(run.clone(), FRAME);
        assert_eq!(reduced.len(), 1);
        let reduced = &reduced[0];

        assert!(reduced.len() <= 4 * 1001, "{} points left", reduced.len());
        assert_eq!(
            (reduced[0], reduced[reduced.len() - 1]),
            (run[0], run[run.len() - 1])
        );
        assert_eq!(column_extremes(reduced), column_extremes(&run));
        assert!(monotonic(reduced));
    }

    #[test]
    fn decreasing_x_is_reduced_too() {
        let mut run = noisy(300_000);
        run.reverse();
        let reduced = &simplify(run.clone(), FRAME)[0];
        assert!(reduced.len() <= 4 * 1001);
        assert_eq!(column_extremes(reduced), column_extremes(&run));
    }

    #[test]
    fn stretches_outside_the_frame_are_dropped() {
        // Forty frame widths of data, of which the frame shows one.
        let run: Run = (0..400_000)
            .map(|i| (-10_000.0 + i as f64 * 0.05, 20.0 + 300.0 * noise(i)))
            .collect();
        let reduced = simplify(run.clone(), FRAME);
        assert_eq!(reduced.len(), 1);
        let (first, last) = (reduced[0][0], reduced[0][reduced[0].len() - 1]);
        // One neighbour outside on each side, so the line reaches the edges.
        assert!(first.0 < 50.0 && first.0 > 49.9, "{first:?}");
        assert!(last.0 > 550.0 && last.0 < 550.1, "{last:?}");
        assert!(reduced[0].len() <= 4 * 1002);
    }

    #[test]
    fn a_segment_that_only_crosses_the_frame_survives() {
        let mut run: Run = (0..10_000)
            .map(|i| (-1000.0 + i as f64 * 0.01, 100.0))
            .collect();
        run.extend((0..10_000).map(|i| (700.0 + i as f64 * 0.01, 100.0)));
        // No point is inside, but the jump from -900.01 to 700 passes through.
        assert_eq!(
            simplify(run, FRAME),
            [vec![(-900.01, 100.0), (700.0, 100.0)]]
        );
    }

    #[test]
    fn curves_that_double_back_keep_their_shape() {
        // A circle traced with far more points than it has pixels.
        let n = 200_000;
        let run: Run = (0..n)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / (n - 1) as f64;
                (300.0 + 100.0 * angle.cos(), 170.0 + 100.0 * angle.sin())
            })
            .collect();
        let reduced = &simplify(run.clone(), FRAME)[0];

        assert!(reduced.len() < 3000, "{} points left", reduced.len());
        assert_eq!(
            (reduced[0], reduced[reduced.len() - 1]),
            (run[0], run[n - 1])
        );
        // Still a circle: every kept point is on it and no gap is long.
        assert!(
            reduced
                .iter()
                .all(|p| ((p.0 - 300.0).hypot(p.1 - 170.0) - 100.0).abs() < 1e-9)
        );
        assert!(
            reduced
                .windows(2)
                .all(|p| (p[0].0 - p[1].0).hypot(p[0].1 - p[1].1) < 0.3)
        );
    }

    #[test]
    fn leaving_and_returning_splits_the_run() {
        // Left to right through the frame, far away, then back through it.
        let n = 20_000;
        let mut run: Run = (0..n).map(|i| (i as f64 * 0.05, 100.0)).collect();
        run.extend((0..n).map(|i| (1000.0, 100.0 + i as f64)));
        run.extend((0..n).map(|i| (1000.0 - i as f64 * 0.05, 200.0)));
        let reduced = simplify(run, FRAME);
        assert_eq!(reduced.len(), 2);
        assert!(reduced[0].iter().all(|p| p.1 == 100.0));
        assert!(reduced[1].iter().all(|p| p.1 == 200.0));
    }
}
