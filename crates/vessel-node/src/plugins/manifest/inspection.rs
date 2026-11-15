//! Read declarations through the existing workspace path/regular-file boundary.
//!
//! This is inspection data, not a prepared executable or source revision. The
//! installer must still own private staging, full artifact identity and trust.

use super::{MAX_MANIFEST_BYTES, Manifest, ManifestError};
use sha2::{Digest as _, Sha256};
use std::{error::Error, fmt};
use vessel_proto::{
    ProtocolVersion, RelPath,
    plugins::{ClientDocument, DocumentError, MAX_DOCUMENT_BYTES},
    workspace::{ContentHash, FileContent},
};
use vessel_workspace::{files, paths::Root};

const MANIFEST_FILE: &str = "vessel-plugin.json";

/// Only the declaration bytes actually decoded are bound here. Other package
/// files/build results are deliberately outside this narrower identity.
#[derive(Debug, PartialEq, Eq)]
struct DeclarationIdentity {
    manifest: ContentHash,
    client: Option<ContentHash>,
}

#[derive(Debug)]
struct Inspection {
    manifest: Manifest,
    document: Option<ClientDocument>,
    identity: DeclarationIdentity,
}

#[derive(Debug, thiserror::Error)]
enum InspectError {
    #[error("plugin declaration could not be read")]
    Read(#[source] PrivateFileError),
    #[error("plugin declaration exceeds its file budget")]
    Capacity,
    #[error("plugin declaration is not complete unchanged UTF-8 text")]
    Incomplete,
    #[error("invalid native plugin manifest")]
    Manifest(#[source] ManifestError),
    #[error("invalid native plugin client declarations")]
    Document(#[source] DocumentError),
    #[error("static plugin declarations cannot register callbacks without a helper")]
    CallbackWithoutHelper,
    #[error("plugin declaration identity changed after inspection")]
    Changed,
    #[error("plugin declaration filename could not be represented")]
    Filename,
}

/// FileError can contain a private node path. Preserve the concrete cause for
/// ownership/debugging without reopening it through public diagnostics/chains.
struct PrivateFileError(files::FileError);
impl fmt::Debug for PrivateFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match &self.0 {
            files::FileError::Path(_) => "path",
            files::FileError::Kind(_, _) => "kind",
            files::FileError::Changed(_) => "changed",
            files::FileError::Exists(_) => "exists",
            files::FileError::NotEmpty(_) => "not_empty",
            files::FileError::TooLarge => "too_large",
            files::FileError::Io(_) => "io",
            files::FileError::Task => "task",
        };
        f.debug_tuple("PrivateFileError").field(&category).finish()
    }
}
impl fmt::Display for PrivateFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("plugin declaration file is unavailable")
    }
}
impl Error for PrivateFileError {}

impl Inspection {
    async fn read(
        root: &Root,
        node: ProtocolVersion,
        client: ProtocolVersion,
    ) -> Result<Self, InspectError> {
        let path: RelPath = MANIFEST_FILE
            .parse()
            .map_err(|_invalid| InspectError::Filename)?;
        let (text, manifest_hash) = read_text(root, &path, MAX_MANIFEST_BYTES).await?;
        let manifest = Manifest::decode(text.as_bytes()).map_err(InspectError::Manifest)?;
        manifest
            .compatible(node, client)
            .map_err(InspectError::Manifest)?;
        let (document, client_hash) = match &manifest.0.client {
            Some(path) => {
                let (text, hash) = read_text(root, &path.relative(), MAX_DOCUMENT_BYTES).await?;
                let document =
                    ClientDocument::decode(text.as_bytes()).map_err(InspectError::Document)?;
                if manifest.0.helpers.is_empty() && !document.methods().is_empty() {
                    return Err(InspectError::CallbackWithoutHelper);
                }
                (Some(document), Some(hash))
            }
            None => (None, None),
        };
        Ok(Self {
            manifest,
            document,
            identity: DeclarationIdentity {
                manifest: manifest_hash,
                client: client_hash,
            },
        })
    }

    /// Re-read rather than treating a timestamp or the earlier inspection as
    /// permission to use declarations that may have changed on disk.
    async fn recheck(
        &self,
        root: &Root,
        node: ProtocolVersion,
        client: ProtocolVersion,
    ) -> Result<(), InspectError> {
        let current = Self::read(root, node, client).await?;
        if current.identity != self.identity {
            return Err(InspectError::Changed);
        }
        Ok(())
    }
}

async fn read_text(
    root: &Root,
    path: &RelPath,
    budget: usize,
) -> Result<(String, ContentHash), InspectError> {
    let limit = u32::try_from(budget).map_err(|_overflow| InspectError::Capacity)?;
    let page = files::read(root, path, 0, Some(limit))
        .await
        .map_err(|cause| InspectError::Read(PrivateFileError(cause)))?;
    let size = usize::try_from(page.version.size).map_err(|_overflow| InspectError::Capacity)?;
    if size > budget {
        return Err(InspectError::Capacity);
    }
    match page.content {
        FileContent::Text { text, next: None } if text.len() == size => {
            let hash = ContentHash::from_sha256(Sha256::digest(text.as_bytes()).into());
            if page.version.hash.as_ref() != Some(&hash) {
                return Err(InspectError::Incomplete);
            }
            Ok((text, hash))
        }
        FileContent::Text { .. } | FileContent::Binary | FileContent::Unknown => {
            Err(InspectError::Incomplete)
        }
    }
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
