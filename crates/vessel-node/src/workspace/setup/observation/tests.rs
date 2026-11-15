use super::*;

#[test]
fn unknown_or_missing_live_token_is_not_process_stop_proof() {
    let token = CancellationToken::new();
    assert_eq!(
        cancel_observed(SetupRunState::Unknown, Some(&token)),
        CancelSetupOutcome::Unknown
    );
    assert!(!token.is_cancelled());
    assert_eq!(
        cancel_observed(SetupRunState::Running, None),
        CancelSetupOutcome::Unknown
    );
    for state in [
        SetupRunState::Succeeded,
        SetupRunState::Failed,
        SetupRunState::Cancelled,
    ] {
        assert_eq!(
            cancel_observed(state, Some(&token)),
            CancelSetupOutcome::NotRunning
        );
        assert!(!token.is_cancelled());
    }
    assert_eq!(
        cancel_observed(SetupRunState::Running, Some(&token)),
        CancelSetupOutcome::Requested
    );
    assert!(token.is_cancelled());
}
