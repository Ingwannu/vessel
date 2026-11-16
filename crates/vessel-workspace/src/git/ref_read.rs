//! Shared strict ref admission: absence is only show-ref's documented code1.
//! A present non-commit/malformed ref must not become permission to substitute
//! another branch, in either history or comparisons.
use super::{Git, GitError};
use std::path::Path;
use vessel_proto::workspace::{CommitId, InvalidText, RefName};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RefReadError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("git printed a ref object that is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("git printed an invalid ref object")]
    Object(#[from] InvalidText),
}

impl Git {
    pub(crate) async fn exact_branch_commit(
        &self,
        dir: &Path,
        name: &RefName,
    ) -> Result<Option<CommitId>, RefReadError> {
        let exists = self
            .command(dir)
            .args(["show-ref", "--verify", "--quiet", name.as_str()])
            .output()
            .await?;
        if exists.code == Some(1) && !exists.stdout_cut {
            return Ok(None);
        }
        let _exists = exists.checked()?;
        let commit = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{name}^{{commit}}"))
            .output()
            .await?
            .checked()?;
        Ok(Some(
            std::str::from_utf8(&commit.stdout)?
                .trim_end_matches(['\r', '\n'])
                .parse()?,
        ))
    }
}

impl From<RefReadError> for super::ComparisonError {
    fn from(error: RefReadError) -> Self {
        match error {
            RefReadError::Git(error) => Self::Git(error),
            RefReadError::Utf8(error) => Self::Utf8(error),
            RefReadError::Object(error) => Self::Object(error),
        }
    }
}
impl From<RefReadError> for super::HistoryError {
    fn from(error: RefReadError) -> Self {
        match error {
            RefReadError::Git(error) => Self::Git(error),
            RefReadError::Utf8(error) => Self::Utf8(error),
            RefReadError::Object(error) => Self::Value(error),
        }
    }
}
