//! The Add project sheet through the real link (`Scenario`): the machine's
//! folders asked once per pause, for the latest key, and kept with their
//! machine and text; a folder chosen is added; an older machine is not
//! asked and gets no New directory; a refusal is said on its page; a
//! success closes the sheet and moves on; a language change keeps the
//! field. The GitHub page searches its machine and keeps typed rows
//! while it searches and under an error.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::Event;
use vessel_app_link::testing::Scenario;
use vessel_app_link::workspace::{Answer, Form};
use vessel_app_view::machine_book::Name;
use vessel_proto::workspace::host_folders::{HOST_FOLDERS_PROTOCOL, SuggestedFolder};
use vessel_proto::{Call, Outcome, Reply};

fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut gpui::VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}

/// The fixture's machine in the book, online, speaking `protocol`; its
/// name.
fn machine(shell: &mut Shell, scene: &Scenario, protocol: Option<ProtocolVersion>) -> String {
    let context = scene.context();
    let name = context.machine().as_str().to_owned();
    let mut machine = vessel_app_view::Machine::unreachable(name.clone(), "fixture");
    machine.reach = vessel_app_view::Reach::Online;
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
            shell
                .areas
                .workspace
                .add_flow
                .field
                .set_text(&key, window, cx);
            shell.add_flow_typed(key, cx);
        });
    }
    cx.executor().advance_clock(PAUSE);
    cx.run_until_parked();
}

fn page_id(shell: &Shell) -> Option<PageId> {
    shell
        .areas
        .workspace
        .add_flow
        .open
        .as_ref()
        .map(|open| open.flow.page().kind.id())
}

#[gpui::test]
fn folders_are_asked_for_the_latest_key_kept_with_their_machine_and_added(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    let name = shell.update_in(cx, |shell, window, cx| {
        let name = machine(shell, &scene, PROJECT_DIRECTORY_PROTOCOL);
        shell.open_add_flow(None, window, cx);
        // One machine: its methods first, no machine page.
        assert_eq!(page_id(shell), Some(PageId::Method));
        shell.add_flow_choose(RowAction::Method(Method::DirectorySearch), window, cx);
        assert_eq!(page_id(shell), Some(PageId::DirectorySearch));
        name
    });
    // The page's first ask (the empty field) is answered first.
    shell.update_in(cx, |shell, _window, _cx| {
        let first = scene.request(shell.fixture_link());
        assert!(
            matches!(&first, Ok(Call::SuggestFolders(ask)) if ask.query.as_str().is_empty()),
            "{first:?}"
        );
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::SuggestFolders(FolderSuggestions {
                    folders: Vec::new(),
                    more: false,
                })),
            )
            .unwrap();
        let _first = workspace_answer(&mut scene, shell);
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
        let (from, answer) = workspace_answer(&mut scene, shell);
        assert_eq!(from, name);
        assert!(shell.workspace_answer(&from, answer, cx));
        let rows = render::rows_of(shell);
        assert_eq!(rows.first().map(|row| &row.action), Some(&RowAction::AddPath("/home/me/code".to_owned())));
        // The same text answered for another machine is not this one's.
        shell.add_flow_folders("elsewhere", "code", Ok(FolderSuggestions { folders: Vec::new(), more: false }));
        assert!(render::rows_of(shell).is_empty());
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert!(search_loading(shell, open));
        // Enter adds the active folder.
        shell.add_flow_folders(&from, "code", Ok(FolderSuggestions {
            folders: vec![SuggestedFolder { path: "/home/me/code".into(), shown: "~/code".to_owned() }],
            more: false,
        }));
        shell.add_flow_key(CommandKey::Enter, window, cx);
        assert!(shell.areas.workspace.add_flow.open.as_ref().unwrap().flow.page().submitting);
        assert!(
            matches!(scene.request(shell.fixture_link()).unwrap(), Call::AddProject(add) if add.path == std::path::Path::new("/home/me/code"))
        );
        let asked = scene
            .transcript(shell.fixture_link())
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::SuggestFolders(_)))
            .count();
        assert_eq!(asked, 2, "the empty field, then only the latest key");
        scene.close(shell.fixture_link()).unwrap();
    });
}

fn search_loading(shell: &Shell, open: &Open) -> bool {
    render::loading(shell, open)
}

#[gpui::test]
fn an_older_machine_is_not_asked_and_cannot_make_a_directory(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let older =
            HOST_FOLDERS_PROTOCOL.map(|needs| ProtocolVersion::new(needs.get().saturating_sub(1)));
        let _name = machine(shell, &scene, older);
        shell.open_add_flow(None, window, cx);
        let rows = render::rows_of(shell);
        let directory = rows
            .iter()
            .find(|row| row.action == RowAction::Method(Method::NewDirectory));
        assert!(directory.is_some_and(|row| row.disabled));
        assert!(
            !rows
                .iter()
                .any(|row| row.action == RowAction::Method(Method::Browse))
        );
        shell.add_flow_choose(RowAction::Method(Method::DirectorySearch), window, cx);
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

#[gpui::test]
fn a_refusal_stays_on_its_page_and_a_success_moves_on(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let name = machine(shell, &scene, PROJECT_DIRECTORY_PROTOCOL);
        shell.open_add_flow(Some(name.clone()), window, cx);
        shell.add_flow_choose(RowAction::Method(Method::NewDirectory), window, cx);
        shell.add_flow_choose(RowAction::Parent("~/code".to_owned()), window, cx);
        assert_eq!(page_id(shell), Some(PageId::NewDirectoryName));
        // Paseo's check before asking.
        shell.add_flow_key(CommandKey::Enter, window, cx);
        let error = |shell: &Shell| {
            shell
                .areas
                .workspace
                .add_flow
                .open
                .as_ref()
                .and_then(|open| open.flow.page().error.clone())
        };
        assert_eq!(error(shell).as_deref(), Some("Enter a directory name"));
        shell
            .areas
            .workspace
            .add_flow
            .field
            .set_text("sketch", window, cx);
        shell.add_flow_typed("sketch".to_owned(), cx);
        shell.add_flow_key(CommandKey::Enter, window, cx);
        assert!(
            shell
                .areas
                .workspace
                .add_flow
                .open
                .as_ref()
                .unwrap()
                .flow
                .page()
                .submitting
        );
        shell.took_project(&name, Form::Directory, Err(Problem::Unsent));
        assert!(error(shell).is_some());
        assert_eq!(page_id(shell), Some(PageId::NewDirectoryName));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_language_change_keeps_the_field(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    crate::shell::language_tests::preserves(
        cx,
        |shell, window, cx| {
            shell.open_add_flow(None, window, cx);
        },
        |shell| &shell.areas.workspace.add_flow.field,
        |shell| assert!(shell.areas.workspace.add_flow.is_open()),
    );
}

#[gpui::test]
fn github_repositories_are_searched_kept_with_their_text_and_typed_rows_stay(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::workspace::add_flow::repositories::{REPOSITORY_ROWS, SearchError};
    use vessel_app_view::workspace::add_flow::rows::RowNote;
    use vessel_proto::forge::{
        ForgeAccess, GhState, REPOSITORY_SEARCH_PROTOCOL, Repository, RepositoryList, Visibility,
    };
    let (shell, cx, mut scene) = setup(cx);
    let name = shell.update_in(cx, |shell, window, cx| {
        let name = machine(shell, &scene, REPOSITORY_SEARCH_PROTOCOL);
        shell.open_add_flow(None, window, cx);
        shell.add_flow_choose(RowAction::Method(Method::Github), window, cx);
        assert_eq!(page_id(shell), Some(PageId::GithubSearch));
        name
    });
    // The empty field lists the account's repositories, 30 of them.
    shell.update_in(cx, |shell, _window, cx| {
        let first = scene.request(shell.fixture_link());
        assert!(
            matches!(&first, Ok(Call::SearchRepositories(ask)) if ask.query.as_str().is_empty() && ask.limit.get() == REPOSITORY_ROWS),
            "{first:?}"
        );
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert!(repositories::shown(open).loading, "asked, not answered");
        let found = RepositoryList {
            access: ForgeAccess::Ready { login: None },
            repositories: vec![Repository {
                path: "acme/widgets".parse().unwrap(),
                description: Some("The widgets".parse().unwrap()),
                visibility: Visibility::Private,
                updated_at: chrono::DateTime::UNIX_EPOCH,
                clone_url: "git@github.com:acme/widgets.git".parse().unwrap(),
            }],
        };
        scene
            .reply(shell.fixture_link(), Outcome::Ok(Reply::SearchRepositories(found)))
            .unwrap();
        let (from, answer) = workspace_answer(&mut scene, shell);
        assert_eq!(from, name);
        assert!(shell.workspace_answer(&from, answer, cx));
        let rows = render::rows_of(shell);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "repository:acme/widgets");
        assert_eq!(rows[0].note, Some(RowNote::Text("The widgets".to_owned())));
    });
    // Typed text searches again; under a signed-out answer the typed rows
    // stay and the page says why (Paseo's rule).
    type_keys(&shell, cx, &["acme/gad", "acme/gadgets"]);
    shell.update_in(cx, |shell, _window, cx| {
        let asked = scene.request(shell.fixture_link());
        assert!(
            matches!(&asked, Ok(Call::SearchRepositories(ask)) if ask.query.as_str() == "acme/gadgets"),
            "{asked:?}"
        );
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert!(repositories::shown(open).loading);
        assert_eq!(render::rows_of(shell).len(), 2, "typed rows while it searches");
        let refused = RepositoryList {
            access: ForgeAccess::SignedOut {
                host: "github.com".parse().unwrap(),
                gh: GhState::Missing,
            },
            repositories: Vec::new(),
        };
        scene
            .reply(shell.fixture_link(), Outcome::Ok(Reply::SearchRepositories(refused)))
            .unwrap();
        let (from, answer) = workspace_answer(&mut scene, shell);
        assert!(shell.workspace_answer(&from, answer, cx));
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert_eq!(repositories::shown(open).error, Some(SearchError::GhMissing));
        assert_eq!(render::rows_of(shell).len(), 2, "typed rows under the error");
        // Asks answer in the order they finish: an older text's late
        // answer is dropped and the current one stays (the app reviewer's
        // R3), for this machine or another.
        shell.add_flow_repositories(&from, "acme/gad", Err(Problem::Unsent));
        shell.add_flow_repositories("elsewhere", "acme/gadgets", Err(Problem::Unsent));
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert_eq!(repositories::shown(open).error, Some(SearchError::GhMissing));
        assert_eq!(render::rows_of(shell).len(), 2);
        shell.add_flow_repositories(&from, "acme/gadgets", Err(Problem::Unsent));
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert_eq!(repositories::shown(open).error, Some(SearchError::Failed));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn an_older_machine_is_not_searched(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let older = vessel_proto::forge::REPOSITORY_SEARCH_PROTOCOL
            .map(|needs| ProtocolVersion::new(needs.get().saturating_sub(1)));
        let _name = machine(shell, &scene, older);
        shell.open_add_flow(None, window, cx);
        shell.add_flow_choose(RowAction::Method(Method::Github), window, cx);
        assert_eq!(page_id(shell), Some(PageId::GithubSearch));
    });
    type_keys(&shell, cx, &["acme/widgets"]);
    shell.update_in(cx, |shell, _window, _cx| {
        let open = shell.areas.workspace.add_flow.open.as_ref().unwrap();
        assert_eq!(
            repositories::shown(open),
            vessel_app_view::workspace::add_flow::repositories::Searched::default(),
            "not asked: no loading, no error"
        );
        assert_eq!(render::rows_of(shell).len(), 2, "the typed rows only");
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, Call::SearchRepositories(_)))
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}
