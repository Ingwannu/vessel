//! `web/src/data/commands.test.ts`: what the command palette lists, and
//! the search its kit's palette runs.

use vessel_proto::{SessionInfo, SessionStatus};

use super::fixture::{fleet, id, in_folder, machine, session};
use crate::Lang;
use crate::palette::{
    PaletteEntry, PaletteGroup, PaletteTarget, kept, marks, moved, palette_entries, palette_groups,
};

mod relevance;
mod subtitle;

/// Settings' sections as the frame names them (the page's
/// `SETTINGS_SECTIONS`).
const SECTIONS: [(usize, &str); 11] = [
    (0, "General"),
    (1, "Appearance"),
    (2, "Shortcuts"),
    (3, "Notifications"),
    (4, "Agents"),
    (5, "Models"),
    (6, "Machines"),
    (7, "Projects"),
    (8, "Devices"),
    (9, "Sync"),
    (10, "About"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    System,
    Light,
    Dark,
}

type Entry = PaletteEntry<usize, Theme>;

// Keep this generic builder fixture's original choices and labels. The
// production preference registry is checked independently below.
fn named_themes() -> [(Theme, &'static str); 3] {
    [
        (Theme::System, "system"),
        (Theme::Light, "light"),
        (Theme::Dark, "dark"),
    ]
}

fn laptop_sessions() -> Vec<SessionInfo> {
    vec![
        in_folder(
            session(
                "aaaa1111",
                "2026-11-07T09:00:00Z",
                SessionStatus::Exited { code: Some(0) },
            ),
            "/w/old",
        ),
        in_folder(
            session("bbbb2222", "2026-11-07T10:00:00Z", SessionStatus::Running),
            "/w/api",
        ),
        in_folder(
            session("cccc3333", "2026-11-07T11:00:00Z", SessionStatus::Running),
            "/w/web",
        ),
    ]
}

fn entries(selected: Option<&str>) -> Vec<Entry> {
    let book = fleet(vec![machine("laptop", true, &laptop_sessions())]);
    palette_entries(&book, selected, &SECTIONS, named_themes(), Lang::En)
}

fn in_group(entries: &[Entry], group: PaletteGroup) -> Vec<&Entry> {
    entries
        .iter()
        .filter(|entry| entry.group == group)
        .collect()
}

#[test]
fn lists_the_actions_the_places_the_machines_the_sessions_the_theme() {
    let mut groups: Vec<PaletteGroup> = Vec::new();
    for entry in entries(Some("laptop")) {
        if !groups.contains(&entry.group) {
            groups.push(entry.group);
        }
    }
    assert_eq!(groups, PaletteGroup::ALL);
    let words = Lang::En.palette();
    let names: Vec<_> = groups.into_iter().map(|group| words.group(group)).collect();
    assert_eq!(names, ["Actions", "Go to", "Machines", "Sessions", "Theme"]);
}

#[test]
fn starts_a_new_session_on_the_selected_machine() {
    let all = entries(Some("laptop"));
    let action = all.iter().find(|entry| entry.id == "new-session").unwrap();
    assert_eq!(
        action.target,
        PaletteTarget::NewSession(Some("laptop".to_owned()))
    );
}

#[test]
fn offers_to_add_a_machine_wherever_the_palette_is_opened() {
    let all = entries(None);
    let action = all.iter().find(|entry| entry.id == "add-machine").unwrap();
    assert_eq!(action.label, "Add a machine");
    assert_eq!(action.group, PaletteGroup::Actions);
    assert_eq!(action.target, PaletteTarget::AddMachine);
}

#[test]
fn puts_live_sessions_first_newest_first_and_ended_ones_after() {
    let all = entries(None);
    let labels: Vec<_> = in_group(&all, PaletteGroup::Sessions)
        .into_iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(labels, ["web", "api", "old"]);
}

#[test]
fn opens_a_session_at_its_route_and_finds_it_by_its_full_id() {
    let all = entries(None);
    let full = id("bbbb2222").to_string();
    let entry = all
        .iter()
        .find(|candidate| candidate.keywords.as_deref() == Some(full.as_str()))
        .unwrap();
    assert_eq!(
        entry.target,
        PaletteTarget::Session {
            machine: "laptop".to_owned(),
            session: id("bbbb2222"),
            structured: true,
        }
    );
    assert_eq!(
        entry
            .detail
            .as_ref()
            .map(crate::palette::Subtitle::search_text)
            .as_deref(),
        Some("running · /w/api")
    );
    assert!(entry.found(&full));
}

#[test]
fn opens_a_machines_detail_in_settings_in_the_pages_words() {
    let all = entries(None);
    let entry = in_group(&all, PaletteGroup::Machines)[0];
    assert_eq!(entry.label, "laptop");
    assert_eq!(entry.target, PaletteTarget::Machine("laptop".to_owned()));
    let detail = entry.detail.as_ref().unwrap().search_text();
    assert!(
        !detail.contains("ws:") && !detail.contains("node"),
        "{detail}"
    );
}

#[test]
fn goes_to_the_board_and_to_each_section_of_settings() {
    let all = entries(None);
    let places: Vec<_> = in_group(&all, PaletteGroup::GoTo)
        .into_iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(
        places,
        [
            "Board",
            "Settings: General",
            "Settings: Appearance",
            "Settings: Shortcuts",
            "Settings: Notifications",
            "Settings: Agents",
            "Settings: Models",
            "Settings: Machines",
            "Settings: Projects",
            "Settings: Devices",
            "Settings: Sync",
            "Settings: About",
        ]
    );
}

#[test]
fn switches_the_theme_three_ways() {
    let all = entries(None);
    let themes: Vec<_> = in_group(&all, PaletteGroup::Theme)
        .into_iter()
        .map(|entry| (entry.label.as_str(), entry.target.clone()))
        .collect();
    assert_eq!(
        themes,
        [
            ("Theme: system", PaletteTarget::Theme(Theme::System)),
            ("Theme: light", PaletteTarget::Theme(Theme::Light)),
            ("Theme: dark", PaletteTarget::Theme(Theme::Dark)),
        ]
    );
}

#[test]
fn the_real_registry_lists_every_named_theme_in_both_languages() {
    use crate::prefs::Theme as PreferenceTheme;
    for lang in [Lang::En, Lang::Ko] {
        let choices = PreferenceTheme::ALL.map(|theme| (theme, lang.settings().theme(theme)));
        let all: Vec<PaletteEntry<(), PreferenceTheme>> =
            palette_entries(&fleet(Vec::new()), None, &[], choices, lang);
        let themes: Vec<_> = all
            .iter()
            .filter(|entry| entry.group == PaletteGroup::Theme)
            .collect();
        assert_eq!(themes.len(), PreferenceTheme::ALL.len());
        for (entry, theme) in themes.into_iter().zip(PreferenceTheme::ALL) {
            assert_eq!(entry.target, PaletteTarget::Theme(theme));
            assert_eq!(
                entry.label,
                lang.palette().theme(lang.settings().theme(theme))
            );
        }
    }
}

/// The kit's palette: typing filters over the label, the detail and the
/// keywords, whatever the case; a group with nothing is not shown.
#[test]
fn typing_filters_by_label_detail_and_id() {
    let all = entries(None);
    let found = palette_groups(&all, "API");
    assert_eq!(found.len(), 1);
    let (group, sessions) = &found[0];
    assert_eq!(*group, PaletteGroup::Sessions);
    assert_eq!(sessions[0].label, "api");
    let by_state = palette_groups(&all, "exited 0");
    assert_eq!(by_state[0].1[0].label, "old");
    assert!(palette_groups(&all, "nothing like it").is_empty());
    assert_eq!(palette_groups(&all, "").len(), PaletteGroup::ALL.len());
}

// Paseo's `command-center/results.test.ts`.

#[test]
fn keeps_the_active_row_while_it_is_listed_else_the_first() {
    let ids = ["workspace:first", "workspace:second"];
    assert_eq!(
        kept(Some("workspace:second"), &ids),
        Some("workspace:second")
    );
    assert_eq!(kept(Some("missing"), &ids), Some("workspace:first"));
    assert_eq!(kept(Some("workspace:first"), &[]), None);
    assert_eq!(kept(None, &ids), Some("workspace:first"));
}

#[test]
fn wraps_keyboard_selection_in_both_directions() {
    let ids = ["workspace:first", "workspace:second"];
    assert_eq!(
        moved(Some("workspace:second"), &ids, true),
        Some("workspace:first")
    );
    assert_eq!(
        moved(Some("workspace:first"), &ids, false),
        Some("workspace:second")
    );
    assert_eq!(moved(None, &ids, true), Some("workspace:first"));
    assert_eq!(moved(None, &ids, false), Some("workspace:second"));
    assert_eq!(moved(None, &[], true), None);
}

#[test]
fn keeps_keyboard_selection_aligned_with_rows_beyond_the_first() {
    let names: Vec<String> = (0..200).map(|index| format!("workspace:{index}")).collect();
    let ids: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut active = ids.first().copied();
    for _ in 0..150 {
        active = moved(active, &ids, true);
    }
    assert_eq!(active, Some("workspace:150"));
}

// The query's mark: the app's own cases, after Paseo's
// `findHighlightRanges` for the one substring this palette searches.

#[test]
fn marks_the_best_place_a_search_occurs_whatever_the_case() {
    assert_eq!(marks("view", "Review the view"), vec![11..15]);
    assert_eq!(marks("  BOARD ", "Go to board"), vec![6..11]);
    assert_eq!(
        marks("", "Go to board"),
        Vec::<std::ops::Range<usize>>::new()
    );
    assert_eq!(
        marks("sync", "Go to board"),
        Vec::<std::ops::Range<usize>>::new()
    );
}

#[test]
fn marks_in_bytes_of_words_in_any_script() {
    let text = "보드로 이동";
    let found = marks("이동", text).into_iter().next();
    assert_eq!(
        found.clone().and_then(|range| text.get(range)),
        Some("이동")
    );
    let text = "ÉTÉ été";
    assert_eq!(
        marks("été", text)
            .into_iter()
            .next()
            .and_then(|range| text.get(range)),
        Some("ÉTÉ")
    );
}
