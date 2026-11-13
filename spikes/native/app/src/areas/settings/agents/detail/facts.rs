//! Installation facts and exact executable arguments supplied by the node.
use super::super::{note, page, ready};
use super::{
    fact_rows::{self, Fact, Value},
    widgets,
};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, px,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_kit::{breakpoint, theme::spacing};
use vessel_app_link::agents::Ask;
use vessel_app_view::agent_status::{pages::Inventory, readiness};
use vessel_proto::{
    HarnessRef,
    agents::{AgentDetail, InstallAgent, InstallPart},
};

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
    let shown = readiness::installation(shell.t.lang, agent, &ready.installed, &inventory.agents);
    let mut card = settings::card(tokens).child(fact_rows::row(
        &Fact {
            id: "agent-status",
            label: words.status(),
            value: Value::Text(&shown.sentence),
            first: true,
        },
        tokens,
        window,
        cx,
    ));
    if let Some(message) = shown.machine_message {
        card = card.child(
            gpui::div()
                .p(px(spacing::S4))
                .child(vessel_app_kit::alert::alert(
                    vessel_app_kit::alert::Content {
                        title: Some(message.label.into()),
                        description: Some(
                            gpui::div()
                                .child(message.text.to_owned())
                                .into_any_element(),
                        ),
                        action: None,
                    },
                    crate::badge::alert_variant(message.variant()),
                    vessel_app_kit::alert::Size::Small,
                    tokens,
                )),
        );
    }
    let version = readiness::version(&ready.installed)
        .map_or_else(|| Value::Text(words.version_unknown()), Value::Code);
    card = card.child(fact_rows::row(
        &Fact {
            id: "agent-version",
            label: words.version(),
            value: version,
            first: false,
        },
        tokens,
        window,
        cx,
    ));
    if let Some(program) = readiness::program(&ready.installed) {
        card = card.child(fact_rows::row(
            &Fact {
                id: "agent-program",
                label: words.program(),
                value: Value::Code(&program),
                first: false,
            },
            tokens,
            window,
            cx,
        ));
    }
    card = card.children(added_arguments(agent, inventory, shell, tokens, window, cx));
    card = card.children(installers(agent, shell, tokens, window, cx));
    settings::section(words.details(), card, tokens)
}

fn added_arguments(
    agent: &HarnessRef,
    inventory: &Inventory,
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &gpui::App,
) -> Option<Div> {
    let words = shell.t.lang.agents();
    let mut card = gpui::div()
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S3));
    if let Some(info) = inventory.agents.iter().find(|info| info.agent == *agent)
        && let AgentDetail::Added { added } = &info.detail
    {
        if let Some(command) = inventory.command_shell.command(added) {
            return Some(
                card.child(note(words.launch_command(), tokens))
                    .child(note(words.launch_note(), tokens))
                    .child(widgets::command_box(
                        "added-command",
                        &command,
                        tokens,
                        window,
                        cx,
                    )),
            );
        }
        card = card
            .child(note(words.program(), tokens))
            .child(widgets::command_box(
                "added-program",
                added.program.as_str(),
                tokens,
                window,
                cx,
            ));
        for (index, arg) in added.args.iter().enumerate() {
            card = card
                .child(note(words.argument(index.saturating_add(1)), tokens))
                .child(widgets::command_box(
                    format!("added-arg-{index}"),
                    arg,
                    tokens,
                    window,
                    cx,
                ));
        }
        return Some(card);
    }
    None
}

fn installers(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let words = shell.t.lang.agents();
    let page = page(shell)?;
    let inventory = page.inventory()?;
    let ready = ready(agent, inventory)?;
    let offers = readiness::installs(agent, &ready.installed, &inventory.agents);
    if offers.is_empty() {
        return None;
    }
    let busy = page.pending().is_some();
    let mut card = gpui::div()
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S3));
    for (index, offer) in offers.into_iter().enumerate() {
        let Some(kind) = agent.known() else {
            continue;
        };
        let scope = shell.areas.settings.agents.machine.clone();
        let install = InstallAgent {
            agent: kind,
            part: offer.part,
        };
        let label = match offer.part {
            InstallPart::Harness => words.install,
            InstallPart::Adapter => words.install_adapter,
        };
        card = card
            .child(note(words.install_warning(), tokens))
            .child(widgets::command_box(
                format!("install-command-{index}"),
                &offer.command,
                tokens,
                window,
                cx,
            ))
            .child(
                outline_button(format!("install-agent-{index}"), label, !busy, tokens)
                    .when(breakpoint::phone(window), |button| {
                        button.min_h(px(breakpoint::TOUCH_TARGET))
                    })
                    .self_start()
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if !busy && shell.agent_context(scope.as_ref()) {
                            shell.agent_ask(Ask::Install(install), cx);
                        }
                    })),
            );
    }
    Some(card)
}
