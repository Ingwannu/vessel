//! Notice meaning belongs to the same typed facts as its sentence, never
//! to a renderer's guess from a translated label or raw diagnostic.
use super::pages::InstallProgress;
use crate::alert::Variant;
use vessel_proto::agents::{SignedIn, UnknownReason};

/// Sign-in help needs attention; an absent verdict never reads as success.
#[must_use]
pub const fn authentication(signed_in: &SignedIn) -> Variant {
    match signed_in {
        SignedIn::Yes | SignedIn::NotRequired => Variant::Default,
        SignedIn::No { .. }
        | SignedIn::Unknown {
            reason: UnknownReason::NotInstalled,
            ..
        } => Variant::Warning,
        SignedIn::Unknown {
            reason: UnknownReason::Failed { .. } | UnknownReason::TimedOut { .. },
        } => Variant::Error,
        SignedIn::Unknown {
            reason:
                UnknownReason::NotChecked
                | UnknownReason::InUse
                | UnknownReason::NotSaid
                | UnknownReason::NoHome
                | UnknownReason::Unread,
            ..
        } => Variant::Info,
    }
}

/// A stopped output reader is distinct from a known failed installer.
#[must_use]
pub const fn installation(progress: &InstallProgress) -> Option<Variant> {
    match progress {
        InstallProgress::None => None,
        InstallProgress::Following(_) => Some(Variant::Info),
        InstallProgress::Finished { completion, .. } => Some(completion.variant()),
        InstallProgress::Unavailable(_) => Some(Variant::Warning),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_sign_in_is_never_success_and_each_reason_has_its_notice_meaning() {
        for (json, variant) in [
            (serde_json::json!({"state":"yes"}), Variant::Default),
            (
                serde_json::json!({"state":"not_required"}),
                Variant::Default,
            ),
            (
                serde_json::json!({"state":"no","help":{"methods":[]}}),
                Variant::Warning,
            ),
            (
                serde_json::json!({"state":"unknown","reason":{"reason":"not_installed"}}),
                Variant::Warning,
            ),
            (
                serde_json::json!({"state":"unknown","reason":{"reason":"timed_out","after":30000}}),
                Variant::Error,
            ),
        ] {
            assert_eq!(
                authentication(&serde_json::from_value(json).unwrap()),
                variant
            );
        }
        for reason in ["not_checked", "in_use", "not_said", "no_home", "unread"] {
            let status = serde_json::from_value(
                serde_json::json!({"state":"unknown","reason":{"reason":reason}}),
            )
            .unwrap();
            assert_eq!(authentication(&status), Variant::Info);
        }
        for step in ["spawn", "initialize", "session_new", "exited"] {
            let status = serde_json::from_value(
                serde_json::json!({"state":"unknown","reason":{"reason":"failed","step":step}}),
            )
            .unwrap();
            assert_eq!(authentication(&status), Variant::Error);
        }
        assert_eq!(installation(&InstallProgress::None), None);
    }
}
