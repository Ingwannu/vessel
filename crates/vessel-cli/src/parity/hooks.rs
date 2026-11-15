//! Missing hook attribution is intentionally silent; no arbitrary activity can alter turn state.
use crate::{args::parity::HookInvocation, error::CliError};
use vessel_proto::notifications::{
    TERMINAL_ACTIVITY_ID_ENV, TERMINAL_ACTIVITY_SECRET_ENV, TERMINAL_ACTIVITY_SOCKET_ENV,
};

pub(super) fn run(invocation: HookInvocation) -> Result<(), CliError> {
    let HookInvocation {
        provider: _provider,
        event: _event,
    } = invocation;
    // Test only presence: the secret never enters diagnostics, argv or a stored snapshot.
    if !configured(|key| std::env::var_os(key).is_some()) {
        return Ok(());
    }
    Err(CliError::ParityBlocked(
        "terminal hook ingestion awaits W7's reviewed restricted IPC SDK/producer",
    ))
}
fn configured(present: impl Fn(&str) -> bool) -> bool {
    [
        TERMINAL_ACTIVITY_ID_ENV,
        TERMINAL_ACTIVITY_SECRET_ENV,
        TERMINAL_ACTIVITY_SOCKET_ENV,
    ]
    .into_iter()
    .all(present)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_attribution_never_reads_stdin_or_connects() {
        for missing in [
            TERMINAL_ACTIVITY_ID_ENV,
            TERMINAL_ACTIVITY_SECRET_ENV,
            TERMINAL_ACTIVITY_SOCKET_ENV,
        ] {
            assert!(!configured(|key| key != missing));
        }
        assert!(configured(|_key| true));
    }
}
