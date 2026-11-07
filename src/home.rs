//! The user-level data root (`<home>/svg_grid`), resolved at **run time** and
//! cross-platform.
//!
//! Two reasons this lives in one place instead of being spelled out per call site:
//!
//! * **Run time, never compiled in.** A compile-time path would be baked into the
//!   binary, so a copy installed elsewhere would keep reading the build machine's
//!   checkout (`ARCHITECTURE/cli.md` §字体目录解析).
//! * **One home variable is not enough.** Linux and macOS set `HOME`; native Windows
//!   shells (`cmd.exe`, PowerShell) set only `USERPROFILE`. Git Bash / MSYS set both.
//!   The order below is `HOME` first — on Windows that keeps Git Bash and native
//!   shells pointing at the same directory only when `HOME` agrees with `USERPROFILE`,
//!   which the Windows guide (`docs/install-windows.md`) tells the user to arrange.
//!
//! An empty value is ignored on purpose: a stripped environment (`env -i`) that sets
//! `HOME=` must fall through instead of turning the data root into `/svg_grid`.

use std::ffi::OsString;
use std::path::PathBuf;

/// Directory name of the data root under the user's home directory.
pub const DATA_ROOT: &str = "svg_grid";

/// The user's home directory: `HOME`, else `USERPROFILE`.
pub fn home_dir() -> Option<PathBuf> {
    pick(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// The data root: `<home>/svg_grid`, or a relative `svg_grid` when neither variable
/// is set (the caller then reports "no font library / no corpus" instead of guessing).
pub fn data_root() -> PathBuf {
    match home_dir() {
        Some(home) => home.join(DATA_ROOT),
        None => PathBuf::from(DATA_ROOT),
    }
}

/// The candidate order, with empty values skipped. Pure, so the order is testable
/// without touching the process environment.
fn pick(home: Option<OsString>, userprofile: Option<OsString>) -> Option<PathBuf> {
    for candidate in [home, userprofile] {
        match candidate {
            Some(value) if !value.is_empty() => return Some(PathBuf::from(value)),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_wins_so_git_bash_and_unix_agree_with_the_documented_path() {
        assert_eq!(
            pick(Some("/home/u".into()), Some("C:\\Users\\u".into())),
            Some(PathBuf::from("/home/u"))
        );
    }

    #[test]
    fn userprofile_is_the_windows_fallback() {
        assert_eq!(
            pick(None, Some("C:\\Users\\u".into())),
            Some(PathBuf::from("C:\\Users\\u"))
        );
    }

    #[test]
    fn an_empty_home_falls_through_instead_of_rooting_the_data_dir() {
        assert_eq!(
            pick(Some("".into()), Some("C:\\Users\\u".into())),
            Some(PathBuf::from("C:\\Users\\u"))
        );
        assert_eq!(pick(Some("".into()), None), None);
        assert_eq!(pick(None, None), None);
    }
}
