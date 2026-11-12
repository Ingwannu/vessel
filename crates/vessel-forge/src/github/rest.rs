//! GitHub's REST answers this provider reads, typed (only the fields it
//! uses; unknown fields are ignored, as GitHub adds them).

use serde::Deserialize;

/// `GET /user`.
#[derive(Debug, Deserialize)]
pub(super) struct User {
    pub(super) login: String,
}

/// One entry of `GET /repos/{repo}/pulls`.
#[derive(Debug, Deserialize)]
pub(super) struct ListedPull {
    pub(super) number: u64,
    pub(super) state: String,
    #[serde(default)]
    pub(super) merged_at: Option<String>,
    pub(super) updated_at: String,
    pub(super) head: Head,
}

/// A pull request's head.
#[derive(Debug, Deserialize)]
pub(super) struct Head {
    pub(super) sha: String,
}

/// `POST /repos/{repo}/pulls`'s answer.
#[derive(Debug, Deserialize)]
pub(super) struct Created {
    pub(super) number: u64,
}

/// An error answer: `message`, and `errors[].message` for a 422.
#[derive(Debug, Default, Deserialize)]
pub(super) struct ErrorAnswer {
    #[serde(default)]
    pub(super) message: String,
    #[serde(default)]
    pub(super) errors: Vec<ErrorItem>,
}

/// One of a 422's errors.
#[derive(Debug, Default, Deserialize)]
pub(super) struct ErrorItem {
    #[serde(default)]
    pub(super) message: Option<String>,
}

impl ErrorAnswer {
    /// Everything it says, joined, for matching and for a person.
    pub(super) fn said(&self) -> String {
        let mut text = self.message.clone();
        for item in &self.errors {
            if let Some(message) = &item.message {
                text.push_str("; ");
                text.push_str(message);
            }
        }
        text
    }
}

/// `PUT /repos/{repo}/pulls/{n}/merge`'s answer.
#[derive(Debug, Deserialize)]
pub(super) struct MergeAnswer {
    #[serde(default)]
    pub(super) merged: bool,
    #[serde(default)]
    pub(super) sha: Option<String>,
}

/// `GET /repos/{repo}/collaborators/{login}/permission`.
#[derive(Debug, Deserialize)]
pub(super) struct Permission {
    pub(super) permission: String,
}
