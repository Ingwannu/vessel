//! The Files tree's order (`sort.rs`) through the real link: Name asks
//! nothing more of the machine; Modified inspects each listed folder and
//! orders by its answer; Size uses the facts already there; an answer for
//! a folder no longer listed is dropped.
use super::source_tests::{prepare, ready, root, setup};
use super::*;
use gpui::{AppContext as _, ParentElement as _, TestAppContext};
use vessel_app_link::Event;
use vessel_app_link::testing::Scenario;
use vessel_app_link::workspace::Answer;
use vessel_proto::workspace::directory_metadata::{
    FileModifiedMillis, IgnoreState, IgnoredEntries, InspectDirectory, InspectedDirectory,
    InspectedEntry,
};
use vessel_proto::workspace::panel_files::{FILE_PANELS_PROTOCOL, FileSize};
use vessel_proto::workspace::{DirEntry, DirListing, EntryKind, ListDir};
use vessel_proto::{Call, Outcome, Reply};

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: None,
        ignored: false,
    }
}

fn fact(path: &str, kind: EntryKind, modified: u64, size: u64) -> InspectedEntry {
    InspectedEntry {
        path: path.parse().unwrap(),
        kind,
        size: Some(FileSize(size)),
        modified: Some(FileModifiedMillis(modified)),
        ignored: IgnoreState::Visible,
    }
}

/// The names of the tree's entry rows, in order.
fn names(shell: &Shell) -> Vec<String> {
    let files = &shell.areas.workspace.code.files;
    files
        .rows
        .iter()
        .filter_map(|row| {
            if let TreeRow::Entry { path, .. } = row {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// The machine's next workspace answer.
fn workspace_answer(scene: &mut Scenario, shell: &Shell) -> (String, Answer) {
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Workspace { machine, answer } = scene.event(shell.fixture_link()).unwrap() {
            return (machine, answer);
        }
    }
    panic!("expected bounded workspace event");
}

#[gpui::test]
fn the_explorer_sort_orders_the_tree_by_the_folders_facts(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _window, cx| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        for machine in &mut shell.fleet.machines {
            machine.protocol = FILE_PANELS_PROTOCOL;
        }
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let root = WorkspaceRoot::Project { project };
        let listing = DirListing {
            entries: vec![
                entry("b.txt", EntryKind::File),
                entry("src", EntryKind::Dir),
                entry("c.txt", EntryKind::File),
                entry("a.txt", EntryKind::File),
            ],
            more: false,
        };
        let list = ListDir {
            root,
            dir: RelPath::root(),
            ignored: true,
        };
        shell.folder_read(&list, Ok(listing));
        assert_eq!(names(shell), ["src", "a.txt", "b.txt", "c.txt"]);

        // Modified: the listed folder is inspected (the first call since
        // the summary: Name asked nothing), then ordered newest first.
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Modified;
        shell.explorer_sorted();
        let asked = InspectDirectory {
            root,
            directory: RelPath::root(),
            ignored: IgnoredEntries::Include,
        };
        assert!(
            matches!(scene.request(shell.fixture_link()).unwrap(), Call::InspectDirectory(ask) if ask == asked)
        );
        let facts = InspectedDirectory {
            entries: vec![
                fact("src", EntryKind::Dir, 50, 0),
                fact("a.txt", EntryKind::File, 100, 10),
                fact("b.txt", EntryKind::File, 300, 50),
                fact("c.txt", EntryKind::File, 200, 3000),
            ],
            more: false,
        };
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::InspectDirectory(facts)),
            )
            .unwrap();
        let (machine, answer) = workspace_answer(&mut scene, shell);
        assert!(shell.workspace_answer(&machine, answer, cx));
        assert_eq!(names(shell), ["src", "b.txt", "c.txt", "a.txt"]);
        assert_eq!(shell.areas.workspace.code.files.sort, ExplorerSort::Modified);

        // Size: the facts are there, nothing more is asked; largest first.
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Size;
        shell.explorer_sorted();
        assert_eq!(names(shell), ["src", "c.txt", "b.txt", "a.txt"]);

        // An answer for a folder no longer listed changes nothing.
        let gone = InspectDirectory {
            directory: "src".parse().unwrap(),
            ..asked
        };
        let late = InspectedDirectory {
            entries: vec![fact("src/z.txt", EntryKind::File, 1, 1)],
            more: false,
        };
        shell.folder_inspected(&gone, Ok(late));
        assert!(
            !shell
                .areas
                .workspace
                .code
                .files
                .facts
                .contains_key(&"src/z.txt".parse::<RelPath>().unwrap())
        );

        // Name again: the listing's own order by name, no facts needed.
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Name;
        shell.explorer_sorted();
        assert_eq!(names(shell), ["src", "a.txt", "b.txt", "c.txt"]);
        // Only the one inspection went out.
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, Call::InspectDirectory(_)))
                .count(),
            1
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_machine_without_the_file_panels_sorts_by_name_and_asks_nothing(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _window, _cx| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Modified;
        shell.explorer_sorted();
        assert!(!shell.code_panels_offered());
        assert_eq!(shell.areas.workspace.code.files.sort, ExplorerSort::Name);
        let list = ListDir {
            root: WorkspaceRoot::Project { project },
            dir: RelPath::root(),
            ignored: true,
        };
        let listing = DirListing {
            entries: vec![
                entry("b.txt", EntryKind::File),
                entry("a.txt", EntryKind::File),
            ],
            more: false,
        };
        shell.folder_read(&list, Ok(listing));
        assert_eq!(names(shell), ["a.txt", "b.txt"]);
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, Call::InspectDirectory(_)))
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

/// Only the sort trigger, drawn for the shell (TRACKER 808).
struct TriggerProbe(gpui::Entity<Shell>);

impl gpui::Render for TriggerProbe {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let tokens = vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark);
        let trigger = self
            .0
            .update(cx, |shell, cx| super::sort::trigger(shell, &tokens, cx));
        vessel_app_kit::ui::div().children(trigger)
    }
}

/// Every `aria` object of GPUI's real accessibility snapshot, as text.
fn arias(cx: &mut gpui::VisualTestContext) -> Vec<String> {
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    json.split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .collect()
}

#[gpui::test]
fn the_sort_trigger_is_a_button_named_by_its_order(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let name = vessel_app_view::machine_book::Name::parse("fixture").unwrap();
    let (link, mut scene) = Scenario::start(&name).unwrap();
    let (probe, cx) = cx.add_window_view(move |window, cx| {
        TriggerProbe(cx.new(|cx| Shell::with_test_link(window, cx, link)))
    });
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, _cx| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        for machine in &mut shell.fleet.machines {
            machine.protocol = FILE_PANELS_PROTOCOL;
        }
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
    });
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    probe.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    let button = |arias: &[String], word: &str| {
        let label = format!(r#""label": "{word}""#);
        arias
            .iter()
            .any(|aria| aria.contains(&label) && aria.contains(r#""Button""#))
    };
    // Paseo's word for the order shown, which a press moves on.
    assert!(button(&arias(cx), "Name"));
    shell.update(cx, |shell, _| {
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Modified;
    });
    probe.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert!(button(&arias(cx), "Modified"));
}
