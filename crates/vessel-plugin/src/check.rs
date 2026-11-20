//! Which fields of a create spec a hook may replace.
//!
//! `agent.create.before` may replace `title` and `harness`. A result that
//! changes anything else is a refusal, and the value is not copied into
//! the error.

use vessel_proto::SessionSpec;

use crate::error::LockedField;

/// The first locked field `next` changed, in a fixed order.
#[must_use]
pub(crate) fn locked_change(previous: &SessionSpec, next: &SessionSpec) -> Option<LockedField> {
    if previous.cwd != next.cwd {
        return Some(LockedField::Cwd);
    }
    if previous.place != next.place {
        return Some(LockedField::Place);
    }
    if previous.profile != next.profile {
        return Some(LockedField::Profile);
    }
    if previous.mode != next.mode {
        return Some(LockedField::Mode);
    }
    if previous.side_question_of != next.side_question_of {
        return Some(LockedField::Side);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vessel_proto::{HarnessKind, SessionMode, SessionSpec, SessionTitle};

    use super::locked_change;
    use crate::error::LockedField;

    fn spec() -> SessionSpec {
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
    }

    #[test]
    fn title_and_harness_are_not_locked() {
        let mut next = spec();
        next.title = SessionTitle::from_prompt("renamed");
        next.harness = HarnessKind::Codex.into();
        assert_eq!(locked_change(&spec(), &next), None);
    }

    #[test]
    fn a_directory_change_names_the_field_and_not_the_path() {
        let previous = spec().with_cwd(PathBuf::from("/plugin-secret-dir"));
        let next = spec().with_cwd(PathBuf::from("/elsewhere"));
        assert_eq!(locked_change(&previous, &next), Some(LockedField::Cwd));
        assert!(!LockedField::Cwd.name().contains("secret"));
    }
}
