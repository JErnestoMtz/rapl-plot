use crate::Theme;
use crate::scene::{Anchor, Paint, Prim, Stroke};

/// A standalone SVG document for a scene.
pub(crate) fn write(width: f64, height: f64, theme: &Theme, scene: &[Prim]) -> String {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"{font}\">\n",
        w = num(width),
        h = num(height),
        font = escape(&theme.font_family),
    );
    if theme.background.a > 0 {
        out.push_str(&format!(
            "<rect width=\"100%\" height=\"100%\"{}/>\n",
            paint_attrs("fill", &Paint::solid(theme.background))
        ));
    }
    let mut clips = Clips {
        prefix: fingerprint(scene),
        count: 0,
    };
    for prim in scene {
        prim_svg(prim, &mut clips, &mut out);
    }
    out.push_str("</svg>\n");
    out
}

/// Names for clip paths. Ids are global once an SVG is inlined in an HTML
/// page, so they carry a fingerprint of the figure: two different figures
/// on one page must not pick up each other's clip rectangles.
struct Clips {
    prefix: u32,
    count: usize,
}

fn fingerprint(scene: &[Prim]) -> u32 {
    fnv(format!("{scene:?}").as_bytes())
}

/// FNV-1a, stable across runs and compiler versions.
pub(crate) fn fnv(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

fn prim_svg(prim: &Prim, clips: &mut Clips, out: &mut String) {
    match prim {
        Prim::Path { lines, stroke } => {
            let d: Vec<String> = lines
                .iter()
                .flat_map(|line| {
                    line.iter().enumerate().map(|(i, &(x, y))| {
                        format!("{}{} {}", if i == 0 { 'M' } else { 'L' }, num(x), num(y))
                    })
                })
                .collect();
            if !d.is_empty() {
                out.push_str(&format!(
                    "<path d=\"{}\" fill=\"none\" stroke-linejoin=\"round\"{}/>\n",
                    d.join(""),
                    stroke_attrs(stroke)
                ));
            }
        }
        Prim::Rect {
            x,
            y,
            w,
            h,
            fill,
            stroke,
        } => {
            let fill = match fill {
                Some(paint) => paint_attrs("fill", paint),
                None => " fill=\"none\"".into(),
            };
            let stroke = stroke.as_ref().map(stroke_attrs).unwrap_or_default();
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{fill}{stroke}/>\n",
                num(*x),
                num(*y),
                num(*w),
                num(*h)
            ));
        }
        Prim::Circle { x, y, r, fill } => out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{}/>\n",
            num(*x),
            num(*y),
            num(*r),
            paint_attrs("fill", fill)
        )),
        Prim::Text {
            x,
            y,
            text,
            size,
            color,
            anchor,
            vertical,
            bold,
        } => {
            let anchor = match anchor {
                Anchor::Start => "",
                Anchor::Middle => " text-anchor=\"middle\"",
                Anchor::End => " text-anchor=\"end\"",
            };
            let (x, y) = (num(*x), num(*y));
            let rotate = if *vertical {
                format!(" transform=\"rotate(-90 {x} {y})\"")
            } else {
                String::new()
            };
            let weight = if *bold { " font-weight=\"600\"" } else { "" };
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-size=\"{}\" fill=\"{}\"{anchor}{weight}{rotate}>{}</text>\n",
                num(*size),
                color.css(),
                escape(text)
            ));
        }
        Prim::Clip { x, y, w, h, items } => {
            clips.count += 1;
            let id = format!("clip-{:08x}-{}", clips.prefix, clips.count);
            out.push_str(&format!(
                "<clipPath id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>\n<g clip-path=\"url(#{id})\">\n",
                num(*x),
                num(*y),
                num(*w),
                num(*h)
            ));
            for item in items {
                prim_svg(item, clips, out);
            }
            out.push_str("</g>\n");
        }
    }
}

fn paint_attrs(property: &str, paint: &Paint) -> String {
    let opacity = paint.opacity.clamp(0.0, 1.0) * f64::from(paint.color.a) / 255.0;
    let mut attrs = format!(" {property}=\"{}\"", paint.color.css());
    if opacity < 1.0 {
        attrs.push_str(&format!(" {property}-opacity=\"{}\"", num(opacity)));
    }
    attrs
}

fn stroke_attrs(stroke: &Stroke) -> String {
    let mut attrs = paint_attrs("stroke", &stroke.paint);
    attrs.push_str(&format!(" stroke-width=\"{}\"", num(stroke.width)));
    if let Some(pattern) = stroke.dash.pattern() {
        attrs.push_str(&format!(" stroke-dasharray=\"{pattern}\""));
    }
    attrs
}

/// A coordinate with at most two decimals and no trailing zeros. Values
/// that are not finite would make the document invalid, so they become 0.
fn num(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(0.126), "0.13");
        assert_eq!(num(-0.001), "0");
        assert_eq!(num(100.0), "100");
        assert_eq!(num(f64::NAN), "0");
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(escape("a < b & \"c\""), "a &lt; b &amp; &quot;c&quot;");
    }
}
