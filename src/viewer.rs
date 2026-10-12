//! Hands a rendered file to whatever the system uses to look at it.

use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Environment variable that replaces the system viewer: a command (with
/// optional arguments) that is given the file, or `none` to open nothing.
pub(crate) const VIEWER_VAR: &str = "RAPL_PLOT_VIEWER";

/// What `show` does with the file it wrote.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Viewer {
    /// Whatever the operating system opens the file type with.
    System,
    /// A program and its leading arguments; the file is appended.
    Command(Vec<String>),
    /// Leave the file where it is.
    None,
}

impl Viewer {
    /// The viewer named by the value of [`VIEWER_VAR`], if it is set.
    pub(crate) fn from_setting(setting: Option<&str>) -> Viewer {
        let words: Vec<String> = setting
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        match words.as_slice() {
            [] => Viewer::System,
            [only] if only.eq_ignore_ascii_case("none") => Viewer::None,
            _ => Viewer::Command(words),
        }
    }

    fn command(&self) -> Option<Vec<String>> {
        let system: &[&str] = if cfg!(target_os = "macos") {
            &["open"]
        } else if cfg!(windows) {
            // The empty string is the window title `start` expects first.
            &["cmd", "/C", "start", ""]
        } else {
            &["xdg-open"]
        };
        match self {
            Viewer::System => Some(system.iter().map(|word| word.to_string()).collect()),
            Viewer::Command(words) => Some(words.clone()),
            Viewer::None => None,
        }
    }

    /// Start the viewer on `file` without waiting for it to close. The
    /// error names both, so a figure that could not be shown can still be
    /// found.
    pub(crate) fn open(&self, file: &Path) -> io::Result<Option<Child>> {
        let Some(words) = self.command() else {
            return Ok(None);
        };
        Command::new(&words[0])
            .args(&words[1..])
            .arg(file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(Some)
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "could not start the viewer `{}` on {}: {e} (set {VIEWER_VAR} to choose another)",
                        words.join(" "),
                        file.display()
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_selects_the_viewer() {
        assert_eq!(Viewer::from_setting(None), Viewer::System);
        assert_eq!(Viewer::from_setting(Some("  ")), Viewer::System);
        assert_eq!(Viewer::from_setting(Some("None")), Viewer::None);
        assert_eq!(
            Viewer::from_setting(Some("firefox --new-window")),
            Viewer::Command(vec!["firefox".into(), "--new-window".into()])
        );
    }

    #[test]
    fn none_starts_nothing() {
        assert!(
            Viewer::None
                .open(Path::new("anything.svg"))
                .unwrap()
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_the_last_argument() {
        let file = std::env::temp_dir().join("rapl-plot-viewer-test.svg");
        std::fs::write(&file, "<svg/>").unwrap();
        // `test -f FILE` succeeds only if it was handed an existing file.
        let viewer = Viewer::from_setting(Some("test -f"));
        let found = viewer.open(&file).unwrap().unwrap().wait().unwrap();
        assert!(found.success());
        let missing = viewer
            .open(Path::new("/no/such/file.svg"))
            .unwrap()
            .unwrap();
        assert!(!{ missing }.wait().unwrap().success());
        std::fs::remove_file(file).unwrap();
    }

    #[test]
    fn a_missing_viewer_names_the_command_and_the_file() {
        let viewer = Viewer::from_setting(Some("no-such-viewer-rapl-plot"));
        let message = viewer.open(Path::new("plot.svg")).unwrap_err().to_string();
        assert!(message.contains("no-such-viewer-rapl-plot") && message.contains("plot.svg"));
    }
}
