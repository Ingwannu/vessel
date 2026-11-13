//! The page's `data/rows.test.ts` and `data/tool-runs.test.ts`.

use std::collections::HashSet;

use vessel_app_view::timeline::{FindCache, Folds, Rows, RunSummary, find_all, outline};
use vessel_proto::ToolKind;

use crate::support::EN;
use crate::support::make::{conversation, message, tool};

#[test]
fn paseos_sentence() {
    use ToolKind::{Edit, Execute, Fetch, Read, Search};
    let words = EN;
    assert_eq!(
        words.run_summary(&RunSummary::of(&[Edit, Edit, Execute, Execute, Execute], 0)),
        "Edited 2 files and ran 3 commands"
    );
    assert_eq!(
        words.run_summary(&RunSummary::of(&[Read, Search, Fetch], 1)),
        "Read 1 file, searched 1 time and used 1 other tool"
    );
    assert_eq!(words.run_summary(&RunSummary::of(&[], 2)), "Thought");
}

#[test]
fn every_run_with_a_tool_call_is_folded_a_message_ends_a_run() {
    let entries = [
        tool(1, ToolKind::Read),
        tool(2, ToolKind::Edit),
        message(3, "done"),
        tool(4, ToolKind::Execute),
    ];
    let folds = Folds::of(&entries);
    let first = folds.run_of(0).unwrap();
    assert_eq!((first.lead, first.end), (0, 2));
    assert_eq!(folds.run_of(1).unwrap().lead, 0);
    assert_eq!(
        EN.run_summary(&folds.run_of(3).unwrap().summary),
        "Ran 1 command"
    );
    assert!(folds.run_of(2).is_none());
}

#[test]
fn rows_are_the_entries_themselves_when_nothing_is_folded() {
    let entries = conversation(30);
    let rows = Rows::of(entries.len(), None, &HashSet::new());
    assert_eq!(rows, Rows::All(31));
    assert_eq!(rows.row_index(10, None), Some(10));
    assert_eq!(rows.row_index(99, None), None);
}

#[test]
fn rows_leave_out_a_closed_runs_members_but_its_lead_and_keep_an_open_runs() {
    let entries = conversation(9);
    let folds = Folds::of(&entries);
    // u0, m1, [h2 t3], m4, [h5 t6], m7, [h8 t9]
    let closed = Rows::of(entries.len(), Some(&folds), &HashSet::new());
    assert_eq!(closed, Rows::Some(vec![0, 1, 2, 4, 5, 7, 8]));
    let opened = Rows::of(entries.len(), Some(&folds), &HashSet::from([5]));
    assert_eq!(opened, Rows::Some(vec![0, 1, 2, 4, 5, 6, 7, 8]));
    // A folded member is shown by its run's lead.
    assert_eq!(closed.row_index(6, Some(&folds)), Some(4));
    assert_eq!(opened.row_index(6, Some(&folds)), Some(5));
    assert_eq!(closed.row_index(99, Some(&folds)), None);
}

#[test]
fn rows_hold_every_entry_find_and_the_outline_hand_over_folded_or_not() {
    // Find looks in prompts and messages, the outline lists prompts: none
    // is ever inside a run, so each is a row of its own either way.
    let entries = conversation(20_000);
    let folds = Folds::of(&entries);
    let mut cache = FindCache::default();
    let wanted: Vec<usize> = find_all(&mut cache, &entries, "names module")
        .iter()
        .map(|found| found.entry)
        .chain(outline(&entries).iter().map(|item| item.entry))
        .collect();
    assert_eq!(wanted.len(), 6_668);
    for rows in [
        Rows::of(entries.len(), None, &HashSet::new()),
        Rows::of(entries.len(), Some(&folds), &HashSet::new()),
    ] {
        for entry in &wanted {
            let row = rows.row_index(*entry, Some(&folds)).unwrap();
            assert_eq!(rows.entry(row), Some(*entry));
        }
    }
    // Folded, a third of the conversation's 20,001 entries are rows.
    assert_eq!(
        Rows::of(entries.len(), Some(&folds), &HashSet::new()).len(),
        13_335
    );
}
