//! `ProviderCatalogList` composition through shared furniture and typed offers.
use crate::shell::Shell;
use std::{cell::Cell, rc::Rc};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, loading, settings, theme::spacing};
use vessel_app_view::agent_status::catalogue::{self, Body, Read};
mod actions;
mod tile;
pub(super) struct State {
    query: TextField,
    width: Rc<Cell<gpui::Pixels>>,
    clear: Cell<bool>,
}
impl State {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            query: TextField::with_change(
                "",
                super::add::edit_only,
                super::add::changed,
                window,
                cx,
            ),
            width: Rc::default(),
            clear: Cell::new(false),
        }
    }
    pub(super) fn changed_machine(&self) {
        self.clear.set(true);
    }
}
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings.agents.catalogue;
    let words = shell.words.agent_catalog();
    if state.clear.replace(false) {
        state.query.clear(window, cx);
    }
    state.query.set_placeholder(words.search(), window, cx);
    let query = state.query.text(cx);
    let read = super::page(shell).map_or(Read::NotRequested, |page| page.catalogue());
    let presentation = catalogue::present(read, &query);
    let mut body = div().flex().flex_col().gap(px(spacing::S3)).child(
        div()
            .flex()
            .items_center()
            .gap(px(spacing::S3))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(state.query.element(tokens, compact, cx)),
            )
            .child(actions::command(shell, tokens, compact, cx)),
    );
    if let Some(error) = presentation.retained_failure {
        body = body.child(problem(error, shell, tokens));
    }
    body = body.child(match presentation.body {
        Body::NotRequested => state_box(words.not_read(), tokens),
        Body::Loading => state_box(words.reading(), tokens).child(loading::activity_indicator(
            "registry-initial",
            loading::Size::Small,
            tokens.neutral.foreground_muted,
        )),
        Body::Failed(error) => div().child(problem(error, shell, tokens)),
        Body::Empty => state_box(words.empty(), tokens),
        Body::NoMatches => state_box(words.no_matches(), tokens),
        Body::Rows { first, rest } => tile::grid(
            std::iter::once(first).chain(rest),
            shell,
            tokens,
            compact,
            cx,
        ),
    });
    settings::section_with_trailing(
        words.title(),
        body,
        Some(actions::head(shell, tokens, compact, cx)),
        tokens,
    )
    .mb(px(spacing::S4))
    .into_any_element()
}
fn state_box(text: &str, tokens: &Tokens) -> Div {
    settings::card(tokens)
        .min_h(px(STATE_HEIGHT))
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::S3))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(text.to_owned()),
        )
}
const STATE_HEIGHT: f32 = 96.0;
fn problem(error: &vessel_app_link::agents::AgentsError, shell: &Shell, tokens: &Tokens) -> Div {
    let problem = error.catalogue_problem();
    super::with_original(
        vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(shell.words.agent_catalog().problem(problem).into()),
                description: None,
                action: None,
            },
            crate::badge::alert_variant(problem.variant()),
            vessel_app_kit::alert::Size::Small,
            tokens,
        ),
        error.to_string(),
        shell,
    )
}
