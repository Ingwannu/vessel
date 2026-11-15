//! Existing composer input wrappers: menu/recall/attachment hook order stays shared.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, hooked};
use vessel_app_kit::field::composer_layout::ComposerLayout;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{PillParts, Tokens, pill_parts};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::{Extent, Layout, RenderedLines};

use super::Over;
use crate::shell::Shell;

/// The session composer's text with its keys offered to the menu first
/// (`field::composer_hooks`); pasted images and files are attached
/// (`files.rs`).
pub(crate) fn text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    inline(shell, Over::Session, tokens, touch, window, cx)
}

/// The new chat's composer text, its keys offered to its `/` menu first
/// (the draft's card draws it in place of the bare field).
pub(crate) fn draft_text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    inline(shell, Over::Draft, tokens, touch, window, cx)
}

fn target(shell: &Shell, over: Over) -> Option<DraftKey> {
    match over {
        Over::Draft => Some(DraftKey::New),
        Over::Session => shell
            .areas
            .session
            .open
            .as_ref()
            .map(|open| DraftKey::Session {
                session: open.session,
            }),
    }
}

fn inline(
    shell: &Shell,
    over: Over,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(target) = target(shell, over) else {
        return div().into_any_element();
    };
    let Some(surface) = shell.composer_surface(target, cx) else {
        return div().into_any_element();
    };
    // The dock never mounts the same Entity while the fullscreen host mounts it.
    if surface.extent() == Extent::Fullscreen && touch {
        return div().min_h(px(spacing::S8)).into_any_element();
    }
    let layout = shell.composer_inline_layout(target, touch, window);
    let input = hooked_text(shell, over, layout, tokens, cx);
    let lines = shell.slash_field(over).rendered_composer_rows(cx);
    let can_expand = touch
        && lines.is_some_and(|rows| {
            surface.can_expand(Layout::Compact, RenderedLines::new(rows.get()))
        });
    let expand = can_expand.then(|| {
        pill_parts(
            "composer-expand",
            PillParts {
                leading: Some(Glyph::Maximize),
                label: Some(shell.words.composer_shell().fullscreen().to_owned().into()),
                ..PillParts::default()
            },
            false,
            touch,
            tokens,
        )
        .on_click(
            cx.listener(move |shell, _, window, cx| shell.composer_expand(target, window, cx)),
        )
    });
    div()
        .flex()
        .flex_col()
        .child(input)
        .children(expand.map(|button| div().flex().justify_end().child(button)))
        .into_any_element()
}

impl Shell {
    pub(crate) fn composer_fullscreen_text(
        &self,
        target: DraftKey,
        layout: ComposerLayout,
        tokens: &Tokens,
        cx: &mut Context<'_, Self>,
    ) -> Option<AnyElement> {
        self.composer_field(target)?;
        let over = match target {
            DraftKey::New => Over::Draft,
            DraftKey::Session { .. } => Over::Session,
        };
        Some(hooked_text(self, over, layout, tokens, cx))
    }
}

fn hooked_text(
    shell: &Shell,
    over: Over,
    layout: ComposerLayout,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let field = shell.slash_field(over);
    let element = field.composer_element(layout, tokens, cx);
    let weak = cx.entity().downgrade();
    let pasting = weak.clone();
    let new_chat = over == Over::Draft;
    let target = target(shell, over);
    hooked(
        field,
        element,
        move |key, window: &mut Window, cx: &mut App| {
            weak.update(cx, |shell, cx| {
                let Some(target) = target.filter(|target| shell.composer_field(*target).is_some())
                else {
                    return KeyAnswer::Pass;
                };
                if shell.areas.composer_shell.surface(target).extent() == Extent::Fullscreen {
                    if key == vessel_app_kit::field::composer_hooks::ComposerKey::Escape {
                        shell.composer_collapse(target, window, cx);
                        return KeyAnswer::Taken;
                    }
                    return KeyAnswer::Pass;
                }
                if shell.areas.composer_shell.permits_menu(target)
                    && shell.composer_file_key(target, key, window, cx) == KeyAnswer::Taken
                {
                    return KeyAnswer::Taken;
                }
                shell.slash_key(over, key, window, cx)
            })
            .unwrap_or(KeyAnswer::Pass)
        },
        // Pasted images and files are the composer's attachments.
        move |pasted, _window, cx: &mut App| {
            pasting
                .update(cx, |shell, cx| {
                    if target.is_none_or(|target| shell.composer_field(target).is_none()) {
                        return KeyAnswer::Pass;
                    }
                    shell.composer_pasted(new_chat, pasted, cx)
                })
                .unwrap_or(KeyAnswer::Pass)
        },
    )
}
