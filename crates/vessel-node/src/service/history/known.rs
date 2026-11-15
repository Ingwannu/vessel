//! Match native transcripts within the account that supplied the history.
//! Default-name aliases and absent profile names denote the same account.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use vessel_proto::{HarnessRef, HarnessSessionId, ProfileName, SessionId, SessionInfo};

pub(super) fn in_account(
    sessions: Vec<SessionInfo>,
    harness: &HarnessRef,
    account: &ProfileName,
    implicit: &ProfileName,
) -> HashMap<HarnessSessionId, SessionId> {
    let mut known: HashMap<HarnessSessionId, (DateTime<Utc>, SessionId)> = HashMap::new();
    for info in sessions {
        if &info.spec.harness != harness || info.profile.as_ref().unwrap_or(implicit) != account {
            continue;
        }
        let Some(conversation) = info.harness_session else {
            continue;
        };
        let newer = known
            .get(&conversation)
            .is_none_or(|(created, _)| info.created_at > *created);
        if newer {
            let _replaced = known.insert(conversation, (info.created_at, info.id));
        }
    }
    known
        .into_iter()
        .map(|(conversation, (_created, id))| (conversation, id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{HarnessKind, SessionMode, SessionSpec};
    fn info(profile: Option<&str>, at: i64) -> SessionInfo {
        let spec = SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        );
        let mut info = SessionInfo::new(
            SessionId::random(),
            spec,
            DateTime::from_timestamp(at, 0).unwrap(),
        );
        info.harness_session = Some(HarnessSessionId::new("copied-native-id"));
        info.profile = profile.map(|name| name.parse().unwrap());
        info
    }
    #[test]
    fn newest_matching_native_id_in_another_account_is_not_already_imported_here() {
        let original = info(None, 1);
        let copy = info(Some("other"), 2);
        let harness = HarnessKind::Codex.into();
        let default = "codex".parse().unwrap();
        let known = in_account(
            vec![original.clone(), copy.clone()],
            &harness,
            &default,
            &default,
        );
        assert_eq!(
            known.get(&HarnessSessionId::new("copied-native-id")),
            Some(&original.id)
        );
        let other = "other".parse().unwrap();
        let known = in_account(vec![original, copy.clone()], &harness, &other, &default);
        assert_eq!(
            known.get(&HarnessSessionId::new("copied-native-id")),
            Some(&copy.id)
        );
    }
    #[test]
    fn default_profile_aliases_are_one_account_but_other_harnesses_are_not() {
        let original = info(None, 1);
        let named = info(Some("codex"), 2);
        let mut wrong_agent = info(None, 3);
        wrong_agent.spec.harness = HarnessKind::ClaudeCode.into();
        let default = "codex".parse().unwrap();
        let known = in_account(
            vec![original, named.clone(), wrong_agent],
            &HarnessKind::Codex.into(),
            &default,
            &default,
        );
        assert_eq!(
            known.get(&HarnessSessionId::new("copied-native-id")),
            Some(&named.id)
        );
    }
}
