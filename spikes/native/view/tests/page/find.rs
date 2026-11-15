//! The page's `data/find.test.ts` and `data/find.speed.test.ts`.

use std::io::Write as _;
use std::time::{Duration, Instant};

use vessel_app_view::timeline::{
    Entry, FindCache, Finder, Match, find_all, fold, occurrences, query_of, readable_text,
    spans_of, start_at, step,
};
use vessel_proto::EventSeq;

use crate::support::EN;
use crate::support::make::{log, message, prompt, thought};

#[test]
fn case_accents_and_white_space_fold_as_agent_orchestrators_do() {
    assert_eq!(fold("Café\n  CRÈME"), "cafe creme");
    assert_eq!(query_of("  Hello   World "), "hello world");
}

#[test]
fn markdowns_marks_are_not_text() {
    assert_eq!(
        readable_text("**done** and `npm test`"),
        "done and npm test"
    );
    assert_eq!(
        readable_text("see [the docs](https://x.test/a)"),
        "see the docs"
    );
    assert_eq!(
        readable_text("# Title\n- one\n> quoted"),
        "Title\none\nquoted"
    );
    // An underscore inside a word is the word's; at its edges, emphasis.
    assert_eq!(
        readable_text("names `src/module_300.rs` and _this_"),
        "names src/module_300.rs and this"
    );
    assert_eq!(
        readable_text("__init__ and snake_case_name"),
        "init and snake_case_name"
    );
}

#[test]
fn occurrences_do_not_overlap() {
    assert_eq!(occurrences("aaaa", "aa"), 2);
    assert_eq!(occurrences("abc", ""), 0);
}

fn entries() -> Vec<Entry> {
    vec![
        prompt(1, "Fix the build"),
        message(2, "The **build** failed; I fixed the build script."),
        thought(3),
        log(4, "build"),
    ]
}

#[test]
fn prompts_and_messages_every_occurrence_in_order() {
    let found = find_all(&mut FindCache::default(), &entries(), "BUILD");
    assert_eq!(
        found,
        [
            Match { entry: 0, nth: 0 },
            Match { entry: 1, nth: 0 },
            Match { entry: 1, nth: 1 },
        ]
    );
}

#[test]
fn an_empty_query_finds_nothing() {
    assert!(find_all(&mut FindCache::default(), &entries(), "   ").is_empty());
}

#[test]
fn in_slices_the_same_answer_and_none_once_dropped() {
    let many: Vec<Entry> = (0..1000)
        .map(|index| {
            if index % 2 == 0 {
                prompt(index, "needle")
            } else {
                message(index, "hay")
            }
        })
        .collect();
    let mut cache = FindCache::default();
    let mut finder = Finder::new("needle");
    let mut slices = 0;
    let found = loop {
        slices += 1;
        if let Some(found) = finder.step(&mut cache, &many) {
            break found;
        }
    };
    assert_eq!(found.len(), 500);
    assert_eq!(slices, 5, "200 entries a slice");
    // A newer query replaces the finder: the old one is dropped after
    // its first slice, and answers nothing.
    let mut old = Finder::new("needle");
    assert!(old.step(&mut cache, &many).is_none());
}

#[test]
fn a_changed_entry_is_folded_again() {
    let mut cache = FindCache::default();
    let mut entries = vec![message(1, "old words")];
    assert!(find_all(&mut cache, &entries, "new").is_empty());
    if let vessel_app_view::timeline::EntryKind::Message { text } = &mut entries[0].kind {
        text.push_str(" and new ones");
    }
    entries[0].rev += 1;
    assert_eq!(find_all(&mut cache, &entries, "new").len(), 1);
}

#[test]
fn next_and_previous_wrap_and_say_so() {
    let at = |stepped: vessel_app_view::timeline::Stepped| (stepped.index, stepped.wrapped);
    assert_eq!(at(step(3, 1, true)), (2, false));
    assert_eq!(at(step(3, 2, true)), (0, true));
    assert_eq!(at(step(3, 0, false)), (2, true));
    assert_eq!(at(step(0, 0, true)), (0, false));
}

#[test]
fn the_count_as_written_and_as_spoken() {
    assert_eq!(EN.count_text(12, 2), "3/12");
    assert_eq!(EN.count_spoken(12, 2), "3 of 12");
    assert_eq!(EN.count_text(0, 0), "No results");
}

fn marked<'a>(text: &'a str, query: &str) -> Vec<&'a str> {
    spans_of(text, query)
        .into_iter()
        .map(|span| text.get(span).unwrap())
        .collect()
}

#[test]
fn folded_as_the_count_folds_mapped_back_to_the_text() {
    let text = "A Café\n\n  and  CAFE au lait";
    assert_eq!(marked(text, "cafe"), ["Café", "CAFE"]);
    // Runs of white space are one space, as in the query.
    assert_eq!(marked(text, "café and"), ["Café\n\n  and"]);
    // Characters outside the basic plane are whole characters.
    let wide = "go 🚀 now, 🚀 again";
    assert_eq!(marked(wide, "🚀 again"), ["🚀 again"]);
    assert!(spans_of(text, "  ").is_empty());
}

#[test]
fn an_accent_written_apart_is_the_letters() {
    let text = "cafe\u{301} noir";
    assert_eq!(marked(text, "café"), ["cafe\u{301}"]);
}

#[test]
fn hangul_is_found_whole_and_marked_whole() {
    let text = "빌드가 실패했습니다. 다시 빌드합니다";
    assert_eq!(marked(text, "빌드"), ["빌드", "빌드"]);
    assert_eq!(
        find_all(&mut FindCache::default(), &[message(1, text)], "빌드").len(),
        2
    );
}

const PARAGRAPH: &str = "The **build** failed in `crates/vessel-web` because the test named \
    `a_qr_code_is_an_svg` expected a header. I changed [the check](https://x.test) \
    and ran the suite again; everything passes now. ";

fn long_conversation(size: u64) -> Vec<Entry> {
    (0..size)
        .map(|index| {
            if index % 2 == 0 {
                prompt(
                    index,
                    &format!("Please look at item {index} and fix what is broken."),
                )
            } else {
                message(index, &PARAGRAPH.repeat(8))
            }
        })
        .collect()
}

/// Keep real timing diagnostics on the page's 5,000-entry workload. Scheduling
/// on a shared VM cannot prove latency: cache unit tests instead bound folds,
/// visits and entries per frame deterministically. `VESSEL_FIND_TIMINGS=1`
/// prints the measurements without turning host contention into a failure.
#[test]
fn a_find_over_5000_entries_preserves_results_cold_warm_and_sliced() {
    let entries = long_conversation(5000);
    let mut cache = FindCache::default();
    let started = Instant::now();
    let found = find_all(&mut cache, &entries, "suite again");
    let cold = started.elapsed();
    let mut warm: Vec<Duration> = (0..20)
        .map(|_| {
            let started = Instant::now();
            assert_eq!(find_all(&mut cache, &entries, "suite again"), found);
            started.elapsed()
        })
        .collect();
    warm.sort();
    let median = warm[warm.len().checked_div(2).unwrap()];
    // The slices the screen runs: the longest one, cold.
    let mut fresh = FindCache::default();
    let mut finder = Finder::new("suite again");
    let mut longest = Duration::ZERO;
    loop {
        let started = Instant::now();
        let result = finder.step(&mut fresh, &entries);
        longest = longest.max(started.elapsed());
        if let Some(sliced) = result {
            assert_eq!(sliced, found);
            break;
        }
    }
    if std::env::var("VESSEL_FIND_TIMINGS").is_ok_and(|value| value == "1") {
        let _shown = writeln!(
            std::io::stderr(),
            "find, 5000 entries: first {cold:?}, later median {median:?}, longest slice (cold) {longest:?}, {} matches",
            found.len()
        );
    }
    assert_eq!(found.len(), 2500 * 8);
}

// A search hit opened from the palette: find starts at its message's
// entry, or the next match after it; it waits while that record is not
// read yet.
#[test]
fn find_starts_at_the_hits_message_or_the_next_match() {
    let entries = [
        prompt(1, "fold"),
        message(4, "no"),
        message(7, "fold"),
        message(9, "fold"),
    ];
    let matches = [
        Match { entry: 0, nth: 0 },
        Match { entry: 2, nth: 0 },
        Match { entry: 3, nth: 0 },
    ];
    let next = EventSeq::new(10);
    assert_eq!(
        start_at(&entries, next, &matches, EventSeq::new(7)),
        Some(1)
    );
    assert_eq!(
        start_at(&entries, next, &matches, EventSeq::new(4)),
        Some(1),
        "the next match after it"
    );
    assert_eq!(
        start_at(&entries, next, &matches, EventSeq::new(9)),
        Some(2)
    );
    assert_eq!(
        start_at(&entries, next, &matches, EventSeq::new(12)),
        None,
        "not read yet"
    );
    assert_eq!(start_at(&entries, next, &[], EventSeq::new(4)), Some(0));
}

#[test]
fn a_warm_find_after_rewind_excludes_hidden_text_without_reindexing_live_matches() {
    use crate::support::Log;
    use serde_json::json;
    use vessel_app_view::timeline::Timeline;
    let mut log = Log::default();
    let records = [
        log.agent(&json!({"event":"turn_started", "content":[{"type":"text", "text":"kept needle"}]})),
        log.agent(&json!({"event":"message_chunk", "content":{"type":"text", "text":"kept reply needle"}})),
        log.agent(&json!({"event":"turn_ended", "reason":"end_turn"})),
        log.agent(&json!({"event":"turn_started", "content":[{"type":"text", "text":"discarded needle"}]})),
        log.agent(&json!({"event":"message_chunk", "content":{"type":"text", "text":"discarded reply needle"}})),
        log.agent(&json!({"event":"turn_ended", "reason":"end_turn"})),
    ];
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    let mut cache = FindCache::default();
    assert_eq!(
        Finder::new("needle")
            .step_timeline(&mut cache, &timeline)
            .unwrap()
            .len(),
        4
    );
    let records = [
        log.record(&json!({"type":"rewound", "after":2, "harness_session":"test-branch", "files_changed":0})),
        log.agent(&json!({"event":"turn_started", "content":[{"type":"text", "text":"new branch"}]})),
        log.agent(&json!({"event":"message_chunk", "content":{"type":"text", "text":"live reply needle"}})),
    ];
    let _change = timeline.apply(&records);
    assert!(timeline.rewound(3));
    assert!(timeline.rewound(4));
    let found = Finder::new("needle")
        .step_timeline(&mut cache, &timeline)
        .unwrap();
    assert_eq!(
        found,
        [
            Match { entry: 0, nth: 0 },
            Match { entry: 1, nth: 0 },
            Match { entry: 7, nth: 0 }
        ]
    );
    assert_eq!(
        timeline.entries()[7].seq,
        EventSeq::new(8),
        "live source addresses stay stable"
    );
    assert_eq!(
        find_all(&mut cache, timeline.entries(), "needle").len(),
        5,
        "raw API keeps its original slice semantics"
    );
}
