//! A Settings page, as Paseo's `PageLayout` (`components/page-layout`):
//! on a wide window a centred column at most 720 wide, 16 around and 24
//! above, the section's title in it (26, medium, 4 in so it lines up
//! with the cards' titles, 24 above the first); on a phone a back header
//! (Paseo's `CompactHeader`: 56 tall, 12 across, the title 16 medium) and
//! the column without a title.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};

use crate::shell::Shell;

/// Paseo's page column.
const PAGE_WIDTH: f32 = 720.0;

/// A section's page: `title` over `body`; `compact` on a phone (the title
/// in a back header to the list).
pub(crate) fn layout(
    title: &str,
    body: AnyElement,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let column = div()
        .w_full()
        .max_w(px(PAGE_WIDTH))
        .p(px(16.0))
        .pt(px(24.0))
        .when(!compact, |column| {
            column.child(
                div()
                    .ml(px(4.0))
                    .mb(px(24.0))
                    // Paseo's 26 at weight 500: the label step's face and
                    // fallbacks (Hangul), larger.
                    .type_(Type::Label)
                    .text_size(px(vessel_app_kit::fonts::ui(26.0)))
                    .line_height(px(vessel_app_kit::fonts::ui(32.0)))
                    .text_color(tokens.neutral.foreground)
                    .child(title.to_owned()),
            )
        })
        .child(body);
    div()
        .size_full()
        .flex()
        .flex_col()
        .when(compact, |page| {
            page.child(compact_header(title, true, tokens, cx))
        })
        .child(
            div()
                .id("settings-page")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .justify_center()
                        .child(column),
                ),
        )
}

/// A phone's back header: a way back (to the list when `to_list`, out of
/// Settings otherwise) and `title`.
pub(crate) fn compact_header(
    title: &str,
    to_list: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .h(px(56.0))
        .px(px(12.0))
        .child(
            div()
                .id("settings-compact-back")
                .size(px(40.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(|style| style.bg(neutral.surface2))
                .child(glyph(Glyph::ArrowLeft, 20.0, neutral.foreground_muted))
                .on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.settings_back(to_list, window, cx);
                })),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Label)
                .text_size(px(vessel_app_kit::fonts::ui(16.0)))
                .text_color(neutral.foreground)
                .child(title.to_owned()),
        )
}

/// The page of a section not drawn yet: one quiet sentence.
pub(crate) fn elsewhere(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .px(px(4.0))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
        .into_any_element()
}
