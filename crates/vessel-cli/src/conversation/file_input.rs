//! One open file and one chunk buffer, shared by single-send and comparison upload.
use crate::error::CliError;
use bytes::Bytes;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use tokio::io::AsyncReadExt as _;
use vessel_proto::conversation::MimeType;
use vessel_proto::{AttachmentName, BlobChunk};

pub(super) struct FileInput {
    pub(super) name: AttachmentName,
    pub(super) mime: MimeType,
    pub(super) size: u64,
    file: tokio::fs::File,
    path: PathBuf,
    offset: u64,
}
impl FileInput {
    pub(super) async fn open(path: &Path) -> Result<Self, CliError> {
        let not_attachable = |source| CliError::NotAttachable {
            path: path.to_owned(),
            source,
        };
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .parse()
            .map_err(not_attachable)?;
        let mime = super::send::mime(path).parse().map_err(not_attachable)?;
        let unreadable = |source| CliError::Attach {
            path: path.to_owned(),
            source,
        };
        let file = tokio::fs::File::open(path).await.map_err(unreadable)?;
        let size = file.metadata().await.map_err(unreadable)?.len();
        Ok(Self {
            name,
            mime,
            size,
            file,
            path: path.to_owned(),
            offset: 0,
        })
    }
    pub(super) async fn next(&mut self, chunk: u32) -> Result<Option<Bytes>, CliError> {
        if self.offset >= self.size {
            return Ok(None);
        }
        let width = usize::try_from(chunk)
            .unwrap_or(BlobChunk::MAX_LEN)
            .clamp(1, BlobChunk::MAX_LEN);
        let left = usize::try_from(self.size.saturating_sub(self.offset)).unwrap_or(usize::MAX);
        let mut buffer = vec![0_u8; width.min(left)];
        let _filled = self.file.read_exact(&mut buffer).await.map_err(|source| {
            if source.kind() == std::io::ErrorKind::UnexpectedEof {
                CliError::AttachmentChanged(self.path.clone())
            } else {
                CliError::Attach {
                    path: self.path.clone(),
                    source,
                }
            }
        })?;
        self.offset = self
            .offset
            .saturating_add(u64::try_from(buffer.len()).unwrap_or(u64::MAX));
        Ok(Some(Bytes::from(buffer)))
    }
}
