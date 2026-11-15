use super::*;
use crate::palette::registration::{
    ChoicePath, Contribution, Presentation, Rank, RegistrationError, Selection, SourceId,
    Visibility,
};

fn entry(id: &str, title: &str, group: &str, rank: i16) -> Contribution<u8> {
    Contribution {
        id: id.parse().unwrap(),
        group: group.parse().unwrap(),
        group_rank: Rank::new(rank),
        section_title: Some(group.to_owned()),
        rank: Rank::default(),
        keywords: Vec::new(),
        visibility: Visibility::Always,
        presentation: Presentation::Action {
            title: title.to_owned(),
            subtitle: None,
        },
        target: 1,
    }
}

fn source() -> SourceId {
    "root".parse().unwrap()
}

#[test]
fn stale_cleanup_cannot_remove_a_replacement_and_duplicate_refusal_is_atomic() {
    let mut registry = Registry::default();
    let old = registry
        .replace(source(), vec![entry("a", "Old", "actions", 0)])
        .unwrap();
    let current = registry
        .replace(source(), vec![entry("b", "Current", "actions", 0)])
        .unwrap();
    registry.remove(&old);
    assert_eq!(registry.snapshot()[0].1.id.as_str(), "b");
    assert_eq!(
        registry.replace(
            source(),
            vec![entry("c", "A", "actions", 0), entry("c", "B", "actions", 0)]
        ),
        Err(RegistrationError::DuplicateId)
    );
    assert_eq!(registry.snapshot()[0].1.id.as_str(), "b");
    registry.remove(&current);
    assert!(registry.snapshot().is_empty());
}

#[test]
fn source_and_local_ids_cannot_alias_through_a_colon() {
    let mut registry = Registry::default();
    let _one = registry
        .replace(
            "a:b".parse().unwrap(),
            vec![entry("c", "One", "actions", 0)],
        )
        .unwrap();
    let _two = registry
        .replace(
            "a".parse().unwrap(),
            vec![entry("b:c", "Two", "actions", 0)],
        )
        .unwrap();
    assert_ne!(registry.snapshot()[0].0, registry.snapshot()[1].0);
}

#[test]
fn empty_query_uses_group_rank_then_group_row_rank_and_id() {
    let mut registry = Registry::default();
    let mut hidden = entry("h", "Hidden", "hidden", -2);
    hidden.visibility = Visibility::Query;
    let _owner = registry
        .replace(
            source(),
            vec![
                entry("b", "Second", "actions", 0),
                hidden,
                entry("a", "First", "workspace", -1),
            ],
        )
        .unwrap();
    let sections = contribution_sections(&registry, "  ");
    assert_eq!(
        sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<Vec<_>>(),
        ["workspace", "actions"]
    );
}

#[test]
fn visible_matches_outrank_exact_hidden_keywords_in_rows_and_sections() {
    let mut registry = Registry::default();
    let mut hidden = entry("hidden", "Schedules", "actions", -1);
    hidden.keywords = vec!["auto".to_owned()];
    let _owner = registry
        .replace(
            source(),
            vec![hidden, entry("visible", "Auto mode", "modes", 12)],
        )
        .unwrap();
    let sections = contribution_sections(&registry, "auto");
    assert_eq!(sections[0].id.as_str(), "modes");
    assert_eq!(sections[1].id.as_str(), "actions");
}

#[test]
fn choice_path_is_visible_searchable_and_subsequences_do_not_cross_words() {
    let mut registry = Registry::default();
    let mut choice = entry("design", "Unused", "labels", 0);
    choice.visibility = Visibility::Query;
    choice.presentation = Presentation::Choice {
        path: ChoicePath::new("Labels".to_owned(), vec!["Design".to_owned()]).unwrap(),
        selection: Selection::Selected,
    };
    let _owner = registry.replace(source(), vec![choice]).unwrap();
    assert!(contribution_sections(&registry, "").is_empty());
    assert_eq!(
        contribution_sections(&registry, "lbl dsgn")[0].rows.len(),
        1
    );
    assert!(contribution_sections(&registry, "labdes").is_empty());
    assert!(ChoicePath::new(String::new(), Vec::new()).is_err());
}

#[test]
fn equal_scores_preserve_registry_order_instead_of_alphabetizing_labels() {
    let mut registry = Registry::default();
    let _owner = registry
        .replace(
            source(),
            vec![
                entry("a", "Open Zebra", "actions", 0),
                entry("b", "Open Alpha", "actions", 0),
            ],
        )
        .unwrap();
    let sections = contribution_sections(&registry, "open");
    assert_eq!(
        sections[0]
            .rows
            .iter()
            .map(|row| row.id.entry.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}
