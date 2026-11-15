//! Image send reuses the existing chunk uploader after validating every local input.
use crate::{
    conversation::{FileInput, upload_file},
    error::CliError,
};
use std::{path::PathBuf, time::Duration};
use vessel_client::NodeClient;
use vessel_proto::{AttachmentKind, ContentBlock, SessionId, conversation::AttachmentLimits};

const STAGING_DEADLINE: Duration = Duration::from_secs(120);
pub(super) struct Images(Vec<FileInput>);
impl Images {
    pub(super) async fn open(paths: &[PathBuf]) -> Result<Self, CliError> {
        let maximum = usize::try_from(AttachmentLimits::DEFAULT.per_prompt).map_err(|_range| {
            CliError::Usage("image count limit is unavailable on this platform")
        })?;
        if paths.len() > maximum {
            return Err(CliError::Usage("too many images for one prompt"));
        }
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            if !tokio::fs::metadata(path).await?.is_file() {
                return Err(CliError::Usage("--image requires a regular file"));
            }
            let file = FileInput::open(path).await?;
            if AttachmentKind::of(&file.mime) != AttachmentKind::Image {
                return Err(CliError::Usage(
                    "--image accepts png, jpg/jpeg, gif or webp files",
                ));
            }
            files.push(file);
        }
        let images = Self(files);
        images.check(&AttachmentLimits::DEFAULT)?;
        Ok(images)
    }
    fn check(&self, limits: &AttachmentLimits) -> Result<(), CliError> {
        let count =
            u32::try_from(self.0.len()).map_err(|_range| CliError::Usage("too many images"))?;
        if count > limits.per_prompt {
            return Err(CliError::Usage(
                "too many images for the node's prompt limit",
            ));
        }
        let mut bytes = 0_u64;
        for file in &self.0 {
            if file.size == 0 || file.size > limits.image {
                return Err(CliError::Usage(
                    "image is empty or exceeds the node's image byte limit",
                ));
            }
            bytes = bytes
                .checked_add(file.size)
                .ok_or(CliError::Usage("image sizes overflow the prompt budget"))?;
        }
        if bytes > limits.inline_per_prompt || bytes > limits.per_session {
            return Err(CliError::Usage(
                "images exceed the node's inline/session byte limit",
            ));
        }
        Ok(())
    }
    pub(super) async fn stage(
        self,
        client: &NodeClient,
        session: SessionId,
    ) -> Result<Vec<ContentBlock>, CliError> {
        if self.0.is_empty() {
            return Ok(Vec::new());
        }
        tokio::time::timeout(STAGING_DEADLINE, async {
            let conversation = client.conversation(session).await?;
            if !conversation
                .abilities
                .is_some_and(|abilities| abilities.image)
            {
                return Err(CliError::ParityBlocked(
                    "the target agent has not advertised image input support",
                ));
            }
            self.check(&conversation.limits)?;
            let mut content = Vec::with_capacity(self.0.len());
            for file in self.0 {
                let attachment = upload_file(client, session, file).await?;
                content.push(ContentBlock::Attachment { attachment });
            }
            Ok(content)
        })
        .await
        .map_err(|source| CliError::ParityImageTimeout { session, source })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unsupported_empty_and_oversized_files_refuse_without_any_node_call() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("note.txt");
        tokio::fs::write(&text, "text").await.unwrap();
        assert!(Images::open(&[text]).await.is_err());
        let image = dir.path().join("image.png");
        tokio::fs::write(&image, b"").await.unwrap();
        assert!(Images::open(std::slice::from_ref(&image)).await.is_err());
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&image)
            .await
            .unwrap();
        file.set_len(AttachmentLimits::DEFAULT.image.checked_add(1).unwrap())
            .await
            .unwrap();
        assert!(Images::open(&[image]).await.is_err());
    }
}
