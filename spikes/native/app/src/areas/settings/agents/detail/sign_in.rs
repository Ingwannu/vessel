//! Typed sign-in instructions from the machine; no guessed login commands.
use super::super::{note, page, ready};
use super::widgets;
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, px,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_kit::{breakpoint, theme::spacing};
use vessel_app_link::agents::Ask;
use vessel_app_view::agent_status::{
    pages::Operation,
    readiness::{self, SignInStep},
};
use vessel_proto::HarnessRef;
pub(super) fn render(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.agents();
    let Some(page) = page(shell) else {
        return settings::card(tokens);
    };
    let Some(inventory) = page.inventory() else {
        return settings::card(tokens);
    };
    let Some(ready) = ready(agent, inventory) else {
        return settings::card(tokens);
    };
    let busy = page.pending().is_some();
    let mut auth = settings::card(tokens)
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: None,
                description: Some(
                    gpui::div()
                        .child(readiness::authentication(shell.t.lang, &ready.signed_in))
                        .into_any_element(),
                ),
                action: None,
            },
            crate::badge::alert_variant(vessel_app_view::agent_status::notices::authentication(
                &ready.signed_in,
            )),
            vessel_app_kit::alert::Size::Small,
            tokens,
        ));
    for (index, row) in readiness::sign_in_help(shell.t.lang, &ready.signed_in)
        .into_iter()
        .enumerate()
    {
        auth = auth
            .child(note(row.label, tokens))
            .child(note(row.instruction, tokens));
        auth = match row.step {
            SignInStep::Command(command) => auth.child(widgets::command_box(
                format!("sign-in-command-{index}"),
                command,
                tokens,
                window,
                cx,
            )),
            SignInStep::AccountFolder(variable) => auth.child(widgets::command_box(
                format!("sign-in-account-{index}"),
                variable.as_str(),
                tokens,
                window,
                cx,
            )),
            SignInStep::Method(method) => auth.child(note(method.name.clone(), tokens)).children(
                method
                    .description
                    .as_ref()
                    .map(|description| note(description.clone(), tokens)),
            ),
            SignInStep::OpenAgent => auth,
        };
    }
    let target = agent.clone();
    let scope = shell.areas.settings.agents.machine.clone();
    auth = auth.child(
        outline_button("agent-check-status", words.check_status, !busy, tokens)
            .when(breakpoint::phone(window), |button| {
                button.min_h(px(breakpoint::TOUCH_TARGET))
            })
            .self_start()
            .on_click(cx.listener(move |shell, _, _, cx| {
                if !busy && shell.agent_context(scope.as_ref()) {
                    shell.agent_ask(Ask::Check(target.clone()), cx);
                }
            })),
    );
    if let Some(issue) = page
        .issue()
        .filter(|issue| issue.operation == Operation::Check(agent.clone()))
    {
        auth = auth.child(super::super::problem(&issue.source, shell, tokens));
    }
    settings::section(words.authentication(), auth, tokens)
}
