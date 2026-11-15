//! Durable bundled-skill selection with the same downgrade protection as settings.
//! JSON trees stay in this file-decoding edge solely to detect unknown fields.

use std::fs::File;
use std::io::{self, Read as _};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::skills::SkillSelection;

use crate::{DataDir, SetAside, StoreError, files, settings_file};

const FILE: &str = "skills.json";
/// Whole-file cap precedes JSON decoding, including fields from a later build.
const MAX_FILE_BYTES: u64 = 256 * 1024;

#[derive(Default, Serialize, Deserialize)]
struct SelectionFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selection: Option<SkillSelection>,
}

impl DataDir {
    /// Read the explicit selection; absence lets the service distinguish initial
    /// all-selection from an already-saved choice during legacy import.
    ///
    /// # Errors
    /// Unreadable, oversized, linked or malformed files are refused.
    pub fn read_skills_selection(&self) -> Result<Option<SkillSelection>, StoreError> {
        let Some(bytes) = read(self)? else {
            return Ok(None);
        };
        let record: SelectionFile =
            serde_json::from_slice(&bytes).map_err(|source| invalid(self, source))?;
        Ok(record.selection)
    }

    /// Atomically save an owner-only choice, preserving unreadable input or
    /// unknown future fields before the first replacement.
    ///
    /// # Errors
    /// A protective backup or the atomic replacement could not be completed.
    pub fn write_skills_selection(
        &self,
        selection: &SkillSelection,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        protect(self, at)?;
        let record = SelectionFile {
            selection: Some(selection.clone()),
        };
        let bytes = serde_json::to_vec_pretty(&record).map_err(|source| invalid(self, source))?;
        files::replace(&self.root().join(FILE), &bytes)
            .map_err(StoreError::io(self.root().join(FILE)))
    }
}

fn invalid(data: &DataDir, source: serde_json::Error) -> StoreError {
    StoreError::io(data.root().join(FILE))(io::Error::other(source))
}

fn protect(data: &DataDir, at: DateTime<Utc>) -> Result<(), StoreError> {
    let path = data.root().join(FILE);
    match std::fs::symlink_metadata(&path) {
        Ok(info) if info.is_file() && !info.is_symlink() => {}
        Ok(_other) => {
            return Err(StoreError::io(&path)(io::Error::other(
                "skills configuration must be a plain file",
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(StoreError::io(&path)(error)),
    }
    let bytes = match read(data) {
        Ok(None) => return Ok(()),
        Ok(Some(bytes)) => bytes,
        Err(_unreadable) => {
            let _aside = data.set_aside_config(FILE, at, SetAside::Unreadable)?;
            return Ok(());
        }
    };
    let record: SelectionFile = match serde_json::from_slice(&bytes) {
        Ok(record) => record,
        Err(_malformed) => {
            let _aside = data.set_aside_config(FILE, at, SetAside::Unreadable)?;
            return Ok(());
        }
    };
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|source| invalid(data, source))?;
    let known = serde_json::to_value(record).map_err(|source| invalid(data, source))?;
    if !settings_file::covered(&raw, &known) {
        let _aside = data.set_aside_config(FILE, at, SetAside::Later)?;
    }
    Ok(())
}

fn read(data: &DataDir) -> Result<Option<Vec<u8>>, StoreError> {
    let path = data.root().join(FILE);
    let file = match open(&path) {
        Ok(file) => file,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(StoreError::io(&path)(source)),
    };
    if !file.metadata().map_err(StoreError::io(&path))?.is_file() {
        return Err(StoreError::io(&path)(io::Error::other(
            "skills configuration must be a regular file",
        )));
    }
    let mut bytes = Vec::new();
    // One excess byte distinguishes a full valid file from truncation.
    let _read = file
        .take(MAX_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(StoreError::io(&path))?;
    if u64::try_from(bytes.len()).map_or(true, |size| size > MAX_FILE_BYTES) {
        return Err(StoreError::io(&path)(io::Error::other(
            "skills configuration exceeds its byte bound",
        )));
    }
    Ok(Some(bytes))
}

#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<File> {
    use rustix::fs::{CWD, Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    Ok(File::from(rustix::fs::openat(
        CWD,
        path,
        flags,
        Mode::empty(),
    )?))
}
#[cfg(not(unix))]
fn open(path: &std::path::Path) -> io::Result<File> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::other("skills configuration cannot be a link"));
    }
    File::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::skills::SkillNames;

    #[test]
    fn selections_are_atomic_private_and_downgrade_safe() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let at = "2026-11-01T00:00:00Z".parse().unwrap();
        assert_eq!(data.read_skills_selection().unwrap(), None);
        let selection = SkillSelection::Custom {
            skills: SkillNames::new(vec!["vessel".parse().unwrap()]).unwrap(),
        };
        data.write_skills_selection(&selection, at).unwrap();
        assert_eq!(
            data.read_skills_selection().unwrap(),
            Some(selection.clone())
        );
        let path = root.path().join(FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let future = br#"{"selection":{"mode":"all","later":"keep"},"future":true}"#;
        std::fs::write(&path, future).unwrap();
        assert_eq!(
            data.read_skills_selection().unwrap(),
            Some(SkillSelection::All)
        );
        data.write_skills_selection(&selection, at).unwrap();
        let aside = root.path().join("skills.json.later-20261101T000000.000Z");
        assert_eq!(std::fs::read(aside).unwrap(), future);
        std::fs::write(&path, b"malformed").unwrap();
        assert!(data.read_skills_selection().is_err());
        data.write_skills_selection(&selection, at).unwrap();
        assert_eq!(
            std::fs::read(
                root.path()
                    .join("skills.json.unreadable-20261101T000000.000Z")
            )
            .unwrap(),
            b"malformed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_configuration_link_is_never_followed() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"preserve").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join(FILE)).unwrap();
        let data = DataDir::new(root.path());
        assert!(data.read_skills_selection().is_err());
        assert!(
            data.write_skills_selection(
                &SkillSelection::All,
                "2026-11-01T00:00:00Z".parse().unwrap()
            )
            .is_err()
        );
        assert!(
            std::fs::symlink_metadata(root.path().join(FILE))
                .unwrap()
                .is_symlink()
        );
        assert_eq!(std::fs::read(outside.path()).unwrap(), b"preserve");
    }
}
