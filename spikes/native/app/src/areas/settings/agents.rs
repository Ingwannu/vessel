//! Agents uses the shared settings furniture and provider row. Per-machine
//! state transitions are UI-free; details and the retained install log draw
//! independently so changing the selected agent cannot hide an active job.
use vessel_app_kit::theme::spacing;
mod add;
mod catalogue;
mod detail;
mod progress;
mod removal;
mod rows;
mod state;
pub(crate) use state::AgentSettings;

use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings};
use vessel_app_link::agents::{Administration, AgentsError};
use vessel_app_view::agent_status::{
    agent_label,
    pages::{Inventory, Issue, Operation, Page},
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
    if let Some(notice) = add::notice(shell, tokens) {
        body = body.child(notice);
    }
    body = body.children(removal::notice(shell, tokens));
    // The machine's failed refresh, above the list or an agent's details,
    // whichever is open (only the list showed it: a refresh that failed
    // while the details were open was not said anywhere, track V1's
    // review).
    if let Some(issue) = page.and_then(machine_alert) {
        body = body.child(problem(&issue.source, shell, tokens));
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
    if page.is_some_and(|page| {
        page.selected().is_none()
            && page
                .inventory()
                .is_some_and(|inventory| inventory.administration == Administration::Supported)
    }) {
        body = body.child(catalogue::render(shell, tokens, compact, window, cx));
    }
    body = body.children(progress::render(shell, tokens, window, cx));
    body.into_any_element()
}

/// The machine-scoped failure the page says once, above the list or the
/// details: a refresh or removal that failed. An agent's own
/// check failure is said in its details, beside its button.
pub(super) fn machine_alert<E>(page: &Page<E>) -> Option<&Issue<E>> {
    page.issue().filter(|issue| {
        matches!(
            issue.operation,
            Operation::Discovery | Operation::Remove { .. }
        )
    })
}

pub(super) fn note(text: impl Into<gpui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}
/// A failure, as the session's lost line says one (the coordinator's
/// decision, 2026-11-13): the visible words are the app's sentence for its
/// kind in the person's language (a kind the app does not tell apart gets
/// its general sentence); the original text, the transport's or the
/// node's, only in the alert's tooltip.
pub(super) fn problem(error: &AgentsError, shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.t.lang.agents();
    let alert = vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(words.problem(error.problem()).into()),
            description: None,
            action: None,
        },
        crate::badge::alert_variant(error.problem().variant()),
        vessel_app_kit::alert::Size::Small,
        tokens,
    );
    with_original(alert, error.to_string(), shell)
}

/// `alert` with `original` (the transport's or the node's own text) in its
/// tooltip, never in its visible body.
pub(super) fn with_original(alert: Div, original: String, shell: &Shell) -> Div {
    let state = &shell.areas.settings.agents;
    let place = Rc::clone(
        state
            .alert_places
            .borrow_mut()
            .entry(original.clone())
            .or_default(),
    );
    let trigger = div()
        .id(gpui::SharedString::from(format!("agents-alert-{original}")))
        .child(alert);
    div().child(state.tips.on(trigger, &place, original, false))
}

pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    detail::defaults::list(shell, tokens, window, cx)
        .into_iter()
        .chain(add::dialog(shell, tokens, window, cx))
        .chain(removal::menu(
            shell,
            tokens,
            vessel_app_kit::breakpoint::phone(window),
            cx,
        ))
        .chain(removal::dialog(shell, tokens, window, cx))
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
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
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

#[cfg(test)]
mod alert_tests {
    use vessel_app_view::agent_status::pages::{MachineKey, Operation, Pages};
    use vessel_proto::{HarnessKind, HarnessRef};

    use super::machine_alert;

    #[test]
    fn a_removal_failure_is_visible_above_the_list_and_an_unrelated_detail() {
        let mut pages: Pages<&str> = Pages::default();
        let machine = MachineKey::from_book("one");
        let request = pages
            .begin(
                machine.clone(),
                Operation::Remove {
                    node: vessel_proto::NodeId::random(),
                    name: "added".parse().unwrap(),
                },
            )
            .unwrap();
        assert!(pages.removal_uncertain(&machine, request, "outcome unknown"));
        for selection in [None, Some(HarnessRef::Known(HarnessKind::Codex))] {
            pages.select("one", selection);
            assert_eq!(
                machine_alert(pages.get("one").unwrap()).map(|issue| issue.source),
                Some("outcome unknown")
            );
        }
    }

    /// Refresh, open an agent's details, the refresh fails: the failure is
    /// the page's alert while the details stay open.
    #[test]
    fn a_refresh_that_fails_while_details_are_open_is_said() {
        let mut pages: Pages<&'static str> = Pages::default();
        let laptop = MachineKey::from_book("laptop");
        let id = pages
            .begin(laptop.clone(), Operation::Discovery)
            .expect("a free page");
        pages.select("laptop", Some(HarnessRef::Known(HarnessKind::Codex)));
        pages.failed(&laptop, id, "the machine is not connected");
        let page = pages.get("laptop").expect("the page");
        assert!(page.selected().is_some(), "the details stay open");
        assert_eq!(
            machine_alert(page).map(|issue| issue.source),
            Some("the machine is not connected")
        );
    }

    /// An agent's own check failure is not the machine's alert.
    #[test]
    fn a_failed_check_is_said_in_its_details_not_above_them() {
        let mut pages: Pages<&'static str> = Pages::default();
        let laptop = MachineKey::from_book("laptop");
        let codex = HarnessRef::Known(HarnessKind::Codex);
        let id = pages
            .begin(laptop.clone(), Operation::Check(codex.clone()))
            .expect("a free page");
        pages.select("laptop", Some(codex));
        pages.failed(&laptop, id, "refused");
        assert!(machine_alert(pages.get("laptop").expect("the page")).is_none());
    }
}
