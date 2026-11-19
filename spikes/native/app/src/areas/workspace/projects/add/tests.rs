//! Add project's folder suggestions through the real link (`Scenario`):
//! one ask per pause, the latest key's; rows kept for their query; a row
//! chosen in Add a folder adds that folder; an older machine is not asked.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::Event;
use vessel_app_link::testing::Scenario;
use vessel_app_link::workspace::Answer;
use vessel_app_view::machine_book::Name;
use vessel_proto::workspace::host_folders::HOST_FOLDERS_PROTOCOL;
use vessel_proto::{Call, Outcome, ProtocolVersion, Reply};

fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut gpui::VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}

/// The fixture's machine in the book, speaking `protocol`; its name.
fn machine(shell: &mut Shell, scene: &Scenario, protocol: Option<ProtocolVersion>) -> String {
    let context = scene.context();
    let name = context.machine().as_str().to_owned();
    let mut machine = vessel_app_view::Machine::unreachable(name.clone(), "fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    machine.protocol = protocol;
    shell.fleet.machines = vec![machine];
    name
}

fn workspace_answer(scene: &mut Scenario, shell: &Shell) -> (String, Answer) {
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Workspace { machine, answer } = scene.event(shell.fixture_link()).unwrap() {
            return (machine, answer);
        }
    }
    panic!("expected bounded workspace event");
}

fn type_keys(shell: &gpui::Entity<Shell>, cx: &mut gpui::VisualTestContext, keys: &[&str]) {
    for key in keys {
        let key = (*key).to_owned();
        shell.update_in(cx, |shell, window, cx| {
            shell.areas.workspace.page.path.set_text(&key, window, cx);
            shell.add_path_changed(key, cx);
        });
    }
    cx.executor().advance_clock(PAUSE);
    cx.run_until_parked();
}

#[gpui::test]
fn the_latest_key_is_asked_after_a_pause_and_a_chosen_row_is_added(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    let name = shell.update_in(cx, |shell, window, cx| {
        let name = machine(shell, &scene, HOST_FOLDERS_PROTOCOL);
        shell.open_add(AddForm::Folder, window, cx);
        name
    });
    type_keys(&shell, cx, &["co", "code"]);
    shell.update_in(cx, |shell, window, cx| {
        let asked = scene.request(shell.fixture_link());
        assert!(
            matches!(&asked, Ok(Call::SuggestFolders(ask)) if ask.query.as_str() == "code" && ask.limit.map(SuggestionLimit::get) == Some(FOLDERS)),
            "{asked:?}"
        );
        let found = FolderSuggestions {
            folders: vec![SuggestedFolder {
                path: "/home/me/code".into(),
                shown: "~/code".to_owned(),
            }],
            more: false,
        };
        scene
            .reply(shell.fixture_link(), Outcome::Ok(Reply::SuggestFolders(found)))
            .unwrap();
        let (machine, answer) = workspace_answer(&mut scene, shell);
        assert_eq!(machine, name);
        assert!(shell.workspace_answer(&machine, answer, cx));
        let rows = &shell.areas.workspace.page.suggestions;
        assert_eq!(rows.query, "code");
        assert_eq!(rows.folders.len(), 1);
        // An answer for another query is kept with its query: not drawn
        // while the field holds "code".
        shell.folders_suggested(&name, "cod", Ok(FolderSuggestions { folders: Vec::new(), more: false }));
        assert_eq!(shell.areas.workspace.page.suggestions.query, "cod");
        shell.add_folder_chosen(Path::new("/home/me/code"), window, cx);
        assert_eq!(shell.areas.workspace.page.path.text(cx), "/home/me/code");
        assert!(
            matches!(scene.request(shell.fixture_link()).unwrap(), Call::AddProject(add) if add.path == Path::new("/home/me/code"))
        );
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, Call::SuggestFolders(_)))
                .count(),
            1
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn an_older_machine_is_not_asked(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let older =
            HOST_FOLDERS_PROTOCOL.map(|needs| ProtocolVersion::new(needs.get().saturating_sub(1)));
        let _name = machine(shell, &scene, older);
        shell.open_add(AddForm::Folder, window, cx);
    });
    type_keys(&shell, cx, &["code"]);
    shell.update_in(cx, |shell, _window, _cx| {
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, Call::SuggestFolders(_)))
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}
