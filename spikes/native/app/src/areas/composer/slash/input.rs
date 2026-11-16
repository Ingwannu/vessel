//! Existing field/layout/hooks are shared by exact draft keys on every surface.
//! Current destination admits actions; rendering does not grant send authority.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, hooked};
use vessel_app_kit::field::composer_layout::ComposerLayout;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{PillParts, Tokens, pill_parts};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::{Access, Extent, Layout, Mode, RenderedLines};

use super::Over;
use crate::shell::Shell;

pub(crate) fn text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    legacy(shell, Over::Session, tokens, touch, window, cx)
}

pub(crate) fn draft_text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    legacy(shell, Over::Draft, tokens, touch, window, cx)
}

/// W3 supplies the exact retained key; absent keys never select global New.
/// Background fields remain readable, with editability supplied by their owner.
pub(crate) fn workspace_text(
    shell: &Shell,
    target: DraftKey,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !matches!(target, DraftKey::Workspace { .. }) {
        return None;
    }
    inline(
        shell,
        target,
        tokens,
        crate::areas::draft::compact(window),
        window,
        cx,
    )
}

fn legacy(
    shell: &Shell,
    over: Over,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let target = match over {
        Over::Draft => Some(DraftKey::New),
        Over::Session => shell
            .areas
            .session
            .open
            .as_ref()
            .map(|open| DraftKey::Session {
                session: open.session,
            }),
    };
    target
        .and_then(|target| inline(shell, target, tokens, touch, window, cx))
        .unwrap_or_else(|| div().into_any_element())
}

fn inline(
    shell: &Shell,
    target: DraftKey,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let surface = shell.composer_surface(target, cx)?;
    let field = shell.composer_field(target)?;
    // Exactly one host mounts this retained Entity at a time.
    if surface.extent() == Extent::Fullscreen && touch {
        return Some(div().min_h(px(spacing::S8)).into_any_element());
    }
    let layout = shell.composer_inline_layout(target, touch, window)?;
    let input = hooked_text(shell, target, layout, tokens, cx)?;
    let can_expand = touch
        && shell.composer_current_target() == Some(target)
        && field.rendered_composer_rows(cx).is_some_and(|rows| {
            surface.can_expand(Layout::Compact, RenderedLines::new(rows.get()))
        });
    let expand = can_expand.then(|| {
        pill_parts(
            "composer-expand",
            PillParts {
                leading: Some(Glyph::Maximize),
                ..PillParts::default()
            },
            false,
            touch,
            tokens,
        )
        .role(gpui::Role::Button)
        .aria_label(shell.words.composer_shell().fullscreen().to_owned())
        .absolute()
        .top(px(spacing::S0))
        .right(px(spacing::S1_5.mul_add(-1.0, 0.0)))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
            window.prevent_default();
        })
        .on_click(
            cx.listener(move |shell, _, window, cx| shell.composer_expand(target, window, cx)),
        )
    });
    Some(
        div()
            .relative()
            .child(input)
            .children(expand)
            .into_any_element(),
    )
}

impl Shell {
    pub(crate) fn composer_fullscreen_text(
        &self,
        target: DraftKey,
        layout: ComposerLayout,
        tokens: &Tokens,
        cx: &mut Context<'_, Self>,
    ) -> Option<AnyElement> {
        let _field = self.composer_shell_field(target)?;
        hooked_text(self, target, layout, tokens, cx)
    }
}

fn hooked_text(
    shell: &Shell,
    target: DraftKey,
    layout: ComposerLayout,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let field = shell.composer_field(target)?;
    let element = field.composer_element(layout, tokens, cx);
    let weak = cx.entity().downgrade();
    let pasting = weak.clone();
    Some(hooked(
        field,
        element,
        move |key, window: &mut Window, cx: &mut App| {
            weak.update(cx, |shell, cx| {
                let Some(_field) = shell.composer_shell_field(target) else {
                    return KeyAnswer::Pass;
                };
                let Some(surface) = shell.composer_surface(target, cx) else {
                    return KeyAnswer::Pass;
                };
                if surface.extent() == Extent::Fullscreen {
                    if key == vessel_app_kit::field::composer_hooks::ComposerKey::Escape {
                        shell.composer_collapse(target, window, cx);
                        return KeyAnswer::Taken;
                    }
                    return KeyAnswer::Pass;
                }
                if surface.autocomplete()
                    && shell.composer_file_key(target, key, window, cx) == KeyAnswer::Taken
                {
                    return KeyAnswer::Taken;
                }
                match Over::for_target(target) {
                    Some(over) => shell.slash_key(over, key, window, cx),
                    // Draft commands have no admitted configuration snapshot yet.
                    // Their provider list cannot come from another draft/session.
                    None => KeyAnswer::Pass,
                }
            })
            .unwrap_or(KeyAnswer::Pass)
        },
        move |pasted, _window, cx: &mut App| {
            pasting
                .update(cx, |shell, cx| {
                    if shell.composer_shell_field(target).is_none()
                        || shell.composer_surface(target, cx).is_none_or(|surface| {
                            surface.mode() != Mode::Chat || surface.access() != Access::Editable
                        })
                    {
                        return KeyAnswer::Pass;
                    }
                    match target {
                        DraftKey::New => shell.composer_pasted(true, pasted, cx),
                        DraftKey::Session { .. } => shell.composer_pasted(false, pasted, cx),
                        DraftKey::Workspace { .. } => shell.composer_pasted_at(target, pasted, cx),
                    }
                })
                .unwrap_or(KeyAnswer::Pass)
        },
    ))
}
