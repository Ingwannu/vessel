//! The node owns download policy, so no workspace can supply a download path.

use serde::Serialize;

use super::{Acknowledged, Method, Request, sealed};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DownloadBehavior {
    Deny,
}

#[derive(Debug, Serialize)]
pub(crate) struct DenyDownloads {
    behavior: DownloadBehavior,
}

impl DenyDownloads {
    pub(crate) const fn new() -> Self {
        Self {
            behavior: DownloadBehavior::Deny,
        }
    }
}

request!(DenyDownloads, Acknowledged, SetDownloadBehavior);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_denial_cannot_supply_a_path_or_mutate_another_context() {
        assert_eq!(
            serde_json::to_value(DenyDownloads::new()).unwrap(),
            serde_json::json!({"behavior": "deny"})
        );
    }
}
