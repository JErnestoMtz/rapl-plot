use crate::render::{self, Rect};
use crate::viewer::{VIEWER_VAR, Viewer};
use crate::{Error, Plot, Theme, svg};
use std::path::{Path, PathBuf};

/// A plot with a size and a theme: the thing that gets rendered.
///
/// Any mark or [`Plot`] converts into a figure, and their `size`, `theme`,
/// `to_svg`, `save` and `show` methods go through one, so `Figure` only needs
/// naming when it is stored or passed around.
#[derive(Debug, Clone)]
pub struct Figure {
    plot: Plot,
    width: f64,
    height: f64,
    theme: Theme,
    full_resolution: bool,
    scale: f64,
}

impl<P: Into<Plot>> From<P> for Figure {
    fn from(plot: P) -> Figure {
        Figure {
            plot: plot.into(),
            width: 640.0,
            height: 420.0,
            theme: Theme::default(),
            full_resolution: false,
            scale: 2.0,
        }
    }
}

impl Figure {
    /// Size of the whole image in pixels.
    pub fn size(mut self, width: f64, height: f64) -> Figure {
        self.width = width;
        self.height = height;
        self
    }

    /// Colors and type to draw with.
    pub fn theme(mut self, theme: Theme) -> Figure {
        self.theme = theme;
        self
    }

    /// Draw every point of every line.
    ///
    /// By default a line with many more points than the figure has pixels
    /// is thinned to the points that show: per pixel column the first,
    /// lowest, highest and last, and nothing that lies outside the axes.
    /// The picture is the same apart from anti-aliasing at the edges of
    /// dense regions, and the file is far smaller. Ask for full resolution
    /// when the SVG itself is the data, for example to edit the path in
    /// another program.
    pub fn full_resolution(mut self) -> Figure {
        self.full_resolution = true;
        self
    }

    /// Render to a standalone SVG document.
    ///
    /// This is where shape mismatches and inconsistent axis settings are
    /// reported.
    pub fn to_svg(&self) -> Result<String, Error> {
        let drawable = |side: f64| side.is_finite() && side >= 1.0;
        if !drawable(self.width) || !drawable(self.height) {
            return Err(Error::Figure(format!(
                "figure size {} x {} must be at least 1 x 1 pixels",
                self.width, self.height
            )));
        }
        let area = Rect {
            x: 0.0,
            y: 0.0,
            w: self.width,
            h: self.height,
        };
        let scene = render::plot(&self.plot, area, &self.theme, self.full_resolution)?;
        Ok(svg::write(self.width, self.height, &self.theme, &scene))
    }

    /// Device pixels per figure pixel in PNG output. The default of 2
    /// makes a 640 x 420 figure a 1280 x 840 image, sharp on dense screens.
    /// SVG output does not depend on it.
    pub fn scale(mut self, scale: f64) -> Figure {
        self.scale = scale;
        self
    }

    /// Render to PNG bytes, at [`scale`](Self::scale) times the figure size.
    ///
    /// Text uses the fonts installed on the machine, so it can differ
    /// slightly from one machine to another.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        crate::png::encode(&self.to_svg()?, self.width, self.height, self.scale)
    }

    /// Render and write a file. The extension picks the format: `.svg`, or
    /// `.png` when the crate is built with the `png` feature.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let bytes = match extension.to_ascii_lowercase().as_str() {
            "svg" => self.to_svg()?.into_bytes(),
            #[cfg(feature = "png")]
            "png" => self.to_png()?,
            #[cfg(not(feature = "png"))]
            "png" => {
                return Err(Error::Figure(format!(
                    "{}: PNG output needs rapl-plot's `png` feature",
                    path.display()
                )));
            }
            _ => {
                return Err(Error::Figure(format!(
                    "{}: the extension must be .svg or .png",
                    path.display()
                )));
            }
        };
        Ok(std::fs::write(path, bytes)?)
    }

    /// Render to a temporary file and open it in the system's viewer,
    /// without waiting for the viewer to close.
    ///
    /// The environment variable `RAPL_PLOT_VIEWER` picks another program
    /// (`RAPL_PLOT_VIEWER="firefox --new-window"`), and
    /// `RAPL_PLOT_VIEWER=none` opens nothing, for scripts that also run
    /// where there is no screen.
    pub fn show(&self) -> Result<(), Error> {
        let setting = std::env::var(VIEWER_VAR).ok();
        let file = self.write_temp()?;
        Viewer::from_setting(setting.as_deref()).open(&file)?;
        Ok(())
    }

    /// Write the figure to the temporary directory. The name depends on
    /// the content, so showing a figure again reuses its file.
    fn write_temp(&self) -> Result<PathBuf, Error> {
        let svg = self.to_svg()?;
        let name = format!("rapl-plot-{:08x}.svg", svg::fnv(svg.as_bytes()));
        let file = std::env::temp_dir().join(name);
        std::fs::write(&file, svg)?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line_y;
    use rapl::Ndarr;

    #[test]
    fn the_same_figure_reuses_its_temporary_file() {
        let a = Figure::from(line_y(&Ndarr::from([1.0, 3.0, 2.0])));
        let b = Figure::from(line_y(&Ndarr::from([1.0, 3.0, 2.5])));
        let file = a.write_temp().unwrap();
        assert_eq!(file, a.write_temp().unwrap());
        assert_ne!(file, b.write_temp().unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), a.to_svg().unwrap());
        std::fs::remove_file(file).unwrap();
        std::fs::remove_file(b.write_temp().unwrap()).unwrap();
    }

    #[test]
    fn save_goes_by_the_extension() {
        let figure = Figure::from(line_y(&Ndarr::from([1.0, 3.0, 2.0])));
        let file = std::env::temp_dir().join("rapl-plot-save-test.SVG");
        figure.save(&file).unwrap();
        assert!(std::fs::read_to_string(&file).unwrap().starts_with("<svg"));
        std::fs::remove_file(file).unwrap();

        for name in ["plot", "plot.pdf", "plot.svg.txt"] {
            let Err(Error::Figure(message)) = figure.save(std::env::temp_dir().join(name)) else {
                panic!("{name} should be refused");
            };
            assert!(message.contains(".svg or .png"), "{message}");
        }
    }

    #[cfg(not(feature = "png"))]
    #[test]
    fn png_without_the_feature_is_an_error_not_an_svg() {
        let figure = Figure::from(line_y(&Ndarr::from([1.0, 3.0, 2.0])));
        let file = std::env::temp_dir().join("rapl-plot-no-feature.png");
        let Err(Error::Figure(message)) = figure.save(&file) else {
            panic!("expected an error");
        };
        assert!(message.contains("`png` feature"), "{message}");
        assert!(!file.exists());
    }
}
