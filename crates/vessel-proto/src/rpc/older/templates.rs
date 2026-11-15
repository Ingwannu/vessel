//! Projects session metadata nested in transfers and reusable templates.
//! The reply encoder is the only caller; storage keeps it independently of protocol assignment.

use crate::automation::{PromptList, SavedPrompt, ScheduleInfo, ScheduleList, ScheduleTarget};
use crate::{ProtocolVersion, SessionImported, SessionSpec, TransferDetail, TransferManifest};

fn spec(value: &SessionSpec, protocol: ProtocolVersion) -> Option<SessionSpec> {
    (!crate::lifecycle_version::visible(crate::lifecycle_version::SINCE, protocol)
        && (!value.labels.is_empty() || value.listing.is_some()))
    .then(|| {
        let mut older = value.clone();
        older.labels.clear();
        older.listing = None;
        older
    })
}

/// A session inside a transfer must obey the same metadata version as get/list.
pub(super) fn exported(
    value: &TransferManifest,
    protocol: ProtocolVersion,
) -> Option<TransferManifest> {
    let TransferDetail::Session { info, .. } = &value.detail else {
        return None;
    };
    let info = info.for_protocol(protocol)?;
    let mut older = value.clone();
    if let TransferDetail::Session { info: nested, .. } = &mut older.detail {
        **nested = info;
    }
    Some(older)
}

/// Import can return a started session; omission still means no session was started.
pub(super) fn imported(
    value: &SessionImported,
    protocol: ProtocolVersion,
) -> Option<SessionImported> {
    let info = value.session.as_ref()?.for_protocol(protocol)?;
    let mut older = value.clone();
    *older.session.as_deref_mut()? = info;
    Some(older)
}

/// A saved prompt's session template may contain the creation labels.
pub(super) fn prompt(value: &SavedPrompt, protocol: ProtocolVersion) -> Option<SavedPrompt> {
    let session = spec(value.session.as_ref()?, protocol)?;
    let mut older = value.clone();
    *older.session.as_deref_mut()? = session;
    Some(older)
}

/// Every saved template is rewritten, with its id and revision unchanged.
pub(super) fn prompts(value: &PromptList, protocol: ProtocolVersion) -> Option<PromptList> {
    changed(&value.prompts, |item| prompt(item, protocol)).map(|prompts| PromptList { prompts })
}

/// A schedule carries labels only when its target creates a new session.
pub(super) fn schedule(value: &ScheduleInfo, protocol: ProtocolVersion) -> Option<ScheduleInfo> {
    let session = match &value.definition.target {
        ScheduleTarget::NewSession { spec: session, .. } => spec(session, protocol)?,
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => return None,
    };
    let mut older = value.clone();
    if let ScheduleTarget::NewSession { spec, .. } = &mut older.definition.target {
        **spec = session;
    }
    Some(older)
}

/// Lists follow the same per-template rule as individual reads and mutations.
pub(super) fn schedules(value: &ScheduleList, protocol: ProtocolVersion) -> Option<ScheduleList> {
    changed(&value.schedules, |item| schedule(item, protocol))
        .map(|schedules| ScheduleList { schedules })
}

fn changed<T: Clone>(items: &[T], rewrite: impl Fn(&T) -> Option<T>) -> Option<Vec<T>> {
    // Delay the list copy until the first changed item, and keep that rewrite.
    let mut tail = items.iter().enumerate();
    let (index, replacement) =
        tail.find_map(|(index, item)| rewrite(item).map(|next| (index, next)))?;
    Some(
        items
            .iter()
            .take(index)
            .cloned()
            .chain(std::iter::once(replacement))
            .chain(tail.map(|(_, item)| rewrite(item).unwrap_or_else(|| item.clone())))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Creator, HarnessKind, HarnessSessionId, Reply, SessionId, SessionInfo, SessionMode,
        TransferId,
    };
    use chrono::DateTime;

    fn info() -> SessionInfo {
        let mut spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        spec.labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
        spec.listing = Some(crate::SessionListing::Background);
        SessionInfo::new(SessionId::random(), spec, DateTime::UNIX_EPOCH)
    }

    fn assert_older(reply: &Reply) {
        assert!(serde_json::to_string(reply).unwrap().contains("labels"));
        assert!(serde_json::to_string(reply).unwrap().contains("listing"));
        let older = crate::PROTOCOL_VERSION;
        let projected = reply.for_protocol(older).unwrap();
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("labels")
        );
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("listing")
        );
    }

    #[test]
    fn every_info_reply_hides_unassigned_labels_at_the_current_protocol() {
        let info = info();
        for reply in [
            Reply::ListSessions(vec![info.clone()]),
            Reply::Subscribe(crate::Subscribed {
                subscription: crate::SubscriptionId::new(1),
                info: info.clone(),
            }),
            Reply::GetSession(info.clone()),
            Reply::CreateSession(info.clone()),
            Reply::MoveProfile(info.clone()),
            Reply::RenameSession(info.clone()),
            Reply::ResumeSession(info.clone()),
            Reply::MarkSession(info.clone()),
            Reply::ParkSession(info.clone()),
            Reply::WakeSession(info.clone()),
            Reply::ForkSession(info.clone()),
            Reply::RewindSession(info.clone()),
            Reply::OpenSideQuestion(info.clone()),
            Reply::ResetSideQuestion(info),
        ] {
            assert_older(&reply);
        }
    }

    #[test]
    fn both_transfer_replies_hide_unassigned_labels_at_the_current_protocol() {
        let info = info();
        let harness_session = HarnessSessionId::new("fake-session");
        let manifest = TransferManifest {
            transfer: TransferId::random(),
            files: std::collections::BTreeMap::new(),
            detail: TransferDetail::Session {
                info: Box::new(info.clone()),
                harness_session: harness_session.clone(),
                cwd: "/work".into(),
                options: Vec::new(),
            },
        };
        assert_older(&Reply::ExportSession(manifest.clone()));
        assert_older(&Reply::ExportMemory(manifest));

        assert_older(&Reply::ImportSession(SessionImported {
            harness_session,
            profile: None,
            cwd: "/work".into(),
            written: 0,
            session: Some(Box::new(info)),
        }));
    }

    #[test]
    fn saved_and_scheduled_specs_hide_unassigned_labels_at_the_current_protocol() {
        let spec = info().spec;
        let saved = SavedPrompt {
            id: crate::SavedPromptId::random(),
            name: "review".parse().unwrap(),
            text: "Review".parse().unwrap(),
            project: None,
            session: Some(Box::new(spec.clone())),
            defined_by: Creator::Owner,
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
            revision: 1,
        };
        for reply in [
            Reply::CreatePrompt(saved.clone()),
            Reply::UpdatePrompt(saved.clone()),
            Reply::ListPrompts(PromptList {
                prompts: vec![saved],
            }),
        ] {
            assert_older(&reply);
        }
        let schedule = ScheduleInfo {
            id: crate::ScheduleId::random(),
            definition: crate::automation::ScheduleDefinition {
                name: None,
                cadence: crate::automation::Cadence::Every {
                    seconds: crate::automation::EverySeconds::try_from(
                        crate::automation::EverySeconds::MIN,
                    )
                    .unwrap(),
                    anchor: None,
                },
                target: ScheduleTarget::NewSession {
                    spec: Box::new(spec),
                    after: crate::automation::AfterRun::default(),
                },
                prompt: "Review".parse().unwrap(),
                catch_up: crate::automation::CatchUp::default(),
                max_runs: None,
                ends_at: None,
            },
            state: crate::automation::ScheduleState::Paused {
                since: DateTime::UNIX_EPOCH,
            },
            runs: 0,
            last_due: None,
            last_run: None,
            defined_by: Creator::Owner,
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
            revision: 1,
        };
        for reply in [
            Reply::GetSchedule(schedule.clone()),
            Reply::CreateSchedule(schedule.clone()),
            Reply::UpdateSchedule(schedule.clone()),
            Reply::PauseSchedule(schedule.clone()),
            Reply::ResumeSchedule(schedule.clone()),
            Reply::ListSchedules(ScheduleList {
                schedules: vec![schedule],
            }),
        ] {
            assert_older(&reply);
        }
    }
    #[test]
    fn forwarded_relayed_and_delivered_lists_hide_unassigned_labels() {
        let list = Reply::ListSessions(vec![info()]);
        for reply in [
            Reply::Forward(crate::Forwarded {
                reply: Box::new(list.clone()),
            }),
            Reply::Relay(crate::Forwarded {
                reply: Box::new(list.clone()),
            }),
            Reply::Deliver(crate::Forwarded {
                reply: Box::new(list),
            }),
        ] {
            assert_older(&reply);
        }
    }
}
