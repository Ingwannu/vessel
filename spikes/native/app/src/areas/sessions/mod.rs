//! Durable session history and lifecycle navigation; provider import lives elsewhere.
mod filters;
mod lifecycle;
#[cfg(test)]
mod tests;
pub(crate) use lifecycle::{error as lifecycle_error, overlay as lifecycle_overlay};
mod navigation;
#[cfg(test)]
pub(crate) mod resident;
mod rows;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, breakpoint, outline_button};
use vessel_app_view::sessions::{
    HistoryAvailability, HistoryPage, HostScope, history, history_availability,
};

/// History fields survive navigation and language changes without losing input.
pub(crate) struct SessionsArea {
    lifecycle: lifecycle::State,
    navigation: vessel_app_view::sessions::Navigation,
    pending_open: Option<vessel_app_link::sessions::Located>,
    query: TextField,
    search: String,
    page: HistoryPage,
    scope: HostScope,
    filters: filters::Filters,
}
impl SessionsArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            lifecycle: lifecycle::State::new(cx),
            navigation: vessel_app_view::sessions::Navigation::default(),
            pending_open: None,
            query: TextField::with_change(
                "",
                |_, _, _, _| {},
                Shell::sessions_searched,
                window,
                cx,
            ),
            search: String::new(),
            page: HistoryPage::default(),
            scope: HostScope::All,
            filters: filters::Filters::new(window, cx),
        }
    }
}
impl Shell {
    /// Forget a removed host filter so its now-hidden picker cannot trap history.
    pub(crate) fn sessions_fleet_changed(&mut self) {
        let area = &mut self.areas.sessions;
        if let HostScope::Host(name) = &area.scope
            && !self
                .fleet
                .machines
                .iter()
                .any(|machine| machine.name == name.as_str())
        {
            area.scope = HostScope::All;
            area.page = HistoryPage::default();
        }
    }

    fn sessions_searched(&mut self, text: String, cx: &mut Context<'_, Self>) {
        self.areas.sessions.search = text;
        self.areas.sessions.page = HistoryPage::default();
        cx.notify();
    }
    pub(crate) fn refresh_sessions_words(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas
            .sessions
            .query
            .set_placeholder(self.words.sessions().search(), window, cx);
    }
}
/// Refresh the saved node directory without starting any session provider.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.refresh_sessions_words(window, cx);
    shell.areas.sessions.query.focus(window, cx);
    shell.reload();
}
/// The history is a complete page; archive remains the board's separate dialog.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.sessions;
    let words = shell.words.sessions();
    let found = history(&shell.fleet, &area.scope, &area.search);
    let shown = area.page.visible(found.len());
    let body = div()
        .id("sessions-history")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .children(rows::failures(shell, tokens))
        .children(rows::list(shell, &found, shown, tokens, cx))
        .when(found.is_empty(), |body| body.child(empty(shell, tokens)))
        .when(area.page.has_more(found.len()), |body| {
            body.child(
                outline_button("sessions-more", words.load_more(), true, tokens)
                    .min_h(px(breakpoint::TOUCH_TARGET))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.areas.sessions.page = shell.areas.sessions.page.next();
                        cx.notify();
                    })),
            )
        });
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(super::common::title_bar(
            words.title().to_owned(),
            None,
            tokens,
        ))
        .child(toolbar(shell, tokens, window, cx))
        .child(body)
        .children(filters::picker(shell, tokens, window, cx))
        .children(super::board::card_question(shell, tokens, window, cx))
        .into_any_element()
}
fn empty(shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.words.sessions();
    let unavailable = match history_availability(&shell.fleet, &shell.areas.sessions.scope) {
        HistoryAvailability::Available => false,
        HistoryAvailability::Unavailable => true,
    };
    let text = if !shell.loaded {
        words.loading()
    } else if unavailable {
        words.unavailable()
    } else if !shell.areas.sessions.search.trim().is_empty() {
        words.no_matches()
    } else {
        words.empty()
    };
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
}

fn toolbar(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.sessions;
    let narrow = breakpoint::phone(window);
    let controls = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .children(filters::trigger(shell, tokens, narrow, cx))
        .child(
            outline_button(
                "sessions-refresh",
                shell.words.sessions().refresh(),
                true,
                tokens,
            )
            .min_h(px(breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.reload();
                cx.notify();
            })),
        );
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .when(narrow, |bar| bar.flex_col().items_stretch())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .when(narrow, |query| query.w_full().flex_none())
                .child(area.query.element(tokens, narrow, cx)),
        )
        .child(controls)
}
