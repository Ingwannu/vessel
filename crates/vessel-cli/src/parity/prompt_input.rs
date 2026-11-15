//! File prompts have the same admission budget as positional prompts, before any RPC.
use crate::error::CliError;
use std::{path::Path, time::Duration};
use tokio::io::AsyncReadExt as _;

/// A CLI prompt must leave room for its JSON envelope within the transport frame.
pub(crate) const MAX_BYTES: usize = 64 * 1024;
const MAX_READ_BYTES: u64 = 64 * 1024 + 1;
const READ_DEADLINE: Duration = Duration::from_secs(30);

pub(crate) async fn file(path: &Path) -> Result<String, CliError> {
    tokio::time::timeout(READ_DEADLINE, async {
        let file = tokio::fs::File::open(path).await?;
        let mut reader = file.take(MAX_READ_BYTES);
        let mut text = String::new();
        let _read = reader.read_to_string(&mut text).await?;
        if text.len() > MAX_BYTES {
            return Err(CliError::Usage("prompt exceeds the CLI input byte budget"));
        }
        Ok(text)
    })
    .await
    .map_err(CliError::ParityPromptReadTimeout)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn a_file_cannot_bypass_the_positional_prompt_budget_or_utf8_contract() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prompt");
        tokio::fs::write(&path, "a".repeat(MAX_BYTES))
            .await
            .unwrap();
        assert_eq!(file(&path).await.unwrap().len(), MAX_BYTES);
        tokio::fs::write(&path, "a".repeat(MAX_BYTES.checked_add(1).unwrap()))
            .await
            .unwrap();
        assert!(matches!(file(&path).await, Err(CliError::Usage(_))));
        tokio::fs::write(&path, [0xff]).await.unwrap();
        assert!(file(&path).await.is_err());
    }
}
