//! The sidebar's Display menu (Paseo's `display-preferences/menu.tsx`):
//! the root's decisions with their values, each page's choices with the
//! chosen one checked, and what choosing does.

use crate::Lang;
use crate::prefs::{Grouping, PreferenceChange, SessionSort, SidebarView};
use crate::sidebar::display::{DisplayAct, DisplayPage, OptionMark, RowKind};

fn view(group: Grouping, sort: SessionSort, ended: bool) -> SidebarView {
    SidebarView { group, sort, ended }
}

#[test]
fn the_root_names_each_decision_with_its_value_and_ended_is_a_switch() {
    let words = Lang::En.sidebar();
    let rows = DisplayPage::Root.rows(view(Grouping::Status, SessionSort::Activity, true), words);
    let shown: Vec<(&str, RowKind)> = rows.iter().map(|row| (row.label, row.kind)).collect();
    assert_eq!(
        shown,
        vec![
            ("Grouping", RowKind::Opens { value: "Status" }),
            (
                "Sort",
                RowKind::Opens {
                    value: "Last activity"
                }
            ),
            ("Show ended sessions", RowKind::Switch { on: true }),
        ]
    );
    assert_eq!(DisplayPage::Root.title(words), None);
}

#[test]
fn a_page_checks_the_choice_in_effect_and_only_grouping_carries_marks() {
    let words = Lang::Ko.sidebar();
    let now = view(Grouping::Machine, SessionSort::Created, false);
    let groups = DisplayPage::Grouping.rows(now, words);
    assert_eq!(
        groups
            .iter()
            .map(|row| (row.label, row.kind))
            .collect::<Vec<_>>(),
        vec![
            (
                "머신",
                RowKind::Choice {
                    checked: true,
                    mark: Some(OptionMark::Machine)
                }
            ),
            (
                "상태",
                RowKind::Choice {
                    checked: false,
                    mark: Some(OptionMark::Status)
                }
            ),
        ]
    );
    let sorts = DisplayPage::Sort.rows(now, words);
    assert!(
        sorts
            .iter()
            .all(|row| matches!(row.kind, RowKind::Choice { mark: None, .. }))
    );
    assert_eq!(
        sorts
            .iter()
            .filter(|row| matches!(row.kind, RowKind::Choice { checked: true, .. }))
            .count(),
        1
    );
    assert_eq!(DisplayPage::Grouping.title(words), Some(words.grouping));
    assert_eq!(DisplayPage::Sort.title(words), Some(words.sort));
}

#[test]
fn choosing_opens_a_page_flips_the_switch_or_sets_and_closes() {
    let now = view(Grouping::Machine, SessionSort::Created, false);
    assert_eq!(
        DisplayPage::Root.choose(0, now),
        Some(DisplayAct::Open(DisplayPage::Grouping))
    );
    assert_eq!(
        DisplayPage::Root.choose(1, now),
        Some(DisplayAct::Open(DisplayPage::Sort))
    );
    assert_eq!(
        DisplayPage::Root.choose(2, now),
        Some(DisplayAct::Flip(PreferenceChange::SidebarEnded(true)))
    );
    assert_eq!(
        DisplayPage::Grouping.choose(1, now),
        Some(DisplayAct::Choose(PreferenceChange::SidebarGroup(
            Grouping::Status
        )))
    );
    assert_eq!(
        DisplayPage::Sort.choose(1, now),
        Some(DisplayAct::Choose(PreferenceChange::SidebarSort(
            SessionSort::Activity
        )))
    );
    assert_eq!(DisplayPage::Root.choose(3, now), None);
    assert_eq!(DisplayPage::Sort.choose(2, now), None);
}
