//! Paseo's import sheet on inert in-memory replies: providers in parallel and
//! merged, limits read through cursor pages, per-provider failures, one import.
use super::*;
use crate::devices::MachineKey;
use crate::history::{Listed, Outcome};
use vessel_proto::{HarnessKind, SessionId, SessionMode};

const NODE: &str = "00000000-0000-0000-0000-000000000001";

fn opening(cwd: Option<&str>) -> Opening {
    Opening {
        context: crate::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("laptop"), NODE.parse().unwrap())
            .unwrap(),
        machine: MachineKey::from_book("laptop"),
        cwd: cwd.map(PathBuf::from),
        ranked: false,
    }
}

fn codex() -> HarnessRef {
    HarnessKind::Codex.into()
}

fn claude() -> HarnessRef {
    HarnessKind::ClaudeCode.into()
}

fn entry(id: &str, minute: i64) -> Entry {
    Entry {
        id: HarnessSessionId::new(id),
        cwd: PathBuf::from("/work/project"),
        title: Some(id.parse::<SessionTitle>().unwrap()),
        updated_at: chrono::DateTime::from_timestamp(minute.saturating_mul(60), 0),
        in_vessel: None,
        first_prompt: None,
        last_prompt: None,
    }
}

fn entries(prefix: &str, count: usize) -> Vec<Entry> {
    (0..count)
        .map(|index| entry(&format!("{prefix}-{index}"), i64::try_from(index).unwrap()))
        .collect()
}

fn reply(ask: &Ask, entries: Vec<Entry>, next: Option<&str>) -> Reply {
    let Ask::List { scope, ticket, .. } = ask else {
        panic!("a listing");
    };
    Reply::Listed {
        scope: scope.clone(),
        ticket: *ticket,
        result: Ok(Listed {
            entries,
            node: NODE.parse().unwrap(),
            next: next.map(|cursor| Cursor::parse(cursor.to_owned()).unwrap()),
            already_imported: 0,
        }),
    }
}

fn harness_of(ask: &Ask) -> &HarnessRef {
    &ask.scope().harness
}

fn opened(cwd: Option<&str>) -> (Sheet, [Ask; 2]) {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(cwd), vec![codex(), claude()]).unwrap();
    let Ok(pair) = <[Ask; 2]>::try_from(asks) else {
        panic!("one read per provider");
    };
    (sheet, pair)
}

fn only(asks: Vec<Ask>) -> Ask {
    let Ok([ask]) = <[Ask; 1]>::try_from(asks) else {
        panic!("one read");
    };
    ask
}

#[test]
fn every_provider_is_read_in_parallel_in_the_workspace_folder() {
    let (sheet, [first, second]) = opened(Some("/work/project"));
    let asks = [&first, &second];
    assert_eq!(harness_of(&first), &codex());
    assert_eq!(harness_of(&second), &claude());
    assert!(
        asks.iter()
            .all(|ask| ask.scope().cwd == Some(PathBuf::from("/work/project")))
    );
    assert!(sheet.scoped());
    assert!(sheet.loading());
    assert_eq!(sheet.providers(), [&codex(), &claude()]);
}

#[test]
fn rows_merge_newest_first_without_imported_sessions_and_filter_by_provider() {
    let (mut sheet, [first, second]) = opened(None);
    let mut imported = entry("codex-done", 50);
    imported.in_vessel = Some(SessionId::random());
    assert!(
        sheet
            .answer(reply(&first, vec![entry("codex-a", 10), imported], None))
            .applied
    );
    assert!(
        sheet
            .answer(reply(&second, vec![entry("claude-a", 20)], None))
            .applied
    );
    let all = Query::new("");
    let ids: Vec<&str> = sheet
        .rows(&all)
        .iter()
        .map(|row| row.entry.id.as_str())
        .collect();
    assert_eq!(ids, ["claude-a", "codex-a"]);
    sheet.set_filter(Filter::Provider(codex()));
    let ids: Vec<&str> = sheet
        .rows(&all)
        .iter()
        .map(|row| row.entry.id.as_str())
        .collect();
    assert_eq!(ids, ["codex-a"]);
    assert!(!sheet.loading());
}

#[test]
fn a_provider_reads_on_through_cursor_pages_until_its_limit() {
    let (mut sheet, [first, _second]) = opened(None);
    // Ten listable sessions and a next page: read on.
    let answer = sheet.answer(reply(&first, entries("codex", 10), Some("page-2")));
    assert!(answer.applied);
    let follow = only(answer.follow);
    let Ask::List { cursor, .. } = &follow else {
        panic!("a listing");
    };
    assert_eq!(cursor.as_ref().map(Cursor::as_str), Some("page-2"));
    // Twenty in all: the first limit (15) is held, so no further read.
    let more: Vec<Entry> = (10..20)
        .map(|index| entry(&format!("codex-{index}"), index))
        .collect();
    let answer = sheet.answer(reply(&follow, more, Some("page-3")));
    assert!(answer.follow.is_empty());
    assert_eq!(sheet.rows(&Query::new("")).len(), 15);
    assert!(sheet.has_more());
    // "Load more": the limit grows to 45 and the provider reads on.
    let _next = only(sheet.more().unwrap());
    assert_eq!(sheet.limit(), 45);
    assert_eq!(sheet.rows(&Query::new("")).len(), 20);
}

#[test]
fn a_failed_provider_keeps_the_others_and_retries_alone() {
    let (mut sheet, [first, second]) = opened(None);
    let failed = first.refused(Problem::Timeout);
    assert!(sheet.answer(failed).applied);
    assert!(
        sheet
            .answer(reply(&second, vec![entry("claude-a", 1)], None))
            .applied
    );
    assert_eq!(sheet.failures(), [(&codex(), Problem::Timeout)]);
    assert_eq!(sheet.rows(&Query::new("")).len(), 1);
    let retried = only(sheet.retry(&codex()).unwrap());
    assert_eq!(harness_of(&retried), &codex());
}

#[test]
fn a_stale_reply_changes_nothing() {
    let (mut sheet, [first, _second]) = opened(None);
    let stale = reply(&first, vec![entry("codex-a", 1)], None);
    let _fresh = sheet.refresh().unwrap();
    assert!(!sheet.answer(stale).applied);
    assert!(sheet.rows(&Query::new("")).is_empty());
}

#[test]
fn a_row_imports_one_session_and_hands_the_new_session_over() {
    let (mut sheet, [first, _second]) = opened(None);
    let answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    assert!(answer.applied);
    let ask = sheet
        .import(&codex(), &HarnessSessionId::new("codex-a"))
        .unwrap()
        .unwrap();
    let Ask::Import(batch) = ask else {
        panic!("an import");
    };
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(
        sheet.importing().map(HarnessSessionId::as_str),
        Some("codex-a")
    );
    // A second press while one is pending does nothing.
    assert!(
        sheet
            .import(&codex(), &HarnessSessionId::new("codex-a"))
            .unwrap()
            .is_none()
    );
    let created = SessionId::random();
    let entry = batch.entries.first().unwrap().clone();
    let outcome = Outcome {
        id: entry.id.clone(),
        result: Ok(Box::new(SessionInfo::new(
            created,
            entry.spec(&batch.scope),
            chrono::DateTime::UNIX_EPOCH,
        ))),
    };
    assert!(
        sheet
            .answer(Reply::Imported {
                scope: batch.scope.clone(),
                ticket: batch.ticket,
                outcomes: vec![outcome],
            })
            .applied
    );
    let (scope, info) = sheet.take_imported().unwrap();
    assert_eq!(info.id, created);
    assert_eq!(scope, batch.scope, "the import's own scope comes with it");
    assert!(sheet.importing().is_none());
    // The imported session leaves the list; nothing left says so.
    assert!(sheet.rows(&Query::new("")).is_empty());
    assert_eq!(sheet.empty(&Query::new("")), Empty::AlreadyImported);
}

#[test]
fn a_failed_import_keeps_the_sheet_and_says_so() {
    let (mut sheet, [first, _second]) = opened(None);
    let _answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let ask = sheet
        .import(&codex(), &HarnessSessionId::new("codex-a"))
        .unwrap()
        .unwrap();
    assert!(
        sheet
            .answer(ask.refused(Problem::Refused(vessel_proto::ErrorCode::Conflict)))
            .applied
    );
    assert_eq!(
        sheet.import_failed(),
        Some(Problem::Refused(vessel_proto::ErrorCode::Conflict))
    );
    assert!(sheet.take_imported().is_none());
    assert_eq!(sheet.rows(&Query::new("")).len(), 1);
}

#[test]
fn empty_states_follow_paseo_s_order() {
    let (mut sheet, [first, second]) = opened(None);
    let _codex = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let _claude = sheet.answer(reply(&second, Vec::new(), None));
    assert_eq!(sheet.empty(&Query::new("zzz")), Empty::NoMatches);
    sheet.set_filter(Filter::Provider(claude()));
    assert_eq!(
        sheet.empty(&Query::new("")),
        Empty::NoProviderSessions(claude())
    );
    sheet.set_filter(Filter::All);
    let (empty, _asks) = opened(None);
    assert_eq!(empty.empty(&Query::new("")), Empty::NoRecent);
}

#[test]
fn the_empty_state_waits_for_every_provider_and_not_every_failure() {
    let (mut sheet, [first, second]) = opened(None);
    let all = Query::new("");
    assert_eq!(sheet.empty_state(&all), None, "still reading");
    let _codex = sheet.answer(reply(&first, Vec::new(), None));
    assert_eq!(sheet.empty_state(&all), None, "one provider still reading");
    let _claude = sheet.answer(second.refused(Problem::Timeout));
    assert_eq!(sheet.empty_state(&all), Some(Empty::NoRecent));
    let _again = sheet.retry(&codex()).unwrap();
    let (mut failed, [first, second]) = opened(None);
    let _codex = failed.answer(first.refused(Problem::Timeout));
    let _claude = failed.answer(second.refused(Problem::Timeout));
    assert_eq!(failed.empty_state(&all), None, "every provider failed");
}

#[test]
fn show_all_reads_the_whole_host_again() {
    let (mut sheet, [first, second]) = opened(Some("/work/project"));
    let _first = sheet.answer(reply(&first, Vec::new(), None));
    let _second = sheet.answer(reply(&second, Vec::new(), None));
    let asks = sheet.show_all().unwrap();
    assert_eq!(asks.len(), 2);
    assert!(asks.iter().all(|ask| ask.scope().cwd.is_none()));
    assert!(!sheet.scoped());
    assert!(sheet.show_all().unwrap().is_empty());
}

#[test]
fn the_import_keeps_the_native_identity_and_directory() {
    let (mut sheet, [first, _second]) = opened(None);
    let _answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let Some(Ask::Import(batch)) = sheet
        .import(&codex(), &HarnessSessionId::new("codex-a"))
        .unwrap()
    else {
        panic!("an import");
    };
    let entry = batch.entries.first().unwrap();
    let spec = entry.spec(&batch.scope);
    assert_eq!(spec.cwd.as_ref(), Some(&entry.cwd));
    assert_eq!(spec.harness, codex());
    assert_eq!(spec.profile, None);
    assert_eq!(spec.title, entry.title);
    assert!(
        matches!(spec.mode, SessionMode::Structured { resume: Some(id), .. } if id == entry.id)
    );
}

#[test]
fn cursor_bound_is_checked_at_the_boundary() {
    assert!(Cursor::parse("x".repeat(Cursor::MAX_BYTES)).is_ok());
    assert_eq!(
        Cursor::parse("x".repeat(Cursor::MAX_BYTES.saturating_add(1))),
        Err(Problem::InvalidReply)
    );
}

#[test]
fn an_import_receipt_must_match_its_scope_and_name_the_session() {
    let (mut sheet, [first, _second]) = opened(None);
    let _answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let Some(Ask::Import(batch)) = sheet
        .import(&codex(), &HarnessSessionId::new("codex-a"))
        .unwrap()
    else {
        panic!("an import");
    };
    let mut wrong = batch.scope.clone();
    wrong.machine = MachineKey::from_book("other");
    let refused = sheet.answer(Reply::Imported {
        scope: wrong,
        ticket: batch.ticket,
        outcomes: Vec::new(),
    });
    assert!(!refused.applied);
    assert!(sheet.importing().is_some());
    let empty = sheet.answer(Reply::Imported {
        scope: batch.scope,
        ticket: batch.ticket,
        outcomes: Vec::new(),
    });
    assert!(empty.applied);
    assert!(sheet.importing().is_none());
    assert_eq!(sheet.import_failed(), Some(Problem::InvalidReply));
    assert!(sheet.take_imported().is_none());
}

#[test]
fn a_new_node_cannot_append_to_the_previous_nodes_pages() {
    let (mut sheet, [first, _second]) = opened(None);
    let answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], Some("page-2")));
    let follow = only(answer.follow);
    let mut changed = reply(&follow, vec![entry("wrong-node", 2)], None);
    if let Reply::Listed {
        result: Ok(page), ..
    } = &mut changed
    {
        page.node = NodeId::random();
    }
    assert!(sheet.answer(changed).applied);
    assert_eq!(sheet.failures(), [(&codex(), Problem::IdentityChanged)]);
    let ids: Vec<&str> = sheet
        .rows(&Query::new(""))
        .iter()
        .map(|row| row.entry.id.as_str())
        .collect();
    assert_eq!(ids, ["codex-a"]);
}

#[test]
fn an_oversized_next_page_keeps_the_accepted_rows() {
    const OVER_BUDGET: usize = 17 * 1024 * 1024;
    let (mut sheet, [first, _second]) = opened(None);
    let answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], Some("page-2")));
    let follow = only(answer.follow);
    let mut oversized = entry("oversized", 2);
    oversized.id = HarnessSessionId::new("x".repeat(OVER_BUDGET));
    assert!(sheet.answer(reply(&follow, vec![oversized], None)).applied);
    assert_eq!(sheet.failures(), [(&codex(), Problem::Capacity)]);
    assert_eq!(sheet.rows(&Query::new("")).len(), 1);
    assert!(sheet.has_more());
}

#[test]
fn a_host_reads_two_providers_at_once_and_the_rest_as_answers_come() {
    let gemini: HarnessRef = HarnessKind::Gemini.into();
    let mut sheet = Sheet::default();
    let asks = sheet
        .open(opening(None), vec![codex(), claude(), gemini.clone()])
        .unwrap();
    let Ok([first, _second]) = <[Ask; 2]>::try_from(asks) else {
        panic!("two reads at once");
    };
    assert!(sheet.loading());
    assert!(sheet.failures().is_empty(), "waiting is not failing");
    let answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let third = only(answer.follow);
    assert_eq!(harness_of(&third), &gemini);
    // A refresh takes the two slots again; the one left waits again.
    // A refresh now waits: the two reads out still hold both slots.
    assert!(sheet.refresh().unwrap().is_empty());
    assert!(sheet.loading());
}

#[test]
fn a_superseded_read_holds_its_slot_until_its_reply_comes() {
    let gemini: HarnessRef = HarnessKind::Gemini.into();
    let mut sheet = Sheet::default();
    let asks = sheet
        .open(
            opening(Some("/work/project")),
            vec![codex(), claude(), gemini.clone()],
        )
        .unwrap();
    let Ok([first, second]) = <[Ask; 2]>::try_from(asks) else {
        panic!("two reads at once");
    };
    // Show all while both run: nothing more goes yet.
    assert!(sheet.show_all().unwrap().is_empty());
    // The superseded reads still answer; each frees its slot for a new read.
    let answer = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    assert!(!answer.applied, "a superseded reply changes nothing");
    let codex_again = only(answer.follow);
    assert_eq!(harness_of(&codex_again), &codex());
    assert_eq!(codex_again.scope().cwd, None, "the whole host now");
    let answer = sheet.answer(second.refused(Problem::Timeout));
    assert!(!answer.applied);
    assert_eq!(harness_of(&only(answer.follow)), &claude());
    assert!(sheet.failures().is_empty(), "no provider failed");
    let answer = sheet.answer(reply(&codex_again, Vec::new(), None));
    assert_eq!(harness_of(&only(answer.follow)), &gemini);
}

#[test]
fn opening_again_waits_for_the_previous_openings_reads() {
    let (mut sheet, [first, second]) = opened(None);
    sheet.close();
    let asks = sheet.open(opening(None), vec![codex(), claude()]).unwrap();
    assert!(asks.is_empty(), "both slots are still taken");
    let answer = sheet.answer(reply(&first, vec![entry("old", 1)], None));
    assert!(!answer.applied);
    assert_eq!(harness_of(&only(answer.follow)), &codex());
    let answer = sheet.answer(reply(&second, Vec::new(), None));
    assert_eq!(harness_of(&only(answer.follow)), &claude());
    assert!(
        sheet.rows(&Query::new("")).is_empty(),
        "the old page is not shown"
    );
}

#[test]
fn reads_asked_during_an_import_go_with_its_receipt() {
    let (mut sheet, [first, second]) = opened(None);
    let _codex = sheet.answer(reply(&first, vec![entry("codex-a", 1)], None));
    let _claude = sheet.answer(reply(&second, Vec::new(), None));
    let Some(Ask::Import(batch)) = sheet
        .import(&codex(), &HarnessSessionId::new("codex-a"))
        .unwrap()
    else {
        panic!("an import");
    };
    // Opened again (here, on the same host) while the import runs.
    sheet.close();
    let asks = sheet.open(opening(None), vec![codex(), claude()]).unwrap();
    assert!(asks.is_empty(), "no read while an import runs");
    assert!(sheet.loading(), "they wait");
    let answer = sheet.answer(Ask::Import(batch).refused(Problem::Timeout));
    assert!(answer.applied);
    assert_eq!(answer.follow.len(), 2, "both go with the receipt");
}
