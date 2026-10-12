//! Turns a [`Plot`] into scene primitives inside a rectangle.

use crate::axis::Scale;
use crate::marks::{Geom, Series};
use crate::scene::{Anchor, Paint, Prim, Stroke, text_width};
use crate::simplify::simplify;
use crate::{Axis, Color, Error, Legend, Plot, Theme};

/// A rectangle in pixels, origin at the top left.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}

const TICK: f64 = 4.0;
const GAP: f64 = 5.0;

/// Lay `plot` out in `area`. With `full_resolution` every point of every
/// line is drawn; otherwise long lines are thinned to what the pixels show.
pub(crate) fn plot(
    plot: &Plot,
    area: Rect,
    theme: &Theme,
    full_resolution: bool,
) -> Result<Vec<Prim>, Error> {
    let mut cycle = theme.cycle.iter().copied().cycle();
    let mut series = Vec::new();
    for (index, mark) in plot.marks.iter().enumerate() {
        let resolved = mark.resolve(&mut cycle).map_err(|message| {
            Error::Shape(format!("{} #{}: {message}", mark.kind(), index + 1))
        })?;
        series.extend(resolved);
    }

    let default_axis = Axis::default();
    let x_axis = plot.x.as_ref().unwrap_or(&default_axis);
    let y_axis = plot.y.as_ref().unwrap_or(&default_axis);
    let font = theme.font_size;
    let title_size = font * 1.2;
    let pad = font;

    // Vertical extent first: it does not depend on how wide the y tick
    // labels turn out, while the horizontal extent does.
    let title_room = plot.title.as_ref().map_or(0.0, |_| title_size * 1.6);
    let x_label_room = x_axis.label.as_ref().map_or(0.0, |_| font * 1.6);
    let top = area.y + pad + title_room;
    let bottom = area.y + area.h - pad - TICK - GAP - font - x_label_room;
    let height = (bottom - top).max(1.0);
    let y = y_axis.resolve(
        "y",
        extent(series.iter().flat_map(|s| &s.y)),
        ((height / (font * 3.5)) as usize).max(2),
    )?;

    let y_tick_room = y
        .ticks
        .iter()
        .map(|(_, label)| text_width(label, font))
        .fold(0.0, f64::max);
    let y_label_room = y_axis.label.as_ref().map_or(0.0, |_| font * 1.6);
    let left = area.x + pad + y_label_room + y_tick_room + GAP + TICK;
    let right = area.x + area.w - pad * 2.0;
    let width = (right - left).max(1.0);
    let x = x_axis.resolve(
        "x",
        extent(series.iter().flat_map(|s| &s.x)),
        ((width / (font * 6.5)) as usize).max(2),
    )?;

    let frame = Rect {
        x: left,
        y: top,
        w: width,
        h: height,
    };
    let to_x = |value: f64| {
        let t = x.fraction(value);
        left + width * if x_axis.reversed { 1.0 - t } else { t }
    };
    let to_y = |value: f64| {
        let t = y.fraction(value);
        top + height * if y_axis.reversed { t } else { 1.0 - t }
    };

    let mut scene = Vec::new();
    let grid = Stroke::solid(theme.grid, 1.0);
    if x_axis.grid {
        scene.push(Prim::Path {
            lines: ticks_at(&x, &to_x)
                .map(|px| vec![(px, top), (px, bottom)])
                .collect(),
            stroke: grid,
        });
    }
    if y_axis.grid {
        scene.push(Prim::Path {
            lines: ticks_at(&y, &to_y)
                .map(|py| vec![(left, py), (right, py)])
                .collect(),
            stroke: grid,
        });
    }

    scene.push(Prim::Clip {
        x: frame.x,
        y: frame.y,
        w: frame.w,
        h: frame.h,
        items: series
            .iter()
            .flat_map(|s| draw(s, &to_x, &to_y, (!full_resolution).then_some(frame)))
            .collect(),
    });

    let ink = Stroke::solid(theme.foreground, 1.0);
    let mut marks: Vec<Vec<(f64, f64)>> = vec![vec![(left, top), (left, bottom), (right, bottom)]];
    marks.extend(ticks_at(&x, &to_x).map(|px| vec![(px, bottom), (px, bottom + TICK)]));
    marks.extend(ticks_at(&y, &to_y).map(|py| vec![(left - TICK, py), (left, py)]));
    scene.push(Prim::Path {
        lines: marks,
        stroke: ink,
    });

    let text = |x, y, text: &str, anchor| label(x, y, text, anchor, font, theme);
    for (value, label) in &x.ticks {
        let baseline = bottom + TICK + GAP + font * 0.8;
        scene.push(text(to_x(*value), baseline, label, Anchor::Middle));
    }
    for (value, label) in &y.ticks {
        let baseline = to_y(*value) + font * 0.35;
        scene.push(text(left - TICK - GAP, baseline, label, Anchor::End));
    }
    if let Some(label) = &x_axis.label {
        let baseline = area.y + area.h - pad - font * 0.2;
        scene.push(text(left + width / 2.0, baseline, label, Anchor::Middle));
    }
    if let Some(label) = &y_axis.label {
        let mut prim = text(
            area.x + pad + font * 0.9,
            top + height / 2.0,
            label,
            Anchor::Middle,
        );
        if let Prim::Text { vertical, .. } = &mut prim {
            *vertical = true;
        }
        scene.push(prim);
    }
    if let Some(title) = &plot.title {
        let baseline = area.y + pad + title_size * 0.85;
        let mut prim = label(left, baseline, title, Anchor::Start, title_size, theme);
        if let Prim::Text { bold, .. } = &mut prim {
            *bold = true;
        }
        scene.push(prim);
    }

    scene.extend(legend(
        plot.legend.unwrap_or_default(),
        &series,
        frame,
        theme,
    ));
    Ok(scene)
}

/// Plain horizontal text in the theme's foreground color.
fn label(x: f64, y: f64, text: &str, anchor: Anchor, size: f64, theme: &Theme) -> Prim {
    Prim::Text {
        x,
        y,
        text: text.to_string(),
        size,
        color: theme.foreground,
        anchor,
        vertical: false,
        bold: false,
    }
}

/// Pixel positions of an axis' ticks.
fn ticks_at<'a>(
    scale: &'a Scale,
    to_pixel: &'a dyn Fn(f64) -> f64,
) -> impl Iterator<Item = f64> + 'a {
    scale.ticks.iter().map(move |(value, _)| to_pixel(*value))
}

/// Smallest and largest finite value, if there is one.
fn extent<'a>(values: impl Iterator<Item = &'a f64>) -> Option<(f64, f64)> {
    values
        .filter(|v| v.is_finite())
        .fold(None, |acc, &v| match acc {
            None => Some((v, v)),
            Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
        })
}

/// The primitives for one series. Long lines are thinned to what shows
/// inside `thin_to`, when it is given.
fn draw(
    series: &Series,
    to_x: &dyn Fn(f64) -> f64,
    to_y: &dyn Fn(f64) -> f64,
    thin_to: Option<Rect>,
) -> Vec<Prim> {
    let points = series.x.iter().zip(&series.y);
    match &series.geom {
        Geom::Line {
            color,
            width,
            opacity,
            dash,
        } => {
            // Points that are not finite split the line into runs.
            let mut lines = vec![Vec::new()];
            for (&x, &y) in points {
                if x.is_finite() && y.is_finite() {
                    lines
                        .last_mut()
                        .expect("never empty")
                        .push((to_x(x), to_y(y)));
                } else {
                    lines.push(Vec::new());
                }
            }
            if let Some(frame) = thin_to {
                // Room for the stroke, so nothing is cut off at the edges.
                let pad = width.max(0.0) + 2.0;
                let frame = Rect {
                    x: frame.x - pad,
                    y: frame.y - pad,
                    w: frame.w + 2.0 * pad,
                    h: frame.h + 2.0 * pad,
                };
                lines = lines
                    .into_iter()
                    .flat_map(|line| simplify(line, frame))
                    .collect();
            }
            lines.retain(|line| line.len() > 1);
            vec![Prim::Path {
                lines,
                stroke: Stroke {
                    paint: Paint {
                        color: *color,
                        opacity: *opacity,
                    },
                    width: *width,
                    dash: *dash,
                },
            }]
        }
        Geom::Scatter {
            color,
            size,
            opacity,
        } => points
            .enumerate()
            .filter(|(_, (x, y))| x.is_finite() && y.is_finite())
            .map(|(i, (&x, &y))| Prim::Circle {
                x: to_x(x),
                y: to_y(y),
                r: size.get(i).max(0.0) / 2.0,
                fill: Paint {
                    color: color.get(i),
                    opacity: opacity.get(i),
                },
            })
            .collect(),
    }
}

/// A box listing every labelled series, in a corner of `frame`.
fn legend(position: Legend, series: &[Series], frame: Rect, theme: &Theme) -> Vec<Prim> {
    let entries: Vec<(&Series, &str)> = series
        .iter()
        .filter_map(|s| Some((s, s.label.as_deref()?)))
        .collect();
    if entries.is_empty() || position == Legend::Hidden {
        return Vec::new();
    }
    let font = theme.font_size;
    let (inset, pad, swatch, row) = (8.0, 7.0, 22.0, font * 1.5);
    let text_room = entries
        .iter()
        .map(|(_, label)| text_width(label, font))
        .fold(0.0, f64::max);
    let w = pad * 2.0 + swatch + GAP + text_room;
    let h = pad * 2.0 + row * entries.len() as f64;
    let x = match position {
        Legend::TopLeft | Legend::BottomLeft => frame.x + inset,
        _ => frame.x + frame.w - inset - w,
    };
    let y = match position {
        Legend::BottomLeft | Legend::BottomRight => frame.y + frame.h - inset - h,
        _ => frame.y + inset,
    };

    let mut prims = vec![Prim::Rect {
        x,
        y,
        w,
        h,
        fill: Some(Paint {
            color: theme.background,
            opacity: 0.85,
        }),
        stroke: Some(Stroke::solid(theme.grid, 1.0)),
    }];
    for (i, (series, label)) in entries.into_iter().enumerate() {
        let middle = y + pad + row * (i as f64 + 0.5);
        prims.push(match &series.geom {
            Geom::Line {
                color,
                width,
                opacity,
                dash,
            } => Prim::Path {
                lines: vec![vec![(x + pad, middle), (x + pad + swatch, middle)]],
                stroke: Stroke {
                    paint: Paint {
                        color: *color,
                        opacity: *opacity,
                    },
                    width: *width,
                    dash: *dash,
                },
            },
            Geom::Scatter { color, opacity, .. } => Prim::Circle {
                x: x + pad + swatch / 2.0,
                y: middle,
                r: 3.5,
                fill: Paint {
                    color: first(color),
                    opacity: first(opacity),
                },
            },
        });
        let baseline = middle + font * 0.35;
        prims.push(self::label(
            x + pad + swatch + GAP,
            baseline,
            label,
            Anchor::Start,
            font,
            theme,
        ));
    }
    prims
}

/// The value a per-point option shows in the legend.
fn first<T: Clone + Default>(per: &crate::channel::Per<T>) -> T {
    match per {
        crate::channel::Per::One(value) => value.clone(),
        crate::channel::Per::Each(values) => values.first().cloned().unwrap_or_default(),
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}
