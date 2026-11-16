//! Settings navigation form from the one shared sidebar arrangement.
//! The body retains its existing control-density policy; this owns only
//! whether a sidebar can coexist or the existing list/back flow is needed.

use gpui::AnimationExt as _;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::settings::Section;

use super::{agents, page, sections};
use vessel_app_view::panels::{Sharing, SidebarLayout};

use crate::{areas::sidebar, routes::Route, shell::Shell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Form {
    Split,
    Stacked,
}

pub(super) fn current(shell: &Shell, window: &Window) -> Form {
    match sidebar::arrangement(shell, window) {
        SidebarLayout::Docked {
            sharing: Sharing::Fits,
            ..
        } => Form::Split,
        SidebarLayout::Sheet
        | SidebarLayout::Hidden
        | SidebarLayout::Docked {
            sharing: Sharing::TooNarrow,
            ..
        } => Form::Stacked,
    }
}

impl Shell {
    /// A popup's old anchor must not survive a navigation-form change.
    /// This uses the existing controller settling point and combobox owner.
    pub(super) fn settle_settings_form(&mut self, window: &Window) {
        if self.route != Route::Settings {
            self.areas.settings.last_form = None;
            return;
        }
        let next = current(self, window);
        let state = &mut self.areas.settings;
        if state.last_form.replace(next).is_some_and(|old| old != next) {
            state.open = None;
            state.list.close();
        }
    }
}

/// The section shown, as a page: its title, a refused change above its
/// body, the body fading in when it has just arrived.
pub(super) fn section_page(
    shell: &Shell,
    tokens: &Tokens,
    form: Form,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings;
    // A section no longer listed (its machine left the book) gives way to
    // the first one, as opening it would.
    let section = if sections::listed(state.section, shell) {
        state.section
    } else {
        Section::DEFAULT
    };
    let title = shell.t.lang.settings().section(section, shell.words);
    let body = sections::draw(section, shell, tokens, phone, window, cx)
        .unwrap_or_else(|| div().into_any_element());
    let body = div()
        .flex()
        .flex_col()
        .children(state.refused.as_ref().map(|refused| {
            div()
                .mb(px(spacing::S4))
                .px(px(spacing::S1))
                .type_(Type::Body)
                .text_color(tokens.neutral.destructive)
                .child(refused.words.clone())
        }))
        .child(body);
    let body = if state.turn == 0 {
        body.into_any_element()
    } else {
        body.with_animation(
            ElementId::NamedInteger("settings-page".into(), state.turn),
            vessel_app_kit::motion::transition(vessel_app_kit::motion::STANDARD),
            gpui::Styled::opacity,
        )
        .into_any_element()
    };
    // Providers (the agents' page) keeps a scroll position per machine and
    // per agent, and an agent's detail has its own header.
    let (scroll_id, detail) = if section == Section::Providers {
        (agents::scroll_id(shell), agents::header(shell))
    } else {
        ("settings-page".into(), None)
    };
    page::layout(
        title,
        body,
        form == Form::Stacked,
        scroll_id,
        detail,
        tokens,
        cx,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests;
