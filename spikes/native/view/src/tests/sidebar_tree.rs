//! The sidebar grouped by project (TRACKER 682): Paseo's tree of projects
//! by name, each holding its sessions under its limit, then each machine's
//! sessions in no listed project.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use vessel_proto::workspace::{GitSupport, GitVersion, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{ProjectId, SessionStatus};

use super::fixture::{fleet, id, machine, session, short};
use crate::board::{Attention, RunsIn};
use crate::prefs::{FoldedProjects, ProjectKey, ProjectOrder, SidebarView};
use crate::sidebar::tree::{PROJECT_SESSIONS, SidebarTree, TreeState, by_name};
use crate::workspace::projects::ProjectsView;
use crate::{Card, Fleet};

const RUNNING: SessionStatus = SessionStatus::Running;

fn project_id(index: u16) -> ProjectId {
    format!("{index:08x}-0000-4000-8000-00000000000a")
        .parse()
        .unwrap()
}

fn list(projects: &[(u16, &str)]) -> ProjectsView {
    ProjectsView::Ready(ProjectList {
        git: GitSupport::Found {
            version: GitVersion::MINIMUM,
        },
        more: false,
        projects: projects
            .iter()
            .map(|&(index, name)| ProjectInfo {
                id: project_id(index),
                name: name.parse().unwrap(),
                root: format!("/fixture/{name}").into(),
                git: true,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: "2026-11-19T08:00:00Z".parse().unwrap(),
                revision: 1,
            })
            .collect(),
    })
}

fn place(fleet: &mut Fleet, machine: &str, short: &str, runs_in: Option<RunsIn>) {
    let card = fleet
        .machines
        .iter_mut()
        .filter(|one| one.name == machine)
        .flat_map(|one| one.sessions.iter_mut())
        .find(|card| card.session == id(short))
        .unwrap();
    card.detail.runs_in = runs_in;
}

/// laptop: shop (two sessions, one in a worktree), docs (none), site (one);
/// a session in no project and one in a project its list does not hold.
/// gpu-box: Atlas (one). build-server: unreachable, its list read before.
fn book() -> (Fleet, HashMap<String, ProjectsView>) {
    let mut fleet = fleet(vec![
        machine(
            "laptop",
            true,
            &[
                session("a1", "2026-11-19T08:00:00Z", RUNNING),
                session("a2", "2026-11-19T09:00:00Z", RUNNING),
                session("a3", "2026-11-19T10:00:00Z", RUNNING),
                session("a4", "2026-11-19T11:00:00Z", RUNNING),
                session("a5", "2026-11-19T12:00:00Z", RUNNING),
            ],
        ),
        machine(
            "gpu-box",
            true,
            &[session("b1", "2026-11-19T08:00:00Z", RUNNING)],
        ),
        machine("build-server", false, &[]),
    ]);
    place(
        &mut fleet,
        "laptop",
        "a1",
        Some(RunsIn::Checkout(project_id(0))),
    );
    place(
        &mut fleet,
        "laptop",
        "a2",
        Some(RunsIn::Worktree(project_id(0))),
    );
    place(
        &mut fleet,
        "laptop",
        "a3",
        Some(RunsIn::Checkout(project_id(2))),
    );
    place(&mut fleet, "laptop", "a4", None);
    place(
        &mut fleet,
        "laptop",
        "a5",
        Some(RunsIn::Checkout(project_id(9))),
    );
    place(
        &mut fleet,
        "gpu-box",
        "b1",
        Some(RunsIn::Checkout(project_id(3))),
    );
    let lists = HashMap::from([
        (
            "laptop".to_owned(),
            list(&[(0, "shop"), (1, "docs"), (2, "site")]),
        ),
        ("gpu-box".to_owned(), list(&[(3, "Atlas")])),
        // Read before it went away: not listed while it is unreachable.
        ("build-server".to_owned(), list(&[(4, "archive")])),
    ]);
    (fleet, lists)
}

fn shorts(cards: &[&Card]) -> Vec<String> {
    cards.iter().map(|card| short(card, 2)).collect()
}

fn names(tree: &SidebarTree<'_>) -> Vec<String> {
    tree.projects
        .iter()
        .map(|project| format!("{}/{}", project.machine, project.project.name))
        .collect()
}

#[test]
fn placed_projects_lead_in_their_order_and_the_rest_follow_by_name() {
    let (fleet, lists) = book();
    let folded = FoldedProjects::default();
    let more = HashSet::new();
    let plain = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: None,
    };
    let keys: Vec<ProjectKey> = fleet
        .sidebar_tree(SidebarView::default(), plain)
        .projects
        .iter()
        .map(|project| ProjectKey::new(project.machine, project.project.id))
        .collect();
    // site, then Atlas, dropped there (TRACKER 811); docs and shop never
    // placed, after them by name.
    let order = ProjectOrder::of(vec![keys[3].clone(), keys[0].clone()]);
    let state = TreeState {
        order: &order,
        ..plain
    };
    assert_eq!(
        names(&fleet.sidebar_tree(SidebarView::default(), state)),
        ["laptop/site", "gpu-box/Atlas", "laptop/docs", "laptop/shop"]
    );
}

#[test]
fn projects_by_name_hold_their_sessions_and_each_machine_keeps_the_rest() {
    let (fleet, lists) = book();
    let folded = FoldedProjects::default();
    let more = HashSet::new();
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: None,
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    // Atlas before docs: a name's case does not order it.
    assert_eq!(
        names(&tree),
        ["gpu-box/Atlas", "laptop/docs", "laptop/shop", "laptop/site"]
    );
    assert!(tree.several_machines);
    let sessions: Vec<Vec<String>> = tree
        .projects
        .iter()
        .map(|project| shorts(&project.sessions))
        .collect();
    // Newest first in a project, as every grouping lists; a worktree's
    // session is its project's.
    assert_eq!(sessions, [vec!["b1"], vec![], vec!["a2", "a1"], vec!["a3"]]);
    assert!(tree.projects.iter().all(|project| !project.can_toggle));
    // laptop's session in no project and the one in a project its list does
    // not hold; gpu-box has none, so no group; build-server is unreachable,
    // so its group stays with nothing in it.
    let machines: Vec<(&str, bool, Vec<String>)> = tree
        .machines
        .iter()
        .map(|group| {
            (
                group.machine.name.as_str(),
                group.connected,
                shorts(&group.sessions),
            )
        })
        .collect();
    assert_eq!(
        machines,
        [
            ("laptop", true, vec!["a5".to_owned(), "a4".to_owned()]),
            ("build-server", false, vec![]),
        ]
    );
    assert_eq!(shorts(&tree.order()), ["b1", "a2", "a1", "a3", "a5", "a4"]);
}

#[test]
fn a_folded_project_draws_no_session_and_numbers_skip_it() {
    let (fleet, lists) = book();
    let mut folded = FoldedProjects::default();
    folded.toggle("laptop", project_id(0));
    let more = HashSet::new();
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: None,
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    let shop = tree
        .projects
        .iter()
        .find(|project| project.project.id == project_id(0))
        .unwrap();
    assert!(shop.folded);
    assert_eq!(shop.total, 2);
    assert_eq!(shorts(&tree.order()), ["b1", "a3", "a5", "a4"]);
}

/// One machine, one project, `count` sessions in it, newest first a0, a1…
fn crowded(count: u16) -> (Fleet, HashMap<String, ProjectsView>) {
    let sessions: Vec<_> = (0..count)
        .map(|index| {
            let hour = 23_u16.saturating_sub(index);
            session(
                &format!("{index:02}"),
                &format!("2026-11-19T{hour:02}:00:00Z"),
                RUNNING,
            )
        })
        .collect();
    let mut fleet = fleet(vec![machine("laptop", true, &sessions)]);
    for card in &mut fleet.machines.first_mut().unwrap().sessions {
        card.detail.runs_in = Some(RunsIn::Checkout(project_id(0)));
    }
    let lists = HashMap::from([("laptop".to_owned(), list(&[(0, "shop")]))]);
    (fleet, lists)
}

#[test]
fn a_project_shows_seven_and_every_one_not_done_then_show_more() {
    let (mut fleet, lists) = crowded(10);
    // The ninth is working: shown though past the limit.
    fleet.machines.first_mut().unwrap().sessions[8].attention = Attention::Working;
    let folded = FoldedProjects::default();
    let mut more = HashSet::new();
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: None,
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    let shop = &tree.projects[0];
    assert_eq!(PROJECT_SESSIONS, 7);
    assert_eq!(
        shorts(&shop.sessions),
        ["00", "01", "02", "03", "04", "05", "06", "08"]
    );
    assert!(shop.can_toggle);
    assert!(!shop.expanded);
    assert_eq!(shop.leading, None);
    assert_eq!(shop.total, 10);
    // "Show more": every one, "Show less" offered.
    let _new = more.insert(("laptop".to_owned(), project_id(0)));
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: None,
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    let shop = &tree.projects[0];
    assert_eq!(shop.sessions.len(), 10);
    assert!(shop.can_toggle);
    assert!(shop.expanded);
}

#[test]
fn the_open_session_past_the_limit_leads_its_project() {
    let folded = FoldedProjects::default();
    let more = HashSet::new();
    // Eight: the open eighth is the only one the limit would hide, so it is
    // drawn in its place and there is nothing to show more of.
    let (fleet, lists) = crowded(8);
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: Some(("laptop", id("07"))),
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    let shop = &tree.projects[0];
    assert_eq!(shop.leading.map(|card| card.session), None);
    assert_eq!(shop.sessions.len(), 8);
    assert!(!shop.can_toggle);
    // Ten: the open ninth leads, the eighth and tenth wait for Show more.
    let (fleet, lists) = crowded(10);
    let state = TreeState {
        lists: &lists,
        folded: &folded,
        order: &ProjectOrder::default(),
        unfolded_more: &more,
        open: Some(("laptop", id("08"))),
    };
    let tree = fleet.sidebar_tree(SidebarView::default(), state);
    let shop = &tree.projects[0];
    assert_eq!(shop.leading.map(|card| card.session), Some(id("08")));
    assert_eq!(shop.sessions.len(), PROJECT_SESSIONS);
    assert!(shop.can_toggle);
    assert_eq!(
        shorts(&tree.order()).first().map(String::as_str),
        Some("08")
    );
}

#[test]
fn names_order_as_people_read_them() {
    assert_eq!(by_name("app2", "app10"), Ordering::Less);
    assert_eq!(by_name("Beta", "alpha"), Ordering::Greater);
    assert_eq!(by_name("docs", "Docs"), Ordering::Equal);
    assert_eq!(by_name("v007", "v7"), Ordering::Equal);
    assert_eq!(by_name("site", "site-2"), Ordering::Less);
    assert_eq!(by_name("보관함", "atlas"), Ordering::Greater);
}
