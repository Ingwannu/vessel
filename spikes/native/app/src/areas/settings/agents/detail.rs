//! Agent details reuse one source of readiness and the new-chat defaults.
use vessel_app_kit::theme::spacing;
pub(super) mod defaults;
mod fact_rows;
mod facts;
mod sign_in;
pub(super) mod widgets;
use super::{note, page, ready};
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_view::agent_status::readiness;
use vessel_proto::HarnessRef;
pub(super) fn render(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let body = div().flex().flex_col().gap(px(spacing::S4));
    let Some(ready) = page(shell)
        .and_then(|page| page.inventory())
        .and_then(|inventory| ready(agent, inventory))
    else {
        let target = agent.clone();
        let scope = shell.areas.settings.agents.machine.clone();
        let words = shell.t.lang.agents();
        let issue = page(shell).and_then(|page| page.issue()).filter(|issue| {
            issue.operation == vessel_app_view::agent_status::pages::Operation::Check(agent.clone())
        });
        return body
            .child(note(words.status_unread(), tokens))
            .children(issue.map(|issue| super::problem(&issue.source, shell, tokens)))
            .child(
                vessel_app_kit::outline_button(
                    "agent-check-status",
                    words.check_status,
                    true,
                    tokens,
                )
                .when(compact, |button| {
                    button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
                })
                .self_start()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.agent_context(scope.as_ref()) {
                        shell.agent_ask(vessel_app_link::agents::Ask::Check(target.clone()), cx);
                    }
                })),
            );
    };
    let mut body = body
        .child(facts::render(agent, shell, tokens, window, cx))
        .child(sign_in::render(agent, shell, tokens, window, cx));
    if readiness::offers_defaults(&ready.installed) {
        body = body.child(defaults::render(agent, shell, tokens, compact, cx));
    }
    body
}
