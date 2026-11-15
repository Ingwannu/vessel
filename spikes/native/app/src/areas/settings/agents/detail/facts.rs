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
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_kit::{breakpoint, theme::spacing};
use vessel_app_link::agents::Ask;
use vessel_app_view::agent_status::{pages::Inventory, readiness};
use vessel_proto::{
    HarnessRef,
    agents::{AgentDetail, InstallAgent, InstallOffer, InstallPart},
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
        None,
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
        None,
        tokens,
        window,
        cx,
    ));
    if let Some(program) = readiness::program(&ready.installed) {
        card = card.child(fact_rows::row(
            &Fact {
                id: "agent-program",
                label: words.program(),
                value: Value::Path(&program),
                first: false,
            },
            Some(&fact_rows::Tip {
                tips: &shell.areas.settings.agents.tips,
                place: &shell.areas.settings.agents.program_place,
            }),
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
) -> Vec<Div> {
    let words = shell.t.lang.agents();
    let compact = breakpoint::phone(window);
    let Some(AgentDetail::Added { added }) = inventory
        .agents
        .iter()
        .find(|info| info.agent == *agent)
        .map(|info| &info.detail)
    else {
        return Vec::new();
    };
    // A block with its own label, hint and box is a row of its own, behind
    // the card's divider, as Paseo's settings card separates its rows (it
    // read as part of the program's row above it: the coordinator's
    // review).
    let block = |title: String, hint: Option<&str>, id: String, text: &str| {
        settings::row_with_content(
            // The row's whole width: in a phone's wrapping row a flexible
            // column took its widest line's width, and its hint ran past
            // the card's edge.
            gpui::div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .child(
                    gpui::div()
                        .type_(Type::Label)
                        .text_color(tokens.neutral.foreground)
                        .child(title),
                )
                .children(hint.map(|hint| {
                    gpui::div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(hint.to_owned())
                }))
                .child(widgets::command_box(id, text, tokens, window, cx))
                .into_any_element(),
            None,
            false,
            compact,
            tokens,
        )
    };
    if let Some(command) = inventory.command_shell.command(added) {
        return vec![block(
            words.launch_command().to_owned(),
            Some(words.launch_note()),
            "added-command".to_owned(),
            &command,
        )];
    }
    std::iter::once(block(
        words.program().to_owned(),
        None,
        "added-program".to_owned(),
        added.program.as_str(),
    ))
    .chain(added.args.iter().enumerate().map(|(index, arg)| {
        block(
            words.argument(index.saturating_add(1)),
            None,
            format!("added-arg-{index}"),
            arg,
        )
    }))
    .collect()
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
    let compact = breakpoint::phone(window);
    let mut card = gpui::div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S3));
    for (index, offer) in offers.into_iter().enumerate() {
        // Every field named, no `..`: the person approves what this shows,
        // so a field the node adds must break the build until the screen
        // shows it or refuses the offer (README, "Rules every screen
        // follows").
        let InstallOffer { part, command } = offer;
        let Some(kind) = agent.known() else {
            continue;
        };
        let scope = shell.areas.settings.agents.machine.clone();
        let install = InstallAgent {
            agent: kind,
            part: *part,
        };
        let label = match part {
            InstallPart::Harness => words.install,
            InstallPart::Adapter => words.install_adapter,
        };
        card = card
            .child(note(words.install_warning(), tokens))
            .child(widgets::command_box(
                format!("install-command-{index}"),
                command,
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
    // A row of its own behind the card's divider, as the launch command's:
    // it read as part of the program's row above it.
    Some(settings::row_with_content(
        card.into_any_element(),
        None,
        false,
        compact,
        tokens,
    ))
}
