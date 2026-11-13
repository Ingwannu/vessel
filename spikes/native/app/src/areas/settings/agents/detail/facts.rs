//! Installation facts and exact executable arguments supplied by the node.
use super::super::{note, page, ready};
use super::widgets;
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, px,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_kit::{popup, theme::spacing};
use vessel_app_link::agents::Ask;
use vessel_app_view::agent_status::{agent_label, pages::Inventory, readiness};
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
    let mut card = settings::card(tokens)
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(note(shown.sentence, tokens));
    if let Some(message) = shown.machine_message {
        card = card.child(vessel_app_kit::alert::alert(
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
        ));
    }
    card = card.child(note(words.version(), tokens));
    card = match readiness::version(&ready.installed) {
        Some(version) => card.child(widgets::command_box(
            "agent-version",
            version,
            tokens,
            window,
            cx,
        )),
        None => card.child(note(words.version_unknown(), tokens)),
    };
    if let Some(program) = readiness::program(&ready.installed) {
        card = card
            .child(note(words.program(), tokens))
            .child(widgets::command_box(
                "agent-path",
                &program,
                tokens,
                window,
                cx,
            ));
    }
    card = added_arguments(card, agent, inventory, shell, tokens, window, cx);
    card = card.child(installers(agent, shell, tokens, window, cx));
    settings::section(agent_label(agent, &inventory.agents), card, tokens)
}

fn added_arguments(
    mut card: Div,
    agent: &HarnessRef,
    inventory: &Inventory,
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &gpui::App,
) -> Div {
    let words = shell.t.lang.agents();
    if let Some(info) = inventory.agents.iter().find(|info| info.agent == *agent)
        && let AgentDetail::Added { added } = &info.detail
    {
        if let Some(command) = inventory.command_shell.command(added) {
            return card
                .child(note(words.launch_command(), tokens))
                .child(note(words.launch_note(), tokens))
                .child(widgets::command_box(
                    "added-command",
                    &command,
                    tokens,
                    window,
                    cx,
                ));
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
    }
    card
}

fn installers(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.agents();
    let Some(page) = page(shell) else {
        return gpui::div();
    };
    let Some(inventory) = page.inventory() else {
        return gpui::div();
    };
    let Some(ready) = ready(agent, inventory) else {
        return gpui::div();
    };
    let busy = page.pending().is_some();
    let mut card = gpui::div().flex().flex_col().gap(px(spacing::S3));
    for (index, offer) in readiness::installs(agent, &ready.installed, &inventory.agents)
        .into_iter()
        .enumerate()
    {
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
                    .min_h(px(popup::row_height(true)))
                    .self_start()
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if !busy && shell.agent_context(scope.as_ref()) {
                            shell.agent_ask(Ask::Install(install), cx);
                        }
                    })),
            );
    }
    card
}
