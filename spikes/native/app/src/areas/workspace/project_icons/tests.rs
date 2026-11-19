//! Projects' icons through the real link (`Scenario`): each listed project
//! asked once on a machine speaking the protocol, two asks out at a time,
//! the answer kept as an image or as none, a refusal asked again with the
//! next list; an older machine is not asked.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::Event;
use vessel_app_link::testing::Scenario;
use vessel_app_link::workspace::Answer;
use vessel_app_view::machine_book::Name;
use vessel_proto::workspace::host_folders::{HOST_FOLDERS_PROTOCOL, IconBytes, ProjectIcon};
use vessel_proto::workspace::{GitSupport, GitVersion, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{Call, Outcome, ProtocolVersion, Reply};

fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut gpui::VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}

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

fn list() -> ProjectList {
    let at = "2026-11-18T08:00:00Z".parse().unwrap();
    ProjectList {
        git: GitSupport::Found {
            version: GitVersion::MINIMUM,
        },
        more: false,
        projects: (1_u8..=5)
            .map(|index| ProjectInfo {
                id: format!("{index:08x}-0000-4000-8000-000000000000")
                    .parse()
                    .unwrap(),
                name: format!("project-{index}").parse().unwrap(),
                root: format!("/fixture/project-{index}").into(),
                git: true,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: at,
                revision: 1,
            })
            .collect(),
    }
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

#[gpui::test]
fn each_listed_project_is_asked_once_and_kept_as_an_image_or_none(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _window, cx| {
        let name = machine(shell, &scene, HOST_FOLDERS_PROTOCOL);
        let projects = list();
        let ids: Vec<ProjectId> = projects.projects.iter().map(|project| project.id).collect();
        assert!(shell.workspace_answer(&name, Answer::Projects(Ok(projects.clone())), cx));
        let svg = IconImage {
            kind: IconType::Svg,
            bytes: IconBytes::new(b"<svg viewBox='0 0 1 1'/>".to_vec().into()).unwrap(),
        };
        // Two asks are out for the machine, the others wait for an answer
        // (`background.rs`: the node's file work refuses beyond four at
        // once); their order on the wire is the link's.
        let background = &shell.areas.workspace.background;
        assert_eq!(
            (background.out(&name), background.waiting(&name)),
            (2, ids.len().saturating_sub(2))
        );
        let mut asked = Vec::new();
        for _ in 0..ids.len() {
            let Call::ProjectIcon(ask) = scene.request(shell.fixture_link()).unwrap() else {
                panic!("expected project.icon");
            };
            asked.push(ask.project);
            let icon = (Some(&ask.project) == ids.first()).then(|| svg.clone());
            scene
                .reply(
                    shell.fixture_link(),
                    Outcome::Ok(Reply::ProjectIcon(ProjectIcon { icon })),
                )
                .unwrap();
            let (machine, answer) = workspace_answer(&mut scene, shell);
            assert!(shell.workspace_answer(&machine, answer, cx));
        }
        asked.sort();
        let mut expected = ids.clone();
        expected.sort();
        assert_eq!(asked, expected);
        let icons = &shell.areas.workspace.icons;
        let mut listed = ids.iter().copied();
        let (first, second) = (listed.next().unwrap(), listed.next().unwrap());
        assert!(icons.icon(&name, first).is_some());
        assert!(icons.icon(&name, second).is_none());
        // Listed again: nothing asked again.
        assert!(shell.workspace_answer(&name, Answer::Projects(Ok(projects.clone())), cx));
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, Call::ProjectIcon(_)))
                .count(),
            ids.len()
        );
        // A refusal (the node's file work busy) is forgotten: the next list
        // asks it again.
        shell.project_icon_read(&name, first, Err(Problem::Gone("busy".to_owned())));
        assert!(shell.workspace_answer(&name, Answer::Projects(Ok(projects)), cx));
        assert!(
            matches!(scene.request(shell.fixture_link()).unwrap(), Call::ProjectIcon(ask) if ask.project == first)
        );
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::ProjectIcon(ProjectIcon { icon: None })),
            )
            .unwrap();
        let (machine, answer) = workspace_answer(&mut scene, shell);
        assert!(shell.workspace_answer(&machine, answer, cx));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn an_older_machine_keeps_the_initial_tiles(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _window, cx| {
        let older =
            HOST_FOLDERS_PROTOCOL.map(|needs| ProtocolVersion::new(needs.get().saturating_sub(1)));
        let name = machine(shell, &scene, older);
        assert!(shell.workspace_answer(&name, Answer::Projects(Ok(list())), cx));
        let calls = scene.transcript(shell.fixture_link()).unwrap();
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, Call::ProjectIcon(_)))
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[test]
fn the_square_takes_paseos_initial() {
    assert_eq!(initial("shop"), "S");
    assert_eq!(initial(" acme/web-app "), "W");
    assert_eq!(initial("한글-프로젝트"), "한");
    assert_eq!(initial("  "), "");
}
