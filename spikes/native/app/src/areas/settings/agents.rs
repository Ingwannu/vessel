//! Agents uses the shared settings furniture and provider row. Per-machine
//! state transitions are UI-free; details and the retained install log draw
//! independently so changing the selected agent cannot hide an active job.
use vessel_app_kit::theme::spacing;
mod detail;
mod progress;
mod rows;
mod state;
pub(crate) use state::AgentSettings;

use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings};
use vessel_app_link::agents::{Administration, AgentsError};
use vessel_app_view::agent_status::{
    agent_label,
    pages::{Inventory, Operation, Page},
};
use vessel_proto::HarnessRef;

pub(super) fn page(shell: &Shell) -> Option<&Page<AgentsError>> {
    let state = &shell.areas.settings.agents;
    state.pages.get(state.machine.as_ref()?.as_str())
}
pub(super) fn ready<'a>(
    agent: &HarnessRef,
    inventory: &'a Inventory,
) -> Option<&'a vessel_proto::agents::AgentReadiness> {
    inventory
        .readiness
        .iter()
        .find(|ready| ready.agent == *agent)
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.agents();
    let Some(machine) = super::choices::chosen_machine(shell) else {
        return settings::section(
            words.list_title(),
            settings::card(tokens)
                .p(px(spacing::S4))
                .items_center()
                .child(note(words.no_machine(), tokens).max_w_full()),
            tokens,
        )
        .mb(px(spacing::S4))
        .into_any_element();
    };
    let state = &shell.areas.settings.agents;
    let page = page(shell);
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
    if let Some(problem) = state.refused {
        body = body.child(vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(words.start_problem(problem).into()),
                description: None,
                action: None,
            },
            crate::badge::alert_variant(problem.variant()),
            vessel_app_kit::alert::Size::Small,
            tokens,
        ));
    }
    if page
        .and_then(Page::pending)
        .is_some_and(|ticket| matches!(ticket.operation, Operation::Check(_)))
    {
        body = body.child(note(words.checking(), tokens));
    }
    body = match page.and_then(|page| page.inventory()) {
        Some(inventory) => match inventory.administration {
            Administration::Supported => {
                if let Some(agent) = page.and_then(|page| page.selected()) {
                    body.child(detail::render(agent, shell, tokens, compact, window, cx))
                } else {
                    body.child(section(
                        words.row_section(&machine.name),
                        Some(
                            rows::render(inventory, shell, tokens, compact, cx).into_any_element(),
                        ),
                        shell,
                        tokens,
                        compact,
                        cx,
                    ))
                }
            }
            Administration::OlderNode => body.child(section(
                words.row_section(&machine.name),
                Some(
                    settings::card(tokens)
                        .p(px(spacing::S4))
                        .child(note(words.older_node(), tokens))
                        .into_any_element(),
                ),
                shell,
                tokens,
                compact,
                cx,
            )),
        },
        None => body.child(section(
            words.row_section(&machine.name),
            page.filter(|page| page.initial_discovery())
                .map(|_| rows::loading(tokens)),
            shell,
            tokens,
            compact,
            cx,
        )),
    };
    body = body.children(progress::render(shell, tokens, window, cx));
    body.into_any_element()
}

pub(super) fn note(text: impl Into<gpui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}
pub(super) fn problem(error: &AgentsError, shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.t.lang.agents();
    let message = words.problem(error.problem());
    #[cfg(not(target_arch = "wasm32"))]
    let description = if let AgentsError::Call(source) = error {
        Some(
            div()
                .child(words.error_detail())
                .child(div().child(source.to_string()))
                .into_any_element(),
        )
    } else {
        None
    };
    #[cfg(target_arch = "wasm32")]
    let description = None;
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(message.into()),
            description,
            action: None,
        },
        crate::badge::alert_variant(error.problem().variant()),
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
}

pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    detail::defaults::list(shell, tokens, window, cx)
        .into_iter()
        .chain(shell.areas.settings.agents.tips.render(tokens))
        .collect()
}

fn section(
    title: String,
    content: Option<AnyElement>,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let issue = page(shell)
        .and_then(Page::issue)
        .filter(|issue| issue.operation == Operation::Discovery);
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .children(issue.map(|issue| problem(&issue.source, shell, tokens)))
        .children(content);
    settings::section_with_trailing(
        title,
        body,
        Some(refresh(shell, tokens, compact, cx)),
        tokens,
    )
    // ProviderSection overrides the shared section's default bottom gap.
    .mb(px(spacing::S4))
}

fn refresh(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    use vessel_app_kit::{ButtonKind, ButtonState, button_with_kind};
    let state = &shell.areas.settings.agents;
    let busy = page(shell).is_some_and(|page| page.pending().is_some());
    let scope = state.machine.clone();
    let button = button_with_kind(
        "agents-refresh",
        shell.t.lang.agents().refresh(),
        ButtonKind::Ghost,
        if busy {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    )
    .when(compact, |button| {
        button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
    })
    .when(!busy, |button| {
        button.on_click(cx.listener(move |shell, _, _, cx| {
            if shell.agent_context(scope.as_ref()) {
                shell.agent_load(true, cx);
            }
        }))
    });
    state
        .tips
        .on(
            button,
            &state.refresh_place,
            shell.t.lang.agents().refresh(),
            compact,
        )
        .into_any_element()
}

/// The shared frame owns the back target; the area owns its destination.
pub(super) fn header(shell: &Shell) -> Option<super::page::DetailHeader> {
    let page = page(shell)?;
    let agent = page.selected()?;
    let inventory = page.inventory()?;
    let scope = shell.areas.settings.agents.machine.clone();
    Some(super::page::DetailHeader {
        title: agent_label(agent, &inventory.agents).into(),
        back: Rc::new(move |shell, _, cx| {
            if shell.agent_context(scope.as_ref()) {
                shell.agent_back(cx);
            }
        }),
    })
}

/// The list and each agent keep their own outer scroll context, as separate
/// provider routes do in Paseo. A detail's offset must not hide the next list’s title.
pub(super) fn scroll_id(shell: &Shell) -> gpui::ElementId {
    let state = &shell.areas.settings.agents;
    state.machine.as_ref().map_or_else(
        || "agents-no-machine".into(),
        |machine| {
            let selected = state
                .pages
                .get(machine.as_str())
                .and_then(|page| page.selected());
            machine_scroll_id(machine, selected)
        },
    )
}

fn machine_scroll_id(
    machine: &vessel_app_link::agents::MachineKey,
    agent: Option<&HarnessRef>,
) -> gpui::ElementId {
    let name = machine.as_str();
    let selected = agent.map_or_else(
        || "list".to_owned(),
        |agent| format!("agent:{}", vessel_app_view::agent_status::agent_key(agent)),
    );
    // Length-prefix the book name: punctuation in a name cannot join two contexts.
    format!("agents:{}:{name}:{selected}", name.len()).into()
}

#[cfg(test)]
mod tests {
    use super::machine_scroll_id;
    use vessel_app_link::agents::MachineKey;
    use vessel_proto::{HarnessKind, HarnessRef};
    #[test]
    fn machine_lists_and_agent_details_never_share_the_outer_scroll_state() {
        let a = MachineKey::from_book("one");
        let b = MachineKey::from_book("two");
        let agent = HarnessRef::Known(HarnessKind::Codex);
        let other = HarnessRef::Known(HarnessKind::ClaudeCode);
        assert_ne!(
            machine_scroll_id(&a, None),
            machine_scroll_id(&a, Some(&agent))
        );
        assert_ne!(
            machine_scroll_id(&a, Some(&agent)),
            machine_scroll_id(&a, Some(&other))
        );
        assert_ne!(machine_scroll_id(&a, None), machine_scroll_id(&b, None));
        assert_eq!(machine_scroll_id(&a, None), machine_scroll_id(&a, None));
        let punctuated = MachineKey::from_book("one:agent:codex");
        assert_ne!(
            machine_scroll_id(&punctuated, None),
            machine_scroll_id(&a, Some(&agent))
        );
    }
}
