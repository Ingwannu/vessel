//! A Settings page, as Paseo's `PageLayout` (`components/page-layout`):
//! on a wide window a centred column at most 720 wide, 16 around and 24
//! above, the section's title in it (26, medium, 4 in so it lines up
//! with the cards' titles, 24 above the first); on a phone a back header
//! (Paseo's `CompactHeader`: 56 tall, 12 across, the title 16 medium) and
//! the column without a title.
//!
//! [Decision Log]
//! - Purpose: item details need their own title and return action in the frame.
//! - Constraint: Machines and Agents share the same desktop/compact surface;
//!   the frame must not choose their destinations or own an area's state.
//! - Alternatives: local back pills, a second modal route, or an Agents-only
//!   destination enum in this shared page.
//! - Choice: one optional title/callback slot using the kit's one renderer.
//!   The UI-thread Rc callback is the open seam for each area's navigation.
//! - Why: ordinary pages retain `PageLayout` while details follow `BackHeader`.
//! - Cost: each caller owns its return callback and stale-machine guard.

use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};

use crate::shell::Shell;

/// Paseo's page column.
const PAGE_WIDTH: f32 = 720.0;

type Back = Rc<dyn Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>)>;

/// A section-independent detail title and its caller-owned return action.
/// The frame never decides which area or item the action returns to.
pub(crate) struct DetailHeader {
    pub(crate) title: SharedString,
    pub(crate) back: Back,
}
impl std::fmt::Debug for DetailHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailHeader").finish_non_exhaustive()
    }
}

/// A section's page: `title` over `body`; `compact` on a phone (the title
/// in a back header to the list).
pub(crate) fn layout(
    title: &str,
    body: AnyElement,
    compact: bool,
    scroll_id: ElementId,
    detail: Option<DetailHeader>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let has_detail = detail.is_some();
    let column = div()
        .w_full()
        .max_w(px(PAGE_WIDTH))
        .p(px(16.0))
        .pt(px(24.0))
        .when(!compact && !has_detail, |column| {
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
        .when(compact && !has_detail, |page| {
            page.child(compact_header(title, true, tokens, cx))
        })
        .children(detail.map(|detail| header(detail, compact, tokens, cx)))
        .child(
            div()
                .id(scroll_id)
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

/// Existing phone navigation delegates to the one exact header renderer.
/// Its public call stays unchanged; its callback still chooses list/exit.
pub(crate) fn compact_header(
    title: &str,
    to_list: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    header(
        DetailHeader {
            title: title.to_owned().into(),
            back: Rc::new(move |shell, window, cx| shell.settings_back(to_list, window, cx)),
        },
        true,
        tokens,
        cx,
    )
}

/// Paseo's `DesktopBackHeader`/`ScreenHeader`, or `CompactHeader` on touch.
/// Detail navigation stays outside the body, as `ProviderDiagnosticSheet`'s
/// header does; ordinary `PageLayout` headings keep their existing geometry.
pub(crate) fn header(
    detail: DetailHeader,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let button = div()
        .id(if compact {
            "settings-compact-back"
        } else {
            "settings-detail-back"
        })
        .size(px(if compact {
            vessel_app_kit::breakpoint::TOUCH_TARGET
        } else {
            36.0
        }))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .when(compact, |button| {
            button
                .rounded_full()
                .hover(move |style| style.bg(neutral.interaction_highlight))
                .active(move |style| style.bg(neutral.interaction_highlight))
        })
        .when(!compact, |button| {
            button.rounded(px(vessel_app_kit::theme::radius::R2))
        })
        .cursor_pointer()
        .child(glyph(Glyph::ArrowLeft, 20.0, neutral.foreground_muted))
        .on_click(cx.listener(move |shell, _, window, cx| (detail.back)(shell, window, cx)));
    vessel_app_kit::screen_header::frame(
        vessel_app_kit::screen_header::Content {
            title: detail.title,
            leading: Some(button.into_any_element()),
            actions: None,
            subtitle: None,
            editing: None,
        },
        if compact {
            vessel_app_kit::screen_header::Density::Compact
        } else {
            vessel_app_kit::screen_header::Density::Desktop
        },
        tokens,
    )
}
