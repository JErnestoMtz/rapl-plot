//! PNG output: the figure's own SVG, rasterised with resvg.

use crate::Error;
use resvg::usvg::fontdb::Database;
use resvg::{tiny_skia, usvg};
use std::sync::{Arc, OnceLock};

/// Largest width or height of a PNG, in pixels.
const MAX_SIDE: f64 = 16_384.0;

/// Sans-serif families to stand in for the generic `sans-serif`, most
/// preferred first. The theme's named families are tried before this.
const SANS: &[&str] = &[
    "Inter",
    "Helvetica Neue",
    "Helvetica",
    "Arial",
    "Segoe UI",
    "DejaVu Sans",
    "Liberation Sans",
    "Noto Sans",
    "Roboto",
    "Ubuntu",
    "Cantarell",
];

/// The system's fonts, scanned once. `None` when there are none.
fn fonts() -> Option<Arc<Database>> {
    static FONTS: OnceLock<Option<Arc<Database>>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut fonts = Database::new();
            fonts.load_system_fonts();
            let family = fallback_family(&fonts)?;
            // usvg falls back to the serif family, so point both at a font
            // that exists; otherwise text silently disappears.
            fonts.set_sans_serif_family(family.clone());
            fonts.set_serif_family(family);
            Some(Arc::new(fonts))
        })
        .clone()
}

/// A family that is installed: the best of [`SANS`], else whatever the
/// first font is.
fn fallback_family(fonts: &Database) -> Option<String> {
    let installed = |name: &str| {
        fonts
            .faces()
            .any(|face| face.families.iter().any(|(family, _)| family == name))
    };
    SANS.iter()
        .find(|name| installed(name))
        .map(|name| name.to_string())
        .or_else(|| Some(fonts.faces().next()?.families.first()?.0.clone()))
}

/// Rasterise an SVG document at `scale` device pixels per SVG pixel.
pub(crate) fn encode(svg: &str, width: f64, height: f64, scale: f64) -> Result<Vec<u8>, Error> {
    let (pixel_width, pixel_height) = ((width * scale).ceil(), (height * scale).ceil());
    let fits = |side: f64| (1.0..=MAX_SIDE).contains(&side);
    if !scale.is_finite() || !fits(pixel_width) || !fits(pixel_height) {
        return Err(Error::Figure(format!(
            "a {width} x {height} figure at scale {scale} is not a PNG of 1 to {MAX_SIDE} pixels per side"
        )));
    }
    let fontdb = fonts().ok_or_else(|| {
        Error::Figure("no fonts are installed, so the text of a PNG cannot be drawn".into())
    })?;
    let options = usvg::Options {
        fontdb,
        ..usvg::Options::default()
    };
    let failed = |what: &dyn std::fmt::Display| Error::Figure(format!("PNG output failed: {what}"));
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| failed(&e))?;
    let mut pixels = tiny_skia::Pixmap::new(pixel_width as u32, pixel_height as u32)
        .ok_or_else(|| failed(&"could not allocate the image"))?;
    let transform = tiny_skia::Transform::from_scale(scale as f32, scale as f32);
    resvg::render(&tree, transform, &mut pixels.as_mut());
    pixels.encode_png().map_err(|e| failed(&e))
}
