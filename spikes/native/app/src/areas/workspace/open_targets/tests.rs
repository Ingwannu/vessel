//! Open in editor on a computer made of scratch folders: the PATH is a
//! folder of empty stub files that are never run, and every launch goes to
//! the recording launcher. Nothing is started and no file manager opens.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{Entity, TestAppContext, VisualTestContext};
use vessel_app_link::testing::Scenario;
use vessel_app_view::machine_book::Name;
use vessel_app_view::open_targets::{Env, Platform, Request, TargetId};
use vessel_proto::{HarnessKind, ProjectId, SessionId, SessionInfo, SessionMode, SessionSpec};

use super::Origin;
use super::launcher::{Launched, Recording};
use super::system::Computer;
use crate::shell::Shell;

/// A scratch folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "vessel-open-{name}-{}-{}",
            std::process::id(),
            SessionId::random()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir.canonicalize().unwrap())
    }

    /// An empty file at `path` under it (a stub: never run).
    fn file(&self, path: &str) -> PathBuf {
        let at = self.0.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, b"").unwrap();
        at
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _gone = std::fs::remove_dir_all(&self.0);
    }
}

/// What a test holds: the shell, its window, the link's fixture, the
/// session's machine and project, and the launcher's record.
struct Fixture<'a> {
    shell: Entity<Shell>,
    cx: &'a mut VisualTestContext,
    _scene: Scenario,
    machine: String,
    project: ProjectId,
    launched: Arc<Recording>,
}

/// The fixture's machine on this computer, a session open in `root`, and
/// a PATH of `bin`.
fn setup<'a>(cx: &'a mut TestAppContext, root: &Path, bin: &Path) -> Fixture<'a> {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let launched = Arc::new(Recording::default());
    let project = ProjectId::random();
    let context = scene.context();
    let machine = context.machine().as_str().to_owned();
    let computer = Arc::new(Computer::with(
        Platform::Other,
        Env {
            path: Some(bin.display().to_string()),
            ..Env::default()
        },
    ));
    shell.update_in(cx, |shell, _window, cx| {
        let session = SessionId::random();
        let mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        let info = SessionInfo::new(
            session,
            SessionSpec::new(HarnessKind::ClaudeCode, mode),
            chrono::Utc::now(),
        );
        let mut card = vessel_app_view::Card::from_session(&machine, &info);
        card.detail.cwd = Some(root.display().to_string());
        card.detail.project = Some(project);
        let mut known = vessel_app_view::Machine::unreachable(machine.clone(), "fixture");
        known.node = Some(context.node());
        known.connection_epoch = Some(context.epoch());
        known.sessions = vec![card];
        shell.fleet.machines = vec![known];
        shell.areas.session.open = Some(crate::areas::session::Open {
            machine: machine.clone(),
            session,
            title: "fixture".to_owned(),
            agent: "fake".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        shell.add_flow_this_computer(vec![context.node()], cx);
        let seam: Arc<dyn super::Launcher> = Arc::<Recording>::clone(&launched);
        shell.areas.workspace.open.set_computer(computer, seam);
    });
    Fixture {
        shell,
        cx,
        _scene: scene,
        machine,
        project,
        launched,
    }
}

/// The targets listed for `root`, once the listing has run.
fn listed(fixture: &mut Fixture<'_>, root: &Path) -> Option<Vec<TargetId>> {
    fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        let _first = shell.open_targets(root, cx);
    });
    fixture.cx.run_until_parked();
    fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        shell.open_targets(root, cx).map(<[TargetId]>::to_vec)
    })
}

/// `request` opened in `target` from `origin`, then the toast shown.
fn open(
    fixture: &mut Fixture<'_>,
    target: TargetId,
    request: Request,
    origin: Origin,
) -> Option<String> {
    fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        shell.open_in_target(target, request, origin, cx);
    });
    fixture.cx.run_until_parked();
    fixture.shell.update_in(fixture.cx, |shell, _window, _cx| {
        shell.areas.toasts.shown().map(|toast| toast.text.clone())
    })
}

fn request(root: &Path, folder: &Path, file: Option<&Path>) -> Request {
    Request {
        root: root.to_path_buf(),
        folder: folder.to_path_buf(),
        file: file.map(Path::to_path_buf),
        line: None,
        column: None,
    }
}

#[gpui::test]
fn the_header_opens_the_session_s_folder_in_the_preferred_editor(cx: &mut TestAppContext) {
    let (bin, work) = (Scratch::new("bin"), Scratch::new("work"));
    let code = bin.file("code");
    let zed = bin.file("zed");
    let main = work.file("src/main.rs");
    let root = work.0.clone();
    let mut fixture = setup(cx, &root, &bin.0);
    assert_eq!(
        listed(&mut fixture, &root),
        Some(vec![TargetId::VsCode, TargetId::Zed, TargetId::FileManager])
    );
    let (machine, project) = (fixture.machine.clone(), fixture.project);
    fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        let shown = shell.shown_workspace().unwrap();
        assert_eq!(shown.root, root);
        assert_eq!(shown.project, Some(project));
        let listed = [TargetId::VsCode, TargetId::Zed, TargetId::FileManager];
        // Nothing chosen: the first listed, as Paseo's.
        assert_eq!(
            shell.preferred_target(&machine, Some(project), &listed),
            Some(TargetId::VsCode)
        );
        shell.choose_target(&machine, Some(project), TargetId::Zed, cx);
        assert_eq!(
            shell.preferred_target(&machine, Some(project), &listed),
            Some(TargetId::Zed)
        );
        assert_eq!(
            shell
                .prefs
                .settings
                .preferred_editors
                .get(&machine, project),
            Some(TargetId::Zed)
        );
        // A folder's "Open in" takes the preferred among editors only.
        assert_eq!(
            shell.preferred_editor(&machine, Some(project), &[TargetId::FileManager]),
            None
        );
    });
    let said = open(
        &mut fixture,
        TargetId::Zed,
        request(&root, &root, None),
        Origin::Editor,
    );
    assert_eq!(said, None);
    let mut at_line = request(&root, &root, Some(&main));
    at_line.line = Some(7);
    let _said = open(&mut fixture, TargetId::VsCode, at_line, Origin::Editor);
    let main_at = format!("{}:7", main.display());
    assert_eq!(
        fixture.launched.asked(),
        [
            Launched::Start(zed, vec![root.clone().into_os_string()]),
            Launched::Start(
                code,
                vec![root.into_os_string(), "--goto".into(), main_at.into()]
            ),
        ]
    );
}

#[gpui::test]
fn files_reveal_in_the_file_manager_and_nothing_leaves_the_workspace(cx: &mut TestAppContext) {
    let (bin, work, outside) = (
        Scratch::new("bin"),
        Scratch::new("work"),
        Scratch::new("outside"),
    );
    let _zed = bin.file("zed");
    let main = work.file("src/main.rs");
    let secret = outside.file("secret.txt");
    let root = work.0.clone();
    let link = root.join("out");
    std::os::unix::fs::symlink(&secret, &link).unwrap();
    let mut fixture = setup(cx, &root, &bin.0);
    let (editor, manager) = fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        shell.files_open_targets(cx)
    });
    assert_eq!(manager, Some(TargetId::FileManager));
    // The editor comes once the list has been read.
    assert_eq!(editor, None);
    let _listed = listed(&mut fixture, &root);
    let (editor, _manager) = fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        shell.files_open_targets(cx)
    });
    assert_eq!(editor, Some(TargetId::Zed));
    let said = open(
        &mut fixture,
        TargetId::FileManager,
        request(&root, &root, Some(&main)),
        Origin::Reveal,
    );
    assert_eq!(said, None);
    let folder = root.join("src");
    let _said = open(
        &mut fixture,
        TargetId::FileManager,
        request(&root, &folder, None),
        Origin::Reveal,
    );
    assert_eq!(
        fixture.launched.asked(),
        [Launched::Reveal(main), Launched::OpenFolder(folder)]
    );
    // A link out of the workspace is refused before anything starts.
    let said = open(
        &mut fixture,
        TargetId::Zed,
        request(&root, &root, Some(&link)),
        Origin::Editor,
    );
    assert_eq!(
        said.as_deref(),
        Some(format!("This path is outside the workspace: {}", link.display()).as_str())
    );
    assert_eq!(fixture.launched.asked().len(), 2);
}

#[gpui::test]
fn a_missing_folder_says_paseo_s_words_and_a_machine_elsewhere_offers_nothing(
    cx: &mut TestAppContext,
) {
    let (bin, work) = (Scratch::new("bin"), Scratch::new("work"));
    let root = work.0.clone();
    let gone = root.join("gone");
    let mut fixture = setup(cx, &root, &bin.0);
    let said = open(
        &mut fixture,
        TargetId::FileManager,
        request(&gone, &gone, None),
        Origin::Sidebar,
    );
    assert_eq!(said.as_deref(), Some("Couldn't open folder"));
    let said = open(
        &mut fixture,
        TargetId::Cursor,
        request(&root, &root, None),
        Origin::Editor,
    );
    assert_eq!(said.as_deref(), Some("Cursor is not installed"));
    assert!(fixture.launched.asked().is_empty());
    fixture.shell.update_in(fixture.cx, |shell, _window, cx| {
        shell.add_flow_this_computer(vec![vessel_proto::NodeId::random()], cx);
        assert!(shell.shown_workspace().is_none());
        assert_eq!(shell.files_open_targets(cx), (None, None));
    });
}

/// Every `aria` object of GPUI's accessibility tree after a fresh frame
/// (`plugins/rows/tests.rs`'s reading); no raw tree is printed.
fn arias(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    json.split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .collect()
}

/// The one node labelled `label`.
fn node(arias: &[String], label: &str) -> String {
    let wanted = format!(r#""label": "{label}""#);
    let found: Vec<&String> = arias.iter().filter(|aria| aria.contains(&wanted)).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].clone()
}

#[gpui::test]
fn the_header_s_open_and_its_caret_are_named_buttons(cx: &mut TestAppContext) {
    let (bin, work) = (Scratch::new("bin"), Scratch::new("work"));
    let zed = bin.file("zed");
    let root = work.0.clone();
    let mut fixture = setup(cx, &root, &bin.0);
    let handle = fixture.cx.update(|window, _| window.window_handle());
    assert!(fixture.cx.simulate_accessibility_activation(handle));
    fixture.shell.update(fixture.cx, |shell, cx| {
        shell.route = crate::routes::Route::Session;
        cx.notify();
    });
    assert_eq!(
        listed(&mut fixture, &root),
        Some(vec![TargetId::Zed, TargetId::FileManager])
    );
    // Paseo's names (button.tsx 233-266): the primary says what it opens
    // and where, the caret is "Choose editor", collapsed.
    let shown = arias(fixture.cx);
    let open = node(&shown, "Open workspace in Zed");
    assert!(
        open.contains(r#""Button""#) && open.contains(r#""Click""#),
        "{open}"
    );
    let caret = node(&shown, "Choose editor");
    assert!(
        caret.contains(r#""Button""#) && caret.contains(r#""expanded": false"#),
        "{caret}"
    );
    // The primary's press opens the workspace in Zed.
    let primary = fixture.cx.debug_bounds("open-in-editor-primary").unwrap();
    fixture
        .cx
        .simulate_click(primary.center(), gpui::Modifiers::none());
    fixture.cx.run_until_parked();
    assert_eq!(
        fixture.launched.asked(),
        [Launched::Start(zed, vec![root.clone().into_os_string()])]
    );
    // The caret's press opens the list: expanded.
    let chevron = fixture.cx.debug_bounds("open-in-editor-caret").unwrap();
    fixture
        .cx
        .simulate_click(chevron.center(), gpui::Modifiers::none());
    fixture.cx.run_until_parked();
    let caret = node(&arias(fixture.cx), "Choose editor");
    assert!(caret.contains(r#""expanded": true"#), "{caret}");
    // With a file shown in Files, the primary names the file.
    let file = super::header::Shown {
        machine: fixture.machine.clone(),
        project: Some(fixture.project),
        root: root.clone(),
        file: Some((root.join("src/main.rs"), Some(3))),
    };
    let words = vessel_app_view::Lang::En.open_targets();
    assert_eq!(
        super::header::primary_name(&file, TargetId::Zed, words),
        "Open main.rs in Zed"
    );
}
