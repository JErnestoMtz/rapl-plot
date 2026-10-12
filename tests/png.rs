//! PNG output, checked on decoded pixels rather than exact bytes: text
//! depends on the fonts installed where the tests run.
#![cfg(feature = "png")]

use rapl::*;
use rapl_plot::*;
use resvg::tiny_skia::Pixmap;

fn figure() -> Figure {
    let x = Ndarr::from_fn([50], |ix| ix[0] as f64 * 0.2);
    Figure::from(line(&x, &x.map(|v| v.sin())).label("sin").title("PNG")).size(300.0, 200.0)
}

/// Pixels that are not the color of the top-left corner.
fn inked(image: &Pixmap) -> usize {
    let background = image.pixels()[0];
    image.pixels().iter().filter(|&&p| p != background).count()
}

#[test]
fn size_follows_the_scale() {
    let default = Pixmap::decode_png(&figure().to_png().unwrap()).unwrap();
    assert_eq!((default.width(), default.height()), (600, 400));

    let plain = Pixmap::decode_png(&figure().scale(1.0).to_png().unwrap()).unwrap();
    assert_eq!((plain.width(), plain.height()), (300, 200));
    assert!(inked(&plain) > 500, "only {} pixels drawn", inked(&plain));
}

#[test]
fn text_is_drawn() {
    // The same figure without any text has visibly fewer inked pixels.
    let x = Ndarr::from([0.0, 1.0]);
    let bare = axis().ticks([f64::NAN]).grid(false);
    let silent = line(&x, &x).x(bare.clone()).y(bare.clone());
    let titled = line(&x, &x)
        .x(bare.clone())
        .y(bare)
        .title("A title with many letters");
    let count = |plot: Plot| inked(&Pixmap::decode_png(&plot.to_png().unwrap()).unwrap());
    let (without, with) = (count(silent), count(titled));
    assert!(
        with > without + 500,
        "{with} inked with a title, {without} without"
    );
}

#[test]
fn transparent_backgrounds_stay_transparent() {
    let theme = Theme {
        background: Color::TRANSPARENT,
        ..Theme::light()
    };
    let image = Pixmap::decode_png(&figure().theme(theme).to_png().unwrap()).unwrap();
    assert_eq!(image.pixels()[0].alpha(), 0);
    assert!(image.pixels().iter().any(|p| p.alpha() == 255));

    let opaque = Pixmap::decode_png(&figure().to_png().unwrap()).unwrap();
    assert!(opaque.pixels().iter().all(|p| p.alpha() == 255));
}

#[test]
fn save_writes_a_png_for_a_png_extension() {
    let file = std::env::temp_dir().join("rapl-plot-save-test.png");
    figure().save(&file).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    std::fs::remove_file(file).unwrap();
}

#[test]
fn impossible_sizes_are_errors() {
    for scale in [0.0, -1.0, f64::NAN, 1000.0] {
        let result = figure().scale(scale).to_png();
        assert!(matches!(result, Err(Error::Figure(_))), "scale {scale}");
    }
}

#[test]
fn thinning_does_not_change_the_picture() {
    // 400 000 noisy points: thinned and full renderings should agree on
    // almost every pixel (anti-aliasing at column borders may differ).
    let n = 400_000;
    let x = Ndarr::from_fn([n], |ix| ix[0] as f64 / n as f64);
    let y = x.map(|v| (v * 9000.0).sin() * (v * 7.0).cos() + (v * 311.0).sin());
    let figure = Figure::from(line(&x, &y)).scale(1.0);

    let thinned = Pixmap::decode_png(&figure.to_png().unwrap()).unwrap();
    let full = Pixmap::decode_png(&figure.full_resolution().to_png().unwrap()).unwrap();
    let channel_gap = |a: u8, b: u8| a.abs_diff(b);
    let differing = thinned
        .pixels()
        .iter()
        .zip(full.pixels())
        .filter(|(a, b)| {
            channel_gap(a.red(), b.red()) > 48
                || channel_gap(a.green(), b.green()) > 48
                || channel_gap(a.blue(), b.blue()) > 48
        })
        .count();
    let total = thinned.pixels().len();
    assert!(
        differing * 200 < total,
        "{differing} of {total} pixels differ"
    );
}
