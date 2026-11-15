//! The form's defaults (`gh pr create --fill`), its checks (the wire's
//! title and description rules) and its answer.

use chrono::{TimeZone as _, Utc};
use vessel_proto::SessionId;
use vessel_proto::forge::{OpenOutcome, OpenRefusal};
use vessel_proto::workspace::CommitInfo;

use super::{OpenDraft, OpenProblem, Opened, open_outcome, open_request, prefill};

fn session() -> SessionId {
    "00000000-0000-4000-8000-000000000042".parse().unwrap()
}

fn commit(subject: &str) -> CommitInfo {
    CommitInfo {
        id: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        parents: vec![],
        author: "A Person".to_owned(),
        time: Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap(),
        subject: subject.to_owned(),
    }
}

fn draft(title: &str, body: &str) -> OpenDraft {
    OpenDraft {
        title: title.to_owned(),
        body: body.to_owned(),
        base: None,
        draft: false,
    }
}

#[test]
fn one_new_commit_gives_its_subject() {
    let start = prefill(&[commit("Add the widget")], "feature/widget", None);
    assert_eq!(start.title, "Add the widget");
    assert_eq!(start.body, "");
    assert!(!start.draft);
}

#[test]
fn several_give_the_branch_read_as_words_and_their_subjects_oldest_first() {
    let newest_first = [commit("Test it"), commit("Draw it"), commit("Model it")];
    let start = prefill(
        &newest_first,
        "feature/new_widget-panel",
        Some("main".parse().unwrap()),
    );
    assert_eq!(start.title, "feature/new widget panel");
    assert_eq!(start.body, "- Model it\n- Draw it\n- Test it\n");
    assert_eq!(
        start.base.map(|base| base.to_string()),
        Some("main".to_owned())
    );
}

#[test]
fn no_commit_read_gives_the_branch() {
    assert_eq!(prefill(&[], "fix-login", None).title, "fix login");
}

#[test]
fn the_request_carries_what_was_typed_trimmed() {
    let mut typed = draft("  Add the widget \n", "Why it is needed.\n\n");
    typed.base = Some("develop".parse().unwrap());
    typed.draft = true;
    let request = open_request(session(), &typed).unwrap();
    assert_eq!(request.title.as_str(), "Add the widget");
    assert_eq!(
        request.body.as_ref().map(|body| body.as_str().to_owned()),
        Some("Why it is needed.".to_owned())
    );
    assert_eq!(
        request.base.map(|base| base.to_string()),
        Some("develop".to_owned())
    );
    assert!(request.draft);
    assert_eq!(request.session, session());
}

#[test]
fn an_empty_description_sends_none_and_a_pasted_line_break_is_a_space() {
    let request = open_request(session(), &draft("Fix\tthe\nlogin", "  \n ")).unwrap();
    assert_eq!(request.title.as_str(), "Fix the login");
    assert!(request.body.is_none());
}

#[test]
fn the_wire_s_rules_are_named() {
    assert_eq!(
        open_request(session(), &draft(" \n ", "")).unwrap_err(),
        OpenProblem::TitleBlank
    );
    let long = "가".repeat(257);
    assert_eq!(
        open_request(session(), &draft(&long, "")).unwrap_err(),
        OpenProblem::TitleLong
    );
    assert!(open_request(session(), &draft(&"가".repeat(256), "")).is_ok());
    let body = "x".repeat(64 * 1024 + 1);
    assert_eq!(
        open_request(session(), &draft("Title", &body)).unwrap_err(),
        OpenProblem::BodyLong
    );
}

#[test]
fn an_unknown_outcome_is_a_refusal_the_form_shows() {
    assert_eq!(
        open_outcome(OpenOutcome::Unknown),
        Opened::Refused(OpenRefusal::Unknown)
    );
    assert_eq!(
        open_outcome(OpenOutcome::Refused {
            refusal: OpenRefusal::NoCommits
        }),
        Opened::Refused(OpenRefusal::NoCommits)
    );
}
