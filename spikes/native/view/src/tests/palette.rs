//! `web/src/data/commands.test.ts`: what the command palette lists, and
//! the search its kit's palette runs.

use vessel_proto::{SessionInfo, SessionStatus};

use super::fixture::{fleet, id, in_folder, machine, session};
use crate::Lang;
use crate::palette::{PaletteEntry, PaletteGroup, PaletteTarget, palette_entries, palette_groups};

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
    palette_entries(
        &book,
        selected,
        &SECTIONS,
        [Theme::System, Theme::Light, Theme::Dark],
        Lang::En,
    )
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
    assert_eq!(entry.detail.as_deref(), Some("running · /w/api"));
    assert!(entry.found(&full));
}

#[test]
fn opens_a_machines_detail_in_settings_in_the_pages_words() {
    let all = entries(None);
    let entry = in_group(&all, PaletteGroup::Machines)[0];
    assert_eq!(entry.label, "laptop");
    assert_eq!(entry.target, PaletteTarget::Machine("laptop".to_owned()));
    let detail = entry.detail.as_deref().unwrap();
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
