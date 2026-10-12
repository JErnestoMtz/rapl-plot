//! A few small figures showing what the API looks like:
//! `cargo run --example gallery` writes `gallery_*.svg`.

use rapl::*;
use rapl_plot::*;
use std::f64::consts::PI;

fn main() -> Result<(), rapl_plot::Error> {
    series_from_shape()?;
    per_point_channels()?;
    custom_ticks()?;
    gaps_and_errors()
}

/// A `[n, 2, 2]` stack is four curves; labels and dashes are `[2, 2]` and
/// `[2]` arrays that broadcast against them.
fn series_from_shape() -> Result<(), rapl_plot::Error> {
    let f = Ndarr::from_fn([300], |ix| 1.0 + ix[0] as f64 * 0.03);
    let s_db = Ndarr::from_fn([300, 2, 2], |ix| {
        let detune = (f[[ix[0]]] - 5.0) / 0.6;
        let notch = 1.0 / (1.0 + detune * detune);
        let through = ix[1] != ix[2];
        10.0 * if through {
            1.0 - 0.97 * notch
        } else {
            0.03 + 0.9 * notch
        }
        .log10()
            - 0.4 * ix[1] as f64
    });
    let names = Ndarr::new(&["S11", "S12", "S21", "S22"], [2, 2])?;

    let plot = line(&f, &s_db)
        .label(&names)
        .dash([Dash::Solid, Dash::Dashed])
        .title("Shape decides the series")
        .x(axis().label("frequency (GHz)"))
        .y(axis().label("magnitude (dB)"))
        .legend(Legend::BottomRight);
    plot.theme(Theme::dark()).save("gallery_shape.svg")
}

/// `size` and `color` with the same rank as `y` are one value per point.
fn per_point_channels() -> Result<(), rapl_plot::Error> {
    let n = 60;
    let x = Ndarr::from_fn([n], |ix| ix[0] as f64 / n as f64 * 4.0 * PI);
    let y = x.map(|v| v.sin() * (-v / 9.0).exp());
    let size = y.map(|v| 3.0 + 14.0 * v.abs());
    let color = y.map(|&v| if v >= 0.0 { Color::BLUE } else { Color::RED });

    scatter(&x, &y)
        .size(&size)
        .color(&color)
        .opacity(0.75)
        .title("Per-point size and color")
        .save("gallery_points.svg")
}

/// Tick positions are an array; tick labels are any array of `Display`.
fn custom_ticks() -> Result<(), rapl_plot::Error> {
    let x = Ndarr::from_fn([400], |ix| ix[0] as f64 / 399.0 * 2.0 * PI);
    let waves = Ndarr::from_fn([400, 2], |ix| {
        let x = x[[ix[0]]];
        if ix[1] == 0 { x.sin() } else { x.cos() }
    });
    let at = Ndarr::from_fn([5], |ix| ix[0] as f64 * PI / 2.0);

    line(&x, &waves)
        .label(["sin", "cos"])
        .width(2)
        .title("Ticks from arrays")
        .x(axis()
            .ticks(&at)
            .tick_labels(["0", "π/2", "π", "3π/2", "2π"]))
        .y(axis().ticks(5).format(|v| format!("{v:+.1}")))
        .legend(Legend::BottomLeft)
        .save("gallery_ticks.svg")
}

/// Values that are not finite are gaps, not errors. Shape mistakes are
/// reported when the figure is rendered.
fn gaps_and_errors() -> Result<(), rapl_plot::Error> {
    let x = Ndarr::from_fn([200], |ix| -2.0 + ix[0] as f64 * 0.02);
    let y = x.map(|&v| if v.abs() < 0.25 { f64::NAN } else { 1.0 / v });

    line(&x, &y)
        .title("Non-finite values break the line")
        .y(axis().lim(-6.0, 6.0))
        .size(640.0, 300.0)
        .save("gallery_gaps.svg")?;

    let wrong = line(&x, &Ndarr::from([1.0, 2.0, 3.0])).label(["a", "b"]);
    println!("error: {}", wrong.to_svg().unwrap_err());
    Ok(())
}
