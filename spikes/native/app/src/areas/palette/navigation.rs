//! A nested-page chip and empty-query Backspace use the same parent operation.
//! Input composition and native editing keep their keys before this scope affordance.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::palette::PaletteMode;

pub(super) fn scope(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.areas.palette.mode != PaletteMode::Commands {
        return None;
    }
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

impl Shell {
    /// The open center's existing kit controls run before global rebinding,
    /// matching the top-overlay priority in Paseo. This query is read-only;
    /// the existing frame/search remains its sole navigation controller.
    pub(crate) fn command_center_owns_key(&self, event: &gpui::KeystrokeEvent) -> bool {
        let modifiers = event.keystroke.modifiers;
        self.areas.palette.is_open()
            && !(modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform)
            && (matches!(
                event.keystroke.key.as_str(),
                "up" | "down" | "enter" | "escape"
            ) || (event.keystroke.key == "backspace"
                && self.areas.palette.mode == PaletteMode::Commands
                && vessel_app_view::matcher::Query::new(&self.areas.palette.query).is_empty()
                && self.areas.palette.page.is_some()))
    }

    /// The sole interceptor has already admitted IME/capture/overlay input.
    /// GPUI resolves TextField Backspace actions before raw key listeners, so
    /// invoke the existing page operation before that action can delete text.
    pub(in crate::areas) fn command_center_backspace(
        &mut self,
        event: &gpui::KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if event.keystroke.key != "backspace" || !self.command_center_owns_key(event) {
            return false;
        }
        self.palette_back(window, cx);
        true
    }
}
