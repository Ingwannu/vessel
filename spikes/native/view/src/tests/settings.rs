//! Settings' sections: Paseo's table (`screens/settings-screen.tsx`) and
//! the page's `settings-place.test.ts`.

use std::collections::BTreeSet;

use crate::Lang;
use crate::settings::{Group, Place, Section, Surface};

const SURFACES: [Surface; 3] = [Surface::Desktop, Surface::Browser, Surface::Phone];

#[test]
fn reads_a_section_from_an_address_and_nothing_else() {
    assert_eq!(Section::of("host"), Some(Section::Overview));
    assert_eq!(Section::of("pair-device"), Some(Section::Devices));
    assert_eq!(Section::of("Providers"), None);
    assert_eq!(Section::of(""), None);
    assert_eq!(Section::of("fleet"), None);
}

#[test]
fn lists_paseos_sections_in_paseos_order_on_the_desktop() {
    let slugs = |group: Group| -> Vec<&str> {
        group
            .sections(Surface::Desktop)
            .into_iter()
            .map(Section::slug)
            .collect()
    };
    // `SIDEBAR_SECTION_ITEMS`; Editor too, as Paseo's desktop app is a web
    // page.
    assert_eq!(
        slugs(Group::App),
        [
            "general",
            "appearance",
            "sidebar",
            "chat",
            "terminal",
            "browser",
            "editor",
            "files",
            "shortcuts",
            "integrations",
            "notifications",
            "permissions",
            "diagnostics",
            "about",
        ]
    );
    // `HOST_SECTION_ITEMS`, then Vessel's own Models (Sync is held back).
    assert_eq!(
        slugs(Group::Machine),
        [
            "host",
            "projects",
            "connections",
            "pair-device",
            "agents",
            "metadata",
            "voice",
            "workspaces",
            "providers",
            "usage",
            "terminals",
            "plugins",
            "models",
        ]
    );
}

#[test]
fn lists_the_desktop_only_sections_on_the_desktop_only() {
    let phone = Group::App.sections(Surface::Phone);
    let browser = Group::App.sections(Surface::Browser);
    for section in [
        Section::Browser,
        Section::Shortcuts,
        Section::Integrations,
        Section::Notifications,
        Section::Permissions,
    ] {
        assert!(!phone.contains(&section), "{section:?} on a phone");
        assert!(!browser.contains(&section), "{section:?} in a browser");
    }
    assert!(browser.contains(&Section::Editor));
    assert!(!phone.contains(&Section::Editor));
    // Every machine section is everywhere.
    for surface in SURFACES {
        assert_eq!(Group::Machine.sections(surface).len(), 13);
    }
}

#[test]
fn puts_every_section_under_exactly_one_heading_in_the_lists_order() {
    let grouped: Vec<Section> = Group::ALL
        .into_iter()
        .flat_map(|group| group.sections(Surface::Desktop))
        .collect();
    let shown: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|section| section.shown())
        .collect();
    assert_eq!(grouped, shown);
    for section in shown {
        assert!(
            section
                .group()
                .sections(Surface::Desktop)
                .contains(&section)
        );
        assert_eq!(section.per_machine(), section.group() == Group::Machine);
    }
    // Sync is held back until its page is right on every kind of node.
    assert!(!Section::Sync.shown());
    assert!(!grouped.contains(&Section::Sync));
    assert_eq!(Section::of("sync"), None);
}

#[test]
fn names_every_section_of_a_group_each_differently() {
    for lang in [Lang::En, Lang::Ko] {
        for group in Group::ALL {
            let sections: Vec<Section> = Section::ALL
                .into_iter()
                .filter(|section| section.group() == group)
                .collect();
            let labels: BTreeSet<&str> = sections
                .iter()
                .map(|section| lang.settings().section(*section, catalog(lang)))
                .collect();
            assert_eq!(labels.len(), sections.len(), "{lang:?} {group:?}");
        }
        let groups: BTreeSet<&str> = Group::ALL
            .into_iter()
            .map(|group| lang.settings().group(group))
            .collect();
        assert_eq!(groups.len(), Group::ALL.len());
    }
    assert!(Section::ALL.contains(&Section::DEFAULT));
    assert_eq!(
        Lang::En
            .settings()
            .section(Section::DEFAULT, catalog(Lang::En)),
        "General"
    );
    assert_eq!(
        Lang::En
            .settings()
            .section(Section::Devices, catalog(Lang::En)),
        "Pair device"
    );
    assert_eq!(
        Lang::Ko
            .settings()
            .section(Section::Overview, catalog(Lang::Ko)),
        "개요"
    );
}

#[test]
fn names_a_section_a_machine_and_an_agent() {
    assert_eq!(
        Place::of(&["settings", "providers"], Some("gpu-box"), Some("codex")),
        Some(Place {
            section: Section::Providers,
            exact: true,
            machine: Some("gpu-box".to_owned()),
            agent: Some("codex".to_owned()),
        })
    );
}

#[test]
fn is_the_default_section_when_it_names_none_or_an_unknown_one() {
    for segments in [&["settings"][..], &["settings", "fleet"]] {
        let place = Place::of(segments, None, None).unwrap();
        assert_eq!(place.section, Section::General);
        assert!(!place.exact);
    }
    assert!(
        !Place::of(&["settings", "host", "extra"], None, None)
            .unwrap()
            .exact
    );
}

#[test]
fn is_nothing_outside_settings() {
    assert_eq!(Place::of(&["sessions"], None, None), None);
    assert_eq!(Place::of(&[], None, None), None);
}

fn catalog(lang: Lang) -> &'static crate::localization::Translator {
    use crate::localization::{Locale, Translator};
    Translator::get(
        Locale::from_tag(match lang {
            Lang::En => "en",
            Lang::Ko => "ko",
        })
        .unwrap(),
    )
    .unwrap()
}
