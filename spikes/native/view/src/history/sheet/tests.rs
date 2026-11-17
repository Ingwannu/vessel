//! Paseo's import sheet on inert in-memory replies: providers in parallel and
//! merged, limits read through cursor pages, per-provider failures, one import.
use super::*;
use crate::devices::MachineKey;
use vessel_proto::{HarnessKind, SessionId};

const NODE: &str = "00000000-0000-0000-0000-000000000001";

fn opening(cwd: Option<&str>) -> Opening {
    Opening {
        context: crate::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("laptop"), NODE.parse().unwrap())
            .unwrap(),
        machine: MachineKey::from_book("laptop"),
        cwd: cwd.map(PathBuf::from),
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
    let (mut sheet, [first, second]) = opened(None);
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
    let (mut sheet, [first, second]) = opened(None);
    let stale = reply(&first, vec![entry("codex-a", 1)], None);
    let _fresh = sheet.refresh().unwrap();
    assert!(!sheet.answer(stale).applied);
    assert!(sheet.rows(&Query::new("")).is_empty());
}

#[test]
fn a_row_imports_one_session_and_hands_the_new_session_over() {
    let (mut sheet, [first, second]) = opened(None);
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
    assert_eq!(sheet.importing().map(HarnessSessionId::as_str), Some("codex-a"));
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
                scope: batch.scope,
                ticket: batch.ticket,
                outcomes: vec![outcome],
            })
            .applied
    );
    assert_eq!(sheet.take_imported().map(|info| info.id), Some(created));
    assert!(sheet.importing().is_none());
    // The imported session leaves the list; nothing left says so.
    assert!(sheet.rows(&Query::new("")).is_empty());
    assert_eq!(sheet.empty(&Query::new("")), Empty::AlreadyImported);
}

#[test]
fn a_failed_import_keeps_the_sheet_and_says_so() {
    let (mut sheet, [first, second]) = opened(None);
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
    assert_eq!(sheet.empty(&Query::new("")), Empty::NoProviderSessions(claude()));
    sheet.set_filter(Filter::All);
    let (empty, _asks) = opened(None);
    assert_eq!(empty.empty(&Query::new("")), Empty::NoRecent);
}

#[test]
fn show_all_reads_the_whole_host_again() {
    let (mut sheet, _asks) = opened(Some("/work/project"));
    let asks = sheet.show_all().unwrap();
    assert_eq!(asks.len(), 2);
    assert!(asks.iter().all(|ask| ask.scope().cwd.is_none()));
    assert!(!sheet.scoped());
    assert!(sheet.show_all().unwrap().is_empty());
}
