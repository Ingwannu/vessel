//! Behavior regressions exercise contracts independent of a toolkit or node.
use super::*;
use vessel_proto::workspace::{RelPath, WorkspaceRoot};
use vessel_proto::{SessionId, TerminalId};

fn open(layout: &mut Layout, target: Target, placement: Placement) -> TabId {
    layout
        .open(
            target,
            OpenIntent::Reveal,
            placement,
            AgentRole::Root,
            LayoutMode::SplitPanes,
        )
        .unwrap()
}
fn file() -> Target {
    Target::File {
        root: WorkspaceRoot::Project {
            project: vessel_proto::ProjectId::random(),
        },
        path: "src/main.rs".parse::<RelPath>().unwrap(),
    }
}
#[test]
fn new_launchers_are_instances_but_resource_reveal_preserves_user_placement() {
    let mut layout = Layout::default();
    let first = open(&mut layout, Target::NewTab, Placement::Focused);
    let second = open(&mut layout, Target::NewTab, Placement::Focused);
    assert_ne!(first, second);
    let target = file();
    let tab = open(&mut layout, target.clone(), Placement::Focused);
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    assert_eq!(
        open(&mut layout, target.clone(), Placement::Pane(side)),
        tab
    );
    assert_eq!(
        open(&mut layout, target, Placement::Prefer(PaneId::MAIN)),
        tab
    );
    assert_eq!(layout.containing(tab), Some(side));
}
#[test]
fn identical_paths_in_different_roots_never_share_a_tab() {
    let mut layout = Layout::default();
    let a = open(&mut layout, file(), Placement::Focused);
    let b = open(&mut layout, file(), Placement::Focused);
    assert_ne!(a, b);
}
#[test]
fn unsupported_host_or_missing_pane_open_is_atomic() {
    let mut layout = Layout::default();
    let before = layout.clone();
    let browser = Target::Browser(ResourceId::parse("preview").unwrap());
    assert_eq!(
        layout.open(
            browser,
            OpenIntent::New,
            Placement::Explorer,
            AgentRole::Root,
            LayoutMode::SplitPanes
        ),
        Err(LayoutError::UnsupportedHost)
    );
    assert_eq!(layout, before);
    assert_eq!(
        layout.open(
            file(),
            OpenIntent::New,
            Placement::Pane(PaneId(99)),
            AgentRole::Root,
            LayoutMode::SplitPanes
        ),
        Err(LayoutError::MissingPane)
    );
    assert_eq!(layout, before);
}
#[test]
fn compact_files_are_dirty_guarded_sheets_and_unknown_agents_are_tabs() {
    let mut layout = Layout::default();
    let agent = layout
        .open(
            Target::Agent(SessionId::random()),
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Root,
            LayoutMode::Compact,
        )
        .unwrap();
    assert!(layout.sheets().is_empty());
    let file = layout
        .open(
            file(),
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Root,
            LayoutMode::Compact,
        )
        .unwrap();
    assert_eq!(layout.sheets().first().map(|sheet| sheet.tab), Some(file));
    assert_eq!(layout.containing(agent), Some(PaneId::MAIN));
    layout.set_modified(file, Modified::Dirty).unwrap();
    assert_eq!(layout.dismiss_sheet(file), Err(LayoutError::Modified));
    assert_eq!(layout.sheets().len(), 1);
    layout.set_modified(file, Modified::Clean).unwrap();
    layout.dismiss_sheet(file).unwrap();
    assert!(layout.tab(file).is_none());
}
#[test]
fn child_promotion_preserves_identity_and_close_never_archives_it() {
    let mut layout = Layout::default();
    let session = SessionId::random();
    let tab = layout
        .open(
            Target::Agent(session),
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Child,
            LayoutMode::Compact,
        )
        .unwrap();
    assert_eq!(
        layout.request_close(tab).unwrap().effect(),
        CloseEffect::HideChild(session)
    );
    layout.promote_sheet(tab).unwrap();
    assert_eq!(layout.containing(tab), Some(PaneId::MAIN));
    assert_eq!(layout.tab(tab).unwrap().target(), &Target::Agent(session));
}
#[test]
fn lifecycle_and_late_dirty_change_block_unconditional_close() {
    let mut layout = Layout::default();
    let terminal = TerminalId::random();
    let tab = open(&mut layout, Target::Terminal(terminal), Placement::Focused);
    let plan = layout.request_close(tab).unwrap();
    assert_eq!(plan.effect(), CloseEffect::CloseTerminal(terminal));
    assert_eq!(
        layout.close_confirmed(&plan, CloseApproval::Local),
        Err(LayoutError::InvalidState)
    );
    layout.set_modified(tab, Modified::Dirty).unwrap();
    assert_eq!(
        layout.close_confirmed(&plan, CloseApproval::LifecycleSettled),
        Err(LayoutError::Modified)
    );
    layout.set_modified(tab, Modified::Clean).unwrap();
    layout
        .close_confirmed(&plan, CloseApproval::LifecycleSettled)
        .unwrap();
    assert!(layout.tab(tab).is_none());
}
#[test]
fn hydration_gap_keeps_tabs_and_owned_snapshot_opens_roots_without_focus_theft() {
    let mut layout = Layout::default();
    let session = SessionId::random();
    let first = open(&mut layout, Target::Agent(session), Placement::Focused);
    layout
        .reconcile(
            &EntitySnapshot {
                agents: Hydration::Pending,
                sessions: &[],
                terminals: Hydration::Pending,
                terminal_ids: &[],
            },
            LayoutMode::SplitPanes,
        )
        .unwrap();
    assert!(layout.tab(first).is_some());
    let root = SessionId::random();
    let child = SessionId::random();
    layout
        .reconcile(
            &EntitySnapshot {
                agents: Hydration::Ready,
                sessions: &[
                    SessionTab {
                        session,
                        role: AgentRole::Root,
                    },
                    SessionTab {
                        session: root,
                        role: AgentRole::Root,
                    },
                    SessionTab {
                        session: child,
                        role: AgentRole::Child,
                    },
                ],
                terminals: Hydration::Ready,
                terminal_ids: &[],
            },
            LayoutMode::SplitPanes,
        )
        .unwrap();
    assert_eq!(
        layout.root().panes().first().unwrap().focused(),
        Some(first)
    );
    assert_eq!(layout.tabs().count(), 2);
}
#[test]
fn explorer_hide_retains_shell_and_last_ordinary_pane_cannot_close() {
    let mut layout = Layout::default();
    let files = open(&mut layout, Target::Files, Placement::Focused);
    assert_eq!(layout.containing(files), Some(PaneId::EXPLORER));
    layout.close_empty_pane(PaneId::EXPLORER).unwrap();
    assert_eq!(layout.explorer_visibility(), ExplorerVisibility::Hidden);
    assert_eq!(layout.explorer().tabs(), &[files]);
    layout.close_empty_pane(PaneId::MAIN).unwrap();
    assert_eq!(layout.root().panes().len(), 1);
}
#[test]
fn platform_capability_and_depth_limit_are_enforced_without_mutation() {
    let mut layout = Layout::default();
    assert_eq!(
        layout.split(PaneId::MAIN, Position::Right, LayoutMode::SinglePane),
        Err(LayoutError::SplitsUnavailable)
    );
    for _ in 1..MAX_TREE_DEPTH {
        let _pane = layout
            .split(PaneId::MAIN, Position::Bottom, LayoutMode::SplitPanes)
            .unwrap();
    }
    let before = layout.clone();
    assert_eq!(
        layout.split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes),
        Err(LayoutError::TreeTooDeep)
    );
    assert_eq!(layout, before);
}
#[test]
fn launcher_uses_reference_order_and_hides_compact_diff() {
    let wide = catalog(LaunchPurpose::Primary, LayoutMode::SplitPanes, |_| {
        Availability::Ready
    });
    assert_eq!(wide.first().map(|entry| entry.0), Some(Launch::Agent));
    let compact = catalog(LaunchPurpose::Supporting, LayoutMode::Compact, |_| {
        Availability::Ready
    });
    assert_eq!(compact.first().map(|entry| entry.0), Some(Launch::Changes));
    assert!(!compact.iter().any(|entry| entry.0 == Launch::Diff));
}

#[test]
fn moving_a_launcher_preserves_its_id_and_dirty_guard() {
    let mut layout = Layout::default();
    let tab = open(&mut layout, Target::NewTab, Placement::Focused);
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    layout.set_modified(tab, Modified::Dirty).unwrap();
    layout
        .drop_tab(tab, side, Position::Center, LayoutMode::SplitPanes)
        .unwrap();
    assert_eq!(layout.containing(tab), Some(side));
    assert_eq!(layout.tabs().count(), 1);
    assert_eq!(layout.tab(tab).unwrap().modified(), Modified::Dirty);
}

#[test]
fn directional_focus_follows_split_axis_and_move_keeps_tab_id() {
    let mut layout = Layout::default();
    let tab = open(&mut layout, Target::NewTab, Placement::Focused);
    let right = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let bottom = layout
        .split(right, Position::Bottom, LayoutMode::SplitPanes)
        .unwrap();
    layout.focus_neighbor(Direction::Up).unwrap();
    assert_eq!(layout.focused_pane(), right);
    layout.focus_neighbor(Direction::Left).unwrap();
    assert_eq!(layout.focused_pane(), PaneId::MAIN);
    layout
        .move_neighbor(Direction::Right, LayoutMode::SplitPanes)
        .unwrap();
    assert_eq!(layout.containing(tab), Some(right));
    assert_ne!(right, bottom);
}
#[test]
fn bulk_close_scope_is_pane_local_and_never_includes_neighbors() {
    let mut layout = Layout::default();
    let a = open(&mut layout, Target::NewTab, Placement::Focused);
    let b = open(&mut layout, Target::NewTab, Placement::Focused);
    let c = open(&mut layout, Target::NewTab, Placement::Focused);
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let d = open(&mut layout, Target::NewTab, Placement::Pane(side));
    assert_eq!(
        layout
            .close_plans(b, CloseScope::Before)
            .unwrap()
            .iter()
            .map(ClosePlan::tab)
            .collect::<Vec<_>>(),
        [a]
    );
    assert_eq!(
        layout
            .close_plans(b, CloseScope::After)
            .unwrap()
            .iter()
            .map(ClosePlan::tab)
            .collect::<Vec<_>>(),
        [c]
    );
    assert!(
        !layout
            .close_plans(b, CloseScope::Pane)
            .unwrap()
            .iter()
            .any(|plan| plan.tab() == d)
    );
}
#[test]
fn invalid_edge_drop_preserves_layout_and_original_resource() {
    let mut layout = Layout::default();
    let browser = open(
        &mut layout,
        Target::Browser(ResourceId::parse("browser").unwrap()),
        Placement::Focused,
    );
    let before = layout.clone();
    assert_eq!(
        layout.drop_tab(
            browser,
            PaneId::EXPLORER,
            Position::Center,
            LayoutMode::SplitPanes
        ),
        Err(LayoutError::UnsupportedHost)
    );
    assert_eq!(layout, before);
    assert_eq!(
        layout.drop_tab(browser, PaneId::MAIN, Position::Right, LayoutMode::Compact),
        Err(LayoutError::SplitsUnavailable)
    );
    assert_eq!(layout, before);
}

#[test]
fn background_reconciliation_never_promotes_an_existing_child_sheet() {
    let mut layout = Layout::default();
    let child = SessionId::random();
    let tab = layout
        .open(
            Target::Agent(child),
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Child,
            LayoutMode::Compact,
        )
        .unwrap();
    assert_eq!(
        layout
            .open(
                Target::Agent(child),
                OpenIntent::Background,
                Placement::Focused,
                AgentRole::Child,
                LayoutMode::Compact
            )
            .unwrap(),
        tab
    );
    assert_eq!(layout.sheets().last().map(|sheet| sheet.tab), Some(tab));
    assert_eq!(layout.containing(tab), None);
}
#[test]
fn group_resize_targets_the_group_not_a_selected_descendant() {
    let mut layout = Layout::default();
    let right = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let outer = match layout.root() {
        SplitNode::Group { id, .. } => *id,
        SplitNode::Pane(_) => panic!("missing split"),
    };
    let _pane = layout
        .split(right, Position::Bottom, LayoutMode::SplitPanes)
        .unwrap();
    layout.resize(outer, 0.75).unwrap();
    match layout.root() {
        SplitNode::Group {
            fraction,
            children: [_, b],
            ..
        } => {
            assert!((fraction.get() - 0.75).abs() < f32::EPSILON);
            assert!(
                matches!(b.as_ref(),SplitNode::Group {fraction,..} if (fraction.get()-0.5).abs()<f32::EPSILON)
            );
        }
        SplitNode::Pane(_) => panic!("missing split"),
    }
    assert_eq!(
        layout.resize(outer, f32::NAN),
        Err(LayoutError::InvalidState)
    );
}

/// Paseo's `removeTabFromTree` (TRACKER 776 item 6): a pane goes with its
/// last tab, but the main pane and the last ordinary pane stay.
#[test]
fn a_pane_goes_with_its_last_tab_but_main_and_the_last_pane_stay() {
    let mut layout = Layout::default();
    let main_tab = open(&mut layout, Target::NewTab, Placement::Focused);
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let side_tab = open(&mut layout, Target::NewTab, Placement::Pane(side));
    let plan = layout.request_close(side_tab).unwrap();
    layout.close_confirmed(&plan, CloseApproval::Local).unwrap();
    assert!(layout.pane(side).is_none());
    assert_eq!(layout.root().panes().len(), 1);
    assert_eq!(layout.focused_pane(), PaneId::MAIN);
    // The main pane stays empty beside another pane.
    let other = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let _other_tab = open(&mut layout, Target::NewTab, Placement::Pane(other));
    let plan = layout.request_close(main_tab).unwrap();
    layout.close_confirmed(&plan, CloseApproval::Local).unwrap();
    assert!(layout.pane(PaneId::MAIN).is_some());
    assert_eq!(layout.root().panes().len(), 2);
}

/// Paseo's move: the source pane goes when the moved tab was its last,
/// unless the tab stays in it.
#[test]
fn moving_a_pane_s_last_tab_away_removes_the_pane() {
    let mut layout = Layout::default();
    let _main_tab = open(&mut layout, Target::NewTab, Placement::Focused);
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let moving = open(&mut layout, Target::NewTab, Placement::Pane(side));
    layout
        .drop_tab(
            moving,
            PaneId::MAIN,
            Position::Center,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    assert!(layout.pane(side).is_none());
    assert_eq!(layout.containing(moving), Some(PaneId::MAIN));
    // Splitting a pane with its own only tab keeps the pane it came from.
    let lone = layout
        .split(PaneId::MAIN, Position::Bottom, LayoutMode::SplitPanes)
        .unwrap();
    let only = open(&mut layout, Target::NewTab, Placement::Pane(lone));
    layout
        .drop_tab(only, lone, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    assert!(layout.pane(lone).is_some());
}
