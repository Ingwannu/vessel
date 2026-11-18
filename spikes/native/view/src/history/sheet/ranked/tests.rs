//! The sheet on a ranked host (ADR 0109): Paseo's limit and search go to the
//! node, every provider is read at once, "Load more" asks again with the
//! next limit, and the imported count says why the list is empty.

use super::super::{Answer, Empty, Opening, PAGE_LIMITS, Sheet};
use crate::devices::MachineKey;
use crate::history::{Ask, Entry, Listed, Ranked, Reply};
use crate::matcher::Query;
use std::path::PathBuf;
use vessel_proto::{HarnessKind, HarnessRef, HarnessSessionId};

const NODE: &str = "00000000-0000-0000-0000-000000000001";

fn opening(ranked: bool) -> Opening {
    Opening {
        context: crate::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("laptop"), NODE.parse().unwrap())
            .unwrap(),
        machine: MachineKey::from_book("laptop"),
        cwd: None,
        ranked,
    }
}

fn providers() -> Vec<HarnessRef> {
    vec![
        HarnessKind::Codex.into(),
        HarnessKind::ClaudeCode.into(),
        HarnessKind::OpenCode.into(),
    ]
}

fn entry(id: &str, minute: i64) -> Entry {
    Entry {
        id: HarnessSessionId::new(id),
        cwd: PathBuf::from("/work/project"),
        title: None,
        updated_at: chrono::DateTime::from_timestamp(minute.saturating_mul(60), 0),
        in_vessel: None,
        first_prompt: Some(format!("first {id}")),
        last_prompt: None,
    }
}

fn entries(prefix: &str, count: usize) -> Vec<Entry> {
    (0..count)
        .map(|index| entry(&format!("{prefix}-{index}"), i64::try_from(index).unwrap()))
        .collect()
}

fn ranked(ask: &Ask) -> &Ranked {
    let Ask::List {
        ranked: Some(ranked),
        cursor: None,
        ..
    } = ask
    else {
        panic!("a ranked listing without a cursor: {ask:?}");
    };
    ranked
}

fn reply(ask: &Ask, entries: Vec<Entry>, already_imported: u32) -> Reply {
    let Ask::List { scope, ticket, .. } = ask else {
        panic!("a listing");
    };
    Reply::Listed {
        scope: scope.clone(),
        ticket: *ticket,
        result: Ok(Listed {
            entries,
            node: NODE.parse().unwrap(),
            next: None,
            already_imported,
        }),
    }
}

fn applied(sheet: &mut Sheet, reply: Reply) -> Answer {
    let answer = sheet.answer(reply);
    assert!(answer.applied);
    answer
}

#[test]
fn a_ranked_host_is_read_all_at_once_with_the_first_limit() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    assert!(sheet.ranked());
    assert_eq!(asks.len(), 3, "no reads wait in the sheet");
    for ask in &asks {
        assert_eq!(
            ranked(ask),
            &Ranked {
                limit: 15,
                query: None
            }
        );
    }
    let mut legacy = Sheet::default();
    assert_eq!(
        legacy.open(opening(false), providers()).unwrap().len(),
        2,
        "an older host still gets two at once"
    );
}

#[test]
fn answers_replace_rows_and_the_node_did_the_searching() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    let answer = applied(&mut sheet, reply(&asks[0], entries("codex", 3), 2));
    assert!(answer.follow.is_empty(), "no cursor pages to read on");
    let _claude = applied(&mut sheet, reply(&asks[1], Vec::new(), 0));
    let _opencode = applied(&mut sheet, reply(&asks[2], Vec::new(), 0));
    // The typed query does not filter here: the node already searched.
    assert_eq!(sheet.rows(&Query::new("nothing like it")).len(), 3);
    assert_eq!(sheet.already_imported(), 2);
    let row = sheet.rows(&Query::new("")).first().copied().unwrap();
    assert_eq!(row.entry.preview(), Some("first codex-2"));
    assert_eq!(row.entry.shown_title(), Some("first codex-2"));
}

#[test]
fn an_empty_ranked_list_says_already_imported_from_the_nodes_count() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    for (ask, count) in asks.iter().zip([1, 0, 0]) {
        let _answer = applied(&mut sheet, reply(ask, Vec::new(), count));
    }
    assert_eq!(
        sheet.empty_state(&Query::new("")),
        Some(Empty::AlreadyImported)
    );
    assert_eq!(sheet.empty_state(&Query::new("x")), Some(Empty::NoMatches));
}

#[test]
fn load_more_asks_again_with_the_next_limit_and_keeps_the_rows_meanwhile() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    let _full = applied(&mut sheet, reply(&asks[0], entries("codex", 15), 0));
    let _short = applied(&mut sheet, reply(&asks[1], entries("claude", 2), 0));
    let _none = applied(&mut sheet, reply(&asks[2], Vec::new(), 0));
    assert!(sheet.has_more(), "a provider answered a full page");
    let more = sheet.more().unwrap();
    assert_eq!(more.len(), 3);
    assert!(more.iter().all(|ask| ranked(ask).limit == 45));
    assert_eq!(
        sheet.rows(&Query::new("")).len(),
        17,
        "rows stay until answers"
    );
    assert!(
        sheet.has_more(),
        "the button holds its place while it reads"
    );
    let _bigger = applied(&mut sheet, reply(&more[0], entries("codex", 20), 0));
    let _same = applied(&mut sheet, reply(&more[1], entries("claude", 2), 0));
    let _still = applied(&mut sheet, reply(&more[2], Vec::new(), 0));
    assert!(!sheet.has_more(), "nobody filled 45");
    assert_eq!(sheet.limit(), PAGE_LIMITS[1]);
}

#[test]
fn a_search_reads_again_from_the_first_limit_once_it_changes() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    for ask in &asks {
        let _answer = applied(&mut sheet, reply(ask, entries("codex", 15), 0));
    }
    let _more = sheet.more().unwrap();
    let searched = sheet.search("  build ").unwrap();
    assert_eq!(searched.len(), 3);
    assert!(searched.iter().all(|ask| ranked(ask)
        == &Ranked {
            limit: 15,
            query: Some("build".into())
        }));
    assert!(sheet.search("build").unwrap().is_empty(), "the same search");
    let cleared = sheet.search("").unwrap();
    assert!(cleared.iter().all(|ask| ranked(ask).query.is_none()));
    let mut legacy = Sheet::default();
    let _asks = legacy.open(opening(false), providers()).unwrap();
    assert!(
        legacy.search("build").unwrap().is_empty(),
        "an older host is filtered here"
    );
}

#[test]
fn a_ranked_read_still_waits_while_an_import_runs() {
    let mut sheet = Sheet::default();
    let asks = sheet.open(opening(true), providers()).unwrap();
    let _rows = applied(&mut sheet, reply(&asks[0], entries("codex", 1), 0));
    let _claude = applied(&mut sheet, reply(&asks[1], Vec::new(), 0));
    let _opencode = applied(&mut sheet, reply(&asks[2], Vec::new(), 0));
    let codex: HarnessRef = HarnessKind::Codex.into();
    let import = sheet
        .import(&codex, &HarnessSessionId::new("codex-0"))
        .unwrap();
    assert!(import.is_some());
    assert!(
        sheet.refresh().unwrap().is_empty(),
        "the receipt changes the list"
    );
    assert!(sheet.failures().is_empty());
    assert!(sheet.loading(), "reads wait for the receipt");
}
