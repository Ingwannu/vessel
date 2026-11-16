//! A mutation acknowledgement cannot bless a replacement node at the same alias.
use super::*;
mod marks;
use gpui::TestAppContext;
use vessel_app_link::sessions::Binding;
use vessel_app_view::{Card, Machine, Reach, sessions::SessionAddress};
use vessel_proto::{HarnessKind, NodeId, SessionId, SessionMode, SessionSpec, SessionStatus};

fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}

fn pending_exit(shell: &mut Shell) -> (SessionInfo, Name, NodeId, LifecycleTicket) {
    let info = info();
    let card = Card::from_session("fixture", &info);
    let admitted = NodeId::random();
    let replacement = NodeId::random();
    let host = Name::parse("fixture").unwrap();
    let target = LifecycleTarget {
        host: host.clone(),
        session: info.id,
        node: Some(admitted),
    };
    let mut open = crate::areas::session::Open::of(&card);
    open.binding = Some(Binding {
        host: host.clone(),
        address: SessionAddress {
            node: admitted,
            session: info.id,
        },
    });
    shell.route = Route::Session;
    shell.areas.session.open = Some(open);
    shell.areas.conversation.info = Some(Box::new(info.clone()));
    shell.fleet.machines = vec![Machine {
        name: "fixture".to_owned(),
        node: Some(replacement),
        reach: Reach::Online,
        sessions: vec![card],
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
        connection_epoch: None,
    }];
    let request = shell
        .areas
        .sessions
        .lifecycle
        .running
        .begin(target.clone(), LifecycleAction::Exit)
        .unwrap();
    shell.areas.sessions.lifecycle.after = Some(Prepared {
        target,
        action: LifecycleAction::Exit,
        seed: None,
        choices: Vec::new(),
        directory: None,
    });
    (info, host, admitted, request.ticket)
}

#[gpui::test]
fn late_archive_ack_cannot_update_a_replacement_or_close_its_session(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (info, host, admitted, ticket) = pending_exit(shell);
        let mut archived = info.clone();
        archived.status = SessionStatus::Exited { code: None };
        archived.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
        shell.sessions_lifecycle_answer(
            Answer {
                ticket,
                result: Ok(Acted::Session(Box::new(archived))),
            },
            cx,
        );
        shell.settle_session_lifecycle(window, cx);
        assert_eq!(shell.route, Route::Session);
        assert!(shell.areas.sessions.lifecycle.ready.is_none());
        assert!(matches!(
            shell.areas.sessions.lifecycle.error,
            Some((_, Problem::DestinationChanged))
        ));
        assert!(
            shell.fleet.machines[0].sessions[0]
                .detail
                .marks
                .archived_at
                .is_none()
        );
        assert_eq!(shell.areas.conversation.info.as_deref(), Some(&info));
        // The read-only bridge retains the admitted address; latest Fleet is
        // never a way to bless a rename against the replacement node.
        assert_eq!(
            shell.opened_session_address(&host, info.id).unwrap().node,
            admitted
        );
        assert!(
            shell
                .opened_session_address(&host, SessionId::random())
                .is_none()
        );
    });
}

#[gpui::test]
fn a_peer_failure_cannot_render_under_another_admitted_node_with_reused_identifiers(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, _| {
        let (info, host, admitted, _) = pending_exit(shell);
        shell.areas.sessions.lifecycle.error = Some((
            LifecycleTarget {
                host,
                session: info.id,
                node: Some(admitted),
            },
            Problem::LostReply(vessel_app_link::ActFailed {
                code: None,
                message: "synthetic original peer detail".to_owned(),
            }),
        ));
        let tokens = vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark);
        assert!(error(shell, &tokens).is_some());
        shell
            .areas
            .session
            .open
            .as_mut()
            .unwrap()
            .binding
            .as_mut()
            .unwrap()
            .address
            .node = NodeId::random();
        assert!(error(shell, &tokens).is_none());
        shell.areas.session.open.as_mut().unwrap().binding = None;
        assert!(error(shell, &tokens).is_none());
    });
}

#[gpui::test]
fn stale_confirmation_callbacks_cannot_dismiss_or_submit_a_newer_decision(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (_, _, _, pending) = pending_exit(shell);
        let _cancelled = shell.areas.sessions.lifecycle.running.finish(pending);
        shell.areas.sessions.lifecycle.after = None;
        shell.prepare_session_lifecycle(LifecycleAction::Archive, window, cx);
        let first = shell
            .areas
            .sessions
            .lifecycle
            .confirming
            .pending()
            .unwrap()
            .ticket;
        shell.dismiss_session_lifecycle(first, window, cx);
        shell.prepare_session_lifecycle(LifecycleAction::Delete, window, cx);
        let second = shell
            .areas
            .sessions
            .lifecycle
            .confirming
            .pending()
            .unwrap()
            .ticket;
        assert_ne!(first, second);
        shell.dismiss_session_lifecycle(first, window, cx);
        shell.confirm_session_lifecycle(first, window, cx);
        assert_eq!(
            shell
                .areas
                .sessions
                .lifecycle
                .confirming
                .pending()
                .unwrap()
                .ticket,
            second
        );
        assert_eq!(
            shell
                .areas
                .sessions
                .lifecycle
                .asking
                .as_ref()
                .unwrap()
                .action,
            LifecycleAction::Delete
        );
        assert!(shell.areas.sessions.lifecycle.running.pending().is_none());
        shell.confirm_session_lifecycle(second, window, cx);
        assert!(shell.areas.sessions.lifecycle.asking.is_none());
        assert!(
            shell
                .areas
                .sessions
                .lifecycle
                .confirming
                .pending()
                .is_none()
        );
        assert_eq!(
            shell
                .areas
                .sessions
                .lifecycle
                .running
                .pending()
                .unwrap()
                .action,
            LifecycleAction::Delete
        );
    });
}

#[gpui::test]
fn unbound_open_cannot_borrow_latest_fleet_identity_for_a_mutation(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (_, _, _, pending) = pending_exit(shell);
        let _cancelled = shell.areas.sessions.lifecycle.running.finish(pending);
        shell.areas.sessions.lifecycle.after = None;
        shell.areas.session.open.as_mut().unwrap().binding = None;
        shell.prepare_session_lifecycle(LifecycleAction::Archive, window, cx);
        assert!(shell.areas.sessions.lifecycle.asking.is_none());
        assert!(!shell.sessions_lifecycle_busy());
        assert!(matches!(
            shell.areas.sessions.lifecycle.error,
            Some((_, Problem::IdentityUnavailable))
        ));
    });
}

#[gpui::test]
fn navigation_ends_old_focus_cycle_before_a_new_confirmation_opens(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (_, _, _, pending) = pending_exit(shell);
        let _cancelled = shell.areas.sessions.lifecycle.running.finish(pending);
        shell.areas.sessions.lifecycle.after = None;
        shell.areas.session.composer.focus(window, cx);
        shell.prepare_session_lifecycle(LifecycleAction::Archive, window, cx);
        let first = shell
            .areas
            .sessions
            .lifecycle
            .confirming
            .pending()
            .unwrap()
            .ticket;
        shell.cancel_session_navigation();
        shell.areas.draft.composer.focus(window, cx);
        let new_opener = shell.areas.draft.composer.focus_handle(cx);
        shell.prepare_session_lifecycle(LifecycleAction::Delete, window, cx);
        let second = shell
            .areas
            .sessions
            .lifecycle
            .confirming
            .pending()
            .unwrap()
            .ticket;
        shell.dismiss_session_lifecycle(first, window, cx);
        assert!(!new_opener.is_focused(window));
        shell.dismiss_session_lifecycle(second, window, cx);
        assert!(new_opener.is_focused(window));
        assert!(
            shell
                .areas
                .sessions
                .lifecycle
                .confirming
                .pending()
                .is_none()
        );
        assert!(
            !shell
                .areas
                .session
                .composer
                .focus_handle(cx)
                .is_focused(window)
        );
    });
}

#[gpui::test]
fn copy_link_uses_admitted_node_without_superseding_an_inflight_open(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, cx| {
        let (info, _, admitted, _) = pending_exit(shell);
        let ticket = shell
            .areas
            .sessions
            .navigation
            .begin(vessel_app_view::sessions::NavigationPurpose::Open)
            .unwrap();
        shell.copy_session_link(cx);
        let copied = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap();
        assert_eq!(
            vessel_app_view::desktop::NavigationTarget::parse(&copied).unwrap(),
            vessel_app_view::desktop::NavigationTarget::Session {
                node: admitted,
                session: info.id
            }
        );
        assert_eq!(
            shell.areas.sessions.navigation.finish(ticket),
            Some(vessel_app_view::sessions::NavigationPurpose::Open)
        );
        shell
            .areas
            .session
            .open
            .as_mut()
            .unwrap()
            .binding
            .as_mut()
            .unwrap()
            .address
            .session = SessionId::random();
        shell.copy_session_link(cx);
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(copied)
        );
        assert_eq!(
            shell.notice.as_deref(),
            Some(shell.words.sessions().identity_unavailable())
        );
    });
}
