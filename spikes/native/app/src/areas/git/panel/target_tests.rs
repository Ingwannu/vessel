//! The real layout can retarget a live tab before a retained git callback runs.
use super::comparison::ComparisonChange;
use super::*;
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, Link};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{AgentRole, HostAlias, LayoutKey, OpenIntent, Placement, Target};
use vessel_app_view::workspace::git::panel::{RepositoryKey, StatusTicket};
use vessel_proto::{NodeId, ProjectId, SessionId};

fn install_context(shell: &mut Shell) -> ConnectionContext {
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

/// Actual owner fixtures use the product initializer before reading pane data.
pub(super) fn prepare(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (ComparisonChange, StatusTicket) {
    assert!(crate::areas::git::start(shell, cx).is_ok());
    prepare_uninitialized(shell, window, cx)
}

/// The negative startup fixture deliberately keeps an unregistered old view.
pub(super) fn prepare_uninitialized(
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
    let instance = shell.areas.git.keys.issue().unwrap();
    let (reads, summary, ticket) =
        accepted_reads(repository, comparison.clone(), &mut shell.areas.git.keys);
    let mut entry = Entry::new(PanelTarget::WorkingDiff, Some(context.clone()));
    entry.instance = Some(instance);
    entry.reads = Some(reads);
    super::scroll::listen(&entry, &address, cx);
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
            summary,
        },
        ticket,
    )
}

pub(super) fn background_agent(
    shell: &mut Shell,
    address: &PanelAddress,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
    inbox: &mut tokio::sync::mpsc::Receiver<Command>,
) {
    let LayoutKey::Session { session, .. } = address.workspace else {
        panic!("legacy session fixture");
    };
    let _tab = shell
        .layout_open_target(
            &address.workspace,
            Target::Agent(session),
            OpenIntent::Background,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    let Command::SavePrefs(prefs) = inbox.try_recv().unwrap() else {
        panic!("one background-agent layout persistence");
    };
    assert_eq!(*prefs, shell.prefs);
    assert!(matches!(
        inbox.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

/// A drawn empty-state callback must have actual accepted data, not an Empty store.
pub(super) fn accepted_reads(
    repository: RepositoryKey,
    comparison: Comparison,
    keys: &mut vessel_app_view::workspace::git::RequestKeys,
) -> (
    PanelReads,
    vessel_app_view::workspace::git::panel::ComparisonTicket,
    StatusTicket,
) {
    let mut reads = PanelReads::new(repository, comparison);
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(
        status,
        Ok(vessel_proto::workspace::GitStatus {
            branch: Some("fixture".parse().unwrap()),
            head: None,
            upstream: None,
            base: None,
            operation: None,
            files: Vec::new(),
            more: false,
        })
    ));
    let summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(
        &summary,
        Ok(vessel_proto::workspace::DiffSummary {
            from: None,
            files: Vec::new(),
            more: false,
        })
    ));
    (reads, summary, status)
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
            &shell.areas.git.panels.entries[address],
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
        assert!(
            !shell.areas.git.panels.entries.contains_key(address),
            "the real owner retires before the transaction returns, not on its late answer"
        );
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

#[gpui::test]
fn a_retained_mode_callback_cannot_reenter_after_identical_summary_round_trip(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = prepare(shell, window, cx);
        while inbox.try_recv().is_ok() {}
        let changed = {
            let git = &mut shell.areas.git;
            let reads = git
                .panels
                .entries
                .get_mut(&old.address)
                .unwrap()
                .reads
                .as_mut()
                .unwrap();
            reads.select(Comparison::new(
                ComparisonMode::Uncommitted,
                None,
                Whitespace::Show,
            ));
            reads.select(old.comparison.clone());
            let changed = reads.begin_summary(git.keys.issue().unwrap());
            assert!(reads.finish_summary(
                &changed,
                Ok(vessel_proto::workspace::DiffSummary {
                    from: None,
                    files: Vec::new(),
                    more: false,
                })
            ));
            changed
        };
        assert_ne!(old.summary, changed);
        assert_eq!(
            shell.areas.git.panels.entries[&old.address].instance,
            Some(old.instance)
        );
        shell.git_panel_compare(&old, ComparisonMode::Committed, window, cx);
        assert!(matches!(
            inbox.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        assert_eq!(
            shell.areas.git.panels.entries[&old.address]
                .reads
                .as_ref()
                .unwrap()
                .comparison(),
            &old.comparison
        );
        let current = ComparisonChange {
            summary: changed,
            ..old
        };
        shell.git_panel_compare(&current, ComparisonMode::Committed, window, cx);
        let Command::GitPanel(vessel_app_link::git_panel::Ask::Summary {
            address,
            context,
            ticket,
        }) = inbox.try_recv().unwrap()
        else {
            panic!("a fresh callback sends one original-context comparison");
        };
        assert_eq!(address, current.address);
        assert_eq!(context, current.context);
        assert_eq!(ticket.repository(), current.repository);
        assert_eq!(ticket.comparison().mode(), Some(ComparisonMode::Committed));
        assert!(matches!(
            inbox.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    });
}
