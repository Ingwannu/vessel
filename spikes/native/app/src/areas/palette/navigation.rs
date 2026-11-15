//! A nested-page chip and empty-query Backspace use the same parent operation.
//! Input composition and native editing keep their keys before this scope affordance.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, TextField, Tokens, Type, TypeExt as _, button_with_kind,
};

pub(super) fn scope(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let page = shell.areas.palette.page.as_ref()?;
    let breadcrumbs = page.breadcrumbs().collect::<Vec<_>>().join(" › ");
    Some(
        div()
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(
                button_with_kind(
                    "command-page-back",
                    shell.words.shortcuts().back(),
                    ButtonKind::Ghost,
                    ButtonState::Enabled,
                    tokens,
                )
                .on_click(cx.listener(|shell, _, window, cx| shell.palette_back(window, cx))),
            )
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(breadcrumbs),
            )
            .into_any_element(),
    )
}

pub(super) fn key(
    shell: &mut Shell,
    event: &gpui::KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let modifiers = event.keystroke.modifiers;
    if event.prefer_character_input
        || TextField::focused_composing(window, cx)
        || modifiers.control
        || modifiers.alt
        || modifiers.shift
        || modifiers.platform
    {
        return;
    }
    if event.keystroke.key == "backspace"
        && vessel_app_view::matcher::Query::new(&shell.areas.palette.query).is_empty()
        && shell.areas.palette.page.is_some()
    {
        shell.palette_back(window, cx);
        cx.stop_propagation();
    }
}

impl Shell {
    /// The open center's existing kit controls run before global rebinding,
    /// matching the top-overlay priority in Paseo. This only yields the event;
    /// the existing frame/search remains its sole navigation controller.
    pub(crate) fn command_center_owns_key(&self, event: &gpui::KeystrokeEvent) -> bool {
        let modifiers = event.keystroke.modifiers;
        self.areas.palette.is_open()
            && !(modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform)
            && (matches!(
                event.keystroke.key.as_str(),
                "up" | "down" | "enter" | "escape"
            ) || (event.keystroke.key == "backspace"
                && vessel_app_view::matcher::Query::new(&self.areas.palette.query).is_empty()
                && self.areas.palette.page.is_some()))
    }
}
