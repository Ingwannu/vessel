//! One usage card frame for source reports and saved sign-ins, after Paseo
//! card.tsx 81-160 and its styles (card.tsx 290-384).
//!
//! [Decision Log] (2026-11-16, coordinator round-1 return)
//! - Purpose: every card reads the same way, its state in the header: the
//!   14 px provider mark, the name, the plan as a muted badge, a status dot
//!   with its word for errors and unavailable sources, and an icon Refresh
//!   whose tooltip says when the values were fetched.
//! - Before: saved sign-in cards put the name above the card with a text
//!   button, the plan as a row and a "last successful check" footer, while
//!   source cards used the settings row frame. Two shapes for one thing.
//! - Choice: this frame owns layout only (padding 16, gap 12, header row,
//!   footer row); each card kind supplies its mark, values and actions.
//! - Footer: the account label, plus the freshness inline only where there is
//!   no hover (compact), as Paseo's `showsFreshnessInline`.
use vessel_app_kit::{
    Tokens, Type, TypeExt as _, settings,
    theme::spacing,
    ui::{
        AnyElement, Div, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px,
    },
};

/// Paseo's status dot (card.tsx `statusDot`), larger than the kit's list dot.
const STATUS_DOT: f32 = 8.0;

/// The word beside the dot. A current report shows none (card.tsx `statusText`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Status {
    /// The last attempt failed; danger dot.
    Error,
    /// The source cannot report here (unsupported, no adapter); muted dot.
    Unavailable,
}

/// What the header shows; the frame lays it out.
pub(super) struct Head {
    pub(super) mark: AnyElement,
    pub(super) name: SharedString,
    pub(super) plan: Option<SharedString>,
    pub(super) status: Option<Status>,
    /// Absent for sources that cannot refresh (unsupported, consent off).
    pub(super) refresh: Option<AnyElement>,
}

/// The account label, and the freshness only when it is printed on the card.
pub(super) struct Footer {
    pub(super) account: Option<SharedString>,
    pub(super) freshness: Option<SharedString>,
}

pub(super) fn frame(
    head: Head,
    body: Vec<AnyElement>,
    footer: Footer,
    words: &'static vessel_app_view::localization::Translator,
    tokens: &Tokens,
) -> Div {
    let mut card = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .p(px(spacing::S4))
        .child(header(head, words, tokens))
        .children(body);
    if footer.account.is_some() || footer.freshness.is_some() {
        card = card.child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .children(footer.account),
                )
                .children(footer.freshness.map(|freshness| {
                    div()
                        .flex_none()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(freshness)
                })),
        );
    }
    settings::card(tokens).child(card)
}

fn header(
    head: Head,
    words: &'static vessel_app_view::localization::Translator,
    tokens: &Tokens,
) -> Div {
    let text = words.usage();
    let status = head.status.map(|status| {
        let (label, color) = match status {
            Status::Error => (text.status_error(), tokens.status().error.foreground),
            Status::Unavailable => (text.status_unavailable(), tokens.neutral.foreground_muted),
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(spacing::S1_5))
            .child(div().size(px(STATUS_DOT)).rounded_full().bg(color))
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(label),
            )
    });
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(div().flex_none().child(head.mark))
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(head.name),
        )
        .children(head.plan.map(|plan| {
            vessel_app_kit::badge::badge(
                plan,
                vessel_app_kit::badge::Variant::Muted,
                vessel_app_kit::badge::Size::ExtraSmall,
                None,
                tokens,
            )
        }))
        .child(div().flex_1())
        .children(status)
        .children(head.refresh)
}

/// Paseo's detail row (card.tsx `detailRow`): muted label, value on the right.
pub(super) fn detail(label: SharedString, value: SharedString, tokens: &Tokens) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .justify_between()
        .gap(px(spacing::S2))
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(label),
        )
        .child(
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground)
                .child(value),
        )
        .into_any_element()
}

/// Login and refresh errors (card.tsx `styles.error`): danger text, wrapped.
pub(super) fn error(text: SharedString, tokens: &Tokens) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .type_(Type::Meta)
        .text_color(tokens.status().error.foreground)
        .child(text)
        .into_any_element()
}

/// A quiet line inside the card (not-yet-checked, cooldown).
pub(super) fn quiet(text: SharedString, tokens: &Tokens) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
        .into_any_element()
}

/// Paseo window-bar.tsx `row` and balance-bar.tsx `container`: the label row,
/// the meter and the reset line, 3 px apart. Paseo uses this exact gap, which
/// is not a spacing step.
const BAR_GAP: f32 = 3.0;

/// One window or balance: vertical padding 6, no inset of its own (the card's
/// padding is the only horizontal inset, so bars align with the header).
pub(super) fn bar() -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(BAR_GAP))
        .py(px(spacing::S1_5))
}

/// Paseo `UsageResetLine`: muted, or danger when it is a runs-out warning.
pub(super) fn reset_line(text: impl Into<SharedString>, at_risk: bool, tokens: &Tokens) -> Div {
    div()
        .min_w_0()
        .type_(Type::Meta)
        .text_color(if at_risk {
            tokens.status().error.foreground
        } else {
            tokens.neutral.foreground_muted
        })
        .child(text.into())
}

/// Paseo window-bar.tsx `UsagePinGlyph` size.
const PIN_GLYPH: f32 = 12.0;

/// Paseo window-bar.tsx `labelRow`: the label taking the room, the value,
/// then, while rows pin to the sidebar, the pin glyph (foreground when
/// pinned, muted otherwise). The whole row is the pin's click target.
/// Text is Paseo's `fontSize.sm` (12, the Meta step), the label medium.
pub(super) fn label_row(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    pinned: Option<bool>,
    tokens: &Tokens,
) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .type_(Type::Meta)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(tokens.neutral.foreground)
                .child(label.into()),
        )
        .child(
            div()
                .flex_none()
                .text_color(tokens.neutral.foreground)
                .child(value.into()),
        )
        .children(pinned.map(|pinned| {
            div().flex_none().child(vessel_app_kit::glyph(
                vessel_app_kit::assets::Glyph::Pin,
                PIN_GLYPH,
                if pinned {
                    tokens.neutral.foreground
                } else {
                    tokens.neutral.foreground_muted
                },
            ))
        }))
}
