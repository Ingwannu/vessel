//! Real pane admission, bounded file requests and cache release counterfactuals.
use super::*;
use crate::areas::git::panel::target_tests;
use crate::shell::language_tests;
use gpui::prelude::*;
use gpui::{Entity, Modifiers, Render, TestAppContext, px, size};
use tokio::sync::mpsc;
use vessel_app_link::git_panel::{Answer, Outcome};
use vessel_app_link::{Command, Link};
use vessel_app_view::workspace::git::panel::FileTicket;
use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffSummary, FileChange, FileDiff, SessionWorkspace, WorkspaceRoot,
};
use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec};

fn prepare(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
    count: usize,
    inbox: &mut mpsc::Receiver<Command>,
) -> (PanelAddress, Vec<RelPath>) {
    let (old, _) = target_tests::prepare(shell, window, cx);
    let Command::SavePrefs(prefs) = inbox.try_recv().unwrap() else {
        panic!("one initial layout persistence");
    };
    assert_eq!(*prefs, shell.prefs);
    assert!(inbox.try_recv().is_err());
    let paths = (0..count)
        .map(|at| format!("source/{at}.rs").parse().unwrap())
        .collect::<Vec<_>>();
    let git = &mut shell.areas.git;
    let entry = git.panels.entries.get_mut(&old.address).unwrap();
    let location_key = git.keys.issue().unwrap();
    entry.instance = Some(location_key);
    let location = entry
        .binding
        .begin(old.address.clone(), old.context.clone(), location_key)
        .unwrap();
    let mut session = SessionInfo::new(
        location.session(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        "2026-01-01T00:00:00Z".parse().unwrap(),
    );
    let WorkspaceRoot::Project { project } = old.repository.root else {
        panic!("project fixture");
    };
    session.workspace = Some(Box::new(SessionWorkspace {
        project,
        worktree: None,
        branch: None,
    }));
    assert!(entry.binding.finish(&location, Ok(Box::new(session))));
    let reads = entry.reads.as_mut().unwrap();
    let ticket = reads.begin_summary(git.keys.issue().unwrap());
    assert!(
        reads.finish_summary(
            &ticket,
            Ok(DiffSummary {
                from: None,
                more: false,
                files: paths
                    .iter()
                    .map(|path| FileChange {
                        path: path.clone(),
                        old_path: None,
                        status: ChangeStatus::Modified,
                        additions: Some(1),
                        deletions: Some(0),
                        binary: false
                    })
                    .collect()
            })
        )
    );
    entry.files.reset(paths.len());
    (old.address, paths)
}
fn captured(shell: &Shell, address: &PanelAddress, path: &RelPath) -> FileActivation {
    FileActivation::capture(
        &shell.areas.git.panels.entries[address],
        address,
        path,
        FileDemand::Visible,
    )
    .unwrap()
}
fn next_file(inbox: &mut mpsc::Receiver<Command>, expected: &FileActivation) -> FileTicket {
    let Command::GitPanel(Ask::File {
        address,
        context,
        ticket,
    }) = inbox.try_recv().unwrap()
    else {
        panic!("one file call");
    };
    assert_eq!(address, expected.address.panel);
    assert_eq!(context, expected.context);
    assert_eq!(ticket.path(), &expected.address.path);
    assert!(inbox.try_recv().is_err());
    ticket
}
fn accept(
    shell: &mut Shell,
    captured: &FileActivation,
    ticket: FileTicket,
    cx: &mut Context<'_, Shell>,
) {
    shell.git_panel_answer(
        Answer {
            address: captured.address.panel.clone(),
            context: captured.context.clone(),
            outcome: Outcome::File(
                ticket.clone(),
                Ok(FileDiff {
                    path: ticket.path().clone(),
                    old_path: None,
                    status: ChangeStatus::Modified,
                    mode: None,
                    content: DiffContent::Unchanged,
                    more_from: None,
                }),
            ),
        },
        cx,
    );
}

#[gpui::test]
fn visible_file_activation_retains_the_original_admission_and_deduplicates_before_keys(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(64).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (address, paths) = prepare(shell, window, cx, 2, &mut inbox);
        let first = captured(shell, &address, &paths[0]);
        shell.git_panel_file(&first, window, cx);
        let ticket = next_file(&mut inbox, &first);
        shell.git_panel_file(&first, window, cx);
        assert!(inbox.try_recv().is_err());
        accept(shell, &first, ticket.clone(), cx);
        shell.git_panel_file(&first, window, cx);
        assert!(inbox.try_recv().is_err());
        assert_eq!(
            shell.areas.git.panels.entries[&address]
                .reads
                .as_ref()
                .unwrap()
                .file(&paths[0])
                .unwrap()
                .ready()
                .unwrap()
                .ticket(),
            &ticket
        );
        // The prior accepted snapshot cannot start reads after epoch retirement.
        let other = captured(shell, &address, &paths[1]);
        shell.fleet.machines[0].connection_epoch = None;
        shell.git_panel_file(&other, window, cx);
        assert!(inbox.try_recv().is_err());
    });
}

#[gpui::test]
fn pending_body_admission_is_bounded_and_does_not_evict_a_waiting_request(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(64).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (address, paths) = prepare(
            shell,
            window,
            cx,
            MAX_PENDING_BODIES.saturating_add(1),
            &mut inbox,
        );
        let mut calls = Vec::new();
        for path in paths.iter().take(MAX_PENDING_BODIES) {
            let captured = captured(shell, &address, path);
            shell.git_panel_file(&captured, window, cx);
            calls.push((captured.clone(), next_file(&mut inbox, &captured)));
        }
        let waiting = captured(shell, &address, paths.last().unwrap());
        shell.git_panel_file(&waiting, window, cx);
        assert!(inbox.try_recv().is_err());
        assert_eq!(shell.areas.git.panels.pending_bodies(), MAX_PENDING_BODIES);
        let (finished, ticket) = calls.remove(0);
        accept(shell, &finished, ticket, cx);
        shell.git_panel_file(&waiting, window, cx);
        let _next = next_file(&mut inbox, &waiting);
        for (captured, ticket) in calls {
            accept(shell, &captured, ticket.clone(), cx);
            assert_eq!(
                shell.areas.git.panels.entries[&address]
                    .reads
                    .as_ref()
                    .unwrap()
                    .file(&captured.address.path)
                    .unwrap()
                    .ready()
                    .unwrap()
                    .ticket(),
                &ticket
            );
        }
    });
}

#[gpui::test]
fn global_body_slots_release_old_payloads_and_reject_their_late_reply(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(64).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (address, paths) = prepare(
            shell,
            window,
            cx,
            MAX_BODY_FILES.saturating_add(1),
            &mut inbox,
        );
        let mut first = None;
        for path in &paths {
            let captured = captured(shell, &address, path);
            shell.git_panel_file(&captured, window, cx);
            let ticket = next_file(&mut inbox, &captured);
            if first.is_none() {
                first = Some((captured.clone(), ticket.clone()));
            }
            accept(shell, &captured, ticket, cx);
            assert!(shell.areas.git.panels.body_order.len() <= MAX_BODY_FILES);
        }
        let (first, old) = first.unwrap();
        assert!(
            shell.areas.git.panels.entries[&address]
                .reads
                .as_ref()
                .unwrap()
                .file(&first.address.path)
                .is_none()
        );
        accept(shell, &first, old, cx);
        assert!(
            shell.areas.git.panels.entries[&address]
                .reads
                .as_ref()
                .unwrap()
                .file(&first.address.path)
                .is_none()
        );
        assert_eq!(shell.areas.git.panels.body_order.len(), MAX_BODY_FILES);
    });
}

struct Mounted {
    shell: Entity<Shell>,
    address: PanelAddress,
}
impl Render for Mounted {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let address = self.address.clone();
        // This actual test root is the whole panel. Production captures the
        // measured list width outside its own row processor instead.
        let width = window.viewport_size().width;
        self.shell.update(cx, |shell, cx| {
            let list = shell.areas.git.panels.entries[&address].files.clone();
            list.element(cx.processor(move |shell: &mut Shell, index, window, cx| {
                gpui::div()
                    .w_full()
                    .debug_selector(|| "git-mounted-body".to_owned())
                    .child(crate::areas::git::panel::file::row(
                        shell,
                        &address,
                        index,
                        width,
                        &vessel_app_kit::tokens(cx),
                        window,
                        cx,
                    ))
                    .into_any_element()
            }))
        })
    }
}

fn changed_file(path: RelPath) -> FileDiff {
    FileDiff {
        path,
        old_path: None,
        status: ChangeStatus::Modified,
        mode: None,
        more_from: None,
        content: DiffContent::Hunks {
            hunks: vec![vessel_proto::workspace::Hunk {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 1,
                section: None,
                lines: vec![
                    vessel_proto::workspace::DiffLine {
                        kind: vessel_proto::workspace::LineKind::Removed,
                        text: "old value".to_owned(),
                        cut: false,
                    },
                    vessel_proto::workspace::DiffLine {
                        kind: vessel_proto::workspace::LineKind::Added,
                        text: "새 값".to_owned(),
                        cut: false,
                    },
                ],
            }],
        },
    }
}

#[gpui::test]
fn the_actual_mounted_unified_body_copies_original_rows_and_refuses_typing(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(64).unwrap();
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let mut address = None;
        let shell = cx.new(|cx| {
            let mut shell = Shell::with_test_link(window, cx, link);
            let (panel, paths) = prepare(&mut shell, window, cx, 1, &mut inbox);
            let captured = captured(&shell, &panel, &paths[0]);
            shell.git_panel_file(&captured, window, cx);
            let ticket = next_file(&mut inbox, &captured);
            shell.git_panel_answer(
                Answer {
                    address: panel.clone(),
                    context: captured.context.clone(),
                    outcome: Outcome::File(ticket, Ok(changed_file(paths[0].clone()))),
                },
                cx,
            );
            address = Some(panel);
            shell
        });
        Mounted {
            shell,
            address: address.unwrap(),
        }
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds("git-mounted-body")
        .expect("real file section mounted");
    assert!(bounds.size.height > px(40.));
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |view, cx| {
        let entry = &view.shell.read(cx).areas.git.panels.entries[&view.address];
        assert_eq!(entry.documents.len(), 1);
    });
    cx.update(|window, cx| {
        let view = view.read(cx);
        let entry = &view.shell.read(cx).areas.git.panels.entries[&view.address];
        assert!(
            entry
                .documents
                .values()
                .any(|document| document.selection_focused(window, cx))
        );
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()));
    });
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    });
    cx.simulate_input("blocked");
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-c"
    } else {
        "ctrl-c"
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some("@@ -1,1 +1,1 @@\nold value\n새 값".to_owned())
        );
    });
    view.update_in(cx, |view, _window, cx| {
        view.shell.update(cx, |shell, _cx| {
            let tag = shell.areas.git.panels.entries[&view.address]
                .context
                .as_ref()
                .unwrap()
                .tag();
            assert!(shell.git_panels_retired(&tag));
            let entry = &shell.areas.git.panels.entries[&view.address];
            assert!(entry.documents.is_empty());
            assert!(
                entry
                    .reads
                    .as_ref()
                    .unwrap()
                    .file(&"source/0.rs".parse().unwrap())
                    .is_none()
            );
        });
    });
}
