//! The page's `data/marks.test.ts` and `data/titles.test.ts`.

use crate::support::EN;
use serde_json::json;
use vessel_app_view::session::{
    MarkAction, ReadView, can_start_again, check_title, has_title, mark_actions, read_through,
    session_name, title_from_prompt,
};
use vessel_proto::{EventSeq, InvalidTitle, ProtocolVersion, SessionInfo, SessionTitle};

fn live() -> SessionInfo {
    serde_json::from_value(json!({
        "id": "8e7f7d69-813c-4f55-9a36-48f03226cfd1",
        "spec": { "harness": "claude-code", "mode": "structured", "cwd": "/work/vessel" },
        "created_by": { "kind": "owner" },
        "status": { "state": "running" },
        "created_at": "2026-11-11T08:00:00Z",
        "next_seq": 4
    }))
    .unwrap()
}

fn ended() -> SessionInfo {
    let mut session = serde_json::to_value(live()).unwrap();
    session["status"] = json!({ "state": "exited", "code": 0 });
    session["harness_session"] = json!("h1");
    session["abilities"] =
        json!({ "steer": false, "resume": true, "image": false, "audio": false });
    serde_json::from_value(session).unwrap()
}

const P9: Option<ProtocolVersion> = Some(ProtocolVersion::new(9));
const P10: Option<ProtocolVersion> = Some(ProtocolVersion::new(10));

#[test]
fn nothing_on_a_machine_before_protocol_10() {
    assert!(mark_actions(&ended(), P9).is_empty());
    assert!(mark_actions(&ended(), None).is_empty());
}

#[test]
fn a_live_session_pin_unread_archive_no_delete_or_start_again() {
    assert_eq!(
        mark_actions(&live(), P10),
        [MarkAction::Pin, MarkAction::MarkUnread, MarkAction::Archive]
    );
}

#[test]
fn an_ended_one_archived_pinned_and_unread() {
    let mut marked = ended();
    marked.marks.pinned = true;
    marked.marks.unread = true;
    marked.marks.archived_at = Some("2026-11-12T00:00:00Z".parse().unwrap());
    let actions = mark_actions(&marked, P10);
    assert_eq!(
        actions,
        [
            MarkAction::Unpin,
            MarkAction::MarkRead,
            MarkAction::StartAgain,
            MarkAction::Unarchive,
            MarkAction::Delete
        ]
    );
    let words: Vec<&str> = actions
        .iter()
        .map(|action| EN.mark_action(*action))
        .collect();
    assert_eq!(
        words,
        [
            "Unpin session",
            "Mark as read",
            "Start again",
            "Unarchive",
            "Delete permanently"
        ]
    );
}

#[test]
fn start_again_only_where_the_agent_can_pick_up_its_conversation() {
    assert!(can_start_again(&ended()));
    let mut cannot = ended();
    cannot.abilities.as_mut().unwrap().resume = false;
    assert!(!can_start_again(&cannot));
    // Stored before protocol 10: no abilities recorded, the node decides.
    let mut older = ended();
    older.abilities = None;
    assert!(can_start_again(&older));
    // No agent session to load.
    let mut nothing = ended();
    nothing.harness_session = None;
    assert!(!can_start_again(&nothing));
    let mut terminal = serde_json::to_value(ended()).unwrap();
    terminal["spec"] = json!({
        "harness": "claude-code", "mode": "terminal", "args": [], "size": { "cols": 80, "rows": 24 }
    });
    assert!(!can_start_again(&serde_json::from_value(terminal).unwrap()));
}

const FRONT: ReadView = ReadView {
    front: true,
    unread: false,
    attention: None,
    next: EventSeq::new(10),
    sent: None,
    held_at: None,
};

const fn seq(n: u64) -> EventSeq {
    EventSeq::new(n)
}

#[test]
fn only_in_front_and_only_after_something_asked_for_attention() {
    assert_eq!(read_through(&FRONT), None);
    assert_eq!(
        read_through(&ReadView {
            attention: Some(seq(7)),
            ..FRONT
        }),
        Some(seq(10))
    );
    assert_eq!(
        read_through(&ReadView {
            attention: Some(seq(7)),
            front: false,
            ..FRONT
        }),
        None
    );
    assert_eq!(
        read_through(&ReadView {
            unread: true,
            ..FRONT
        }),
        Some(seq(10))
    );
}

#[test]
fn not_again_for_what_was_already_sent() {
    assert_eq!(
        read_through(&ReadView {
            attention: Some(seq(7)),
            sent: Some(seq(10)),
            ..FRONT
        }),
        None
    );
    // A new turn's end after the last read.
    assert_eq!(
        read_through(&ReadView {
            attention: Some(seq(11)),
            next: EventSeq::new(12),
            sent: Some(seq(10)),
            ..FRONT
        }),
        Some(seq(12))
    );
}

#[test]
fn after_mark_unread_not_until_a_record_comes_after_it() {
    let held = ReadView {
        unread: true,
        attention: Some(seq(7)),
        sent: Some(seq(8)),
        held_at: Some(seq(10)),
        ..FRONT
    };
    assert_eq!(read_through(&held), None);
    assert_eq!(
        read_through(&ReadView {
            next: EventSeq::new(11),
            attention: Some(seq(10)),
            ..held
        }),
        Some(seq(11))
    );
}

#[test]
fn what_became_of_the_worktree_in_words() {
    use vessel_proto::workspace::{KeptReason, RemoveWorktreeOutcome};
    let kept = RemoveWorktreeOutcome::Kept {
        reasons: vec![
            KeptReason::Uncommitted { files: 2 },
            KeptReason::UniqueCommits { commits: 1 },
        ],
    };
    assert_eq!(
        EN.deleted_worktree(&kept),
        "Its worktree was kept: 2 uncommitted files, 1 commit no other branch has."
    );
    assert_eq!(
        format!(
            "{} {}",
            EN.session_deleted(),
            EN.deleted_worktree(&RemoveWorktreeOutcome::Removed { snapshot: None })
        ),
        "Session deleted Its worktree was removed."
    );
}

#[test]
fn the_archives_confirmation_says_the_agent_stops_and_what_ends_beside_it() {
    assert_eq!(
        EN.archive_words(true, "laptop", true),
        "Archiving stops the agent on laptop first. The terminals and services running beside it end too; their output is kept. You can always restore it from the Archive section later."
    );
    assert_eq!(
        EN.archive_words(false, "laptop", false),
        "You can always restore it from the Archive section later."
    );
}

#[test]
fn takes_the_logs_rename_then_the_nodes_title_then_the_folder() {
    assert_eq!(session_name(&live(), None).as_deref(), Some("vessel"));
    assert!(!has_title(&live()));
    let mut titled = live();
    titled.title = Some("Fix the fleet overview".parse().unwrap());
    assert_eq!(
        session_name(&titled, None).as_deref(),
        Some("Fix the fleet overview")
    );
    assert!(has_title(&titled));
    let renamed: SessionTitle = "Renamed".parse().unwrap();
    assert_eq!(
        session_name(&titled, Some(&renamed)).as_deref(),
        Some("Renamed")
    );
    let mut homeless = serde_json::to_value(live()).unwrap();
    homeless["spec"]["cwd"] = json!("/");
    let homeless: SessionInfo = serde_json::from_value(homeless).unwrap();
    assert_eq!(session_name(&homeless, None), None);
    assert_eq!(EN.untitled(), "Untitled session");
}

#[test]
fn trims_and_refuses_what_the_node_refuses_counting_characters() {
    assert_eq!(
        check_title("  Release notes  ").map(|title| title.to_string()),
        Ok("Release notes".to_owned())
    );
    assert_eq!(check_title("   ").map(|_| ()), Err(InvalidTitle::Empty));
    assert_eq!(
        check_title("two\nlines").map(|_| ()),
        Err(InvalidTitle::ControlCharacter)
    );
    let max = SessionTitle::MAX_CHARS;
    let over = max.saturating_add(1);
    assert!(check_title(&"x".repeat(max)).is_ok());
    assert!(check_title(&"x".repeat(over)).is_err());
    assert!(check_title(&"한".repeat(max)).is_ok());
    assert!(check_title(&"🙂".repeat(max)).is_ok());
    assert_eq!(
        EN.title_refused(InvalidTitle::TooLong { chars: over }),
        format!("A title is at most {max} characters.")
    );
}

#[test]
fn takes_the_first_line_with_text_cut_at_the_prompt_title_length() {
    let title = |text: &str| title_from_prompt(text).map(|title| title.to_string());
    assert_eq!(
        title("\n\n  Tidy the overview \nand more").as_deref(),
        Some("Tidy the overview")
    );
    assert_eq!(
        title(&"a".repeat(100)),
        Some("a".repeat(SessionTitle::FROM_PROMPT_CHARS))
    );
    assert_eq!(title("  \n "), None);
}
