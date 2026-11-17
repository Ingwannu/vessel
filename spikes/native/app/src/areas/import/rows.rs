//! Paseo's `ImportSessionSheetRow` and `ProviderErrorBanner`.

use super::draw::small_button;
use super::intent::{Action, listener, state};
use crate::shell::Shell;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::theme::spacing::{S0_5, S1, S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{ButtonState, Tokens, Type, TypeExt as _};
use vessel_app_view::devices::time::Age;
use vessel_app_view::history::{Problem, Row};
use vessel_proto::HarnessRef;

/// Paseo's row margin: the fill reaches 8 past the content on both sides.
const PULL: f32 = -8.0;

/// The rows, 4 apart (Paseo's `listRow` bottom padding).
pub(super) fn list(
    shell: &Shell,
    rows: &[Row<'_>],
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div().flex().flex_col().min_w_0().children(
        rows.iter()
            .map(|item| div().pb(px(S1)).child(row(shell, item, tokens, cx))),
    )
}

/// A pressable row: the agent's mark (16, 2 down), then the title with its
/// time (or "Importing...") on the right, and the folder only when the list
/// is not limited to one. Its fill reaches 8 past the text on both sides.
fn row(shell: &Shell, item: &Row<'_>, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let entry = item.entry;
    let action = Action::Row {
        harness: item.harness.clone(),
        id: entry.id.clone(),
    };
    let enabled = state(&action, shell) == ButtonState::Enabled;
    let importing = area.model.importing() == Some(&entry.id);
    let act = listener(shell, action);
    // Paseo's testID: the provider and its id (two agents may share an id).
    let selector = format!("history-row-{}-{}", item.harness, entry.id.as_str());
    let title = entry
        .title
        .as_ref()
        .map_or_else(|| words.untitled().to_owned(), ToString::to_string);
    let meta = if importing {
        Some(words.importing().to_owned())
    } else {
        entry.updated_at.map(|at| {
            shell.t.lang.devices().compact_age(Age::of(
                at,
                chrono::Utc::now(),
                *chrono::Local::now().offset(),
            ))
        })
    };
    let hover = tokens.neutral.surface1;
    let pressed = tokens.neutral.surface2;
    div()
        .id(ElementId::Name(selector.clone().into()))
        .debug_selector(move || selector.clone())
        .role(gpui::Role::Button)
        .aria_label(title.clone())
        .flex()
        .flex_row()
        .items_start()
        .gap(px(S2))
        .p(px(S2))
        .mx(px(PULL))
        .min_w_0()
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .when(enabled, |row| {
            row.cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(pressed))
                .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx)))
        })
        // Paseo's `rowIconWrap`: a 16 slot, so every title starts at one
        // edge whatever the artwork's width.
        .child(
            div()
                .flex_shrink_0()
                .w(px(vessel_app_kit::theme::icon::MD))
                .flex()
                .justify_center()
                .pt(px(S0_5))
                .child(crate::agent_mark::mark_muted(
                    Some(item.harness),
                    Size::Sixteen,
                    Space::Compact,
                    tokens,
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(S1))
                .child(heading(title, meta, tokens))
                .when(!area.model.scoped(), |content| {
                    content.child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.neutral.foreground_muted)
                            .truncate()
                            .child(entry.cwd.to_string_lossy().into_owned()),
                    )
                }),
        )
        .into_any_element()
}

/// Paseo's `rowHeader`: the title (14, one line) and, on its baseline at the
/// right, the activity time or "Importing..." (12 muted).
fn heading(title: String, meta: Option<String>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .items_baseline()
        .justify_between()
        .gap(px(S2))
        .min_w_0()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .truncate()
                .child(title),
        )
        .children(meta.map(|meta| {
            div()
                .flex_shrink_0()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(meta)
        }))
}

/// Paseo's `ProviderErrorBanner`: one bordered box (radius 8, 4 by 12) with a
/// row per failed provider: its words in red 12 and a small ghost Retry.
pub(super) fn failures(
    shell: &Shell,
    failures: &[(&HarnessRef, Problem)],
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.history();
    div()
        .debug_selector(|| "history-provider-errors".to_owned())
        .flex()
        .flex_col()
        .border_1()
        .border_color(tokens.neutral.border)
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .py(px(S1))
        .px(px(S3))
        .children(failures.iter().map(|(harness, _problem)| {
            let name = vessel_app_view::agent_name(harness);
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(S2))
                .child(
                    div()
                        .min_w_0()
                        .type_(Type::Meta)
                        .text_color(tokens.status().red_300)
                        .child(words.failed_provider(&name).unwrap_or(name)),
                )
                .child(small_button(
                    format!("history-retry-{harness}"),
                    words.retry(),
                    Action::Retry((*harness).clone()),
                    shell,
                    tokens,
                    compact,
                    cx,
                ))
        }))
        .into_any_element()
}
