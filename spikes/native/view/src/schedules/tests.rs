//! The schedule rules: badges, order, the form's refusals, and what a
//! forwarded node may be sent.

use chrono::{DateTime, TimeZone, Utc};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EverySeconds, PromptText, RunNumber, RunOutcome, RunTrigger,
    ScheduleDefinition, ScheduleInfo, ScheduleList, ScheduleRun, ScheduleState, ScheduleTarget,
};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    Creator, HarnessKind, HarnessRef, NodeId, ProjectId, ScheduleId, SessionId, SessionMode,
    SessionSpec,
};

use super::{
    AfterChoice, ArchiveOutcome, BlockedAct, CadenceField, CapField, CronPreset, Draft,
    ExpiryField, FormJob, LoadFault, MenuAction, MetaPart, NodeNotice, NodeReport, PreviewLane,
    PreviewPlan, ReadyPeer, RowOpen, ScheduleBadge, ScheduleBucket, ScheduleFormError,
    ScheduleKind, SchedulePeer, ScheduleReach, SessionDirectory, ShownOutcome, ShownTrigger,
    TargetField, TitleSource, actions, archive, badge, bucket, kind, new_cron_zone, opened_zone,
    page, plan, reusable, review, run_request, run_row,
};

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).single().unwrap()
}

fn prompt(text: &str) -> PromptText {
    text.parse().unwrap()
}

fn zone(name: &str) -> vessel_proto::automation::TimeZoneName {
    name.parse().unwrap()
}

fn spec() -> SessionSpec {
    SessionSpec::new(
        HarnessRef::Known(HarnessKind::Codex),
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_place(SessionPlace::Project {
        project: ProjectId::random(),
        dir: None,
    })
}

fn definition(target: ScheduleTarget, cadence: Cadence) -> ScheduleDefinition {
    ScheduleDefinition {
        name: None,
        cadence,
        target,
        prompt: prompt("Review the queue"),
        catch_up: CatchUp::Once,
        max_runs: None,
        ends_at: None,
    }
}

fn info(
    id: ScheduleId,
    defined_by: Creator,
    target: ScheduleTarget,
    state: ScheduleState,
    created: DateTime<Utc>,
) -> ScheduleInfo {
    ScheduleInfo {
        id,
        definition: definition(target, every(3600)),
        state,
        runs: 0,
        last_due: None,
        last_run: None,
        defined_by,
        created_at: created,
        updated_at: created,
        revision: 3,
    }
}

fn every(seconds: u32) -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(seconds).unwrap(),
        anchor: None,
    }
}

fn active(next: DateTime<Utc>) -> ScheduleState {
    ScheduleState::Active { next_due: next }
}

fn direct(zone_name: Option<&str>) -> SchedulePeer {
    SchedulePeer::Ready(ReadyPeer {
        preview: PreviewLane::Offered,
        zone: zone_name.map(zone),
        reach: ScheduleReach::Direct,
    })
}

fn draft(cadence: CadenceField, target: TargetField) -> Draft {
    Draft {
        name: String::new(),
        prompt: "Review the queue".to_owned(),
        cadence,
        zone: Some(zone("Asia/Seoul")),
        max_runs: CapField::Blank,
        ends_at: ExpiryField::None,
        after: AfterChoice::Archive,
        target,
    }
}

#[test]
fn badge_and_bucket_cover_every_state_and_unknown_is_kept() {
    let next = at(10);
    assert_eq!(badge(&active(next)), ScheduleBadge::Active);
    assert_eq!(bucket(&active(next)), ScheduleBucket::Runnable);
    assert_eq!(
        badge(&ScheduleState::Paused { since: next }),
        ScheduleBadge::Paused
    );
    assert_eq!(
        bucket(&ScheduleState::Paused { since: next }),
        ScheduleBucket::Runnable
    );
    let cases = [
        (
            vessel_proto::automation::EndReason::MaxRuns,
            ScheduleBadge::Finished,
        ),
        (
            vessel_proto::automation::EndReason::Expired,
            ScheduleBadge::Expired,
        ),
        (
            vessel_proto::automation::EndReason::TargetGone,
            ScheduleBadge::TargetGone,
        ),
        (
            vessel_proto::automation::EndReason::CreatorGone,
            ScheduleBadge::CreatorGone,
        ),
        (
            vessel_proto::automation::EndReason::Unknown,
            ScheduleBadge::Unknown,
        ),
    ];
    for (reason, expected) in cases {
        let state = ScheduleState::Ended { at: next, reason };
        assert_eq!(badge(&state), expected);
        assert_eq!(bucket(&state), ScheduleBucket::Ended);
    }
    assert_eq!(badge(&ScheduleState::Unknown), ScheduleBadge::Unknown);
    assert_eq!(bucket(&ScheduleState::Unknown), ScheduleBucket::Ended);
}

#[test]
fn a_person_session_target_is_a_schedule_and_keeps_pause_resume_and_run() {
    let session = SessionId::random();
    let record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session { session },
        active(at(10)),
        at(1),
    );
    assert_eq!(kind(&record), ScheduleKind::Schedule);
    assert_eq!(
        actions(&record, &direct(None), super::RunHold::Clear),
        vec![
            MenuAction::Edit,
            MenuAction::Pause,
            MenuAction::RunNow,
            MenuAction::Delete
        ]
    );
    let paused = ScheduleInfo {
        state: ScheduleState::Paused { since: at(2) },
        ..record
    };
    assert!(actions(&paused, &direct(None), super::RunHold::Clear).contains(&MenuAction::Resume));
}

#[test]
fn a_heartbeat_is_delete_only_and_a_forwarded_person_schedule_cannot_run_or_edit() {
    let session = SessionId::random();
    let heartbeat = info(
        ScheduleId::random(),
        Creator::Agent {
            node: NodeId::random(),
            session,
        },
        ScheduleTarget::Session { session },
        active(at(10)),
        at(1),
    );
    assert_eq!(kind(&heartbeat), ScheduleKind::Heartbeat);
    assert_eq!(
        actions(&heartbeat, &direct(None), super::RunHold::Clear),
        vec![MenuAction::Delete]
    );
    let person = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(10)),
        at(1),
    );
    let forwarded = SchedulePeer::Ready(ReadyPeer {
        preview: PreviewLane::Offered,
        zone: None,
        reach: ScheduleReach::Forwarded,
    });
    assert_eq!(
        actions(&person, &forwarded, super::RunHold::Clear),
        vec![MenuAction::Pause, MenuAction::Delete]
    );
    assert!(run_request(&person, &forwarded).is_err());
    assert!(run_request(&person, &direct(None)).is_ok());
}

fn nid(n: u32) -> NodeId {
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

fn sid(n: u32) -> ScheduleId {
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

#[test]
fn sort_is_newest_first_then_node_then_schedule() {
    let older = info(
        sid(3),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(1)),
        at(50),
    );
    let higher_id = info(
        sid(2),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(1)),
        at(100),
    );
    let lower_id = info(
        sid(1),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(1)),
        at(100),
    );
    let other_node = info(
        sid(1),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(1)),
        at(100),
    );
    let shown = page(
        &[
            NodeReport::Listed {
                node: nid(2),
                peer: direct(None),
                directory: SessionDirectory::Loaded,
                list: ScheduleList {
                    schedules: vec![other_node.clone()],
                },
            },
            NodeReport::Listed {
                node: nid(1),
                peer: direct(None),
                directory: SessionDirectory::Loaded,
                list: ScheduleList {
                    schedules: vec![older.clone(), higher_id.clone(), lower_id.clone()],
                },
            },
        ],
        &[],
    );
    let ids: Vec<_> = shown
        .rows
        .iter()
        .map(|row| (row.node, row.info.id))
        .collect();
    assert_eq!(
        ids,
        vec![
            (nid(1), lower_id.id),
            (nid(1), higher_id.id),
            (nid(2), other_node.id),
            (nid(1), older.id)
        ]
    );
}

#[test]
fn a_missing_session_does_not_change_the_badge() {
    let session = SessionId::random();
    let record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session { session },
        active(at(10)),
        at(1),
    );
    let listed = |directory| NodeReport::Listed {
        node: NodeId::random(),
        peer: direct(None),
        directory,
        list: ScheduleList {
            schedules: vec![record.clone()],
        },
    };
    let pending = page(&[listed(SessionDirectory::Pending)], &[]);
    let loaded = page(&[listed(SessionDirectory::Loaded)], &[]);
    assert_eq!(pending.rows[0].badge, ScheduleBadge::Active);
    assert_eq!(loaded.rows[0].badge, ScheduleBadge::Active);
    assert_eq!(pending.rows[0].badge, loaded.rows[0].badge);
}

#[test]
fn title_order_is_name_then_session_title_then_prompt_line() {
    let mut record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::NewSession {
            spec: Box::new(spec().with_title("Nightly".parse().unwrap())),
            after: AfterRun::Archive,
        },
        active(at(10)),
        at(1),
    );
    assert!(matches!(
        super::title(&record),
        TitleSource::SessionTitle(_)
    ));
    record.definition.name = Some("Standup".parse().unwrap());
    assert!(matches!(super::title(&record), TitleSource::Name(_)));
    record.definition.name = None;
    record.definition.target = ScheduleTarget::Session {
        session: SessionId::random(),
    };
    assert!(
        matches!(super::title(&record), TitleSource::PromptLine(line) if line.as_str() == "Review the queue")
    );
}

#[test]
fn presets_stay_cron_every_is_not_rewritten_and_a_short_interval_is_not_rounded() {
    let peer = direct(Some("UTC"));
    let saved = review(
        &FormJob::Create,
        &draft(
            CadenceField::Preset(CronPreset::EveryHour),
            TargetField::NewSession(spec()),
        ),
        &peer,
        None,
    );
    let super::Save::Create(create) = saved.save.unwrap() else {
        panic!("a valid create");
    };
    assert!(matches!(create.definition.cadence, Cadence::Cron { .. }));
    assert!(!create.paused);
    let every_save = review(
        &FormJob::Create,
        &draft(
            CadenceField::Every(90),
            TargetField::Session(SessionId::random()),
        ),
        &peer,
        None,
    );
    let super::Save::Create(create) = every_save.save.unwrap() else {
        panic!("ninety seconds is an interval");
    };
    assert_eq!(create.definition.cadence, every(90));
    let short = review(
        &FormJob::Create,
        &draft(
            CadenceField::Every(59),
            TargetField::Session(SessionId::random()),
        ),
        &peer,
        None,
    );
    assert!(short.save.is_none());
    assert!(
        short
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Interval(_)))
    );
}

#[test]
fn blank_cap_is_absent_zero_is_an_error_and_a_bad_field_does_not_hide_another() {
    let peer = direct(Some("UTC"));
    let mut fields = draft(
        CadenceField::Cron("not cron".to_owned()),
        TargetField::NewSession(spec()),
    );
    fields.zone = None;
    fields.prompt.clear();
    fields.max_runs = CapField::Text("0".to_owned());
    fields.ends_at = ExpiryField::Invalid;
    let reviewed = review(&FormJob::Create, &fields, &peer, None);
    assert!(reviewed.save.is_none());
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Cron(_)))
    );
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Zone))
    );
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Prompt(_)))
    );
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::ZeroCap))
    );
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Expiry))
    );
    fields.prompt = "Review the queue".to_owned();
    fields.cadence = CadenceField::Preset(CronPreset::EveryHour);
    fields.zone = Some(zone("UTC"));
    fields.max_runs = CapField::Blank;
    fields.ends_at = ExpiryField::None;
    let blank = review(&FormJob::Create, &fields, &peer, None).save.unwrap();
    let super::Save::Create(create) = blank else {
        panic!("blank cap creates");
    };
    assert!(create.definition.max_runs.is_none());
    fields.max_runs = CapField::Text("1000001".to_owned());
    let wide = review(&FormJob::Create, &fields, &peer, None);
    assert!(wide.save.is_none());
    assert!(
        wide.errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Cap))
    );
}

#[test]
fn a_new_cron_takes_the_device_zone_and_the_machine_zone_only_as_fallback() {
    let seoul = zone("Asia/Seoul");
    let utc = zone("UTC");
    let peer = direct(Some("UTC"));
    assert_eq!(new_cron_zone(Some(&seoul), &peer), Some(seoul));
    assert_eq!(new_cron_zone(None, &peer), Some(utc.clone()));
    assert_eq!(new_cron_zone(None, &SchedulePeer::Older), None);
    let stored = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(1)),
        at(1),
    );
    let mut stored = stored;
    stored.definition.cadence = Cadence::Cron {
        expr: "0 * * * *".parse().unwrap(),
        zone: zone("America/New_York"),
    };
    assert_eq!(opened_zone(&stored).unwrap().as_str(), "America/New_York");
    let _ = utc;
}

#[test]
fn preview_reuse_needs_the_same_interval_and_an_older_peer_builds_no_request() {
    let form = every(3600);
    let mut filled = every(3600);
    if let Cadence::Every { anchor, .. } = &mut filled {
        *anchor = Some(at(50));
    }
    let preview = vessel_proto::automation::SchedulePreview {
        after: at(40),
        cadence: filled.clone(),
        runs: vec![at(50)],
    };
    assert_eq!(reusable(&form, &preview), Some(&preview.cadence));
    let other = every(120);
    assert!(reusable(&other, &preview).is_none());
    assert!(matches!(
        plan(&SchedulePeer::Older, &form, None),
        PreviewPlan::OlderPeer
    ));
    assert!(matches!(
        plan(
            &SchedulePeer::Ready(ReadyPeer {
                preview: PreviewLane::Older,
                zone: None,
                reach: ScheduleReach::Direct,
            }),
            &form,
            None
        ),
        PreviewPlan::OlderPeer
    ));
    assert!(matches!(
        plan(&direct(None), &form, None),
        PreviewPlan::Request(_)
    ));
}

#[test]
fn a_heartbeat_produces_no_update_and_a_person_edit_is_the_whole_definition() {
    let session = SessionId::random();
    let heartbeat = info(
        ScheduleId::random(),
        Creator::Agent {
            node: NodeId::random(),
            session,
        },
        ScheduleTarget::Session { session },
        active(at(10)),
        at(1),
    );
    let peer = direct(None);
    let reviewed = review(
        &FormJob::Edit(heartbeat.clone()),
        &draft(CadenceField::Every(120), TargetField::Session(session)),
        &peer,
        None,
    );
    assert!(reviewed.save.is_none());
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Owner))
    );
    let person = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session { session },
        active(at(10)),
        at(1),
    );
    let reviewed = review(
        &FormJob::Edit(person.clone()),
        &draft(CadenceField::Every(120), TargetField::Session(session)),
        &peer,
        None,
    );
    let super::Save::Update(update) = reviewed.save.unwrap() else {
        panic!("a person can edit");
    };
    assert_eq!(update.schedule, person.id);
    assert_eq!(update.revision, person.revision);
    assert_eq!(update.definition.prompt, person.definition.prompt);
    assert_eq!(update.definition.target, person.definition.target);
    assert_eq!(update.definition.cadence, every(120));
    assert!(matches!(
        archive(&person, &peer, AfterChoice::Keep),
        ArchiveOutcome::Refused(_)
    ));
    let created = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::NewSession {
            spec: Box::new(spec()),
            after: AfterRun::Archive,
        },
        active(at(10)),
        at(1),
    );
    let ArchiveOutcome::Update(update) = archive(&created, &peer, AfterChoice::Keep) else {
        panic!("archive is an update");
    };
    assert_eq!(update.definition.prompt, created.definition.prompt);
    assert_eq!(update.definition.cadence, created.definition.cadence);
    assert_eq!(update.revision, created.revision);
    match update.definition.target {
        ScheduleTarget::NewSession { after, .. } => assert_eq!(after, AfterRun::Keep),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => panic!("still a new session"),
    }
}

#[test]
fn an_unknown_definition_does_not_produce_an_update_and_a_forwarded_peer_does_not_create() {
    let session = SessionId::random();
    let mut record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session { session },
        active(at(1)),
        at(1),
    );
    record.definition.cadence = Cadence::Unknown;
    let reviewed = review(
        &FormJob::Edit(record.clone()),
        &draft(CadenceField::Every(120), TargetField::Session(session)),
        &direct(None),
        None,
    );
    assert!(reviewed.save.is_none());
    assert!(
        reviewed
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::UnknownDefinition(_)))
    );
    let forwarded = SchedulePeer::Ready(ReadyPeer {
        preview: PreviewLane::Offered,
        zone: None,
        reach: ScheduleReach::Forwarded,
    });
    let created = review(
        &FormJob::Create,
        &draft(CadenceField::Every(120), TargetField::Session(session)),
        &forwarded,
        None,
    );
    assert!(created.save.is_none());
    assert!(
        created
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Reach(BlockedAct::Create)))
    );
    let updated = review(
        &FormJob::Edit(record),
        &draft(CadenceField::Every(120), TargetField::Session(session)),
        &forwarded,
        None,
    );
    assert!(updated.save.is_none());
    assert!(
        updated
            .errors
            .iter()
            .any(|error| matches!(error, ScheduleFormError::Reach(BlockedAct::Update)))
    );
}

#[test]
fn one_node_failing_keeps_the_other_and_waiting_is_not_an_empty_list() {
    let node_a = NodeId::random();
    let node_b = NodeId::random();
    let record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::Session {
            session: SessionId::random(),
        },
        active(at(4)),
        at(4),
    );
    let listed = page(
        &[
            NodeReport::Failed {
                node: node_a,
                fault: LoadFault::Unreachable,
            },
            NodeReport::Listed {
                node: node_b,
                peer: direct(None),
                directory: SessionDirectory::Loaded,
                list: ScheduleList {
                    schedules: vec![record.clone()],
                },
            },
            NodeReport::Waiting { node: node_a },
            NodeReport::Older { node: node_b },
        ],
        &[],
    );
    assert_eq!(listed.rows.len(), 1);
    assert_eq!(listed.rows[0].info.id, record.id);
    assert!(
        listed
            .notices
            .iter()
            .any(|notice| matches!(notice, NodeNotice::Failed { .. }))
    );
    assert!(
        listed
            .notices
            .iter()
            .any(|notice| matches!(notice, NodeNotice::Waiting(_)))
    );
    assert!(
        listed
            .notices
            .iter()
            .any(|notice| matches!(notice, NodeNotice::Older(_)))
    );
    let empty = page(
        &[NodeReport::Listed {
            node: node_a,
            peer: direct(None),
            directory: SessionDirectory::Loaded,
            list: ScheduleList {
                schedules: Vec::new(),
            },
        }],
        &[],
    );
    assert!(empty.rows.is_empty());
    assert!(empty.notices.is_empty());
}

#[test]
fn run_rows_match_trigger_and_outcome_and_making_hides_run_now() {
    let run = ScheduleRun {
        number: RunNumber::FIRST,
        due_at: at(1),
        at: at(2),
        trigger: RunTrigger::CaughtUp { missed: 3 },
        outcome: RunOutcome::Making,
    };
    let row = run_row(&run);
    assert!(matches!(row.trigger, ShownTrigger::CaughtUp(missed) if missed.get() == 3));
    assert!(matches!(row.outcome, ShownOutcome::Making));
    let mut record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::NewSession {
            spec: Box::new(spec()),
            after: AfterRun::Archive,
        },
        active(at(9)),
        at(1),
    );
    record.last_run = Some(run);
    assert!(!actions(&record, &direct(None), super::RunHold::Clear).contains(&MenuAction::RunNow));
    let _ = MetaPart::Created(at(1));
}

#[test]
fn next_run_is_meta_only_while_active_and_the_row_opens_the_editor_only_then() {
    let record = info(
        ScheduleId::random(),
        Creator::Owner,
        ScheduleTarget::NewSession {
            spec: Box::new(spec()),
            after: AfterRun::Archive,
        },
        active(at(9)),
        at(1),
    );
    let shown = page(
        &[NodeReport::Listed {
            node: NodeId::random(),
            peer: direct(None),
            directory: SessionDirectory::Pending,
            list: ScheduleList {
                schedules: vec![record],
            },
        }],
        &[],
    );
    assert!(
        shown.rows[0]
            .meta
            .iter()
            .any(|part| matches!(part, MetaPart::NextDue(_)))
    );
    assert_eq!(shown.rows[0].open, RowOpen::Editor);
}

#[test]
fn a_full_people_list_is_capped_and_an_older_listed_peer_hides_its_rows() {
    let node = nid(1);
    let schedules = (1..=super::list::PEOPLE_LIST_CAP)
        .map(|n| {
            info(
                sid(u32::try_from(n).unwrap()),
                Creator::Owner,
                ScheduleTarget::Session {
                    session: SessionId::random(),
                },
                active(at(1)),
                at(i64::try_from(n).unwrap()),
            )
        })
        .collect();
    let shown = page(
        &[NodeReport::Listed {
            node,
            peer: direct(None),
            directory: SessionDirectory::Loaded,
            list: ScheduleList { schedules },
        }],
        &[],
    );
    assert_eq!(shown.rows.len(), super::list::PEOPLE_LIST_CAP);
    assert!(
        shown
            .notices
            .iter()
            .any(|notice| matches!(notice, NodeNotice::Capped(id) if *id == node))
    );
    let hidden = page(
        &[NodeReport::Listed {
            node,
            peer: SchedulePeer::Older,
            directory: SessionDirectory::Pending,
            list: ScheduleList {
                schedules: vec![info(
                    sid(1),
                    Creator::Owner,
                    ScheduleTarget::Session {
                        session: SessionId::random(),
                    },
                    active(at(1)),
                    at(1),
                )],
            },
        }],
        &[],
    );
    assert!(hidden.rows.is_empty());
    assert!(
        hidden
            .notices
            .iter()
            .any(|notice| matches!(notice, NodeNotice::Older(_)))
    );
}
