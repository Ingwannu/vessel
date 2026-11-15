//! `ProviderRow` and `StatusIndicator` from Paseo's providers-section.tsx.
//! The shared row owns insets/borders; this area supplies only its contents.
use super::{page, ready};
use crate::shell::Shell;
use vessel_app_kit::agent_mark::{self, SettingsRowEdge, settings_mark};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, assets::Glyph, glyph, settings, theme::spacing};
use vessel_app_view::{
    agent_status::{
        agent_label, agents_on,
        pages::Inventory,
        row::{self, Activity, Density, Indicator, Marker},
    },
    badge::Variant,
};
use vessel_proto::{
    HarnessRef,
    agents::{Installed, UnknownReason},
};

const ROW_HEIGHT: f32 = 56.0;
const INITIAL_ROWS: usize = 4;

pub(super) fn render(
    inventory: &Inventory,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut card = settings::card(tokens);
    for (index, entry) in agents_on(&inventory.harnesses, &inventory.agents)
        .into_iter()
        .enumerate()
    {
        let scope = shell.areas.settings.agents.machine.clone();
        let target = entry.agent.clone();
        let edge = if index == 0 {
            SettingsRowEdge::First
        } else {
            SettingsRowEdge::Following
        };
        let contents = contents(&entry.agent, edge, inventory, shell, tokens, compact);
        let hover = tokens.neutral.surface2;
        let pressed = tokens.neutral.surface3;
        card = card.child(
            div()
                .id(format!("agent-row-{index}"))
                .group("agent-row")
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(pressed))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.agent_context(scope.as_ref()) {
                        shell.agent_select(target.clone(), cx);
                    }
                }))
                .child(
                    settings::row_with_content(
                        contents.into_any_element(),
                        None,
                        index == 0,
                        compact,
                        tokens,
                    )
                    .min_h(px(ROW_HEIGHT)),
                ),
        );
    }
    card
}
fn contents(
    agent: &HarnessRef,
    edge: SettingsRowEdge,
    inventory: &Inventory,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
) -> Div {
    let density = if compact {
        Density::Compact
    } else {
        Density::Expanded
    };
    let state = presentation(agent, inventory, shell);
    let title = div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(agent_label(agent, &inventory.agents)),
        )
        .when(!compact, |title| title.child(separator(tokens)))
        .child(indicator(state.indicator(shell.t.lang, density), tokens));
    let column = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(title)
        .children(state.error(density).map(|message| {
            div()
                .mt(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(tokens.status().red_300)
                .line_clamp(3)
                .child(message.to_owned())
        }));
    let foreground = tokens.neutral.foreground;
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            glyph(Glyph::ChevronRight, 14.0, tokens.neutral.foreground_muted)
                .group_hover("agent-row", move |style| style.text_color(foreground)),
        )
        // The agent's official mark (track K's, ADR 0048) in the slot
        // Paseo's ProviderRow gives its provider icon; an agent with none
        // keeps the generic glyph in the row's foreground.
        .child(settings_mark(
            crate::agent_mark::of(agent),
            agent_mark::Size::Sixteen,
            edge,
            tokens,
        ))
        .child(column)
}
fn presentation(agent: &HarnessRef, inventory: &Inventory, shell: &Shell) -> row::Row {
    let unknown = Installed::Unknown {
        reason: UnknownReason::NotChecked,
    };
    let facts = ready(agent, inventory);
    let options = shell
        .areas
        .settings
        .agents
        .machine
        .as_ref()
        .and_then(|machine| shell.offer_options(machine.as_str(), agent));
    row::row(
        shell.t.lang,
        row::Facts {
            agent,
            installed: facts.map_or(&unknown, |facts| &facts.installed),
            signed_in: facts.map(|facts| &facts.signed_in),
            options,
            activity: Activity::for_agent(
                agent,
                page(shell)
                    .and_then(|page| page.pending())
                    .map(|ticket| &ticket.operation),
            ),
        },
    )
}
fn separator(tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child("·")
}
fn indicator(indicator: Indicator, tokens: &Tokens) -> Div {
    let (tone, marker, label, models) = match indicator {
        Indicator::Compact { tone, marker } => (tone, marker, None, None),
        Indicator::Expanded {
            tone,
            marker,
            label,
            models,
        } => (tone, marker, Some(label), models),
    };
    let colour = match tone {
        Variant::Success => tokens.status().success.foreground,
        Variant::Warning => tokens.status().warning.foreground,
        Variant::Error => tokens.status().error.foreground,
        Variant::Muted => tokens.neutral.foreground_muted,
    };
    let marker: AnyElement = match marker {
        Marker::Dot => vessel_app_kit::dot(colour).size(px(8.0)).into_any_element(),
        Marker::Spinner => vessel_app_kit::slider::spinner("agent-status-loading", 10.0, colour),
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(spacing::S1_5))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(marker)
        .children(label.map(|label| div().child(label)))
        .children(models.map(|models| {
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S1_5))
                .child(separator(tokens))
                .child(models)
        }))
}

/// `ProviderRowsSkeleton`: one kit pulse, four existing settings row frames.
pub(super) fn loading(tokens: &Tokens) -> AnyElement {
    let tokens = *tokens;
    vessel_app_kit::skeleton::pulse("agents-initial-discovery", move |opacity| {
        loading_card(opacity, &tokens)
    })
}
fn loading_card(opacity: vessel_app_kit::skeleton::Opacity, tokens: &Tokens) -> Div {
    use vessel_app_kit::skeleton::{self, Rounding, Shape};
    let mut card = settings::card(tokens);
    for index in 0..INITIAL_ROWS {
        let contents = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(
                skeleton::shape(
                    Shape {
                        size: gpui::size(px(20.0), px(20.0)),
                        rounding: Rounding::Medium,
                    },
                    opacity,
                    tokens,
                )
                .ml(px(spacing::S6)),
            )
            .child(skeleton::shape(
                Shape {
                    size: gpui::size(px(160.0), px(12.0)),
                    rounding: Rounding::Small,
                },
                opacity,
                tokens,
            ));
        card = card.child(
            settings::row_with_content(
                contents.into_any_element(),
                None,
                index == 0,
                false,
                tokens,
            )
            .min_h(px(ROW_HEIGHT)),
        );
    }
    card
}
#[cfg(test)]
mod tests;
