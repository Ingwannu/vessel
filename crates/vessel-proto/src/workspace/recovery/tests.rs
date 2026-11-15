//! Future values are preserved as explicit unsupported states, never default actions.
use super::*;
use crate::ErrorCode;

#[test]
fn future_actions_states_and_restore_results_remain_explicit() {
    let inspection = serde_json::from_str::<WorkspaceRecoveryState>(
        r#"{"state":"recoverable","action":"future_action"}"#,
    )
    .unwrap();
    assert_eq!(
        inspection,
        WorkspaceRecoveryState::Recoverable {
            action: WorkspaceRecoveryAction::Unknown
        }
    );
    assert_eq!(
        serde_json::from_str::<WorkspaceRecoveryState>(r#"{"state":"future_state"}"#).unwrap(),
        WorkspaceRecoveryState::Unknown
    );
    assert_eq!(
        serde_json::from_str::<WorkspaceRestoreOutcome>(r#"{"outcome":"future_outcome"}"#).unwrap(),
        WorkspaceRestoreOutcome::Unknown
    );
    assert_eq!(
        serde_json::from_str::<WorkspaceRecoveryUnavailable>(r#""future_reason""#).unwrap(),
        WorkspaceRecoveryUnavailable::Unknown
    );
    assert_eq!(
        serde_json::from_str::<WorkspaceRecoveryStage>(r#""future_stage""#).unwrap(),
        WorkspaceRecoveryStage::Unknown
    );
}

#[test]
fn inspection_preserves_identity_and_every_known_unavailable_reason() {
    let workspace = WorkspaceId::random();
    for reason in [
        WorkspaceRecoveryUnavailable::WorkspaceNotFound,
        WorkspaceRecoveryUnavailable::WorkspaceNotArchived,
        WorkspaceRecoveryUnavailable::ProjectNotFound,
        WorkspaceRecoveryUnavailable::WorkspaceDirectoryMissing,
        WorkspaceRecoveryUnavailable::ProjectDirectoryMissing,
        WorkspaceRecoveryUnavailable::UnsupportedBacking,
        WorkspaceRecoveryUnavailable::UncertainStarts,
    ] {
        let inspection = WorkspaceRecoveryInspection {
            workspace,
            state: WorkspaceRecoveryState::Unavailable { reason },
        };
        assert_eq!(
            serde_json::from_slice::<WorkspaceRecoveryInspection>(
                &serde_json::to_vec(&inspection).unwrap()
            )
            .unwrap(),
            inspection
        );
    }
    let request = InspectWorkspaceRecovery { workspace };
    assert_eq!(
        serde_json::from_slice::<InspectWorkspaceRecovery>(&serde_json::to_vec(&request).unwrap())
            .unwrap(),
        request
    );
}

#[test]
fn restore_keeps_stage_and_source_taxonomy_without_becoming_a_new_session_launch() {
    let workspace = WorkspaceId::random();
    for outcome in [
        WorkspaceRestoreOutcome::Restored {
            action: WorkspaceRecoveryAction::Unarchive,
        },
        WorkspaceRestoreOutcome::Restored {
            action: WorkspaceRecoveryAction::Restore,
        },
        WorkspaceRestoreOutcome::Unavailable {
            reason: WorkspaceRecoveryUnavailable::ProjectDirectoryMissing,
        },
        WorkspaceRestoreOutcome::Failed {
            stage: WorkspaceRecoveryStage::Backing,
            error: WireError::new(ErrorCode::Conflict, "recorded subdirectory unavailable"),
        },
        WorkspaceRestoreOutcome::Uncertain,
    ] {
        let report = WorkspaceRestoreReport { workspace, outcome };
        assert_eq!(
            serde_json::from_slice::<WorkspaceRestoreReport>(&serde_json::to_vec(&report).unwrap())
                .unwrap(),
            report
        );
    }
}
