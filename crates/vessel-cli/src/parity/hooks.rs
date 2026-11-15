//! Missing hook attribution is intentionally silent; no arbitrary activity can alter turn state.
use crate::{args::parity::HookInvocation, error::CliError};

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
        "terminal hook ingestion awaits W7's reviewed attribution/token contract",
    ))
}
fn configured(present: impl Fn(&str) -> bool) -> bool {
    [
        "VESSEL_TERMINAL_ID",
        "VESSEL_ACTIVITY_TOKEN",
        "VESSEL_TERMINAL_ACTIVITY_URL",
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
            "VESSEL_TERMINAL_ID",
            "VESSEL_ACTIVITY_TOKEN",
            "VESSEL_TERMINAL_ACTIVITY_URL",
        ] {
            assert!(!configured(|key| key != missing));
        }
        assert!(configured(|_key| true));
    }
}
