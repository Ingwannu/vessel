//! Counterfactual: a stale window used to erase another window's added layout.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::layout::{AgentRole, LayoutMode, OpenIntent, Placement, Target};
use vessel_proto::{NodeId, WorkspaceId};

fn key() -> LayoutKey {
    LayoutKey::Workspace {
        node: NodeId::random(),
        workspace: WorkspaceId::random(),
    }
}
fn record(key: &LayoutKey) -> RememberedLayout {
    RememberedLayout::new(key.clone(), &Layout::default())
}
fn open(layout: &mut Layout) -> vessel_app_view::layout::TabId {
    layout
        .open(
            Target::Draft(vessel_app_view::layout::DraftId::random()),
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap()
}
#[test]
fn rebase_incorporates_unrelated_additions_and_removals_without_replacing_local_changes() {
    let (a, b, removed) = (key(), key(), key());
    let old = vec![record(&a), record(&removed)];
    let new = vec![record(&a), record(&b)];
    let mut local = Layout::default();
    let tab = open(&mut local);
    let mut book = BTreeMap::from([
        (a.clone(), local.clone()),
        (removed.clone(), Layout::default()),
    ]);
    rebase(&mut book, &old, &new, &BTreeSet::new());
    assert_eq!(book.get(&a), Some(&local));
    assert!(book.get(&a).is_some_and(|layout| layout.tab(tab).is_some()));
    assert!(book.contains_key(&b));
    assert!(!book.contains_key(&removed));
}
#[test]
fn dirty_and_owner_held_trees_survive_even_when_the_old_canonical_tree_matches() {
    let (dirty, held, clean) = (key(), key(), key());
    let mut layout = Layout::default();
    let tab = open(&mut layout);
    layout.set_modified(tab, Modified::Dirty).unwrap();
    let old = vec![
        RememberedLayout::new(dirty.clone(), &layout),
        record(&held),
        record(&clean),
    ];
    let mut book = BTreeMap::from([
        (dirty.clone(), layout.clone()),
        (held.clone(), Layout::default()),
        (clean.clone(), Layout::default()),
    ]);
    rebase(&mut book, &old, &[], &BTreeSet::from([held.clone()]));
    assert_eq!(book.get(&dirty), Some(&layout));
    assert!(book.contains_key(&held));
    assert!(!book.contains_key(&clean));
}
#[gpui::test]
fn actual_cache_hook_keeps_another_windows_layout_in_the_next_local_projection(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let (a, b) = (key(), key());
        let old = vec![record(&a)];
        let new = vec![record(&a), record(&b)];
        shell.prefs.layouts = old.clone();
        super::super::initialized(shell);
        shell.layout_preferences_applied(&old, &new, cx);
        // The coordinator replaces projected prefs after rebasing this cache.
        shell.prefs.layouts = new;
        shell
            .update_layout(
                &a,
                |layout| {
                    let _tab = open(layout);
                    Ok(())
                },
                cx,
            )
            .unwrap();
        assert!(shell.prefs.layouts.iter().any(|entry| entry.key == b));
    });
}
