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
    profile: &Option<ProfileName>,
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
    spec.profile.clone_from(profile);
    spec.title.clone_from(&info.title);
    spec.labels.clone_from(&info.labels);
    spec
}
