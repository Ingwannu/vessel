//! Paseo's provider filter: a pill over the list and its non-searchable list
//! ("All", then each provider), opened below the pill at its start.

use super::intent::{Action, Capture, listener};
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing::{S1_5, S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ComboHandlers, ComboRow, Leading, Tokens, Type, TypeExt as _, combobox};
use vessel_app_view::history::Filter;

/// The filter's choices in Paseo's order ("All", then the providers sorted
/// by their id, `[...providersToFetch].sort()`), each with its words and
/// its 14 muted leading.
fn choices(shell: &Shell) -> Vec<(Filter, String, Leading)> {
    let mut providers = shell.areas.import.model.providers();
    providers.sort_by_key(ToString::to_string);
    let mut choices = vec![(
        Filter::All,
        shell.words.history().filter_all().to_owned(),
        Leading::SmallGlyph(Glyph::Layers),
    )];
    choices.extend(providers.into_iter().map(|harness| {
        (
            Filter::Provider(harness.clone()),
            vessel_app_view::agent_name(harness),
            Leading::SmallAgentMark(crate::agent_mark::of(harness)),
        )
    }));
    choices
}

/// Paseo's filter trigger: the choice's 14 icon, its words (14 medium) and a
/// 14 chevron; 6 by 12, radius 6, `surface1` with a hairline `border`,
/// `surface2` hovered, `surface3` pressed; 8 above the list.
pub(super) fn pill(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let area = &shell.areas.import;
    let neutral = tokens.neutral;
    let current = area.model.filter();
    let (label, icon) = match current {
        Filter::All => (
            shell.words.history().filter_all().to_owned(),
            vessel_app_kit::glyph(Glyph::Layers, 14.0, neutral.foreground_muted).into_any_element(),
        ),
        Filter::Provider(harness) => (
            vessel_app_view::agent_name(harness),
            crate::agent_mark::mark_muted(
                Some(harness),
                vessel_app_kit::agent_mark::Size::Fourteen,
                vessel_app_kit::agent_mark::Space::Compact,
                tokens,
            )
            .into_any_element(),
        ),
    };
    let act = listener(shell, Action::FilterOpen);
    let hover = neutral.surface2;
    let pressed = neutral.surface3;
    div()
        .flex()
        .pb(px(S2))
        .child(
            div()
                .id("history-filter")
                .debug_selector(|| "history-filter".to_owned())
                .role(gpui::Role::Button)
                .aria_label(
                    shell
                        .words
                        .history()
                        .filter_label(&label)
                        .unwrap_or_else(|_| label.clone()),
                )
                .relative()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(S1_5))
                .max_w_full()
                .py(px(S1_5))
                .px(px(S3))
                .rounded(px(vessel_app_kit::theme::radius::MD))
                .bg(neutral.surface1)
                .border_1()
                .border_color(neutral.border)
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(pressed))
                .child(vessel_app_kit::anchor_probe(&area.filter_anchor))
                .child(icon)
                .child(
                    div()
                        .min_w_0()
                        .type_(Type::Label)
                        .text_color(neutral.foreground)
                        .truncate()
                        .child(label),
                )
                .child(vessel_app_kit::glyph(
                    Glyph::ChevronDown,
                    14.0,
                    neutral.foreground_muted,
                ))
                .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx))),
        )
        .into_any_element()
}

impl Shell {
    pub(super) fn history_filter_open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.import;
        area.filter.set_anchor(area.filter_anchor.get());
        area.filter.open(None, window, cx);
    }
}

/// The open list: "Filter by provider" titles it on a phone only, as
/// Paseo's combobox titles only its sheet.
pub(super) fn list(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.import;
    if !area.filter.is_open() {
        return None;
    }
    let current = area.model.filter().clone();
    let (filters, rows): (Vec<Filter>, Vec<ComboRow>) = choices(shell)
        .into_iter()
        .map(|(filter, label, leading)| {
            let mut row = ComboRow::new(label);
            row.leading = Some(leading);
            row.checked = filter == current;
            (filter, row)
        })
        .unzip();
    let mut header = vessel_app_kit::select::header();
    if crate::areas::draft::compact(window) {
        header.title = Some(shell.words.history().filter_title().to_owned().into());
    }
    let capture = Capture::of(shell);
    combobox(
        "history-filter-list",
        &area.filter,
        &header,
        &rows,
        &ComboHandlers {
            state: |shell: &mut Shell| &mut shell.areas.import.filter,
            chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
                let Some(filter) = filters.get(index) else {
                    return;
                };
                if !capture.accepts(shell) {
                    return;
                }
                listener(shell, Action::Filter(filter.clone()))(shell, window, cx);
            }),
            back: None,
        },
        vessel_app_kit::select::WIDTH,
        tokens,
        window,
        cx,
    )
}
