//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use super::LinkState;
use tokio::sync::watch;
use vessel_proto::diagnostics::{DiagnosticCount, DiagnosticRelayCounts};

/// Observe one relay watch without copying its route or refusal details.
/// A closed watch is unknown even if its final value was registered.
#[must_use]
pub fn observation(source: &watch::Receiver<LinkState>) -> DiagnosticRelayCounts {
    let state = source.borrow();
    // A closed producer is not evidence that its last Registered value is
    // still live. The close check is the observation admission boundary.
    if source.has_changed().is_err() {
        return DiagnosticRelayCounts {
            unrecognized: DiagnosticCount::new(1),
            ..DiagnosticRelayCounts::default()
        };
    }
    let mut counts = DiagnosticRelayCounts::default();
    let slot = match *state {
        LinkState::Connecting => &mut counts.connecting,
        LinkState::Registered => &mut counts.registered,
        LinkState::Refused(_) => &mut counts.refused,
        LinkState::Stopped => &mut counts.stopped,
    };
    *slot = DiagnosticCount::new(1);
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::RegistrationRefusal;

    fn total(counts: &DiagnosticRelayCounts) -> DiagnosticCount {
        counts
            .connecting
            .saturating_add(counts.registered)
            .saturating_add(counts.refused)
            .saturating_add(counts.stopped)
            .saturating_add(counts.unrecognized)
    }
    #[test]
    fn each_real_state_is_classified_without_address_or_credential_fields() {
        for state in [
            LinkState::Connecting,
            LinkState::Registered,
            LinkState::Refused(RegistrationRefusal::Token),
            LinkState::Stopped,
        ] {
            let (_send, receive) = watch::channel(state);
            let facts = observation(&receive);
            assert_eq!(total(&facts).get(), 1);
            let selected = match state {
                LinkState::Connecting => facts.connecting,
                LinkState::Registered => facts.registered,
                LinkState::Refused(_) => facts.refused,
                LinkState::Stopped => facts.stopped,
            };
            assert_eq!(selected.get(), 1);
            assert_eq!(facts.unrecognized.get(), 0);
        }
    }
    #[test]
    fn closed_registered_watch_is_not_reported_as_registered() {
        let (send, receive) = watch::channel(LinkState::Registered);
        drop(send);
        let facts = observation(&receive);
        assert_eq!(facts.registered.get(), 0);
        assert_eq!(facts.unrecognized.get(), 1);
        assert_eq!(total(&facts).get(), 1);
    }
    #[test]
    fn changed_watch_is_observed_without_consuming_its_notification() {
        let (send, receive) = watch::channel(LinkState::Connecting);
        send.send(LinkState::Registered).unwrap();
        let facts = observation(&receive);
        assert_eq!(facts.registered.get(), 1);
        assert!(receive.has_changed().unwrap());
    }
}
