//! The page's `data/rows.test.ts` and `data/tool-runs.test.ts`.

use std::collections::HashSet;

use vessel_app_view::timeline::{FindCache, Folds, Logs, Rows, RunSummary, find_all, outline};
use vessel_proto::ToolKind;

use crate::support::EN;
use crate::support::make::{conversation, log, message, prompt, tool};

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
    let rows = Rows::of(entries.len(), None, &Logs::default(), &HashSet::new());
    assert_eq!(rows, Rows::All(31));
    assert_eq!(rows.row_index(10, None, &Logs::default()), Some(10));
    assert_eq!(rows.row_index(99, None, &Logs::default()), None);
}

#[test]
fn rows_leave_out_a_closed_runs_members_but_its_lead_and_keep_an_open_runs() {
    let entries = conversation(9);
    let folds = Folds::of(&entries);
    // u0, m1, [h2 t3], m4, [h5 t6], m7, [h8 t9]
    let closed = Rows::of(
        entries.len(),
        Some(&folds),
        &Logs::default(),
        &HashSet::new(),
    );
    assert_eq!(closed, Rows::Some(vec![0, 1, 2, 4, 5, 7, 8]));
    let opened = Rows::of(
        entries.len(),
        Some(&folds),
        &Logs::default(),
        &HashSet::from([5]),
    );
    assert_eq!(opened, Rows::Some(vec![0, 1, 2, 4, 5, 6, 7, 8]));
    // A folded member is shown by its run's lead.
    assert_eq!(closed.row_index(6, Some(&folds), &Logs::default()), Some(4));
    assert_eq!(opened.row_index(6, Some(&folds), &Logs::default()), Some(5));
    assert_eq!(closed.row_index(99, Some(&folds), &Logs::default()), None);
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
        Rows::of(entries.len(), None, &Logs::default(), &HashSet::new()),
        Rows::of(
            entries.len(),
            Some(&folds),
            &Logs::default(),
            &HashSet::new(),
        ),
    ] {
        for entry in &wanted {
            let row = rows
                .row_index(*entry, Some(&folds), &Logs::default())
                .unwrap();
            assert_eq!(rows.entry(row), Some(*entry));
        }
    }
    // Folded, a third of the conversation's 20,001 entries are rows.
    assert_eq!(
        Rows::of(
            entries.len(),
            Some(&folds),
            &Logs::default(),
            &HashSet::new()
        )
        .len(),
        13_335
    );
}

// An adapter's start-up log (a real Claude Code's `[session/create] …`
// lines): one row at its first line while tool calls are not collapsed,
// gone while they are; a log line is shown by its run's row.
#[test]
fn a_run_of_log_lines_is_one_row_or_none() {
    let entries = [
        log(
            0,
            "[session/create] sessionId=s phase=settings durationMs=40 totalMs=41",
        ),
        log(1, "[session/query] sessionId=s resume=none apiType=native"),
        log(
            2,
            "[session/create] sessionId=s phase=register durationMs=7 totalMs=1459",
        ),
        prompt(3, "go"),
        tool(4, ToolKind::Read),
        log(5, "a line in the turn"),
        message(6, "done"),
    ];
    let logs = Logs::of(&entries);
    assert_eq!(logs.runs().len(), 2);
    let first = logs.run_of(2).unwrap();
    assert_eq!((first.lead, first.end, first.lines()), (0, 3, 3));
    assert!(logs.run_of(3).is_none());
    assert_eq!(
        Logs::text(first, &entries).lines().count(),
        3,
        "the box holds every line of the run"
    );
    assert_eq!(
        Logs::all(&entries).lines().collect::<Vec<_>>().len(),
        4,
        "the Session actions' Agent log holds every run's lines"
    );
    assert!(Logs::all(&entries[3..5]).is_empty());
    let full = Rows::of(entries.len(), None, &logs, &HashSet::new());
    assert_eq!(full, Rows::Some(vec![0, 3, 4, 5, 6]));
    assert_eq!(full.row_index(1, None, &logs), Some(0));
    let folds = Folds::of(&entries);
    let collapsed = Rows::of(entries.len(), Some(&folds), &logs, &HashSet::new());
    assert_eq!(collapsed, Rows::Some(vec![3, 4, 6]));
    assert_eq!(collapsed.row_index(1, Some(&folds), &logs), None);
    assert_eq!(collapsed.row_index(5, Some(&folds), &logs), None);
}

// The agent's circle goes beside a turn's first reply only: a reply after
// another reply (a tool call between them or not) has none; the person's
// next message (a prompt or a steer) starts a new turn.
#[test]
fn only_a_turns_first_reply_carries_the_agents_circle() {
    let entries = [
        prompt(1, "go"),
        message(2, "first"),
        tool(3, ToolKind::Read),
        message(4, "second"),
        prompt(5, "again"),
        message(6, "first again"),
    ];
    let firsts: Vec<usize> = (0..entries.len())
        .filter(|&index| {
            matches!(
                entries[index].kind,
                vessel_app_view::timeline::EntryKind::Message { .. }
            ) && vessel_app_view::timeline::first_reply(&entries, index)
        })
        .collect();
    assert_eq!(firsts, [1, 5]);
    assert!(
        vessel_app_view::timeline::first_reply(&[], 0),
        "a reply with nothing before it"
    );
}
