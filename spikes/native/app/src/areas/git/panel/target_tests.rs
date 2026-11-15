//! The real layout can retarget a live tab before a retained git callback runs.
use super::comparison::ComparisonChange;
use super::*;
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, TestAppContext};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, Link};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{AgentRole, HostAlias, LayoutKey, OpenIntent, Placement, Target};
use vessel_app_view::workspace::git::panel::{RepositoryKey, StatusTicket};
use vessel_app_view::workspace::git::{ReadState, RequestKeys};
use vessel_proto::{NodeId, ProjectId, SessionId};

fn install_context(shell: &mut Shell) -> vessel_app_view::connection::ConnectionContext {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut machine = vessel_app_view::Machine::unreachable("fixture".into(), "sessions refused");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    context
}

fn prepare(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (ComparisonChange, StatusTicket) {
    let session = SessionId::random();
    shell.route = crate::routes::Route::Session;
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: "fixture".into(),
        session,
        title: "git target".into(),
        agent: "fake".into(),
        structured: true,
        live: true,
    });
    let context = install_context(shell);
    let workspace = LayoutKey::Session {
        machine: HostAlias::parse("fixture").unwrap(),
        session,
    };
    let tab = shell
        .layout_open_target(
            &workspace,
            Target::WorkingDiff,
            OpenIntent::Reveal,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    let address = PanelAddress { workspace, tab };
    let repository = RepositoryKey {
        node: context.node(),
        root: vessel_proto::workspace::WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    };
    let comparison = Comparison::new(ComparisonMode::Uncommitted, None, Whitespace::Show);
    let mut keys = RequestKeys::default();
    let instance = keys.issue().unwrap();
    let mut reads = PanelReads::new(repository, comparison.clone());
    let ticket = reads.begin_status(keys.issue().unwrap());
    let mut entry = Entry::new(PanelTarget::WorkingDiff, Some(context.clone()));
    entry.instance = Some(instance);
    entry.reads = Some(reads);
    let _old = shell
        .areas
        .git
        .panels
        .entries
        .insert(address.clone(), entry);
    (
        ComparisonChange {
            address,
            target: PanelTarget::WorkingDiff,
            instance,
            context,
            repository,
            comparison,
        },
        ticket,
    )
}

#[gpui::test]
fn retargeted_visible_tab_drops_its_old_read_and_cannot_start_an_empty_mode_call(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (captured, ticket) = prepare(shell, window, cx);
        let address = &captured.address;
        assert!(shell.layout_panel_visible(&address.workspace, address.tab, window));
        assert!(captured.is_current(
            shell.areas.git.panels.entries.get(address).unwrap(),
            Some(&captured.context)
        ));
        assert_eq!(
            shell.git_panel_context(address),
            Some(captured.context.clone())
        );
        let retargeted = shell
            .update_layout(
                &address.workspace,
                |layout| {
                    layout.retarget(vessel_app_view::layout::RetargetRequest {
                        tab: address.tab,
                        target: Target::Scripts,
                        intent: OpenIntent::Reveal,
                        role: AgentRole::Unknown,
                    })
                },
                cx,
            )
            .unwrap();
        assert_eq!(retargeted, address.tab);
        assert!(shell.layout_panel_visible(&address.workspace, address.tab, window));
        assert!(!shell.layout_tab_matches(&address.workspace, address.tab, &Target::WorkingDiff));
        while let Ok(command) = inbox.try_recv() {
            assert!(
                !matches!(command, Command::GitPanel(_)),
                "setup starts no git call"
            );
        }
        shell.git_panel_compare(&captured, ComparisonMode::Committed, window, cx);
        assert!(matches!(
            inbox.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            shell
                .areas
                .git
                .panels
                .entries
                .get(address)
                .unwrap()
                .reads
                .as_ref()
                .unwrap()
                .summary(),
            ReadState::Empty
        ));
        shell.git_panel_answer(
            vessel_app_link::git_panel::Answer {
                address: address.clone(),
                context: captured.context,
                outcome: vessel_app_link::git_panel::Outcome::Status(
                    ticket,
                    Err(ReadFailure::TimedOut),
                ),
            },
            cx,
        );
        assert!(!shell.areas.git.panels.entries.contains_key(address));
    });
}
