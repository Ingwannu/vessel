//! Conflict-aware source editing reuses the project-file reader and ordinary file writer.
//!
//! Raw TOML preserves comments and unknown keys. Saving validates the same schema as
//! execution, but has no approval or process effects. Hash preconditions detect external
//! edits without mtime precision; like ordinary file.write, external processes can still
//! race between comparison and rename. The node serializes its own config writes.

use sha2::{Digest as _, Sha256};
use std::{
    path::{Path, PathBuf},
    str,
};
use vessel_proto::{
    registry::{ConfigDocument, ConfigRevision, ConfigText, ConfigTextError, ConfigValidity},
    workspace::{ContentHash, InvalidRelPath, RelPath, WriteExpect},
};

use crate::{
    files,
    paths::{PathError, Root},
    project_file::{self, ProjectFile, ProjectFileError},
};

/// Failures intentionally do not format source bytes or parser diagnostics into messages.
#[derive(Debug, thiserror::Error)]
pub enum ConfigEditError {
    /// The registered root is gone or unreadable.
    #[error("the project directory cannot be read")]
    Root(#[source] PathError),
    /// The existing bounded regular-file reader refused the source.
    #[error("the project configuration cannot be read")]
    Read(#[source] ProjectFileError),
    /// Binary documents cannot be edited as TOML source.
    #[error("the project configuration is not UTF-8 text")]
    NotText,
    /// The wire and filesystem share the same source bound.
    #[error("the project configuration text is invalid")]
    Text(#[source] ConfigTextError),
    /// Validation fails before writing; parser source is not echoed.
    #[error("the project configuration does not match the vessel.toml schema")]
    Invalid,
    /// Ordinary atomic-file replacement or its content precondition failed.
    #[error("the project configuration could not be saved")]
    Write(#[source] files::FileError),
    /// A constant path did not satisfy the shared path type.
    #[error("the project configuration filename is invalid")]
    Name(#[source] InvalidRelPath),
    /// Runtime shutdown/panic is not a successful read or write.
    #[error("the project configuration task stopped")]
    Task(#[source] tokio::task::JoinError),
}

/// Read missing files as an editable empty document; malformed TOML stays repairable.
///
/// # Errors
/// Unavailable root, nonregular/oversized/binary source or stopped blocking task.
pub async fn read(folder: PathBuf) -> Result<ConfigDocument, ConfigEditError> {
    tokio::task::spawn_blocking(move || read_now(&folder))
        .await
        .map_err(ConfigEditError::Task)?
}

fn read_now(folder: &Path) -> Result<ConfigDocument, ConfigEditError> {
    let root = Root::open(folder).map_err(ConfigEditError::Root)?;
    let bytes = project_file::read_bytes(root.path()).map_err(ConfigEditError::Read)?;
    let Some(bytes) = bytes else {
        return Ok(ConfigDocument {
            text: ConfigText::parse("").map_err(ConfigEditError::Text)?,
            revision: ConfigRevision::Missing,
            validity: ConfigValidity::Valid,
        });
    };
    let source = str::from_utf8(&bytes).map_err(|_| ConfigEditError::NotText)?;
    let text = ConfigText::parse(source).map_err(ConfigEditError::Text)?;
    let validity = if ProjectFile::parse(&bytes).is_ok() {
        ConfigValidity::Valid
    } else {
        ConfigValidity::Invalid
    };
    Ok(ConfigDocument {
        text,
        revision: revision(&bytes),
        validity,
    })
}

/// Save only valid source under its exact read revision, preserving unknown fields.
///
/// # Errors
/// Invalid schema, unavailable root, stale content, unsafe path or failed replacement.
pub async fn write(
    folder: PathBuf,
    expected: ConfigRevision,
    text: ConfigText,
) -> Result<ConfigDocument, ConfigEditError> {
    if ProjectFile::parse(text.as_str().as_bytes()).is_err() {
        return Err(ConfigEditError::Invalid);
    }
    let root = tokio::task::spawn_blocking(move || Root::open(&folder))
        .await
        .map_err(ConfigEditError::Task)?
        .map_err(ConfigEditError::Root)?;
    let path: RelPath = project_file::FILE_NAME
        .parse()
        .map_err(ConfigEditError::Name)?;
    let expect = match expected {
        ConfigRevision::Missing => WriteExpect::Absent,
        ConfigRevision::Present { hash } => WriteExpect::Content { hash },
    };
    let _written = files::write(&root, &path, text.as_str().to_owned(), expect)
        .await
        .map_err(ConfigEditError::Write)?;
    Ok(ConfigDocument {
        revision: revision(text.as_str().as_bytes()),
        text,
        validity: ConfigValidity::Valid,
    })
}

fn revision(bytes: &[u8]) -> ConfigRevision {
    ConfigRevision::Present {
        hash: ContentHash::from_sha256(Sha256::digest(bytes).into()),
    }
}

#[cfg(test)]
mod tests;
