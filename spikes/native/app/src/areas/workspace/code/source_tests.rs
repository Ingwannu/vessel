//! Actual framed root/summary/diff replies enter the real `CodePanel` owner.
use super::*;
use crate::areas::workspace::{ShownChangesContext, ShownDiffContext};
use gpui::TestAppContext;
use vessel_app_link::{
    Event,
    testing::Scenario,
    workspace::code_source::{Answer, Request},
};
use vessel_app_view::{connection::ConnectionContext, machine_book::Name};
use vessel_proto::{
    Call, HarnessKind, Outcome, ProjectId, Reply, SessionInfo, SessionMode, SessionSpec,
    workspace::{ChangeStatus, DiffContent, FileChange, FileDiff, SessionWorkspace},
};

fn prepare(shell: &mut Shell, context: &ConnectionContext) -> (SessionId, ProjectId) {
    shell.route = crate::routes::Route::Session;
    let session = SessionId::random();
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: context.machine().as_str().to_owned(),
        session,
        title: "source fixture".to_owned(),
        agent: "fake".to_owned(),
        structured: true,
        live: true,
        archived_at: None,
        binding: None,
    });
    let mut machine =
        vessel_app_view::Machine::unreachable(context.machine().as_str().to_owned(), "fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.areas.workspace.code.open = true;
    shell.code_follow();
    (session, ProjectId::random())
}
fn metadata(session: SessionId, project: ProjectId) -> SessionInfo {
    let mut info = SessionInfo::new(
        session,
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    info.workspace = Some(Box::new(SessionWorkspace {
        project,
        worktree: None,
        branch: None,
    }));
    info
}
fn summary() -> DiffSummary {
    DiffSummary {
        from: None,
        more: false,
        files: vec![FileChange {
            path: "a.rs".parse().unwrap(),
            old_path: None,
            status: ChangeStatus::Modified,
            additions: Some(1),
            deletions: Some(1),
            binary: false,
        }],
    }
}
fn answer(scene: &mut Scenario, shell: &Shell) -> Answer {
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::CodeSource(answer) = scene.event(shell.fixture_link()).unwrap() {
            return *answer;
        }
    }
    panic!("expected bounded code-source event");
}
fn root(scene: &mut Scenario, shell: &mut Shell, session: SessionId, project: ProjectId) {
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetSession(ask) if ask.session == session)
    );
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GetSession(metadata(session, project))),
        )
        .unwrap();
    shell.code_source_answer(answer(scene, shell));
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(ask) if ask.root == WorkspaceRoot::Project { project })
    );
}
fn ready(scene: &mut Scenario, shell: &mut Shell) {
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GitChanges(summary())),
        )
        .unwrap();
    shell.code_source_answer(answer(scene, shell));
    let shown: ShownChangesContext = shell.shown_changes_context().unwrap();
    assert_eq!(shown.connection(), &scene.context());
    assert_eq!(shown.comparison(), ScopeKind::Uncommitted);
    assert_eq!(shown.root(), *shell.areas.workspace.code.root().unwrap().1);
    assert!(shell.shown_changes_is_current(&shown));
}
fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut gpui::VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}
#[gpui::test]
fn queued_summary_cannot_install_after_original_retirement(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::GitChanges(summary())),
            )
            .unwrap();
        let held = answer(&mut scene, shell);
        assert!(held.is_current());
        scene.retire(shell.fixture_link()).unwrap();
        shell.code_source_answer(held);
        assert!(shell.shown_changes_context().is_none());
        assert!(!matches!(
            shell.areas.workspace.code.changes.summary,
            Summary::Ready(_)
        ));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn installed_summary_getter_observes_retirement_without_fleet_delivery(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let shown = shell.shown_changes_context().unwrap();
        scene.retire(shell.fixture_link()).unwrap();
        assert!(shell.shown_changes_context().is_none());
        assert!(!shell.shown_changes_is_current(&shown));
        assert!(matches!(
            shell.areas.workspace.code.changes.summary,
            Summary::Ready(_)
        ));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn refresh_and_comparison_aba_cannot_reuse_the_old_ready_stamp(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let old = shell.shown_changes_context().unwrap();
        shell.reread_changes();
        assert!(!shell.shown_changes_is_current(&old));
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(_)));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GitChanges(summary()))).unwrap();
        let held = answer(&mut scene, shell);
        shell.areas.workspace.code.changes.scope = ScopeKind::Staged;
        shell.source_summary();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(ask) if ask.scope == ScopeKind::Staged.scope()));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GitChanges(summary()))).unwrap();
        let staged = answer(&mut scene, shell);
        shell.areas.workspace.code.changes.scope = ScopeKind::Uncommitted;
        shell.source_summary();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(_)));
        shell.code_source_answer(held);
        shell.code_source_answer(staged);
        assert!(shell.shown_changes_context().is_none());
        ready(&mut scene, shell);
        assert!(!shell.shown_changes_is_current(&old));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn diff_and_legacy_answers_cannot_cross_the_parent_summary(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let path = "a.rs".parse().unwrap();
        shell.read_diff(&path, None);
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitDiff(ask) if ask.path == path));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GitDiff(FileDiff {
            path: path.clone(), old_path: None, status: ChangeStatus::Modified,
            mode: None, content: DiffContent::Unchanged, more_from: None }))).unwrap();
        let page = answer(&mut scene, shell);
        let request = page.request().clone();
        shell.code_source_answer(page);
        let shown: ShownDiffContext = shell.shown_diff_context(&path).unwrap();
        assert_eq!(shown.connection(), &scene.context());
        assert_eq!(shown.path(), &path);
        assert!(shell.shown_diff_is_current(&shown));
        shell.reread_changes();
        assert!(!shell.shown_diff_is_current(&shown));
        let Request::Diff { ask, .. } = request else { panic!("actual diff request"); };
        shell.diff_read(&ask, Ok(FileDiff { path: path.clone(), old_path: None, status: ChangeStatus::Modified,
            mode: None, content: DiffContent::Unchanged, more_from: None }));
        assert!(shell.shown_diff_context(&path).is_none());
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(_)));
        ready(&mut scene, shell);
        assert!(!shell.shown_diff_is_current(&shown));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn actual_git_retirement_delegation_invalidates_without_another_owner(cx: &mut TestAppContext) {
    use crate::areas::layout::registry::{
        PanelContext, PanelFocus,
        retirement::{Notice, Owner},
    };
    use vessel_app_view::layout::{
        AgentRole, LayoutKey, OpenIntent, PaneId, Placement, RetargetRequest, Target,
    };
    fn retired(shell: &mut Shell, notice: &Notice, _cx: &mut Context<'_, Shell>) {
        shell.code_changes_retired(notice);
    }
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let context = scene.context();
        let (session, project) = prepare(shell, &context);
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let old = shell.shown_changes_context().unwrap();
        let key = LayoutKey::Session {
            machine: vessel_app_view::layout::HostAlias::parse(context.machine().as_str()).unwrap(),
            session,
        };
        shell
            .layout_register_retirement(Owner::Git, retired)
            .unwrap();
        let tab = shell
            .layout_open_target(
                &key,
                Target::Changes,
                OpenIntent::Reveal,
                Placement::Explorer,
                window,
                cx,
            )
            .unwrap();
        let panel = PanelContext {
            workspace: &key,
            pane: PaneId::EXPLORER,
            tab,
            target: &Target::Changes,
            focus: PanelFocus::Focused,
        };
        assert!(
            shell.shown_changes_at(&panel, window).is_none(),
            "unmounted Explorer is unavailable"
        );
        shell
            .update_layout(
                &key,
                |layout| {
                    layout
                        .retarget(RetargetRequest {
                            tab,
                            target: Target::Files,
                            intent: OpenIntent::Reveal,
                            role: AgentRole::Unknown,
                        })
                        .map(|_tab| ())
                },
                cx,
            )
            .unwrap();
        assert!(shell.shown_changes_context().is_none());
        assert!(!shell.shown_changes_is_current(&old));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn framed_wrong_session_reply_never_installs_a_physical_root(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (_session, project) = prepare(shell, &scene.context());
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GetSession(_)
        ));
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::GetSession(metadata(SessionId::random(), project))),
            )
            .unwrap();
        shell.code_source_answer(answer(&mut scene, shell));
        assert!(shell.areas.workspace.code.root().is_none());
        assert!(shell.shown_changes_context().is_none());
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn same_alias_same_node_reconnect_does_not_restamp_the_old_summary(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let old = shell.shown_changes_context().unwrap();
        shell.reread_changes();
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GitChanges(_)
        ));
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::GitChanges(summary())),
            )
            .unwrap();
        let held = answer(&mut scene, shell);
        let replacement = scene
            .reconnect(
                shell.fixture_link(),
                vessel_app_link::testing::Reconnect::SameNode,
            )
            .unwrap();
        assert_eq!(replacement.node(), original.node());
        assert_ne!(replacement.epoch(), original.epoch());
        let (session, project) = prepare(shell, &replacement);
        shell.code_source_answer(held);
        assert!(shell.areas.workspace.code.root().is_none());
        assert!(shell.shown_changes_context().is_none());
        root(&mut scene, shell, session, project);
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::GitChanges(summary())),
            )
            .unwrap();
        shell.code_source_answer(answer(&mut scene, shell));
        let current = shell.shown_changes_context().unwrap();
        assert_eq!(current.connection(), &replacement);
        assert!(!shell.shown_changes_is_current(&old));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn readonly_root_refresh_preserves_a_legacy_viewer_without_starting_a_flush(
    cx: &mut TestAppContext,
) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let mut edit = editing::Editing::new();
        let first = edit.model.first_read();
        let _step = edit.model.read(
            first,
            vessel_app_view::workspace::editor::Read::Text(
                String::new(),
                vessel_proto::workspace::FileVersion {
                    size: 0,
                    modified: 0,
                    hash: Some("a".repeat(64).parse().unwrap()),
                },
            ),
        );
        let _step = edit.model.edit("retained draft");
        assert!(edit.model.modified());
        shell.areas.workspace.code.files.viewer = Some(Viewer {
            path: "a.rs".parse().unwrap(),
            text: Shown::Loading,
            list: VirtualList::new(false),
            goto: None,
            chosen: None,
            edit: Some(edit),
            watch: vessel_app_view::workspace::watch::Watch::default(),
        });
        let replacement = scene
            .reconnect(
                shell.fixture_link(),
                vessel_app_link::testing::Reconnect::SameNode,
            )
            .unwrap();
        shell.fleet.machines[0].connection_epoch = Some(replacement.epoch());
        shell.code_follow();
        let panel = &shell.areas.workspace.code;
        let editing = panel.files.viewer.as_ref().unwrap().edit.as_ref().unwrap();
        assert_eq!(editing.model.content(), "retained draft");
        assert!(editing.model.writing().is_none());
        assert_eq!(
            panel
                .target
                .as_ref()
                .unwrap()
                .receipt
                .as_ref()
                .unwrap()
                .context,
            original
        );
        assert!(shell.shown_changes_context().is_none());
        assert!(
            scene
                .transcript(shell.fixture_link())
                .unwrap()
                .iter()
                .all(|call| !matches!(call, Call::WriteFile(_)))
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_change_the_watch_sees_reads_the_shown_file_again(cx: &mut TestAppContext) {
    use vessel_app_view::workspace::editor::Read;
    use vessel_app_view::workspace::watch::Watch;
    use vessel_proto::workspace::FileVersion;
    use vessel_proto::workspace::panel_files::{FileRevision, FileSize, ObservedFile};

    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, cx| {
        let original = scene.context();
        let (session, project) = prepare(shell, &original);
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let root = WorkspaceRoot::Project { project };
        let path: RelPath = "a.rs".parse().unwrap();
        let mut edit = editing::Editing::new();
        let first = edit.model.first_read();
        let version = FileVersion {
            size: 3,
            modified: 0,
            hash: Some("a".repeat(64).parse().unwrap()),
        };
        let _step = edit.model.read(first, Read::Text("old".to_owned(), version));
        let mut watch = Watch::default();
        // The file's read landed (the editor's), so the watch may ask.
        watch.seen(None);
        let ask = watch
            .ask(root, path.clone(), std::time::Instant::now())
            .unwrap();
        shell.areas.workspace.code.files.viewer = Some(Viewer {
            path: path.clone(),
            text: Shown::Loading,
            list: VirtualList::new(false),
            goto: None,
            chosen: None,
            edit: Some(edit),
            watch,
        });
        let changed = ObservedFile::Ready {
            revision: FileRevision {
                size: FileSize(3),
                hash: "b".repeat(64).parse().unwrap(),
            },
        };
        shell.file_observed(&ask, Ok(changed), cx);
        assert!(
            matches!(scene.request(shell.fixture_link()).unwrap(), Call::ReadFile(read) if read.root == root && read.path == path)
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}
