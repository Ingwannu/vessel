//! Regressions use session records, never a provider or network.
use super::{ClosePolicy, DateBucket, HistoryPage, close_policy};
use chrono::{Days, NaiveDate};
use vessel_proto::{Creator, NodeId, SessionId};
#[test]
fn child_close_preserves_child_and_parent() {
    assert_eq!(close_policy(&Creator::Owner), ClosePolicy::Archive);
    assert_eq!(
        close_policy(&Creator::Agent {
            node: NodeId::random(),
            session: SessionId::random()
        }),
        ClosePolicy::LayoutOnly
    );
}
#[test]
fn date_sections_use_calendar_days_and_reference_boundaries() {
    let today = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    for (days, bucket) in [
        (0, DateBucket::Today),
        (1, DateBucket::Yesterday),
        (2, DateBucket::ThisWeek),
        (7, DateBucket::ThisWeek),
        (8, DateBucket::ThisMonth),
        (30, DateBucket::ThisMonth),
        (31, DateBucket::Older),
    ] {
        assert_eq!(
            DateBucket::of(today.checked_sub_days(Days::new(days)).unwrap(), today),
            bucket
        );
    }
    assert_eq!(
        DateBucket::of(today.checked_add_days(Days::new(1)).unwrap(), today),
        DateBucket::Today
    );
}
#[test]
fn page_boundary_and_reset_do_not_truncate_history() {
    let first = HistoryPage::default();
    assert_eq!(first.visible(120), 50);
    assert!(first.has_more(51));
    assert!(!first.has_more(50));
    assert_eq!(first.next().visible(120), 100);
    assert_eq!(first.next().next().visible(120), 120);
    assert_eq!(HistoryPage::default().visible(120), 50);
}

fn machine(name: &str, when: &str, title: &str, archived: bool) -> crate::Machine {
    use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec};
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(SessionId::random(), spec, when.parse().unwrap());
    info.title = Some(title.parse().unwrap());
    if archived {
        info.marks.archived_at = Some(info.created_at);
    }
    crate::Machine {
        node: None,
        connection_epoch: None,
        name: name.to_owned(),
        reach: crate::Reach::Online,
        sessions: vec![crate::Card::from_session(name, &info)],
        agents: Vec::new(),
        protocol: None,
        version: None,
    }
}

#[test]
fn history_keeps_archived_rows_and_sorts_activity_across_hosts() {
    let fleet = crate::Fleet {
        machines: vec![
            machine("older-host", "2026-11-01T09:00:00Z", "Archived title", true),
            machine("newer-host", "2026-11-15T09:00:00Z", "Fresh title", false),
        ],
        read_at: None,
    };
    let rows = super::history(&fleet, &super::HostScope::All, "");
    assert_eq!(
        rows.iter()
            .map(|card| card.title.as_str())
            .collect::<Vec<_>>(),
        ["Fresh title", "Archived title"]
    );
    assert!(rows.last().unwrap().detail.marks.archived_at.is_some());
    let matched = super::history(&fleet, &super::HostScope::All, "ARCHIVED");
    assert_eq!(matched.len(), 1);
    assert_eq!(matched.first().unwrap().machine, "older-host");
    assert!(super::history(&fleet, &super::HostScope::All, "older-host").is_empty());
    assert!(
        super::history(
            &fleet,
            &super::HostScope::Host(crate::machine_book::Name::parse("newer-host").unwrap()),
            "archived"
        )
        .is_empty()
    );
}

#[test]
fn metadata_query_preserves_chronology_and_requires_every_term() {
    let fleet = crate::Fleet {
        machines: vec![
            machine("work", "2026-11-01T09:00:00Z", "Fix login", false),
            machine("work", "2026-11-15T09:00:00Z", "Login fix followup", false),
        ],
        read_at: None,
    };
    let rows = super::history(&fleet, &super::HostScope::All, "fix login");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.first().unwrap().title, "Login fix followup");
    assert!(super::history(&fleet, &super::HostScope::All, "fix absent").is_empty());
}

#[test]
fn history_forgives_title_typos_without_relevance_reordering() {
    let fleet = crate::Fleet {
        machines: vec![
            machine("work", "2026-11-01T09:00:00Z", "Stripe billing", true),
            machine("work", "2026-11-15T09:00:00Z", "Fix stripe billing", false),
        ],
        read_at: None,
    };
    let rows = super::history(&fleet, &super::HostScope::All, "stripe bulling");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.first().unwrap().title, "Fix stripe billing");
}

#[test]
fn late_navigation_cannot_open_after_a_new_request_or_manual_route() {
    use super::{Navigation, NavigationPurpose};
    let mut state = Navigation::default();
    let older = state.begin(NavigationPurpose::Open).unwrap();
    let newer = state.begin(NavigationPurpose::CopyLink).unwrap();
    assert_eq!(state.finish(older), None);
    assert_eq!(state.finish(newer), Some(NavigationPurpose::CopyLink));
    assert_eq!(state.finish(newer), None);
    let canceled = state.begin(NavigationPurpose::Open).unwrap();
    state.cancel();
    assert_eq!(state.finish(canceled), None);
}

#[test]
fn lifecycle_busy_and_stale_ack_preserve_the_real_pending_action() {
    use super::{Lifecycle, LifecycleAction, LifecycleBusy, LifecycleTarget};
    let target = LifecycleTarget {
        host: crate::machine_book::Name::parse("work").unwrap(),
        session: SessionId::random(),
        node: None,
    };
    let mut state = Lifecycle::default();
    let first = state.begin(target.clone(), LifecycleAction::Exit).unwrap();
    assert_eq!(
        state.begin(target.clone(), LifecycleAction::Delete),
        Err(LifecycleBusy::Pending)
    );
    assert_eq!(
        state.finish(first.ticket).unwrap().action,
        LifecycleAction::Exit
    );
    let second = state.begin(target, LifecycleAction::Fresh).unwrap();
    assert_eq!(state.finish(first.ticket), None);
    assert_eq!(state.pending().unwrap().ticket, second.ticket);
    assert_eq!(
        state.finish(second.ticket).unwrap().action,
        LifecycleAction::Fresh
    );
    assert!(state.pending().is_none());
}

#[test]
fn successful_reply_must_prove_the_requested_postcondition_before_closing() {
    use super::{LifecycleAction, satisfies_lifecycle};
    use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec, SessionStatus};
    let mut info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    assert!(!satisfies_lifecycle(LifecycleAction::Exit, &info));
    info.marks.archived_at = Some(info.created_at);
    assert!(!satisfies_lifecycle(LifecycleAction::Exit, &info));
    info.status = SessionStatus::Interrupted;
    assert!(satisfies_lifecycle(LifecycleAction::Exit, &info));
    assert!(!satisfies_lifecycle(LifecycleAction::Resume, &info));
    info.marks.archived_at = None;
    assert!(satisfies_lifecycle(LifecycleAction::Unarchive, &info));
    assert!(!satisfies_lifecycle(LifecycleAction::Resume, &info));
    info.status = SessionStatus::Running;
    assert!(satisfies_lifecycle(LifecycleAction::Resume, &info));
    assert!(!satisfies_lifecycle(LifecycleAction::Delete, &info));
}
