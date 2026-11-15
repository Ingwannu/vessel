//! Platform lookup is blocking I/O, kept off both the actor and render thread.
//! The resulting calendar still resolves rules from the node-pinned bundle.
use vessel_app_view::schedules::clock::{Clock, ClockError};
use vessel_proto::automation::InvalidAutomationText;

/// Preserve the original cause while the screen uses its explicit UTC fallback.
#[derive(Debug, thiserror::Error)]
pub enum ViewerClockError {
    /// The platform has no stable IANA identity, or lookup has not completed.
    #[error("viewer time zone is unavailable")]
    Unavailable,
    /// Platform lookup failed; never show its diagnostic string in the screen.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("viewer time zone lookup: {0}")]
    Lookup(#[source] jiff::Error),
    /// A platform identity is outside the protocol's validated name shape.
    #[error("viewer time zone identity: {0}")]
    Name(#[source] InvalidAutomationText),
    /// The pinned display calendar cannot resolve the identity.
    #[error("viewer calendar: {0}")]
    Calendar(#[source] ClockError),
    /// Retain a failed blocking task, rather than disguising it as a node error.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("viewer time zone task: {0}")]
    Task(#[source] tokio::task::JoinError),
}

#[cfg(not(target_arch = "wasm32"))]
fn named(name: Option<&str>) -> Result<Clock, ViewerClockError> {
    let name = name.ok_or(ViewerClockError::Unavailable)?;
    let name = name.parse().map_err(ViewerClockError::Name)?;
    Clock::known(name).map_err(ViewerClockError::Calendar)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn viewer_clock() -> Result<Clock, ViewerClockError> {
    tokio::task::spawn_blocking(|| {
        let zone = jiff::tz::TimeZone::try_system().map_err(ViewerClockError::Lookup)?;
        named(zone.iana_name())
    })
    .await
    .map_err(ViewerClockError::Task)?
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn an_identity_is_never_replaced_with_a_guessed_zone() {
        for name in ["Asia/Seoul", "America/New_York", "UTC"] {
            let clock = named(Some(name)).unwrap();
            assert_eq!(clock.name().unwrap().as_str(), name);
        }
        assert!(matches!(named(None), Err(ViewerClockError::Unavailable)));
        let malformed = named(Some("bad zone")).unwrap_err();
        assert!(matches!(malformed, ViewerClockError::Name(_)));
        assert!(std::error::Error::source(&malformed).is_some());
        let unknown = named(Some("Not/A_Zone")).unwrap_err();
        assert!(matches!(unknown, ViewerClockError::Calendar(_)));
        assert!(std::error::Error::source(&unknown).is_some());
    }
}
