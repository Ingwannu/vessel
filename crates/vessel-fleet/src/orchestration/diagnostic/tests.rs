//! The real selection transaction retains typed causes without exposing their input.
use crate::orchestration::{
    SelectionError, SkillBundles, SkillError, SkillLocations, SkillManager,
};
use std::error::Error;
use vessel_proto::skills::{SkillNames, SkillSelection};

const CANARY: &str = "PRIVATE_SELECTION_PERSISTENCE_OR_ROLLBACK";
const MAX_ERROR_CHAIN: usize = 16;

#[cfg(test)]
fn assert_closed(error: &(dyn Error + 'static)) {
    let mut at = error;
    for _level in 0..MAX_ERROR_CHAIN {
        assert!(!format!("{at}").contains(CANARY));
        assert!(!format!("{at:?}").contains(CANARY));
        let Some(next) = at.source() else {
            return;
        };
        at = next;
    }
    panic!("diagnostic source chain did not terminate");
}

#[test]
fn persistence_failure_from_the_actual_save_and_rollback_diagnostics_are_closed() {
    let root = tempfile::tempdir().unwrap();
    let locations = SkillLocations::new(
        root.path().join("shared"),
        root.path().join("claude"),
        root.path().join("codex"),
        root.path().join("state"),
    )
    .unwrap();
    let mut manager = SkillManager::new(SkillBundles::shipped().unwrap(), locations);
    let error = manager
        .save(
            None,
            &SkillSelection::All,
            &SkillNames::default(),
            |_selection| Err(std::io::Error::other(CANARY)),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        SelectionError::Persist { rollback: None, .. }
    ));
    assert_closed(&error);
    let rollback: SelectionError<std::io::Error> = SelectionError::Rollback {
        source: SkillError::from(std::io::Error::other(CANARY)),
        rollback: SkillError::from(std::io::Error::other(CANARY)),
    };
    assert_closed(&rollback);
}
