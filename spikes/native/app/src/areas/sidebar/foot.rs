//! The sidebar's foot (Paseo's `components/left-sidebar.tsx` footer).

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::settings::Section;

use crate::routes::Route;
use crate::shell::Shell;

/// Out to the sidebar's edges: its padding (`spacing::S2`) taken back. A
/// negative literal, since the workspace denies arithmetic on sizes.
const OUT: f32 = -8.0;

/// The sidebar's foot, as Paseo's (`components/left-sidebar.tsx`): a
/// hairline over one line, its items at the left and Settings, a gear, at
/// the right end. Here the build and "새로고침" at the left; the gear opens
/// Settings, the build Settings › About.
///
/// At the sidebar's least width (200, a window of 600 to about 700) the line
/// is fuller than the sidebar. Labels and the gear keep their size; the
/// build and its "·" show whole or not at all, so the foot reads "reload
/// Labels ⚙" (the coordinator, 2026-11-18: a lone "…" read as a glitch;
/// before, Labels was squeezed to one letter a line).
///
/// [Decision Log]
/// - Constraint: whether the build fits depends on its words (the build's
///   label, the language's "reload") and the sidebar's width, so a fixed
///   width threshold would be wrong in some language or build.
/// - Alternatives: measure the words with the text system and add up the
///   foot's parts (every gap and button inset copied here); record last
///   frame's width with a probe (stale after the last resize frame).
/// - Choice: layout alone. The left part is a reversed, wrapping row one
///   line tall that clips: "reload" comes first so it always stays on the
///   line; the build with its "·" sits before it when there is room and
///   otherwise wraps onto the clipped second line. Packed to the physical
///   start (`justify_start`, not the reversed row's flex start), the line
///   begins at the left.
/// - Cost: the hidden build is still laid out (one line below, clipped).
pub(super) fn foot(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    div()
        // Out to the sidebar's edges through its padding (`spacing::S2`).
        .mx(px(OUT))
        .mb(px(OUT))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1_5))
        .px(px(spacing::S2))
        .py(px(spacing::S1_5))
        .border_t_1()
        .border_color(tokens.neutral.border)
        .type_(Type::Tiny)
        .text_color(tokens.text_tertiary)
        .child(
            // One line tall, clipped: "reload", then the build and its "·"
            // before it, or on the hidden second line (the decision log).
            div()
                .debug_selector(|| "sidebar-foot-start".to_owned())
                .flex_1()
                .min_w(px(spacing::S0))
                .h(px(Type::Tiny.metrics().1))
                .overflow_hidden()
                .flex()
                .flex_row_reverse()
                .flex_wrap()
                .justify_start()
                .items_center()
                .gap(px(spacing::S1_5))
                .child(
                    div()
                        .id("reload")
                        .debug_selector(|| "sidebar-foot-reload".to_owned())
                        .flex_none()
                        .cursor_pointer()
                        .hover(|style| style.text_color(tokens.text_secondary))
                        .child(t.reload)
                        .on_click(cx.listener(|this, _event, _window, _cx| this.reload())),
                )
                .child(
                    div()
                        .debug_selector(|| "sidebar-foot-build".to_owned())
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(spacing::S1_5))
                        .whitespace_nowrap()
                        .child(
                            // The build, and its updates: Settings › About.
                            div()
                                .id("build")
                                .pl(px(spacing::S0_5))
                                .cursor_pointer()
                                .hover(|style| style.text_color(tokens.text_secondary))
                                .child(t.build(crate::LABEL))
                                .on_click(cx.listener(|this, _event, window, cx| {
                                    this.open_settings(Some(Section::About), window, cx);
                                })),
                        )
                        .child("·"),
                ),
        )
        .child(
            div()
                .debug_selector(|| "sidebar-foot-labels".to_owned())
                .flex_none()
                .child(crate::areas::labels::entry(shell, tokens, cx)),
        )
        .child(
            vessel_app_kit::pill_parts(
                "settings",
                vessel_app_kit::PillParts {
                    leading: Some(vessel_app_kit::assets::Glyph::Gear),
                    ..vessel_app_kit::PillParts::default()
                },
                shell.route == Route::Settings,
                false,
                tokens,
            )
            .flex_none()
            .track_focus(&shell.areas.sidebar.layout.footer)
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_settings(None, window, cx);
            })),
        )
}
