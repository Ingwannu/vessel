//! Paseo's `menu-navigation.test.ts`, case for case.
use super::{MenuPath, PageId};

fn path(pages: &[&str]) -> MenuPath {
    let mut path = MenuPath::default();
    for (depth, page) in pages.iter().enumerate() {
        path.open(&PageId::from((*page).to_owned()), depth);
    }
    path
}

fn page(name: &str) -> PageId {
    PageId::from(name.to_owned())
}

#[test]
fn opening_from_the_root_nests_and_swaps_sibling_branches() {
    let mut root = MenuPath::default();
    root.open(&page("show"), 0);
    assert_eq!(root, path(&["show"]));
    let mut nested = path(&["show"]);
    nested.open(&page("colors"), 1);
    assert_eq!(nested, path(&["show", "colors"]));
    // A sibling trigger swaps branches instead of nesting.
    let mut swapped = path(&["grouping"]);
    swapped.open(&page("show"), 0);
    assert_eq!(swapped, path(&["show"]));
    // An ancestor's sibling drops the deeper pages.
    let mut ancestor = path(&["show", "colors"]);
    ancestor.open(&page("grouping"), 0);
    assert_eq!(ancestor, path(&["grouping"]));
    // Already open: unchanged.
    let mut same = path(&["show", "colors"]);
    same.open(&page("show"), 0);
    assert_eq!(same, path(&["show", "colors"]));
}

#[test]
fn closing_a_depth_closes_what_it_opened_and_back_pops_one() {
    let mut all = path(&["show", "colors"]);
    all.close(0);
    assert_eq!(all, MenuPath::default());
    let mut deeper = path(&["show", "colors"]);
    deeper.close(1);
    assert_eq!(deeper, path(&["show"]));
    let mut nothing = path(&["show"]);
    nothing.close(1);
    assert_eq!(nothing, path(&["show"]));
    let mut back = path(&["show", "colors"]);
    assert!(back.back());
    assert_eq!(back, path(&["show"]));
    assert!(back.back());
    assert!(!back.back());
    assert_eq!(back, MenuPath::default());
}

#[test]
fn the_current_page_and_open_checks_use_id_and_depth_together() {
    assert_eq!(MenuPath::default().current(), None);
    let open = path(&["show", "colors"]);
    assert_eq!(open.current(), Some(&page("colors")));
    assert!(open.is_open(&page("show"), 0));
    assert!(open.is_open(&page("colors"), 1));
    assert!(!open.is_open(&page("colors"), 0));
    assert!(!open.is_open(&page("grouping"), 0));
}

#[test]
fn a_page_that_vanished_is_pruned_with_what_it_opened() {
    let mut pruned = path(&["show", "colors", "shade"]);
    pruned.prune(|page| page.as_ref() != "colors");
    assert_eq!(pruned, path(&["show"]));
    let mut kept = path(&["show"]);
    kept.prune(|_| true);
    assert_eq!(kept, path(&["show"]));
}
