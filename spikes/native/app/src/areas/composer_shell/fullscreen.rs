//! One retained input fills the compact viewport; attachments keep their dock owner.
//! Escape is handled by the IME-aware composer hook, never a second key listener.

use gpui::{anchored, deferred};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::field::composer_layout::ComposerLayout;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, point, px,
};
use vessel_app_kit::{PillParts, SendKind, Tokens, pill_parts, send_button};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::{Access, Extent};

use crate::shell::Shell;

const HEADER: f32 = spacing::S16;
const FOOTER: f32 = 68.0;
/// Header, footer and the two editor insets leave one bounded input slot.
const CHROME: f32 = 164.0;
const INSET: f32 = spacing::S4;

pub(crate) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let target = shell.composer_fullscreen_target()?;
    if shell.areas.composer_shell.surface(target).extent() != Extent::Fullscreen {
        return None;
    }
    let surface = shell.composer_surface(target, cx)?;
    if !super::super::draft::compact(window) || surface.access() != Access::Editable {
        shell.areas.composer_shell.collapse(target);
        return None;
    }
    let size = window.viewport_size();
    let layout = ComposerLayout::fullscreen(size.height - px(CHROME), shell.composer_font(target));
    let text = shell.composer_fullscreen_text(target, layout, tokens, cx)?;
    let enabled = shell.composer_may_submit(target, cx);
    let close = pill_parts(
        "composer-collapse",
        PillParts {
            leading: Some(Glyph::Minimize),
            ..PillParts::default()
        },
        false,
        true,
        tokens,
    )
    .role(gpui::Role::Button)
    .aria_label(shell.words.composer_shell().exit_fullscreen().to_owned())
    .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
        window.prevent_default()
    })
    .on_click(cx.listener(move |shell, _, window, cx| shell.composer_collapse(target, window, cx)));
    let send = send_button("fullscreen-send", SendKind::Send, enabled, true, tokens).on_click(
        cx.listener(move |shell, _, window, cx| {
            shell.composer_submit_fullscreen(target, window, cx)
        }),
    );
    let overlay = div()
        .id("composer-fullscreen")
        .w(size.width)
        .h(size.height)
        .flex()
        .flex_col()
        .occlude()
        .bg(tokens.neutral.surface1)
        .text_color(tokens.neutral.foreground)
        .child(
            div()
                .h(px(HEADER))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_end()
                .px(px(INSET))
                .child(close),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(spacing::S0))
                .overflow_hidden()
                .p(px(INSET))
                .child(text),
        )
        .child(
            div()
                .h(px(FOOTER))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_end()
                .px(px(INSET))
                .child(send),
        );
    Some(
        deferred(
            anchored()
                .position(point(px(spacing::S0), px(spacing::S0)))
                .child(overlay),
        )
        .with_priority(2)
        .into_any_element(),
    )
}

impl Shell {
    fn composer_may_submit(&self, target: DraftKey, cx: &gpui::App) -> bool {
        if self.composer_fullscreen_target() != Some(target) {
            return false;
        }
        let Some(field) = self.composer_shell_field(target) else {
            return false;
        };
        if self
            .composer_surface(target, cx)
            .is_none_or(|surface| surface.access() != Access::Editable)
        {
            return false;
        }
        let text = field.text(cx);
        match target {
            DraftKey::Session { .. } => self.session_may_send(&text),
            DraftKey::New => {
                (!text.trim().is_empty() || self.draft_has_files())
                    && self.areas.draft.agent.is_some()
                    && self.areas.draft.creation.flow.may_submit()
            }
        }
    }

    fn composer_submit_fullscreen(
        &mut self,
        target: DraftKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.composer_may_submit(target, cx) {
            return;
        }
        let Some(field) = self.composer_shell_field(target) else {
            return;
        };
        let text = field.text(cx);
        self.areas.composer_shell.collapse(target);
        match target {
            DraftKey::New => self.send_draft(text, window, cx),
            DraftKey::Session { .. } => self.submit_prompt(text, window, cx),
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
