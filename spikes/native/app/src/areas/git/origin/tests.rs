//! The unscoped legacy request never reaches even a detached actor queue.
use super::*;
use crate::areas::git::Target;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_link::Link;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_proto::workspace::{Pull, Push, WorkspaceRoot};
use vessel_proto::{NodeId, ProjectId};

fn followed_alias_with_current_context(shell: &mut Shell) -> WorkspaceRoot {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut machine = vessel_app_view::Machine::unreachable("fixture".into(), "sessions refused");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    assert_eq!(machine.connection_context(), Some(context));
    shell.fleet.machines = vec![machine];
    let root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    shell.areas.git.target = Some(Target {
        machine: "fixture".to_owned(),
        root,
    });
    root
}
fn push(root: WorkspaceRoot) -> GitWrite {
    GitWrite::Push(Push {
        root,
        remote: None,
        set_upstream: false,
    })
}

#[gpui::test]
fn current_alias_context_does_not_upgrade_an_unstamped_view_to_a_write(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, cx| {
        let root = followed_alias_with_current_context(shell);
        while inbox.try_recv().is_ok() {}
        let mut expected = RequestKeys::default();
        assert_eq!(
            shell.areas.git.keys.issue().unwrap(),
            expected.issue().unwrap()
        );
        shell.git_write(push(root), WriteIntent::Quick(GitAction::Push), cx);
        assert_eq!(
            shell.areas.git.keys.issue().unwrap(),
            expected.issue().unwrap()
        );
        assert!(matches!(
            inbox.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        assert!(!shell.areas.git.writing.busy());
        assert!(shell.areas.git.pending.is_none());
        let toast = shell.areas.toasts.list().last().unwrap();
        assert_eq!(toast.tone, ToastTone::Info);
        assert_eq!(
            toast.text,
            "The repository shown here cannot be verified. Reopen the workspace before changing it."
        );
    });
}

#[gpui::test]
fn refusing_a_new_unscoped_request_retains_the_existing_payload_and_intention(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, cx| {
        let root = followed_alias_with_current_context(shell);
        while inbox.try_recv().is_ok() {}
        let key = shell.areas.git.keys.issue().unwrap();
        let pending = push(root);
        let intent = WriteIntent::Quick(GitAction::Push);
        assert!(shell.areas.git.writing.begin_action(key, intent.clone()));
        shell.areas.git.pending = Some((key, pending.clone()));
        shell.git_write(
            GitWrite::Pull(Pull { root }),
            WriteIntent::Quick(GitAction::Pull),
            cx,
        );
        assert_eq!(shell.areas.git.pending, Some((key, pending)));
        assert!(shell.areas.git.writing.busy());
        assert_eq!(shell.areas.git.writing.intent(), Some(&intent));
        assert!(matches!(
            inbox.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    });
}
