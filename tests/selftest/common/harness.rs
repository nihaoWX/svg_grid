//! Shared helpers: where the binary is, a temporary corpus, and one `doctor`
//! invocation.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The binary a doctor run needs.
///
/// There is exactly **one**: the converter is linked into `svg_grid` as a library
/// (`ARCHITECTURE/cli.md`), so nothing has to be discovered for the first stage.
pub struct Binaries {
    pub svg_grid: PathBuf,
}

impl Binaries {
    /// `svg_grid` comes from Cargo (`CARGO_BIN_EXE_svg_grid`).
    pub fn discover() -> Binaries {
        Binaries {
            svg_grid: PathBuf::from(env!("CARGO_BIN_EXE_svg_grid")),
        }
    }
}

/// The outcome of one `doctor` invocation.
pub struct Report {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// A fresh temporary directory: unique per call, so the tests can run in parallel
/// and a helper may ask for a directory more than once.
pub fn temp_dir(name: &str) -> PathBuf {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "svg-grid-doctor-it-{}-{name}-{call}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a temporary directory must be creatable");
    dir
}

pub fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory");
    }
    fs::write(path, contents).expect("file");
}

/// A tagged panel that paints a bar and an axis: it passes both stages.
const GOOD_PANEL: &str = "\
<svg width=\"100pt\" height=\"80pt\" viewBox=\"0 0 100 80\" xmlns=\"http://www.w3.org/2000/svg\">
<rect data-panel-box=\"main\" x=\"0\" y=\"0\" width=\"100\" height=\"80\" fill=\"none\" stroke=\"none\"/>
<g data-scale=\"xy\"><rect x=\"10\" y=\"10\" width=\"40\" height=\"30\" fill=\"#3366cc\"/>
<line x1=\"10\" y1=\"60\" x2=\"90\" y2=\"60\" stroke=\"#000000\" stroke-width=\"1\"/></g>
</svg>
";

/// A tagged panel whose fill points at an id nothing defines: the composer warns
/// W9 and paints nothing, so `doctor` must FAIL it.
const DANGLING_PANEL: &str = "\
<svg width=\"100pt\" height=\"80pt\" viewBox=\"0 0 100 80\" xmlns=\"http://www.w3.org/2000/svg\">
<rect data-panel-box=\"main\" x=\"0\" y=\"0\" width=\"100\" height=\"80\" fill=\"none\" stroke=\"none\"/>
<g data-scale=\"xy\"><rect x=\"10\" y=\"10\" width=\"40\" height=\"30\" fill=\"url(#nope)\"/></g>
</svg>
";

/// A dialect SVG the converter deliberately refuses: `<text>` under a rotated
/// ancestor cannot be baked without changing its size and orientation.
const ROTATED_TEXT_PANEL: &str = "\
<svg width=\"100pt\" height=\"80pt\" viewBox=\"0 0 100 80\" xmlns=\"http://www.w3.org/2000/svg\">
<rect x=\"0\" y=\"0\" width=\"100\" height=\"80\" fill=\"#ffffff\"/>
<g transform=\"translate(14 87) rotate(-90)\"><text x=\"0\" y=\"0\" font-size=\"10\" \
fill=\"#000000\">y label</text></g>
</svg>
";

/// Build a corpus in `dir` and return it.
///
/// `with_bad` adds the W9 fixture; the `unsupported` one is always registered (so
/// every corpus exercises the SKIP path with its printed `why`).
pub fn corpus(name: &str, with_bad: bool) -> PathBuf {
    let dir = temp_dir(name);
    let mut manifest = String::from(
        "# Integration-test corpus (built by tests/selftest/doctor).\n\
         [[fixture]]\n\
         id = \"ggplot2/ok\"\n\
         package = \"ggplot2\"\n\
         expect = \"pass\"\n\
         \n\
         [[fixture]]\n\
         id = \"ggplot2/ok2\"\n\
         package = \"ggplot2\"\n\
         expect = \"pass\"\n",
    );
    write(&dir.join("ggplot2/ok.svg"), GOOD_PANEL);
    write(&dir.join("ggplot2/ok2.svg"), GOOD_PANEL);
    if with_bad {
        manifest.push_str(
            "[[fixture]]\n\
             id = \"matplotlib/dangling\"\n\
             package = \"matplotlib\"\n\
             expect = \"pass\"\n\
             \n",
        );
        write(&dir.join("matplotlib/dangling.svg"), DANGLING_PANEL);
    }
    manifest.push_str(
        "[[fixture]]\n\
         id = \"matplotlib/rotated_text\"\n\
         package = \"matplotlib\"\n\
         expect = \"unsupported\"\n\
         why = \"<text> under a rotated ancestor <g transform=\\\"translate(...) rotate(-90)\\\">\"\n",
    );
    write(&dir.join("matplotlib/rotated_text.svg"), ROTATED_TEXT_PANEL);
    write(&dir.join("MANIFEST.toml"), &manifest);
    dir
}

/// Run `doctor` with the given arguments.
///
/// `PATH` is replaced by an **empty temporary directory**: the run must not depend
/// on — or accidentally pick up — any installed binary, and the converter is a
/// library call, so an empty `PATH` cannot make it fail. The rest of the
/// environment (notably `SVG_GRID_FONT_DIR`) is inherited: the font library is the
/// caller's business, not the test's.
pub fn doctor(bins: &Binaries, args: &[&str]) -> Report {
    run(&bins.svg_grid, args, &temp_dir("path"))
}

/// Run `doctor` with an empty `PATH` — the same thing every `doctor` run does here,
/// named explicitly for the test that asserts `doctor` has no external dependency.
pub fn doctor_with_empty_path(bins: &Binaries, args: &[&str]) -> Report {
    run(&bins.svg_grid, args, &temp_dir("empty-path"))
}

fn run(binary: &Path, args: &[&str], path: &Path) -> Report {
    let output = Command::new(binary)
        .arg("doctor")
        .args(args)
        .env("PATH", path)
        .output()
        .expect("the svg_grid binary must be runnable");
    Report {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Whether a font library is present: rendering a PNG needs glyphs, and an empty
/// library is exit 3 by design, so the PNG-dependent assertions SKIP instead of
/// failing on a machine that never ran `fontlib/setup.sh`.
pub fn font_library_available() -> bool {
    let dir = match std::env::var_os("SVG_GRID_FONT_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join("svg_grid").join("fontlib"),
            None => return false,
        },
    };
    has_font(&dir)
}

fn has_font(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_font(&path) {
                return true;
            }
            continue;
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(extension.as_str(), "ttf" | "otf" | "ttc") {
            return true;
        }
    }
    false
}

/// Run `svg_grid` with the given arguments (used for the `--help` checks).
pub fn svg_grid(bins: &Binaries, args: &[&str]) -> Report {
    let output = Command::new(&bins.svg_grid)
        .args(args)
        .output()
        .expect("the svg_grid binary must be runnable");
    Report {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The `summary:` line of a report, with the surrounding blanks trimmed.
pub fn summary(stdout: &str) -> String {
    stdout
        .lines()
        .find(|line| line.starts_with("summary: "))
        .unwrap_or("<no summary line>")
        .trim()
        .to_string()
}

/// The per-fixture lines (`PASS|FAIL|SKIP  …`).
pub fn verdict_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| {
            line.starts_with("PASS ") || line.starts_with("FAIL ") || line.starts_with("SKIP ")
        })
        .map(|line| line.trim_end().to_string())
        .collect()
}
