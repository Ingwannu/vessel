//! Late progress can add content but cannot make settled work appear running again.

use vessel_proto::ToolCallStatus;

pub(super) const fn merge_status(old: ToolCallStatus, incoming: ToolCallStatus) -> ToolCallStatus {
    use ToolCallStatus::{Completed, Failed, InProgress, Pending};
    match (old, incoming) {
        (Failed, Pending | InProgress | Completed | Failed)
        | (Pending | InProgress | Completed, Failed) => Failed,
        (Completed, Pending | InProgress | Completed) | (Pending | InProgress, Completed) => {
            Completed
        }
        (InProgress, Pending | InProgress) | (Pending, InProgress) => InProgress,
        (Pending, Pending) => Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::merge_status;
    use vessel_proto::ToolCallStatus::{Completed, Failed, InProgress, Pending};

    #[test]
    fn terminal_outcomes_survive_late_progress_and_failure_is_sticky() {
        for (old, incoming, expected) in [
            (Pending, InProgress, InProgress),
            (InProgress, Pending, InProgress),
            (Completed, InProgress, Completed),
            (Completed, Pending, Completed),
            (InProgress, Completed, Completed),
            (Failed, Completed, Failed),
            (Failed, InProgress, Failed),
            (Completed, Failed, Failed),
        ] {
            assert_eq!(merge_status(old, incoming), expected);
        }
    }
}
