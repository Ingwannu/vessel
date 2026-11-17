//! Paseo's import entry and adaptive sheet through the shared kit dialog.

use super::Page;
use super::intent::{Action, listener, state};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{CompactLayout, DesktopHeight, DialogInput, DialogLayout};
use vessel_app_kit::sheet::snap::{Percentage, SnapPoints};
use vessel_app_kit::theme::spacing::{S1, S2, S4, S8};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonSize, ButtonState, ControlSizing,
    DialogContent, DialogSize, Tokens, Type, TypeExt as _, button_control, button_with_kind,
};
use vessel_app_view::history::Empty;

/// Paseo's `desktopMaxWidth` for this sheet.
const WIDTH: u16 = 560;
/// The shared dialog's desktop height cap, content-sized below it.
const DESKTOP_CAP: u8 = 85;
/// Paseo's `subtitleAction`: the ghost button pulled 8 into the line's gap.
const SUBTITLE_ACTION_PULL: f32 = -8.0;
/// Paseo's compact snap points for this sheet (70 % and 92 %).
const SNAP: (u8, u8) = (70, 92);

/// The new-conversation affordance mirrors Paseo's import pill.
pub(crate) fn entry(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    button_with_kind(
        "history-open",
        shell.words.history().title(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(|shell, _, window, cx| shell.open_import(window, cx)))
    .into_any_element()
}

/// Paseo's `AdaptiveModalSheet` geometry: 560 at most on a desktop, a sheet
/// dragged between its two snap points on a phone.
fn size() -> DialogSize {
    Percentage::new(DESKTOP_CAP)
        .zip(SnapPoints::new(SNAP.0, SNAP.1))
        .and_then(|(cap, points)| {
            DialogLayout::new(
                WIDTH,
                DesktopHeight::AtMost(cap),
                CompactLayout::Draggable(points),
            )
        })
        .map_or(DialogSize::Normal, DialogSize::Configured)
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.import;
    if !area.open {
        return None;
    }
    let compact = crate::areas::draft::compact(window);
    let words = shell.words.history();
    let search = area.search.header_search(tokens, cx);
    let input = match area.page {
        Page::Hosts => DialogInput::from(DialogContent {
            title: words.choose_host().into(),
            body: super::hosts::body(shell, tokens, cx).into_any_element(),
            footer: Vec::new(),
            size: size(),
        })
        .with_header_search(search),
        Page::Sessions => DialogContent {
            title: words.title().into(),
            body: body(shell, tokens, compact, cx).into_any_element(),
            footer: Vec::new(),
            size: size(),
        }
        .with_header_actions(refresh(shell, tokens, compact, cx))
        .with_header_search(search)
        .with_header_subtitle(scope_line(shell, tokens, compact, cx)),
    };
    let modal = vessel_app_kit::dialog(
        &area.dialog,
        input,
        tokens,
        window,
        compact,
        listener(shell, Action::Close),
        cx,
    );
    Some(
        div()
            .child(modal)
            .children(super::filter::list(shell, tokens, window, cx))
            .children(area.tips.render(tokens))
            .into_any_element(),
    )
}

/// Paseo's `RefreshAction`: a 16 `RotateCw` (a spinner while reading) in a
/// 32 square, radius 8, `surface2` only while pressed; its own pressable in
/// Paseo too, not the shared button.
fn refresh(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let enabled = state(&Action::Refresh, shell) == ButtonState::Enabled;
    let reading = area.model.loading();
    let ink = tokens.neutral.foreground_muted;
    let pressed = tokens.neutral.surface2;
    let act = listener(shell, Action::Refresh);
    let button = div()
        .id("history-refresh")
        .debug_selector(|| "history-refresh".to_owned())
        .role(gpui::Role::Button)
        .aria_label(words.refresh().to_owned())
        .p(px(S2))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .flex()
        .items_center()
        .justify_center()
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .active(move |style| style.bg(pressed))
                .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx)))
        })
        .child(
            div()
                .size(px(vessel_app_kit::theme::icon::MD))
                .flex()
                .items_center()
                .justify_center()
                .child(if reading {
                    vessel_app_kit::slider::spinner(
                        "history-refresh-spinner",
                        vessel_app_kit::theme::icon::MD,
                        ink,
                    )
                } else {
                    vessel_app_kit::glyph(Glyph::RotateCw, vessel_app_kit::theme::icon::MD, ink)
                        .into_any_element()
                }),
        );
    // Paseo's 4 after the action, before the close button.
    div().mr(px(S1)).child(area.tips.on(
        button,
        &area.refresh_place,
        words.refresh().to_owned(),
        compact,
    ))
}

/// Paseo's `ScopeSubtitle`: the scope in 12 muted, one line, and from a
/// workspace a small ghost "Show all" pulled 8 into the gap.
fn scope_line(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let scoped = area.model.scoped();
    let text = if scoped {
        words.scope_workspace().to_owned()
    } else {
        let host = area
            .model
            .opening()
            .map_or("", |opening| opening.machine.as_str());
        words.scope_host(host).unwrap_or_else(|_| host.to_owned())
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S2))
        .min_w_0()
        .child(
            div()
                .debug_selector(|| "history-scope".to_owned())
                .flex_shrink()
                .min_w_0()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .truncate()
                .child(text),
        )
        .when(scoped, |line| {
            line.child(
                div()
                    .mx(px(SUBTITLE_ACTION_PULL))
                    .child(small_button(
                        "history-show-all".to_owned(),
                        words.show_all(),
                        Action::ShowAll,
                        shell,
                        tokens,
                        compact,
                        cx,
                    )),
            )
        })
}

/// Paseo's `Button variant="ghost" size="sm"`.
pub(super) fn small_button(
    id: String,
    label: &str,
    action: Action,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let config = ButtonConfig::new(
        ButtonKind::Ghost,
        state(&action, shell),
        ControlSizing::from_compact(compact),
    )
    .with_size(ButtonSize::Small);
    let act = listener(shell, action);
    let selector = id.clone();
    div()
        .debug_selector(move || selector.clone())
        .child(
            button_control(
                ElementId::Name(id.into()),
                ButtonContent::Label(label.to_owned().into()),
                config,
                tokens,
            )
            .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx))),
        )
        .into_any_element()
}

/// Paseo's list header (filter, status, provider errors; 16 apart, 16
/// below), the rows, then its footer ("Load more", the empty state).
fn body(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.import;
    let rows = area.model.rows(&area.query);
    let mut header: Vec<AnyElement> = Vec::new();
    if area.model.providers().len() > 1 {
        header.push(super::filter::pill(shell, tokens, cx));
    }
    header.extend(status(shell, tokens, !rows.is_empty()));
    let failures = area.model.failures();
    if !failures.is_empty() {
        header.push(super::rows::failures(shell, &failures, tokens, compact, cx));
    }
    let empty = area.model.empty_state(&area.query);
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .when(!header.is_empty(), |body| {
            body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(S4))
                    .pb(px(S4))
                    .children(header),
            )
        })
        .child(super::rows::list(shell, &rows, tokens, cx))
        .when(area.model.has_more(), |body| {
            body.child(
                div()
                    .flex()
                    .justify_center()
                    .pt(px(S2))
                    .child(more(shell, tokens, compact, cx)),
            )
        })
        .children(empty.map(|empty| empty_state(shell, &empty, tokens)))
}

fn more(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let config = ButtonConfig::new(
        ButtonKind::Ghost,
        state(&Action::More, shell),
        ControlSizing::from_compact(compact),
    );
    let act = listener(shell, Action::More);
    button_control(
        "history-more",
        ButtonContent::Label(shell.words.history().more().to_owned().into()),
        config,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx)))
    .into_any_element()
}

/// Paseo's `SheetStatusMessages`: 14 muted lines; loading is a spinner row
/// only while nothing is listed.
fn status(shell: &Shell, tokens: &Tokens, has_rows: bool) -> Vec<AnyElement> {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let ink = tokens.neutral.foreground_muted;
    let line = |text: String| {
        div()
            .type_(Type::Body)
            .text_color(ink)
            .child(text)
            .into_any_element()
    };
    let mut lines = Vec::new();
    if area.model.providers().is_empty() {
        lines.push(line(words.no_providers().to_owned()));
    }
    if area.model.loading() && !has_rows {
        lines.push(
            div()
                .debug_selector(|| "history-loading".to_owned())
                .flex()
                .flex_row()
                .items_center()
                .gap(px(S2))
                .py(px(S2))
                .child(vessel_app_kit::slider::spinner(
                    "history-loading-spinner",
                    vessel_app_kit::theme::icon::MD,
                    ink,
                ))
                .child(div().type_(Type::Body).text_color(ink).child(words.loading()))
                .into_any_element(),
        );
    }
    if area.model.import_failed().is_some() {
        lines.push(line(words.failed_import().to_owned()));
    }
    if let Some(problem) = area.failure {
        lines.push(line(problem.words(shell.words).to_owned()));
    }
    lines
}

/// Paseo's `SheetEmptyState`: a 20 `Inbox` at 0.6 over a centred 14 title,
/// 32 by 16 around them, 8 apart.
fn empty_state(shell: &Shell, empty: &Empty, tokens: &Tokens) -> AnyElement {
    let words = shell.words.history();
    let title = match empty {
        Empty::NoMatches => words.empty_matches().to_owned(),
        Empty::NoProviderSessions(harness) => {
            let name = vessel_app_view::agent_name(harness);
            words.empty_provider(&name).unwrap_or(name)
        }
        Empty::AlreadyImported => words.empty_imported().to_owned(),
        Empty::NoRecent => words.empty_recent().to_owned(),
    };
    div()
        .debug_selector(|| "history-empty".to_owned())
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(S2))
        .py(px(S8))
        .px(px(S4))
        .child(
            div().mb(px(S1)).opacity(0.6).child(vessel_app_kit::glyph(
                Glyph::Inbox,
                vessel_app_kit::theme::icon::LG,
                tokens.neutral.foreground_muted,
            )),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .text_center()
                .child(title),
        )
        .into_any_element()
}
