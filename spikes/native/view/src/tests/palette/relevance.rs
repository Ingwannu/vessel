//! Cases from Paseo's `command-center/results.test.ts`. The native
//! targets differ; query text, field roles and ordering do not.

use super::{Entry, Theme};
use crate::palette::{PaletteEntry, PaletteGroup, PaletteTarget, Subtitle, palette_groups};

fn entry(id: &str, title: &str, group: PaletteGroup, hidden: Option<&str>) -> Entry {
    PaletteEntry {
        id: id.to_owned(),
        label: title.to_owned(),
        detail: None,
        keywords: hidden.map(str::to_owned),
        group,
        target: PaletteTarget::Theme(Theme::System),
        activity_at: None,
    }
}

fn ids(entries: &[Entry], query: &str) -> Vec<String> {
    palette_groups(entries, query)
        .into_iter()
        .flat_map(|(_, found)| found.into_iter().map(|entry| entry.id.clone()))
        .collect()
}

#[test]
fn visible_matches_outrank_hidden_keywords_at_every_prefix_and_across_groups() {
    let rows = [
        entry(
            "schedules",
            "Schedules",
            PaletteGroup::Actions,
            Some("automation"),
        ),
        entry("mode:auto", "Auto mode", PaletteGroup::Theme, None),
    ];
    for query in ["a", "au", "aut", "auto"] {
        assert_eq!(ids(&rows, query), ["mode:auto", "schedules"]);
    }
}

#[test]
fn matches_rank_within_sections_and_equal_scores_keep_registry_order() {
    let rows = [
        entry("reset", "Reset device", PaletteGroup::Actions, None),
        entry("settings", "Settings", PaletteGroup::Actions, None),
    ];
    assert_eq!(ids(&rows, "set"), ["settings", "reset"]);
    let ties = [
        entry("z-first", "Auto one", PaletteGroup::Actions, None),
        entry("a-second", "Auto two", PaletteGroup::Actions, None),
    ];
    assert_eq!(ids(&ties, "auto"), ["z-first", "a-second"]);
    assert_eq!(ids(&rows, ""), ["reset", "settings"]);
}

#[test]
fn sessions_remain_below_contributions_even_on_a_perfect_match() {
    let rows = [
        entry(
            "schedules",
            "Schedules",
            PaletteGroup::Actions,
            Some("automation"),
        ),
        entry("session:auto", "auto", PaletteGroup::Sessions, None),
    ];
    assert_eq!(ids(&rows, "auto"), ["schedules", "session:auto"]);
}

#[test]
fn only_five_recent_sessions_show_until_a_query_narrows_them() {
    let rows: Vec<_> = (1..=8)
        .map(|hour| {
            let mut row = entry(
                &format!("w{hour}"),
                &format!("w{hour}"),
                PaletteGroup::Sessions,
                None,
            );
            row.activity_at = Some(format!("2026-10-18T{hour:02}:00:00Z").parse().unwrap());
            row
        })
        .collect();
    assert_eq!(ids(&rows, ""), ["w8", "w7", "w6", "w5", "w4"]);
    assert_eq!(
        ids(&rows, "w"),
        ["w8", "w7", "w6", "w5", "w4", "w3", "w2", "w1"]
    );
}

#[test]
fn session_ties_use_activity_instead_of_sentence_position() {
    let mut older = entry(
        "older",
        "Deliver sidebar usage",
        PaletteGroup::Sessions,
        None,
    );
    older.activity_at = Some("2026-10-18T11:00:00Z".parse().unwrap());
    let mut newer = entry(
        "newer",
        "design the cli interface to expose usage data",
        PaletteGroup::Sessions,
        None,
    );
    newer.activity_at = Some("2026-10-18T11:55:00Z".parse().unwrap());
    assert_eq!(ids(&[older, newer], "usage"), ["newer", "older"]);
}

#[test]
fn a_better_tier_outranks_a_more_recent_subsequence() {
    let mut weaker = entry(
        "subsequence",
        "Fix stuck toast",
        PaletteGroup::Sessions,
        None,
    );
    weaker.detail = Some(Subtitle::Plain("fix/updating-messages".to_owned()));
    weaker.activity_at = Some("2026-10-18T11:59:00Z".parse().unwrap());
    let mut whole = entry(
        "whole-word",
        "Usage hover card",
        PaletteGroup::Sessions,
        None,
    );
    whole.activity_at = Some("2026-10-18T02:00:00Z".parse().unwrap());
    assert_eq!(
        ids(&[weaker, whole], "usage"),
        ["whole-word", "subsequence"]
    );
}

#[test]
fn archived_sessions_are_not_navigation_results() {
    use super::super::fixture::{at, fleet, in_folder, machine, session};
    use crate::Lang;
    use crate::palette::palette_entries;
    use vessel_proto::SessionStatus;

    let mut archived = in_folder(
        session("aaaa1111", "2026-10-18T11:59:00Z", SessionStatus::Running),
        "/w/archived",
    );
    archived.marks.archived_at = Some(at("2026-10-18T12:00:00Z"));
    let visible = in_folder(
        session("bbbb2222", "2026-10-18T11:00:00Z", SessionStatus::Running),
        "/w/visible",
    );
    let book = fleet(vec![machine("laptop", true, &[archived, visible])]);
    let all: Vec<Entry> = palette_entries(
        &book,
        None,
        &[],
        [Theme::System, Theme::Light, Theme::Dark],
        Lang::En,
    );
    let sessions: Vec<_> = all
        .iter()
        .filter(|entry| entry.group == PaletteGroup::Sessions)
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(sessions, ["visible"]);
}
