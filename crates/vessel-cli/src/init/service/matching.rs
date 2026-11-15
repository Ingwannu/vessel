//! Classify disk templates beside their installer, preserving only saved PATH.
//! It must not infer loaded manager state or execute any text read from a unit.

use super::paths::{Manager, validate_paths, validate_search};
use super::{Node, launchd_plist, systemd_unit};
use std::io::{self, Read};
use std::path::Path;

/// Inspection errors retain their cause without exposing file contents.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileError {
    /// A missing file is handled separately from inaccessible or invalid text.
    #[error("cannot inspect the user service file")]
    Io(#[from] io::Error),
    /// Encoding errors retain the safe cause without owning the file bytes.
    #[error("cannot inspect the user service file: invalid Unicode")]
    Encoding(#[source] std::str::Utf8Error),
    /// The decoder reads only one byte beyond this named cap.
    #[error("service file exceeds {} bytes", ServiceFile::MAX_BYTES)]
    TooLong,
    /// Preserve the host's count-conversion cause.
    #[error("service file length cannot be represented")]
    Length(#[source] std::num::TryFromIntError),
}

/// A read is bounded and never exposes an unclassified environment to the CLI.
pub(crate) struct ServiceFile(String);

impl ServiceFile {
    const MAX_BYTES: u64 = 1_048_576;

    /// Missing is different from an unreadable service; neither creates a file.
    ///
    /// # Errors
    /// The file is inaccessible, invalid UTF-8 or larger than the bound.
    pub(crate) fn read(path: &Path) -> Result<Option<Self>, FileError> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        Self::from_read(file).map(Some)
    }

    fn from_read(input: impl Read) -> Result<Self, FileError> {
        let mut bytes = Vec::new();
        // One extra byte distinguishes an exact cap from excess without
        // reading the rest; saturation keeps the cap arithmetic bounded.
        let count = input
            .take(Self::MAX_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if count > usize::try_from(Self::MAX_BYTES).map_err(FileError::Length)? {
            return Err(FileError::TooLong);
        }
        let text =
            String::from_utf8(bytes).map_err(|error| FileError::Encoding(error.utf8_error()))?;
        Ok(Self(text))
    }

    /// Only the trusted executable and an absolute directory can match a unit.
    /// The undo text is derived from its location, never trusted from its comment.
    pub(crate) fn classify(&self, manager: Manager, file: &Path, exe: &Path, data: &Path) -> Match {
        if !data.is_absolute() || !exe.is_absolute() || validate_paths(exe, data).is_err() {
            return Match::Different;
        }
        let Ok(undo) = manager.undo(file) else {
            return Match::Different;
        };
        let same = match manager {
            Manager::Systemd => systemd_matches(&self.0, exe, data, undo.as_str()),
            Manager::Launchd => launchd_matches(&self.0, exe, data, undo.as_str()),
        };
        if same { Match::Exact } else { Match::Different }
    }
}

/// The CLI consumes a classification, never a service file's executable text.
#[derive(Debug)]
pub(crate) enum Match {
    /// Every byte is the current renderer, with its saved PATH.
    Exact,
    /// An unmatched unit is never an automatic manager target.
    Different,
}

fn systemd_matches(text: &str, exe: &Path, data: &Path, undo: &str) -> bool {
    let Some(quoted) = text
        .lines()
        .find_map(|line| line.strip_prefix("Environment="))
    else {
        return false;
    };
    let Some(path) =
        systemd_unquote(quoted).and_then(|value| value.strip_prefix("PATH=").map(str::to_owned))
    else {
        return false;
    };
    if validate_search(&path).is_err() {
        return false;
    }
    systemd_unit(
        &Node {
            exe: exe.to_owned(),
            data_dir: data.to_owned(),
            path,
        },
        undo,
    ) == text
}

fn launchd_matches(text: &str, exe: &Path, data: &Path, undo: &str) -> bool {
    let Some((_, rest)) = text.split_once("<key>PATH</key><string>") else {
        return false;
    };
    let Some((path, _)) = rest.split_once("</string>") else {
        return false;
    };
    let path = xml_unescape(path);
    if validate_search(&path).is_err() {
        return false;
    }
    launchd_plist(
        &Node {
            exe: exe.to_owned(),
            data_dir: data.to_owned(),
            path,
        },
        undo,
    ) == text
}

fn systemd_unquote(text: &str) -> Option<String> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => out.push(chars.next()?),
            '%' => {
                if chars.next() != Some('%') {
                    return None;
                }
                out.push('%');
            }
            '"' => return None,
            other => out.push(other),
        }
    }
    Some(out)
}

fn xml_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> Node {
        Node {
            exe: std::env::temp_dir().join("my tools/vessel"),
            data_dir: std::env::temp_dir().join("${HOME}/data & \"node\""),
            path: "/a&b<c:/100%/quoted\"bin".to_owned(),
        }
    }

    fn rendered(manager: Manager, node: &Node, file: &Path) -> String {
        let undo = manager.undo(file).unwrap();
        match manager {
            Manager::Systemd => systemd_unit(node, undo.as_str()),
            Manager::Launchd => launchd_plist(node, undo.as_str()),
        }
    }

    #[test]
    fn only_current_templates_for_the_trusted_executable_and_directory_match() {
        let node = node();
        let file = Path::new("/example/it's a unit");
        for manager in [Manager::Systemd, Manager::Launchd] {
            let text = rendered(manager, &node, file);
            assert!(matches!(
                ServiceFile(text.clone()).classify(manager, file, &node.exe, &node.data_dir),
                Match::Exact
            ));
            for (exe, data) in [
                (Path::new("/different/program"), node.data_dir.as_path()),
                (node.exe.as_path(), Path::new("/different/data")),
            ] {
                assert!(matches!(
                    ServiceFile(text.clone()).classify(manager, file, exe, data),
                    Match::Different
                ));
            }
            assert!(matches!(
                ServiceFile(format!("{text}\n")).classify(manager, file, &node.exe, &node.data_dir),
                Match::Different
            ));
            let changed = text.replace("To undo:", "To change:");
            assert!(matches!(
                ServiceFile(changed).classify(manager, file, &node.exe, &node.data_dir),
                Match::Different
            ));
        }
    }

    #[test]
    fn an_older_unconditional_launchd_agent_is_a_mismatch() {
        let node = node();
        let file = Path::new("/example/unit");
        let old = rendered(Manager::Launchd, &node, file)
            .replace("<dict><key>SuccessfulExit</key><false/></dict>", "<true/>");
        assert!(matches!(
            ServiceFile(old).classify(Manager::Launchd, file, &node.exe, &node.data_dir),
            Match::Different,
        ));
    }

    #[test]
    fn relative_or_unrepresentable_paths_never_match_a_manager_target() {
        let mut node = node();
        let file = Path::new("/example/unit");
        let paths = [
            Path::new("relative").to_owned(),
            std::env::temp_dir().join("a\nb"),
        ]
        .into_iter();
        #[cfg(unix)]
        let paths = {
            use std::os::unix::ffi::OsStringExt as _;
            paths.chain(Some(
                std::ffi::OsString::from_vec(b"/example/invalid-\xff".to_vec()).into(),
            ))
        };
        for data in paths {
            node.data_dir = data;
            let text = rendered(Manager::Systemd, &node, file);
            assert!(matches!(
                ServiceFile(text).classify(Manager::Systemd, file, &node.exe, &node.data_dir),
                Match::Different
            ));
        }
    }

    #[test]
    fn reading_is_bounded_and_a_missing_file_is_not_created() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("missing");
        assert!(ServiceFile::read(&file).unwrap().is_none());
        assert!(!file.exists());
        let Err(error) = ServiceFile::from_read(io::repeat(b'x')) else {
            panic!("accepted unlimited input");
        };
        assert!(matches!(error, FileError::TooLong));
        assert!(ServiceFile::from_read(io::repeat(b'a').take(ServiceFile::MAX_BYTES)).is_ok());
        let capped = io::repeat(b'a').take(ServiceFile::MAX_BYTES);
        assert!(matches!(
            ServiceFile::from_read(capped.chain(&b"\xff"[..])),
            Err(FileError::TooLong)
        ));
        assert!(matches!(
            ServiceFile::from_read(&b"\xff"[..]),
            Err(FileError::Encoding(_))
        ));
        assert!(
            error
                .to_string()
                .contains(&ServiceFile::MAX_BYTES.to_string())
        );
    }

    #[test]
    fn an_old_unit_that_expands_the_selected_data_path_is_not_an_exact_template() {
        let node = node();
        let file = Path::new("/example/unit");
        let text = rendered(Manager::Systemd, &node, file).replace("$${HOME}", "${HOME}");
        assert!(matches!(
            ServiceFile(text).classify(Manager::Systemd, file, &node.exe, &node.data_dir),
            Match::Different
        ));
    }
}
