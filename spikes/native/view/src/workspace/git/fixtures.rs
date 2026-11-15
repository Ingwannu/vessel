//! The same status input for action and sentence tests, kept in one place.

use vessel_proto::workspace::{GitStatus, StatusEntry};

pub(crate) fn status(files: Vec<StatusEntry>) -> GitStatus {
    GitStatus {
        branch: Some("feature".parse().unwrap()),
        head: None,
        upstream: None,
        base: None,
        operation: None,
        files,
        more: false,
    }
}
