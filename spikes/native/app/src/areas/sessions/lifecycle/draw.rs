//! One kit dialog renders the retained lifecycle decision and target error.
use super::{LifecycleAction, LifecycleTicket, Problem, Shell};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _, breakpoint,
    button, button_with_kind, dialog,
};

pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.sessions.lifecycle;
    let asking = state.asking.as_ref()?;
    let ticket = state.confirming.pending()?.ticket;
    let words = shell.t.lang.conversation();
    let destructive = asking.action == LifecycleAction::Delete;
    let title = if destructive {
        words.delete_title()
    } else {
        words.archive_title()
    };
    let body = if destructive {
        words.delete_words(asking.target.host.as_str())
    } else {
        words.archive_words(
            shell
                .fleet
                .machines
                .iter()
                .filter(|machine| {
                    machine.name == asking.target.host.as_str()
                        && asking.target.matches_node(machine.node)
                })
                .flat_map(|machine| &machine.sessions)
                .any(|card| card.session == asking.target.session && card.live),
            asking.target.host.as_str(),
            shell
                .fleet
                .protocol_of(asking.target.host.as_str())
                .is_some_and(|protocol| protocol >= vessel_app_view::session::ENDS_BESIDE_PROTOCOL),
        )
    };
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: title.into(),
            body: div().type_(Type::Body).child(body).into_any_element(),
            footer: footer(shell, asking.action, ticket, tokens, cx),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        breakpoint::phone(window),
        move |shell, window, cx| shell.dismiss_session_lifecycle(ticket, window, cx),
        cx,
    ))
}

/// Only the session whose operation failed shows the retained refusal.
pub(crate) fn error(shell: &Shell, tokens: &Tokens) -> Option<AnyElement> {
    let (target, problem) = shell.areas.sessions.lifecycle.error.as_ref()?;
    let open = shell.areas.session.open.as_ref()?;
    if open.machine != target.host.as_str()
        || open.session != target.session
        || !target.matches_node(open.binding.as_ref().map(|binding| binding.address.node))
    {
        return None;
    }
    let message = problem_words(shell, problem);
    let mut body = div()
        .type_(Type::Body)
        .text_color(tokens.neutral.destructive)
        .child(message);
    if let Problem::LostReply(failed) = problem {
        body = body.child(div().child(failed.message.clone()));
    }
    Some(body.into_any_element())
}

pub(super) fn problem_words(shell: &Shell, problem: &Problem) -> String {
    let words = shell.words.sessions();
    let message = match problem {
        Problem::Busy => words.navigation_busy(),
        Problem::ExistingDraft => words.existing_draft(),
        Problem::ProfileUnavailable => words.profile_unavailable(),
        Problem::Refused(failed) => &failed.message,
        Problem::LostReply(_) => words.reply_uncertain(),
        Problem::Uncertain => words.outcome_uncertain(),
        Problem::Offline => words.navigation_offline(),
        Problem::WrongReply => words.navigation_refused(),
        Problem::DestinationChanged => words.destination_changed(),
        Problem::IdentityUnavailable => words.identity_unavailable(),
    };
    message.to_owned()
}

fn footer(
    shell: &Shell,
    action: LifecycleAction,
    ticket: LifecycleTicket,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.t.lang.conversation();
    vec![
        button("sessions-lifecycle-cancel", words.cancel(), false, tokens)
            .min_h(vessel_app_kit::ui::px(breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(move |shell, _, window, cx| {
                shell.dismiss_session_lifecycle(ticket, window, cx);
            }))
            .into_any_element(),
        button_with_kind(
            "sessions-lifecycle-confirm",
            words.confirm(),
            if action == LifecycleAction::Delete {
                ButtonKind::Destructive
            } else {
                ButtonKind::Primary
            },
            ButtonState::Enabled,
            tokens,
        )
        .min_h(vessel_app_kit::ui::px(breakpoint::TOUCH_TARGET))
        .on_click(cx.listener(move |shell, _, window, cx| {
            shell.confirm_session_lifecycle(ticket, window, cx);
        }))
        .into_any_element(),
    ]
}
