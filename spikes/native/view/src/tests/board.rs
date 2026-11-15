//! `web/src/data/board.test.ts`, "the sessions board".

use chrono::{DateTime, TimeZone as _, Utc};
use vessel_proto::{
    Activity, DisplayStatus, HarnessKind, KanbanColumn, SessionBoard, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus,
};

use crate::board::Tone;
use crate::{Attention, Card, Fleet, Lane, Lang, Machine, Phrase, Reach};

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn placed(column: KanbanColumn, status: DisplayStatus, when: &str) -> SessionBoard {
    SessionBoard::new(column, status, Activity::Idle, at(when))
}

fn session(status: SessionStatus, board: Option<SessionBoard>) -> SessionInfo {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(SessionId::random(), spec, at("2026-11-07T09:00:00Z"));
    info.status = status;
    info.board = board;
    info
}

fn card(status: SessionStatus, board: Option<SessionBoard>) -> Card {
    Card::from_session("laptop", &session(status, board))
}

const RUNNING: SessionStatus = SessionStatus::Running;

#[test]
fn places_a_session_where_its_node_says() {
    let card = card(
        RUNNING,
        Some(placed(
            KanbanColumn::NeedsReview,
            DisplayStatus::ChangesRequested,
            "2026-11-07T10:00:00Z",
        )),
    );
    assert_eq!(card.lane, Some(Lane::InReview));
    assert_eq!(Lang::En.text().phrase(&card.phrase), "Changes requested");
}

#[test]
fn keeps_the_place_a_status_implies_for_a_node_that_sends_none() {
    let t = Lang::En.text();
    let live = card(RUNNING, None);
    assert_eq!(live.lane, Some(Lane::Building));
    assert_eq!(t.phrase(&live.phrase), "running");
    assert_eq!(
        card(SessionStatus::Starting, None).lane,
        Some(Lane::Building)
    );
    let ended = card(SessionStatus::Exited { code: Some(0) }, None);
    assert_eq!(ended.lane, None);
    assert_eq!(t.phrase(&ended.phrase), "exited 0");
    let failed = card(SessionStatus::failed("no login"), None);
    assert_eq!(failed.lane, None);
    assert_eq!(failed.phrase, Phrase::Failed);
}

#[test]
fn names_the_lanes_and_the_phrases_as_the_reference_does() {
    let t = Lang::En.text();
    let lanes: Vec<&str> = Lane::ALL.iter().map(|lane| t.lane(*lane)).collect();
    assert_eq!(lanes, ["Building", "Validating", "In review", "Ready"]);
    assert_eq!(t.display_status(DisplayStatus::AwaitingPr), "Awaiting PR");
    assert_eq!(
        t.display_status(DisplayStatus::FixingCi),
        "Fixing CI failures"
    );
    assert_eq!(
        t.display_status(DisplayStatus::Closed),
        "Closed without merge"
    );
    assert_eq!(
        t.display_status(DisplayStatus::NeedsHumanReview),
        "Needs human review"
    );
}

#[test]
fn marks_who_waits_on_the_owner_and_what_is_turning() {
    let attention = |status| {
        card(
            RUNNING,
            Some(placed(
                KanbanColumn::Building,
                status,
                "2026-11-07T10:00:00Z",
            )),
        )
        .attention
    };
    for status in [
        DisplayStatus::Blocked,
        DisplayStatus::CiFailing,
        DisplayStatus::ChangesRequested,
    ] {
        assert_eq!(attention(status), Attention::NeedsYou, "{status:?}");
    }
    assert_ne!(attention(DisplayStatus::Working), Attention::NeedsYou);
    assert_ne!(attention(DisplayStatus::Commented), Attention::NeedsYou);
    // An older node never says a session needs its owner.
    assert_ne!(card(RUNNING, None).attention, Attention::NeedsYou);
    for status in [
        DisplayStatus::Working,
        DisplayStatus::FixingCi,
        DisplayStatus::AddressingComments,
        DisplayStatus::Reviewing,
        DisplayStatus::ReviewPending,
    ] {
        assert_eq!(attention(status), Attention::Working, "{status:?}");
    }
    assert_ne!(attention(DisplayStatus::Mergeable), Attention::Working);
    assert_ne!(attention(DisplayStatus::Blocked), Attention::Working);
    assert_eq!(
        card(SessionStatus::Starting, None).attention,
        Attention::Working
    );
    assert_ne!(card(RUNNING, None).attention, Attention::Working);
}

#[test]
fn deals_rows_into_lanes_the_waiting_first_then_the_most_recent() {
    let named = |title: &str, status, board| {
        let mut card = card(status, board);
        card.title = title.to_owned();
        card
    };
    let exited = || SessionStatus::Exited { code: Some(0) };
    let sessions = vec![
        named(
            "old",
            RUNNING,
            Some(placed(
                KanbanColumn::Building,
                DisplayStatus::Working,
                "2026-11-07T10:00:00Z",
            )),
        ),
        named(
            "new",
            RUNNING,
            Some(placed(
                KanbanColumn::Building,
                DisplayStatus::Idle,
                "2026-11-07T12:00:00Z",
            )),
        ),
        named(
            "stuck",
            RUNNING,
            Some(placed(
                KanbanColumn::Building,
                DisplayStatus::Blocked,
                "2026-11-07T08:00:00Z",
            )),
        ),
        named(
            "review",
            RUNNING,
            Some(placed(
                KanbanColumn::NeedsReview,
                DisplayStatus::Commented,
                "2026-11-07T10:00:00Z",
            )),
        ),
        named(
            "done",
            exited(),
            Some(placed(
                KanbanColumn::Archive,
                DisplayStatus::Terminated,
                "2026-11-07T11:00:00Z",
            )),
        ),
        named("gone", exited(), None),
    ];
    let fleet = Fleet {
        machines: vec![Machine {
            node: None,
            name: "laptop".to_owned(),
            reach: Reach::Online,
            sessions,
            agents: Vec::new(),
            protocol: None,
            version: None,
        }],
        read_at: None,
    };
    let titles = |cards: Vec<&Card>| {
        cards
            .iter()
            .map(|card| card.title.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(titles(fleet.lane(Lane::Building)), ["stuck", "new", "old"]);
    assert_eq!(titles(fleet.lane(Lane::InReview)), ["review"]);
    assert!(fleet.lane(Lane::Validating).is_empty());
    assert!(fleet.lane(Lane::Ready).is_empty());
    // The archive is newest first; an older node's ended session sorts by
    // its start when it reports no end.
    assert_eq!(titles(fleet.archive()), ["done", "gone"]);
    assert_eq!(fleet.on_board(), 4);
}

#[test]
fn writes_how_long_ago_never_in_weeks() {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap();
    let since = |when: &str| {
        let card = Card {
            at: at(when),
            ..card(RUNNING, None)
        };
        Lang::En.text().since(card.minutes_since(now))
    };
    assert_eq!(since("2026-11-07T11:59:40Z"), "just now");
    assert_eq!(since("2026-11-07T12:05:00Z"), "just now");
    assert_eq!(since("2026-11-07T11:54:00Z"), "6m ago");
    assert_eq!(since("2026-11-07T09:00:00Z"), "3h ago");
    assert_eq!(since("2026-10-20T12:00:00Z"), "18d ago");
    // The same ages in Korean.
    assert_eq!(Lang::Ko.text().since(6), "6분 전");
    assert_eq!(Lang::Ko.text().since(180), "3시간 전");
}

#[test]
fn colours_a_status_line_by_its_lane_unless_the_phrase_says_more() {
    let tone = |column, status| {
        card(
            RUNNING,
            Some(placed(column, status, "2026-11-07T10:00:00Z")),
        )
        .tone()
    };
    assert_eq!(
        tone(KanbanColumn::Building, DisplayStatus::Working),
        Tone::Lane
    );
    assert_eq!(
        tone(KanbanColumn::Building, DisplayStatus::Blocked),
        Tone::NeedsYou
    );
    assert_eq!(
        tone(KanbanColumn::Ready, DisplayStatus::Mergeable),
        Tone::Ready
    );
    assert_eq!(
        tone(KanbanColumn::Ready, DisplayStatus::Merged),
        Tone::Merged
    );
    assert_eq!(
        tone(KanbanColumn::Ready, DisplayStatus::Closed),
        Tone::Danger
    );
    assert_eq!(
        tone(KanbanColumn::Archive, DisplayStatus::Terminated),
        Tone::Quiet
    );
    // An ended session of an older node shows its own word; a failure
    // stays visible in the archive.
    let failed = SessionStatus::failed("no login");
    assert_eq!(card(failed, None).tone(), Tone::Danger);
    assert_eq!(
        card(SessionStatus::Exited { code: Some(0) }, None).tone(),
        Tone::Quiet
    );
}

#[test]
fn offers_kill_on_a_live_card_archive_on_an_ended_one_where_marks_are_kept() {
    use crate::board::CardActions;
    use vessel_proto::ProtocolVersion;
    let marks = Some(ProtocolVersion::new(10));
    let older = Some(ProtocolVersion::new(9));
    let live = card(RUNNING, None);
    assert_eq!(live.actions(marks, false), CardActions::Kill);
    assert_eq!(live.actions(None, false), CardActions::Kill);
    let ended = card(SessionStatus::Exited { code: Some(0) }, None);
    assert_eq!(ended.actions(marks, false), CardActions::Archive);
    assert_eq!(ended.actions(older, false), CardActions::None);
    assert_eq!(ended.actions(None, false), CardActions::None);
    // In the archive: only what was archived is restored or deleted, and
    // restoring starts it again where its agent can pick up.
    assert_eq!(ended.actions(marks, true), CardActions::None);
    let mut archived = ended;
    archived.detail.marks.archived_at = Some(at("2026-11-07T11:00:00Z"));
    assert_eq!(
        archived.actions(marks, true),
        CardActions::Restore { again: false }
    );
    archived.detail.can_start_again = true;
    assert_eq!(
        archived.actions(marks, true),
        CardActions::Restore { again: true }
    );
}

#[test]
fn reads_a_machines_protocol_only_while_it_is_connected() {
    use vessel_proto::ProtocolVersion;
    let machine = |name: &str, protocol| Machine {
        node: None,
        name: name.to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol,
        version: None,
    };
    let fleet = Fleet {
        machines: vec![
            machine("laptop", Some(ProtocolVersion::new(11))),
            machine("old-box", None),
        ],
        read_at: None,
    };
    assert_eq!(fleet.protocol_of("laptop"), Some(ProtocolVersion::new(11)));
    assert_eq!(fleet.protocol_of("old-box"), None);
    assert_eq!(fleet.protocol_of("elsewhere"), None);
}

#[test]
fn puts_an_answer_in_place_of_its_card_and_drops_a_deleted_one() {
    let mut info = session(SessionStatus::Exited { code: Some(0) }, None);
    let listed = Card::from_session("laptop", &info);
    let other = card(RUNNING, None);
    let mut fleet = Fleet {
        machines: vec![Machine {
            node: None,
            name: "laptop".to_owned(),
            reach: Reach::Online,
            sessions: vec![listed, other.clone()],
            agents: Vec::new(),
            protocol: None,
            version: None,
        }],
        read_at: None,
    };
    assert_eq!(fleet.board(None).lane(Lane::Building).len(), 1);
    info.marks.archived_at = Some(at("2026-11-07T11:00:00Z"));
    fleet.put_session("elsewhere", &info);
    assert!(
        fleet.machines[0].sessions[0]
            .detail
            .marks
            .archived_at
            .is_none()
    );
    fleet.put_session("laptop", &info);
    assert!(
        fleet.machines[0].sessions[0]
            .detail
            .marks
            .archived_at
            .is_some()
    );
    fleet.drop_session("laptop", info.id);
    let left: Vec<_> = fleet.machines[0]
        .sessions
        .iter()
        .map(|card| card.session)
        .collect();
    assert_eq!(left, [other.session]);
}

#[test]
fn a_fork_is_listed_on_its_machine_at_once_and_only_once() {
    let listed = card(RUNNING, None);
    let mut fleet = Fleet {
        machines: vec![Machine {
            node: None,
            name: "laptop".to_owned(),
            reach: Reach::Online,
            sessions: vec![listed.clone()],
            agents: Vec::new(),
            protocol: None,
            version: None,
        }],
        read_at: None,
    };
    let fork = session(RUNNING, None);
    fleet.add_session("elsewhere", &fork);
    assert_eq!(fleet.machines[0].sessions.len(), 1, "not its machine");
    fleet.add_session("laptop", &fork);
    // The fleet's next read may list it before the answer is taken.
    fleet.add_session("laptop", &fork);
    let ids: Vec<_> = fleet.machines[0]
        .sessions
        .iter()
        .map(|card| card.session)
        .collect();
    assert_eq!(ids, [listed.session, fork.id]);
    // Its first line names the source by the title the list holds.
    fleet.machines[0].sessions[0].title = "plain".to_owned();
    assert_eq!(fleet.title_of(listed.session), Some("plain"));
    assert_eq!(fleet.title_of(fork.id), None, "no title yet");
    assert_eq!(fleet.title_of(SessionId::random()), None);
}

#[test]
fn says_why_a_restore_could_not_start_again_in_the_pages_words() {
    use vessel_proto::ErrorCode;
    let words = Lang::En.board();
    assert_eq!(
        words.again_refused(Some(ErrorCode::Unsupported), "x"),
        "This agent cannot pick up where it left off."
    );
    assert_eq!(
        words.again_refused(Some(ErrorCode::InvalidRequest), "no profile fast"),
        "Its profile is gone: no profile fast"
    );
    assert_eq!(
        words.again_refused(Some(ErrorCode::Conflict), "its log is still being written"),
        "its log is still being written"
    );
    assert_eq!(
        words.again_refused(None, "connection lost"),
        "connection lost"
    );
}
