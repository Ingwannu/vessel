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
        .child("·")
        .child(
            div()
                .id("reload")
                .cursor_pointer()
                .hover(|style| style.text_color(tokens.text_secondary))
                .child(t.reload)
                .on_click(cx.listener(|this, _event, _window, _cx| this.reload())),
        )
        .child(div().flex_1())
        .child(crate::areas::labels::entry(shell, tokens, cx))
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
            .track_focus(&shell.areas.sidebar.layout.footer)
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_settings(None, window, cx);
            })),
        )
}
