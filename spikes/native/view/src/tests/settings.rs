//! The page's `data/settings.test.ts` and `settings-place.test.ts`.

use std::collections::BTreeSet;

use crate::Lang;
use crate::settings::{Group, Place, Section};

#[test]
fn reads_a_section_from_an_address_and_nothing_else() {
    assert_eq!(Section::of("machines"), Some(Section::Machines));
    assert_eq!(Section::of("Machines"), None);
    assert_eq!(Section::of(""), None);
    assert_eq!(Section::of("fleet"), None);
}

#[test]
fn puts_every_section_under_exactly_one_heading_in_the_lists_order() {
    let grouped: Vec<Section> = Group::ALL.into_iter().flat_map(Group::sections).collect();
    let shown: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|section| section.shown())
        .collect();
    assert_eq!(grouped, shown);
    for section in shown {
        assert!(section.group().sections().contains(&section));
    }
    // Sync is held back until its page is right on every kind of node.
    assert!(!Section::Sync.shown());
    assert!(!grouped.contains(&Section::Sync));
    assert_eq!(Section::of("sync"), None);
}

#[test]
fn names_every_section_each_differently() {
    for lang in [Lang::En, Lang::Ko] {
        let labels: BTreeSet<&str> = Section::ALL
            .into_iter()
            .map(|section| lang.settings().section(section))
            .collect();
        assert_eq!(labels.len(), Section::ALL.len());
    }
    assert!(Section::ALL.contains(&Section::DEFAULT));
    assert_eq!(Lang::En.settings().section(Section::DEFAULT), "General");
}

#[test]
fn names_a_section_a_machine_and_an_agent() {
    assert_eq!(
        Place::of(&["settings", "agents"], Some("gpu-box"), Some("codex")),
        Some(Place {
            section: Section::Agents,
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
        !Place::of(&["settings", "machines", "extra"], None, None)
            .unwrap()
            .exact
    );
}

#[test]
fn is_nothing_outside_settings() {
    assert_eq!(Place::of(&["sessions"], None, None), None);
    assert_eq!(Place::of(&[], None, None), None);
}
