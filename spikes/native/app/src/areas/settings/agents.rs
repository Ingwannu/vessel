//! Agents uses the shared settings furniture and status badge. Per-machine
//! state transitions are UI-free; details and the retained install log draw
//! independently so changing the selected agent cannot hide an active job.
use vessel_app_kit::{popup, theme::spacing};
mod detail;
mod progress;
mod state;
pub(crate) use state::AgentSettings;

use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, badge, glyph, outline_button, settings};
use vessel_app_link::agents::{Administration, AgentsError};
use vessel_app_view::agent_status::{
    agent_label, agents_on,
    pages::{Inventory, Operation, Page},
    readiness,
};
use vessel_proto::{
    HarnessRef,
    agents::{Installed, UnknownReason},
};

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
        return note(words.no_machine(), tokens).into_any_element();
    };
    let state = &shell.areas.settings.agents;
    let page = page(shell);
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(note(words.on_machine(&machine.name), tokens));
    let refresh_scope = state.machine.clone();
    let busy = page.is_some_and(|page| page.pending().is_some());
    body = body.child(
        outline_button("agents-refresh", words.refresh(), !busy, tokens)
            .min_h(px(popup::row_height(true)))
            .self_start()
            .on_click(cx.listener(move |shell, _, _, cx| {
                if !busy && shell.agent_context(refresh_scope.as_ref()) {
                    shell.agent_load(true, cx);
                }
            })),
    );
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
    if let Some(issue) = page
        .and_then(Page::issue)
        .filter(|issue| issue.operation == Operation::Discovery)
    {
        body = body.child(problem(&issue.source, shell, tokens));
    }
    if let Some(ticket) = page.and_then(Page::pending) {
        body = body.child(note(words.operation(&ticket.operation), tokens));
    }
    let Some(page) = page else {
        return body.into_any_element();
    };
    let Some(inventory) = page.inventory() else {
        return body
            .children(progress::render(shell, tokens, window, cx))
            .into_any_element();
    };
    match inventory.administration {
        Administration::Supported => {}
        Administration::OlderNode => {
            return body
                .child(note(words.older_node(), tokens))
                .into_any_element();
        }
    }
    body = if let Some(agent) = page.selected() {
        let back_scope = state.machine.clone();
        body.child(
            outline_button("agents-back", words.back_to_agents(), true, tokens)
                .min_h(px(popup::row_height(true)))
                .self_start()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.agent_context(back_scope.as_ref()) {
                        shell.agent_back(cx);
                    }
                })),
        )
        .child(detail::render(agent, shell, tokens, compact, window, cx))
    } else {
        body.child(rows(inventory, shell, tokens, compact, cx))
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
) -> Option<AnyElement> {
    detail::defaults::list(shell, tokens, window, cx)
}

fn rows(
    inventory: &Inventory,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.agents();
    let mut rows = settings::card(tokens);
    for (index, entry) in agents_on(&inventory.harnesses, &inventory.agents)
        .into_iter()
        .enumerate()
    {
        let unknown = Installed::Unknown {
            reason: UnknownReason::NotChecked,
        };
        let installed = ready(&entry.agent, inventory).map_or(&unknown, |ready| &ready.installed);
        let presentation =
            readiness::installation(shell.t.lang, &entry.agent, installed, &inventory.agents);
        let target = entry.agent.clone();
        let scope = shell.areas.settings.agents.machine.clone();
        let status = div()
            .flex()
            .items_center()
            .gap(px(spacing::S3))
            .child(badge::badge(
                presentation.badge.label,
                crate::badge::variant(presentation.badge.variant),
                badge::Size::Small,
                None,
                tokens,
            ))
            .child(glyph(
                Glyph::ChevronRight,
                16.0,
                tokens.neutral.foreground_muted,
            ));
        let control = div()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(spacing::S2))
            .child(status)
            .children(ready(&entry.agent, inventory).map(|ready| {
                div()
                    .max_w(px(180.0))
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(readiness::authentication_summary(
                        shell.t.lang,
                        &ready.signed_in,
                    ))
            }));
        let hover = tokens.neutral.surface2;
        rows = rows.child(
            div()
                .id(format!("agent-row-{index}"))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.agent_context(scope.as_ref()) {
                        shell.agent_select(target.clone(), cx);
                    }
                }))
                .child(settings::row_at(
                    agent_label(&entry.agent, &inventory.agents),
                    Some(
                        readiness::summary(
                            shell.t.lang,
                            &entry.agent,
                            installed,
                            &inventory.agents,
                        )
                        .into(),
                    ),
                    None,
                    Some(control.into_any_element()),
                    index == 0,
                    compact,
                    tokens,
                )),
        );
    }
    settings::section(words.list_title(), rows, tokens)
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
