use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
struct Workspace {
    node: u8,
    id: u8,
}
fn workspace(node: u8, id: u8) -> Workspace {
    Workspace { node, id }
}
#[test]
fn workspace_targets_keep_the_whole_open_order_before_show_more_and_only_number_first_nine() {
    let pinned = [workspace(2, 8)];
    let project = (0..12).map(|id| workspace(1, id)).collect::<Vec<_>>();
    let hidden = [workspace(9, 9)];
    let targets = Targets::from_sections(&[
        Section {
            targets: &pinned,
            expansion: Expansion::Open,
        },
        Section {
            targets: &hidden,
            expansion: Expansion::Collapsed,
        },
        Section {
            targets: &project,
            expansion: Expansion::Open,
        },
    ]);
    assert_eq!(targets.items().len(), 13);
    assert_eq!(targets.items().first(), pinned.first());
    assert_eq!(targets.items().last(), project.last());
    assert_eq!(targets.number(&pinned[0]).unwrap().get(), 1);
    assert_eq!(targets.number(&project[7]).unwrap().get(), 9);
    assert_eq!(targets.number(&project[8]), None);
    assert_eq!(targets.number(&hidden[0]), None);
    assert_eq!(targets.numbered(Number::new(9).unwrap()), Some(&project[7]));
    // The tenth/twelfth placements may not yet be painted behind Show more.
    assert_eq!(
        targets.relative(Some(&project[10]), Step::Next),
        Some(&project[11])
    );
    assert_eq!(
        targets.relative(Some(&project[11]), Step::Next),
        Some(&pinned[0])
    );
    assert_eq!(
        targets.relative(Some(&pinned[0]), Step::Previous),
        Some(&project[11])
    );
}
#[test]
fn absent_selection_and_empty_sections_follow_reference_first_last_fallback() {
    let rows = [workspace(1, 7), workspace(2, 7)];
    let targets = Targets::from_sections(&[Section {
        targets: &rows,
        expansion: Expansion::Open,
    }]);
    for selection in [None, Some(&workspace(3, 7))] {
        assert_eq!(targets.relative(selection, Step::Next), Some(&rows[0]));
        assert_eq!(targets.relative(selection, Step::Previous), Some(&rows[1]));
    }
    assert_eq!(targets.number(&rows[0]).unwrap().get(), 1);
    assert_eq!(targets.number(&rows[1]).unwrap().get(), 2);
    let empty = Targets::<Workspace>::from_sections(&[]);
    assert_eq!(empty.relative(None, Step::Next), None);
    assert_eq!(empty.numbered(Number::new(1).unwrap()), None);
    assert_eq!(Number::new(0), None);
    assert_eq!(Number::new(10), None);
}
