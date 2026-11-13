//! `web/src/data/board.test.ts`, "the sessions board".

use chrono::{DateTime, TimeZone as _, Utc};
use vessel_proto::{
    Activity, DisplayStatus, HarnessKind, KanbanColumn, SessionBoard, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus,
};

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
    let failed = card(
        SessionStatus::Failed {
            message: "no login".to_owned(),
        },
        None,
    );
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
            name: "laptop".to_owned(),
            reach: Reach::Online,
            sessions,
            agents: Vec::new(),
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
