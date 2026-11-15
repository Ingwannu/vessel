//! The provider sheet's read-only model rows in the existing Settings frame.
use crate::{routes::Route, shell::Shell};
use std::cell::Cell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{SearchField, Tokens, Type, TypeExt as _, settings, theme::spacing};
use vessel_app_link::{
    Command,
    models::{Answer, Ask, ModelsError},
};
use vessel_app_view::{
    models::{self, page::StartProblem},
    settings::Section,
};

mod actions;
mod body;
mod rows;

#[derive(Clone, Copy)]
enum SearchReset {
    Keep,
    Clear,
}
pub(super) struct State {
    page: models::page::State<ModelsError>,
    refused: Option<StartProblem>,
    query: SearchField,
    reset: Cell<SearchReset>,
}
impl State {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            page: models::page::State::default(),
            refused: None,
            query: SearchField::new("", |_shell: &mut Shell, _text, cx| cx.notify(), window, cx),
            reset: Cell::new(SearchReset::Keep),
        }
    }
}
impl Shell {
    pub(crate) fn models_connected(&mut self, connection: &models::page::Connection) {
        let state = &mut self.areas.settings.models;
        if state.page.connected(connection) {
            state.refused = None;
        }
    }
    /// Like the other machine sections: navigation is reconciled before drawing.
    pub(crate) fn settle_models(&mut self, window: &Window) {
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Models
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let selection = super::choices::chosen_machine(self)
            .filter(|_| shown)
            .map(|machine| models::page::Selection {
                machine: vessel_app_link::agents::MachineKey::from_book(machine.name.as_str()),
                node: machine.node,
            });
        let state = &mut self.areas.settings.models;
        if state.page.select_reported(selection) {
            state.refused = None;
            state.reset.set(SearchReset::Clear);
        }
        let page = state.page.page();
        if state.page.machine().is_some()
            && page.pending().is_none()
            && page.snapshot().is_none()
            && page.problem().is_none()
            && state.refused.is_none()
        {
            self.models_refresh();
        }
    }
    fn models_refresh(&mut self) {
        let state = &mut self.areas.settings.models;
        let Some(machine) = state.page.machine().cloned() else {
            return;
        };
        match state.page.begin() {
            Ok(ticket) => {
                state.refused = None;
                self.command(Command::Models(Ask { machine, ticket }));
            }
            Err(problem) => state.refused = Some(problem),
        }
    }
    pub(crate) fn models_answer(&mut self, answer: Answer) {
        let state = &mut self.areas.settings.models;
        if state
            .page
            .received(&answer.machine, answer.ticket, answer.result)
        {
            state.refused = None;
        }
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings.models;
    let words = shell.words.models();
    if matches!(state.reset.replace(SearchReset::Keep), SearchReset::Clear) {
        state.query.clear(window, cx);
    }
    state.query.set_placeholder(words.search(), window, cx);
    let query = state.query.text(cx);
    let presentation =
        models::presentation(&state.page, state.refused, &query, ModelsError::problem);
    if matches!(&presentation.content, models::Content::NoMachine) {
        return settings::section(
            words.routes(),
            empty(tokens).child(text(words.no_machine(), tokens)),
            tokens,
        )
        .into_any_element();
    }
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
    if let Some(issue) = presentation.retained_issue {
        body = body.child(notice(issue, shell, tokens));
    }
    body = body.child(text(words.read_only(), tokens)).child(
        div()
            .flex()
            .w_full()
            .child(state.query.element("models-search", tokens, cx)),
    );
    body = body.child(body::render(
        presentation.content,
        presentation.refresh,
        shell,
        tokens,
        compact,
        window,
        cx,
    ));
    settings::section_with_trailing(
        words.routes(),
        body,
        Some(actions::refresh(
            presentation.refresh,
            shell,
            tokens,
            compact,
            cx,
        )),
        tokens,
    )
    .into_any_element()
}
fn text(value: impl Into<gpui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(value.into())
}
fn empty(tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(spacing::S3))
        .py(px(spacing::S8))
        .text_color(tokens.neutral.foreground_muted)
}
fn notice(issue: models::Issue, shell: &Shell, tokens: &Tokens) -> Div {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(shell.words.models().issue(issue).into()),
            description: None,
            action: None,
        },
        crate::badge::alert_variant(issue.variant()),
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
}
