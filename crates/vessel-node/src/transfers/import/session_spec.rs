//! Assemble the resumed request from an import's validated session snapshot.
//! Current metadata wins over the original creation request; choices are applied after load.

use std::path::PathBuf;
use vessel_proto::{
    HarnessKind, HarnessSessionId, OptionChoice, ProfileName, SessionInfo, SessionMode, SessionSpec,
};

pub(super) fn resumed(
    info: &SessionInfo,
    harness: HarnessKind,
    session: HarnessSessionId,
    options: Vec<OptionChoice>,
    cwd: PathBuf,
    profile: Option<&ProfileName>,
) -> SessionSpec {
    let model = match &info.spec.mode {
        SessionMode::Structured { model, .. } => model.clone(),
        SessionMode::Terminal { .. } => None,
    };
    let mut spec = SessionSpec::new(
        harness,
        SessionMode::Structured {
            model,
            resume: Some(session),
            // Applied after the load, in order; one this node's agent
            // does not offer fails the start, as a model it does not
            // offer does, rather than run in the agent's default mode.
            options,
        },
    )
    .with_cwd(cwd);
    spec.profile = profile.cloned();
    spec.title.clone_from(&info.title);
    spec.labels.clone_from(&info.labels);
    spec.listing = Some(info.listing);
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use vessel_proto::{SessionId, SessionListing};

    #[test]
    fn imports_copy_effective_listing_explicitly_instead_of_inheriting() {
        let original = SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let mut info = SessionInfo::new(SessionId::random(), original, DateTime::UNIX_EPOCH);
        info.labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
        for listing in [SessionListing::Shown, SessionListing::Background] {
            info.listing = listing;
            let session = HarnessSessionId::new("imported-fixture");
            let spec = resumed(
                &info,
                HarnessKind::Codex,
                session.clone(),
                Vec::new(),
                PathBuf::from("/fixture"),
                None,
            );
            assert_eq!(spec.listing, Some(listing));
            assert_eq!(spec.labels, info.labels);
            assert_eq!(info.spec.listing, None);
            assert!(
                matches!(spec.mode, SessionMode::Structured { resume: Some(id), .. } if id == session)
            );
        }
    }
}
