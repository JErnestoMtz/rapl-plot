//! End-to-end checks on the rendered SVG: structure, not pixels.

use rapl::*;
use rapl_plot::*;

fn count(svg: &str, needle: &str) -> usize {
    svg.matches(needle).count()
}

#[test]
fn one_path_per_series_and_a_legend_entry_per_label() {
    let x = Ndarr::from_fn([20], |ix| ix[0] as f64);
    let y = Ndarr::from_fn([20, 2, 2], |ix| (ix[0] * (ix[1] * 2 + ix[2] + 1)) as f64);
    let names = Ndarr::new(&["S11", "S12", "S21", "S22"], [2, 2]).unwrap();

    let svg = line(&x, &y)
        .label(&names)
        .x(axis().grid(false))
        .y(axis().grid(false))
        .to_svg()
        .unwrap();

    // Four curves and four legend swatches, plus the axis lines.
    assert_eq!(count(&svg, "<path"), 4 + 4 + 1);
    for name in ["S11", "S12", "S21", "S22"] {
        assert!(svg.contains(&format!(">{name}</text>")), "{name}");
    }
}

#[test]
fn slices_plot_without_copying_first() {
    let x = Ndarr::from_fn([10], |ix| ix[0] as f64);
    let y = Ndarr::from_fn([10, 2, 2], |ix| (ix[0] + ix[1] * 10 + ix[2] * 100) as f64);

    let s21 = y.slice(s![.., 1, 0]).unwrap();
    let svg = scatter(&x, &s21).to_svg().unwrap();
    assert_eq!(count(&svg, "<circle"), 10);
}

#[test]
fn non_finite_points_break_lines_and_drop_markers() {
    let y = Ndarr::from([0.0, 1.0, f64::NAN, 1.0, 0.0, f64::INFINITY, 2.0]);
    let none = axis().grid(false);

    let svg = line_y(&y).x(none.clone()).y(none).to_svg().unwrap();
    let curve = svg.lines().find(|l| l.starts_with("<path")).unwrap();
    assert_eq!(
        count(curve, "M"),
        2,
        "two runs of at least two points: {curve}"
    );

    assert_eq!(count(&scatter_y(&y).to_svg().unwrap(), "<circle"), 5);
}

#[test]
fn scatter_options_vary_per_point() {
    let x = Ndarr::from([0, 1, 2]);
    let y = Ndarr::from([0, 1, 0]);
    let colors = Ndarr::from(vec![Color::RED, Color::GREEN, Color::BLUE]);

    let svg = scatter(&x, &y)
        .size([4, 8, 16])
        .color(&colors)
        .to_svg()
        .unwrap();
    for r in ["r=\"2\"", "r=\"4\"", "r=\"8\""] {
        assert_eq!(count(&svg, r), 1, "{r}");
    }
    assert_eq!(count(&svg, "fill=\"#ff725c\""), 1);
}

#[test]
fn layers_share_axes_and_text_is_escaped() {
    let x = Ndarr::from([0.0, 1.0, 2.0]);
    let plot = (line(&x, &x) + scatter(&x, &x.map(|v| v * 10.0)))
        .title("a < b")
        .x(axis().label("x & y"));
    let svg = plot.to_svg().unwrap();

    assert!(svg.contains("a &lt; b") && svg.contains("x &amp; y"));
    // The y axis covers the larger layer.
    assert!(svg.contains(">20</text>"));
    assert_eq!(count(&svg, "<circle"), 3);
}

#[test]
fn problems_surface_at_render_and_name_the_mark() {
    let x = Ndarr::from([0.0, 1.0, 2.0]);
    let good = line(&x, &x);
    let bad = scatter(&x, &Ndarr::from([1.0, 2.0]));
    let Err(Error::Shape(message)) = (good.clone() + bad).to_svg() else {
        panic!("expected a shape error");
    };
    assert!(
        message.starts_with("scatter #2: x has shape [3]"),
        "{message}"
    );

    let labels = good.label(["a", "b"]).to_svg();
    assert!(matches!(labels, Err(Error::Shape(m)) if m.starts_with("line #1: label")));
}

#[test]
fn figure_size_and_theme() {
    let y = Ndarr::from([1.0, 3.0, 2.0]);
    let svg = Figure::from(line_y(&y))
        .size(300.0, 200.0)
        .theme(Theme::dark())
        .to_svg()
        .unwrap();
    assert!(svg.contains("width=\"300\" height=\"200\""));
    assert!(svg.contains("fill=\"#16181d\""));
}

#[test]
fn a_transparent_background_draws_no_fill() {
    let y = Ndarr::from([1.0, 3.0, 2.0]);
    let theme = Theme {
        background: Color::TRANSPARENT,
        ..Theme::light()
    };
    let svg = Figure::from(line_y(&y).label("y"))
        .theme(theme)
        .to_svg()
        .unwrap();
    assert!(!svg.contains("width=\"100%\""));
    // The legend box, which is filled with the background, is see-through too.
    assert!(svg.contains("fill-opacity=\"0\""));

    let half = Theme {
        background: Color::rgba(255, 255, 255, 128),
        ..Theme::light()
    };
    let svg = Figure::from(line_y(&y)).theme(half).to_svg().unwrap();
    assert!(svg.contains("width=\"100%\" height=\"100%\" fill=\"#ffffff\" fill-opacity=\"0.5\""));
}

#[test]
fn clip_ids_differ_between_figures_and_are_stable() {
    let id = |svg: &str| svg.split("clipPath id=\"").nth(1).unwrap()[..15].to_string();
    let a = line_y(&Ndarr::from([1.0, 3.0, 2.0]));
    let b = line_y(&Ndarr::from([1.0, 3.0, 2.5]));
    assert_eq!(id(&a.to_svg().unwrap()), id(&a.to_svg().unwrap()));
    assert_ne!(id(&a.to_svg().unwrap()), id(&b.to_svg().unwrap()));
}

#[test]
fn bad_numbers_never_reach_the_document() {
    let y = Ndarr::from([1.0, 3.0, 2.0]);
    let svg = line_y(&y)
        .width(f64::NAN)
        .opacity(f64::NAN)
        .to_svg()
        .unwrap();
    assert!(!svg.contains("NaN") && !svg.contains("inf"));

    for (w, h) in [(0.0, 100.0), (100.0, f64::NAN), (-5.0, 5.0)] {
        let figure = Figure::from(line_y(&y)).size(w, h);
        assert!(
            matches!(figure.to_svg(), Err(Error::Figure(_))),
            "{w} x {h}"
        );
    }
}

#[test]
fn long_lines_are_thinned_unless_full_resolution_is_asked_for() {
    let n = 300_000;
    let x = Ndarr::from_fn([n], |ix| ix[0] as f64 / n as f64);
    let y = x.map(|v| (v * 2000.0).sin() * (v * 3.0).cos());
    let figure = Figure::from(line(&x, &y));

    let thinned = figure.to_svg().unwrap();
    let full = figure.clone().full_resolution().to_svg().unwrap();
    let points = |svg: &str| svg.matches('L').count() + svg.matches('M').count();

    assert!(points(&full) >= n);
    assert!(points(&thinned) < 6000, "{} points", points(&thinned));
    assert!(thinned.len() < 100_000, "{} bytes", thinned.len());

    // Zoomed in on a hundredth of the data, the rest is not written at all.
    let zoomed = line(&x, &y).x(axis().lim(0.50, 0.51)).to_svg().unwrap();
    assert!(points(&zoomed) < 4000, "{} points", points(&zoomed));
}

#[test]
fn an_empty_plot_still_renders_axes() {
    let svg = Plot::new().to_svg().unwrap();
    assert!(svg.contains(">0.0</text>") && svg.contains(">1.0</text>"));
}
