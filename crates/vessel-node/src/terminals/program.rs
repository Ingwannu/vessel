//! Where a terminal profile's program is, and whether it can take its
//! arguments (ADR 0117).
//!
//! [Decision Log]
//! - Purpose: the person reads `<program>: command not found` before
//!   anything starts, not a terminal that ends at once (Paseo's
//!   `assertTerminalCommandCanStart`, `server/src/terminal/terminal.ts`
//!   706-747), and the program checked is the program run.
//! - A program with a folder in it is taken from the terminal's folder; a
//!   bare name only from the absolute entries of the node's `PATH`, which
//!   its terminals inherit. The found path is absolute, so the pseudo-
//!   terminal runs exactly it. Stated difference: Paseo resolves a relative
//!   entry (`.`, `bin`) against the terminal's folder; here it is skipped,
//!   since that folder is one an agent can write to (node review R1). No
//!   `PATH`: nothing is found.
//! - Windows: `PATHEXT` names what a bare name may end in (`claude` is
//!   npm's `claude.cmd`). A `.cmd` or `.bat` is run by cmd.exe, which reads
//!   its command line again: `&`, `%`, quotes and line breaks become syntax.
//!   Paseo builds a caret-escaped `cmd /d /s /c` line for it
//!   (`terminal.ts` 296-411). The pseudo-terminal library here quotes every
//!   argument itself and takes no raw command line, so that line cannot be
//!   handed over. Such a script is run only with arguments cmd.exe leaves
//!   alone, and a launch whose arguments are not plain is refused with the
//!   reason (node review R2); a prompt has spaces, so it needs the
//!   program's `.exe`.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Windows' `PATHEXT` when the node has none.
const DEFAULT_EXTENSIONS: &str = ".COM;.EXE;.BAT;.CMD";

/// How the node looks a program up.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lookup<'a> {
    /// The node's `PATH`.
    pub(crate) search: Option<&'a OsStr>,
    /// The node's `PATHEXT` (Windows).
    pub(crate) extensions: Option<&'a OsStr>,
    /// Whether names take Windows' extensions.
    pub(crate) windows: bool,
}

/// The files `base` may be: itself, then, on Windows, with each extension
/// (itself first only when it has one already).
fn candidates(base: &Path, lookup: Lookup<'_>) -> Vec<PathBuf> {
    if !lookup.windows {
        return vec![base.to_path_buf()];
    }
    let extensions = lookup
        .extensions
        .and_then(OsStr::to_str)
        .unwrap_or(DEFAULT_EXTENSIONS);
    let mut found = Vec::new();
    if base.extension().is_some() {
        found.push(base.to_path_buf());
    }
    for extension in extensions.split(';').filter(|text| !text.is_empty()) {
        let mut named = OsString::from(base.as_os_str());
        named.push(extension);
        found.push(PathBuf::from(named));
    }
    found
}

/// Where `program` is for a terminal in `folder`, absolute; `None` when it
/// is not there or may not be run.
pub(crate) fn find_program(
    program: &str,
    folder: &Path,
    lookup: Lookup<'_>,
    runnable: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() || path.components().nth(1).is_some() {
        let base = folder.join(path);
        return candidates(&base, lookup)
            .into_iter()
            .find(|candidate| candidate.is_absolute() && runnable(candidate));
    }
    std::env::split_paths(lookup.search?)
        .filter(|dir| dir.is_absolute())
        .flat_map(|dir| candidates(&dir.join(path), lookup))
        .find(|candidate| runnable(candidate))
}

/// Whether `program` is a batch script, which Windows runs through cmd.exe.
pub(crate) fn batch_script(program: &Path) -> bool {
    program
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
}

/// Whether cmd.exe hands `arg` to a script as it is: ASCII letters, digits
/// and `-_./:=+@\` only (no space, quote, `%`, `&`, `^` or line break).
pub(crate) fn plain_for_batch(arg: &str) -> bool {
    !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=+@\\".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIX: Lookup<'static> = Lookup {
        search: None,
        extensions: None,
        windows: false,
    };

    fn on(found: &'static [&'static str]) -> impl Fn(&Path) -> bool + Copy {
        move |path: &Path| found.iter().any(|f| path == Path::new(f))
    }

    #[test]
    fn a_bare_name_is_found_only_on_absolute_path_entries() {
        let here = Path::new("/work/site");
        let search = OsString::from("bin:.::/opt/a:/usr/bin");
        let lookup = Lookup {
            search: Some(&search),
            ..UNIX
        };
        // R1: `bin/claude` and `./claude` would be found from the node's
        // folder and run from the terminal's; they are not looked at.
        let found = on(&["bin/claude", "./claude", "claude", "/usr/bin/claude"]);
        assert_eq!(
            find_program("claude", here, lookup, found),
            Some(PathBuf::from("/usr/bin/claude"))
        );
        let relative_only = on(&["bin/claude", "./claude", "claude"]);
        assert_eq!(find_program("claude", here, lookup, relative_only), None);
        // No PATH: not found (Paseo's "command not found", not a spawn error).
        assert_eq!(find_program("claude", here, UNIX, found), None);
    }

    #[test]
    fn a_program_with_a_folder_is_taken_from_the_terminals_folder() {
        let here = Path::new("/work/site");
        let found = on(&["/work/site/bin/tool", "/opt/tools/x"]);
        assert_eq!(
            find_program("bin/tool", here, UNIX, found),
            Some(PathBuf::from("/work/site/bin/tool"))
        );
        assert_eq!(
            find_program("/opt/tools/x", here, UNIX, found),
            Some(PathBuf::from("/opt/tools/x"))
        );
        assert_eq!(find_program("./missing", here, UNIX, found), None);
    }

    #[test]
    fn on_windows_a_bare_name_takes_the_path_extensions_in_order() {
        let here = Path::new("/work/site");
        let search = OsString::from("/node:/usr/bin");
        let extensions = OsString::from(".EXE;.CMD");
        let lookup = Lookup {
            search: Some(&search),
            extensions: Some(&extensions),
            windows: true,
        };
        let shim = on(&["/node/claude.CMD", "/usr/bin/claude.EXE"]);
        // The first entry that has any of them wins, as Windows searches.
        assert_eq!(
            find_program("claude", here, lookup, shim),
            Some(PathBuf::from("/node/claude.CMD"))
        );
        let named = on(&["/node/claude.cmd"]);
        assert_eq!(
            find_program("claude.cmd", here, lookup, named),
            Some(PathBuf::from("/node/claude.cmd"))
        );
        let defaults = Lookup {
            extensions: None,
            ..lookup
        };
        let bat = on(&["/usr/bin/tool.BAT"]);
        assert_eq!(
            find_program("tool", here, defaults, bat),
            Some(PathBuf::from("/usr/bin/tool.BAT"))
        );
    }

    #[test]
    fn a_batch_script_takes_only_arguments_cmd_leaves_alone() {
        assert!(batch_script(Path::new("C:/node/claude.cmd")));
        assert!(batch_script(Path::new("C:/tools/run.BAT")));
        assert!(!batch_script(Path::new("C:/tools/codex.exe")));
        assert!(!batch_script(Path::new("/usr/bin/claude")));
        for plain in ["--yolo", "--model=opus", "C:\\work\\x", "a.b/c:d+e@f"] {
            assert!(plain_for_batch(plain), "{plain}");
        }
        for text in [
            "fix it",
            "\" & calc & \"",
            "%PATH%",
            "a^b",
            "line\nbreak",
            "",
            "한글",
        ] {
            assert!(!plain_for_batch(text), "{text:?}");
        }
    }
}
