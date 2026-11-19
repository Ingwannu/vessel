//! The Files tree's order (`sort.rs`) through the real link: Name asks
//! nothing more of the machine; Modified inspects each listed folder and
//! orders by its answer; Size uses the facts already there; an answer for
//! a folder no longer listed is dropped; many folders are inspected two at
//! a time, and one refused is asked again with the next listing.
use super::source_tests::{prepare, ready, root, setup};
use super::*;
use gpui::TestAppContext;
use vessel_app_link::Event;
use vessel_app_link::testing::Scenario;
use vessel_app_link::workspace::Answer;
use vessel_proto::workspace::directory_metadata::{
    FileModifiedMillis, IgnoreState, IgnoredEntries, InspectDirectory, InspectedDirectory,
    InspectedEntry,
};
use vessel_proto::workspace::panel_files::{FILE_PANELS_PROTOCOL, FileSize};
use vessel_proto::workspace::{DirEntry, DirListing, EntryKind, ListDir};
use vessel_proto::{Call, ErrorCode, Outcome, Reply, WireError};

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

/// List folder `dir` of `root` (`""` for the root) with `entries`.
fn read(shell: &mut Shell, root: WorkspaceRoot, dir: &str, entries: Vec<DirEntry>) {
    let list = ListDir {
        root,
        dir: if dir.is_empty() {
            RelPath::root()
        } else {
            dir.parse().unwrap()
        },
        ignored: true,
    };
    shell.folder_read(
        &list,
        Ok(DirListing {
            entries,
            more: false,
        }),
    );
}

/// Answer the machine's next `file.inspect` (refused as busy for folder
/// `busy`, else with no facts) and hand the answer to the shell; returns
/// the folder it asked.
fn answer_inspection(
    scene: &mut Scenario,
    shell: &mut Shell,
    busy: Option<&str>,
    cx: &mut Context<'_, Shell>,
) -> String {
    let Call::InspectDirectory(ask) = scene.request(shell.fixture_link()).unwrap() else {
        panic!("expected file.inspect");
    };
    let outcome = if busy == Some(ask.directory.to_string().as_str()) {
        Outcome::Err(WireError::new(
            ErrorCode::Unavailable,
            "workspace file operations are busy",
        ))
    } else {
        Outcome::Ok(Reply::InspectDirectory(InspectedDirectory {
            entries: Vec::new(),
            more: false,
        }))
    };
    scene.reply(shell.fixture_link(), outcome).unwrap();
    let (from, answer) = workspace_answer(scene, shell);
    assert!(shell.workspace_answer(&from, answer, cx));
    ask.directory.to_string()
}

#[gpui::test]
fn many_folders_are_inspected_two_at_a_time_and_a_refused_one_again(cx: &mut TestAppContext) {
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
        let folders = ["d1", "d2", "d3", "d4"];
        let dirs = folders.iter().map(|name| entry(name, EntryKind::Dir));
        read(shell, root, "", dirs.collect());
        for name in folders {
            read(shell, root, name, vec![entry("x.txt", EntryKind::File)]);
        }
        let machine = shell.areas.workspace.code.root().unwrap().0.to_owned();
        let line = |shell: &Shell| {
            let background = &shell.areas.workspace.background;
            (background.out(&machine), background.waiting(&machine))
        };

        // Modified with the root and four folders listed: two inspections
        // out, three waiting (the node refuses a fifth file work at once).
        shell.prefs.settings.layout.explorer_sort = ExplorerSort::Modified;
        shell.explorer_sorted();
        assert_eq!(line(shell), (2, 3));
        let mut asked = Vec::new();
        for _ in 0..5 {
            asked.push(answer_inspection(&mut scene, shell, Some("d2"), cx));
            assert!(line(shell).0 <= 2);
        }
        asked.sort();
        asked.dedup();
        assert_eq!(asked.len(), 5);
        assert_eq!(line(shell), (0, 0));
        let refused = &shell.areas.workspace.code.files.refused;
        assert!(refused.contains(&"d2".parse::<RelPath>().unwrap()));

        // The next folder listed asks the refused one again.
        read(shell, root, "d1", vec![entry("y.txt", EntryKind::File)]);
        let mut again = vec![
            answer_inspection(&mut scene, shell, None, cx),
            answer_inspection(&mut scene, shell, None, cx),
        ];
        again.sort();
        assert_eq!(again, ["d1", "d2"]);
        assert!(shell.areas.workspace.code.files.refused.is_empty());
        assert_eq!(line(shell), (0, 0));
        scene.close(shell.fixture_link()).unwrap();
    });
}
