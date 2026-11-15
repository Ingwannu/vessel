//! A notice, after Paseo's `alert.tsx` and `control-geometry.ts`.
//!
//! [Decision Log]
//! - Purpose: pairing warnings, failed reads and refusals share one notice.
//! - Constraints: the variant comes from pure view logic; the kit has no
//!   view dependency. A notice is not a button and receives no focus ring.
//! - Alternatives: a screen-local warning; reusing the badge's four variants
//!   or its muted warning colour; truncating a description to one line.
//! - Choice: all five reference variants and four density sizes, transparent
//!   fill, one-point border, icon-led text, and an indented description and
//!   optional action. Title and description share a type size; only the
//!   title is medium weight. Text wraps with no fixed height or ellipsis.
//! - Why: Paseo's alert accents intentionally differ from its status badge.
//!   Existing success/destructive/neutral tokens stay the authority; only
//!   the missing palette blue 300 and amber 500 are added to status tokens.
//! - Cost: callers supply localized content and a working action; the kit
//!   supplies presentation only. Glyphs are the kit's own vector drawings.
//!
//! [Decision Log: host errors]
//! - Purpose: Paseo's schedules partial-load banner is a notice with its
//!   own geometry, not the Error variant with an icon removed by a caller.
//! - Constraint: ordinary alerts keep their defaults and render tree.
//! - Alternatives: screen-local paint or an arbitrary style escape hatch.
//! - Choice: one typed `HostErrors` presentation of this same frame, with
//!   sibling messages, neutral border and the existing palette red 300.
//! - Why: the kit owns padding, stack gap and text, including wrapping.
//! - Cost: a narrow messages-only entry; visibility and words stay in view.

use crate::{Tokens, Type, TypeExt as _, assets::Glyph, theme::radius, theme::spacing};
use gpui::{AnyElement, Div, FontWeight, Hsla, SharedString, div, prelude::*, px, svg};

/// The reference's five notice meanings, independent of the view crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Neutral, without an icon.
    Default,
    /// Information, with an information circle.
    Info,
    /// Success, with a circled check.
    Success,
    /// Warning, with an exclamation triangle.
    Warning,
    /// Error, with a circled cross.
    Error,
}

/// Alerts are containers and have Paseo's four density steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// Twelve-point text and the tightest padding.
    ExtraSmall,
    /// The pairing section's fourteen-point notice.
    Small,
    /// Normal text with more room around it.
    Medium,
    /// The roomiest notice, with a twenty-point icon.
    Large,
}

impl Size {
    const fn step(self) -> Type {
        match self {
            Self::ExtraSmall => Type::Meta,
            Self::Small | Self::Medium | Self::Large => Type::Body,
        }
    }
}

/// A notice's localized content. A description may contain styled code
/// or paths; its parent supplies the normal, muted description style.
pub struct Content {
    /// Optional medium-weight lead; absent uses the description as lead.
    pub title: Option<SharedString>,
    /// Optional wrapping explanation, in the normal weight.
    pub description: Option<AnyElement>,
    /// Optional working action, below the text as in Paseo's children slot.
    pub action: Option<AnyElement>,
}

impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Contents can come from an access warning. Debug needs structure,
        // not another route through which its text could enter a log.
        f.debug_struct("Content").finish_non_exhaustive()
    }
}

/// Draw a wrapping, non-interactive notice. Only `action`, if supplied,
/// has the interaction its caller explicitly wires.
#[must_use]
pub fn alert(mut content: Content, variant: Variant, size: Size, tokens: &Tokens) -> Div {
    let profile = profile(Presentation::Notice { variant, size }, tokens);
    let geometry = profile.geometry;
    let look = profile.look;
    let lead = match content.title {
        Some(title) => div()
            .font_weight(FontWeight::MEDIUM)
            .text_color(look.accent.unwrap_or(tokens.neutral.foreground))
            .child(title)
            .into_any_element(),
        None => div()
            .children(content.description.take())
            .into_any_element(),
    };
    let mut lead_row = div().flex().items_center().gap(px(geometry.icon_gap));
    if let Some(glyph) = look.icon {
        lead_row = lead_row.child(
            div()
                .w(px(geometry.icon))
                .flex_shrink_0()
                .flex()
                .items_center()
                .child(
                    svg()
                        .path(glyph.path())
                        .size(px(geometry.icon))
                        .text_color(look.accent.unwrap_or(tokens.neutral.foreground)),
                ),
        );
    }
    lead_row = lead_row.child(div().flex_1().min_w_0().child(lead));
    let mut notice = frame(profile).child(lead_row);
    if content.description.is_some() || content.action.is_some() {
        let mut below = div()
            .flex()
            .flex_col()
            .min_w_0()
            .when(look.icon.is_some(), |below| below.ml(px(geometry.indent())))
            .children(content.description);
        if let Some(action) = content.action {
            below = below.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .mt(px(8.0))
                    .child(action),
            );
        }
        notice = notice.child(below);
    }
    notice
}

/// Paseo's partial-host-error banner: one wrapping localized message per
/// host, without a title, icon or action. The caller shows it only when
/// there are errors; ordinary initial-load failures keep their own surface.
#[must_use]
pub fn host_errors(messages: impl IntoIterator<Item = SharedString>, tokens: &Tokens) -> Div {
    frame(profile(Presentation::HostErrors, tokens)).children(
        messages
            .into_iter()
            .map(|message| div().min_w_0().child(message)),
    )
}

/// Both entry points share the frame; a profile cannot add interaction,
/// truncation or a focus ring to this non-interactive container.
fn frame(profile: Profile) -> Div {
    let geometry = profile.geometry;
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .w_full()
        .border_1()
        .border_color(profile.look.border)
        .rounded(px(geometry.radius))
        .px(px(geometry.horizontal))
        .py(px(geometry.vertical))
        .gap(px(geometry.text_gap))
        .type_(profile.step)
        .text_color(profile.text)
        .whitespace_normal()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presentation {
    Notice { variant: Variant, size: Size },
    HostErrors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Profile {
    geometry: Geometry,
    look: Look,
    step: Type,
    text: Hsla,
}

fn profile(presentation: Presentation, tokens: &Tokens) -> Profile {
    match presentation {
        Presentation::Notice { variant, size } => Profile {
            geometry: geometry(size),
            look: look(variant, tokens),
            step: size.step(),
            text: tokens.neutral.foreground_muted,
        },
        Presentation::HostErrors => Profile {
            geometry: Geometry {
                horizontal: spacing::S3,
                vertical: spacing::S3,
                radius: radius::R5,
                text_gap: spacing::S1,
                ..geometry(Size::ExtraSmall)
            },
            look: Look {
                accent: None,
                border: tokens.neutral.border,
                icon: None,
            },
            step: Type::Meta,
            text: tokens.status().red_300,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Look {
    accent: Option<Hsla>,
    border: Hsla,
    icon: Option<Glyph>,
}

fn look(variant: Variant, tokens: &Tokens) -> Look {
    let status = tokens.status();
    let (accent, icon) = match variant {
        Variant::Default => (None, None),
        Variant::Info => (Some(status.blue_300), Some(Glyph::Info)),
        Variant::Success => (Some(status.success.foreground), Some(Glyph::CircleCheck)),
        Variant::Warning => (Some(status.amber_500), Some(Glyph::Alert)),
        Variant::Error => (Some(tokens.neutral.destructive), Some(Glyph::CircleX)),
    };
    let border = match (variant, accent) {
        (Variant::Default | Variant::Success, _) | (_, None) => tokens.neutral.border,
        (_, Some(mut accent)) => {
            // hexColorWithAlpha emits rgba(..., 0.5), not a rounded alpha byte.
            accent.a = 0.5;
            accent
        }
    };
    Look {
        accent,
        border,
        icon,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    horizontal: f32,
    vertical: f32,
    radius: f32,
    icon: f32,
    icon_gap: f32,
    text_gap: f32,
}

impl Geometry {
    fn indent(self) -> f32 {
        // Body indent = icon + icon_gap. The workspace denies plain float
        // addition through clippy::float_arithmetic and
        // clippy::arithmetic_side_effects. mul_add with 1 computes that
        // sum explicitly, without a lint suppression.
        self.icon.mul_add(1.0, self.icon_gap)
    }
}

const fn geometry(size: Size) -> Geometry {
    match size {
        Size::ExtraSmall => Geometry {
            horizontal: 12.0,
            vertical: 8.0,
            radius: 12.0,
            icon: 12.0,
            icon_gap: 8.0,
            text_gap: 0.0,
        },
        Size::Small => Geometry {
            horizontal: 16.0,
            vertical: 12.0,
            radius: 16.0,
            icon: 14.0,
            icon_gap: 12.0,
            text_gap: 2.0,
        },
        Size::Medium => Geometry {
            horizontal: 24.0,
            vertical: 16.0,
            radius: 16.0,
            icon: 16.0,
            icon_gap: 12.0,
            text_gap: 2.0,
        },
        Size::Large => Geometry {
            horizontal: 32.0,
            vertical: 24.0,
            radius: 16.0,
            icon: 20.0,
            icon_gap: 16.0,
            text_gap: 4.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;

    #[test]
    fn partial_host_errors_use_the_reference_frame_and_normal_red_text_in_both_themes() {
        // schedules-screen.tsx:400–417, not Alert's semantic Error profile.
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            let actual = profile(Presentation::HostErrors, &tokens);
            assert_eq!(actual.geometry.horizontal.to_bits(), 12.0_f32.to_bits());
            assert_eq!(actual.geometry.vertical.to_bits(), 12.0_f32.to_bits());
            assert_eq!(actual.geometry.radius.to_bits(), 12.0_f32.to_bits());
            assert_eq!(actual.geometry.text_gap.to_bits(), 4.0_f32.to_bits());
            assert_eq!(actual.look.border, tokens.neutral.border);
            assert_eq!(actual.look.icon, None);
            assert_eq!(actual.look.accent, None);
            assert_eq!(actual.text, gpui::rgb(0x00fc_a5a5).into());
            assert_eq!(actual.step, Type::Meta);
            let (size, _, weight) = actual.step.metrics();
            assert_eq!(size.to_bits(), 12.0_f32.to_bits());
            assert_eq!(weight, FontWeight::NORMAL);
        }
    }

    #[test]
    fn every_ordinary_notice_keeps_its_geometry_type_look_and_muted_description() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            for size in [Size::ExtraSmall, Size::Small, Size::Medium, Size::Large] {
                for variant in [
                    Variant::Default,
                    Variant::Info,
                    Variant::Success,
                    Variant::Warning,
                    Variant::Error,
                ] {
                    assert_eq!(
                        profile(Presentation::Notice { variant, size }, &tokens),
                        Profile {
                            geometry: geometry(size),
                            look: look(variant, &tokens),
                            step: size.step(),
                            text: tokens.neutral.foreground_muted,
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn every_variant_maps_to_the_reference_accent_border_and_icon_in_both_themes() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            let status = tokens.status();
            assert_eq!(status.blue_300, gpui::rgb(0x0093_c5fd).into());
            assert_eq!(status.amber_500, gpui::rgb(0x00f5_9e0b).into());
            for (variant, accent, icon) in [
                (Variant::Default, None, None),
                (Variant::Info, Some(status.blue_300), Some(Glyph::Info)),
                (
                    Variant::Success,
                    Some(status.success.foreground),
                    Some(Glyph::CircleCheck),
                ),
                (Variant::Warning, Some(status.amber_500), Some(Glyph::Alert)),
                (
                    Variant::Error,
                    Some(tokens.neutral.destructive),
                    Some(Glyph::CircleX),
                ),
            ] {
                let actual = look(variant, &tokens);
                assert_eq!(actual.accent, accent);
                assert_eq!(actual.icon, icon);
                if matches!(variant, Variant::Default | Variant::Success) {
                    assert_eq!(actual.border, tokens.neutral.border);
                } else {
                    let mut expected = accent.unwrap();
                    expected.a = 0.5;
                    assert_eq!(actual.border, expected);
                }
            }
        }
    }

    #[test]
    fn all_four_reference_density_cases_keep_the_text_and_body_aligned() {
        // Paseo control-geometry.test.ts: one text size, a tight gap,
        // icon-slot width equal to icon size, body at the lead's text edge.
        for (size, horizontal, vertical, radius, icon, gap, text_gap, indent) in [
            (Size::ExtraSmall, 12.0, 8.0, 12.0, 12.0, 8.0, 0.0, 20.0_f32),
            (Size::Small, 16.0, 12.0, 16.0, 14.0, 12.0, 2.0, 26.0),
            (Size::Medium, 24.0, 16.0, 16.0, 16.0, 12.0, 2.0, 28.0),
            (Size::Large, 32.0, 24.0, 16.0, 20.0, 16.0, 4.0, 36.0),
        ] {
            let actual = geometry(size);
            assert_eq!(
                size.step(),
                if size == Size::ExtraSmall {
                    Type::Meta
                } else {
                    Type::Body
                }
            );
            assert_eq!(
                actual,
                Geometry {
                    horizontal,
                    vertical,
                    radius,
                    icon,
                    icon_gap: gap,
                    text_gap
                }
            );
            assert_eq!(actual.indent().to_bits(), indent.to_bits());
        }
    }
}
