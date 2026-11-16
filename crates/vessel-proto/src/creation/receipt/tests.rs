//! Every irreversible boundary is recorded before effects and recovered as unknown.

use super::*;
use crate::BlobHash;

fn receipt(resource: Option<Resource>, first_input: FirstInput) -> Receipt {
    Receipt::admitted(
        Kind::Workspace,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace: WorkspaceId::random(),
            workspace_claim: WorkspaceClaim::Create,
            resource,
            first_input,
        },
        Fingerprint::new(BlobHash::from_bytes([7; 32])),
    )
    .unwrap()
}

fn advance(receipt: &mut Receipt, update: Advance) {
    receipt
        .advance(receipt.snapshot().revision, update)
        .unwrap();
}

fn resource() -> Resource {
    Resource::Agent {
        session: SessionId::random(),
    }
}

#[test]
fn empty_workspace_has_no_agent_or_prompt_stage() {
    let mut receipt = receipt(None, FirstInput::Absent);
    assert!(
        receipt
            .advance(Revision::INITIAL, Advance::Complete)
            .is_err()
    );
    advance(&mut receipt, Advance::Begin(Stage::Workspace));
    advance(&mut receipt, Advance::Finished(Stage::Workspace));
    assert!(
        receipt
            .advance(receipt.snapshot().revision, Advance::Begin(Stage::Resource))
            .is_err()
    );
    advance(&mut receipt, Advance::Complete);
    assert_eq!(receipt.snapshot().phase, Phase::Completed);
}

#[test]
fn chat_cannot_complete_before_the_requested_first_message_is_accepted() {
    let mut receipt = receipt(Some(resource()), FirstInput::Present);
    for stage in [Stage::Workspace, Stage::Resource] {
        advance(&mut receipt, Advance::Begin(stage));
        advance(&mut receipt, Advance::Finished(stage));
    }
    assert!(
        receipt
            .advance(receipt.snapshot().revision, Advance::Complete)
            .is_err()
    );
    advance(&mut receipt, Advance::Begin(Stage::Prompt));
    advance(&mut receipt, Advance::Finished(Stage::Prompt));
    advance(&mut receipt, Advance::Complete);
    assert_eq!(receipt.snapshot().phase, Phase::Completed);
}

#[test]
fn every_interrupted_side_effect_becomes_unknown_and_cannot_be_replayed() {
    for stage in [Stage::Workspace, Stage::Resource, Stage::Prompt] {
        let mut receipt = receipt(Some(resource()), FirstInput::Present);
        for before in [Stage::Workspace, Stage::Resource, Stage::Prompt] {
            advance(&mut receipt, Advance::Begin(before));
            if before == stage {
                break;
            }
            advance(&mut receipt, Advance::Finished(before));
        }
        let mut recovered: Receipt =
            serde_json::from_str(&serde_json::to_string(&receipt).unwrap()).unwrap();
        advance(&mut recovered, Advance::Recover);
        assert_eq!(
            recovered.snapshot().phase,
            Phase::Failed {
                stage,
                certainty: Certainty::Unknown,
                code: ErrorCode::Unavailable
            }
        );
        let frozen = recovered.clone();
        assert_eq!(
            recovered.advance(recovered.snapshot().revision, Advance::Begin(stage)),
            Err(CreationError::InvalidTransition)
        );
        assert_eq!(recovered, frozen);
    }
}

#[test]
fn known_prompt_failure_retries_only_prompt_and_retains_the_same_reserved_agent() {
    let mut receipt = receipt(Some(resource()), FirstInput::Present);
    for stage in [Stage::Workspace, Stage::Resource] {
        advance(&mut receipt, Advance::Begin(stage));
        advance(&mut receipt, Advance::Finished(stage));
    }
    let reserved = receipt.snapshot().reserved.clone();
    advance(&mut receipt, Advance::Begin(Stage::Prompt));
    advance(
        &mut receipt,
        Advance::Fail {
            code: ErrorCode::Conflict,
            certainty: Certainty::Known,
        },
    );
    let revision = receipt.snapshot().revision;
    assert_eq!(
        receipt.advance(Revision::INITIAL, Advance::Begin(Stage::Prompt)),
        Err(CreationError::StaleRevision)
    );
    assert_eq!(
        receipt.advance(revision, Advance::Begin(Stage::Resource)),
        Err(CreationError::InvalidTransition)
    );
    advance(&mut receipt, Advance::Begin(Stage::Prompt));
    advance(&mut receipt, Advance::Finished(Stage::Prompt));
    assert_eq!(receipt.snapshot().reserved, reserved);
}

#[test]
fn invalid_resource_combinations_and_duplicate_stage_starts_are_refused() {
    let mut receipt = receipt(None, FirstInput::Absent);
    let invalid = Reservation {
        first_input: FirstInput::Present,
        ..receipt.snapshot().reserved.clone()
    };
    assert!(Receipt::admitted(Kind::Workspace, invalid, receipt.fingerprint()).is_err());
    advance(&mut receipt, Advance::Begin(Stage::Workspace));
    let frozen = receipt.clone();
    assert!(
        receipt
            .advance(
                receipt.snapshot().revision,
                Advance::Begin(Stage::Workspace)
            )
            .is_err()
    );
    assert_eq!(receipt, frozen);
}

#[test]
fn keys_validate_on_decode_without_trimming_or_debug_leakage() {
    let key = super::super::Key::parse(" private key ").unwrap();
    assert_eq!(key.as_str(), " private key ");
    assert!(!format!("{key:?}").contains("private key"));
    assert!(super::super::Key::parse("한".repeat(super::super::Key::MAX_CHARS)).is_ok());
    for raw in [
        String::new(),
        "x\ny".to_owned(),
        "x".repeat(super::super::Key::MAX_CHARS.saturating_add(1)),
    ] {
        assert!(
            serde_json::from_str::<super::super::Key>(&serde_json::to_string(&raw).unwrap())
                .is_err()
        );
    }
}

#[test]
fn forged_inflight_markers_and_resource_kind_mismatches_do_not_decode() {
    let receipt = receipt(None, FirstInput::Absent);
    let mut forged = serde_json::to_value(&receipt).unwrap();
    forged["in_flight"] = serde_json::json!("prompt");
    assert!(serde_json::from_value::<Receipt>(forged).is_err());
    let mut forged = serde_json::to_value(&receipt).unwrap();
    forged["snapshot"]["kind"] = serde_json::json!("agent");
    assert!(serde_json::from_value::<Receipt>(forged).is_err());
}

#[test]
fn canonical_first_input_id_survives_known_retry_serialization_and_unknown_restart() {
    let message = crate::timeline::ClientMessageId::random();
    let mut receipt = receipt(
        Some(resource()),
        FirstInput::Identified {
            message: message.clone(),
        },
    );
    for stage in [Stage::Workspace, Stage::Resource] {
        advance(&mut receipt, Advance::Begin(stage));
        advance(&mut receipt, Advance::Finished(stage));
    }
    advance(&mut receipt, Advance::Begin(Stage::Prompt));
    advance(
        &mut receipt,
        Advance::Fail {
            code: ErrorCode::Forbidden,
            certainty: Certainty::Known,
        },
    );
    advance(&mut receipt, Advance::Begin(Stage::Prompt));
    let encoded = serde_json::to_vec(&receipt).unwrap();
    let mut restarted: Receipt = serde_json::from_slice(&encoded).unwrap();
    advance(&mut restarted, Advance::Recover);
    assert_eq!(
        restarted.snapshot().reserved.first_input.message(),
        Some(&message)
    );
    assert!(
        restarted
            .advance(restarted.snapshot().revision, Advance::Begin(Stage::Prompt))
            .is_err()
    );
    let mut terminal = restarted.snapshot().reserved.clone();
    terminal.resource = Some(Resource::Terminal {
        terminal: TerminalId::random(),
    });
    assert!(Receipt::admitted(Kind::Workspace, terminal, receipt.fingerprint()).is_err());
}
