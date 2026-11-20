//! A review thread's block in the activity: Paseo's `pane.tsx`
//! `ThreadBlock` and `ThreadComment` (its header names the file and lines
//! with Resolved and Outdated badges and folds it; the root comment, the
//! replies on a rail, and "Add to chat" for the whole thread), split from
//! `pane_cards` for the file budget.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::ui::{
    AnyElement, Context, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Styled as _, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};
use vessel_app_view::workspace::pull::timeline::{Thread, thread_path};
use vessel_proto::forge::TimelineItem;

use super::pane_cards::{Adds, Cards, HEADER_MIN, folds};
use crate::shell::Shell;

/// A badge's padding above and below (Paseo's `StatusBadge`).
const BADGE_Y: f32 = 3.0;
/// A count chip's gap between its glyph and number (Paseo's 3).
const CHIP_GAP: f32 = 3.0;

impl Cards<'_> {
    /// A count of comments or threads (Paseo's chip: the message glyph at
    /// 11 and the number).
    pub(super) fn count_chip(&self, count: usize) -> AnyElement {
        let muted = self.tokens.neutral.foreground_muted;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(CHIP_GAP))
            .type_(Type::Meta)
            .text_color(muted)
            .child(glyph(Glyph::Message, 11.0, muted))
            .child(count.to_string())
            .into_any_element()
    }

    /// A badge (Paseo's `StatusBadge`): "Resolved" in the success tint,
    /// "Outdated" muted.
    fn badge(&self, text: &str, resolved: bool) -> AnyElement {
        let neutral = self.tokens.neutral;
        let success = self.tokens.status().success;
        let (back, edge, ink): (Hsla, Hsla, Hsla) = if resolved {
            (success.tint, gpui::transparent_black(), success.foreground)
        } else {
            (neutral.surface3, neutral.border, neutral.foreground_muted)
        };
        div()
            .flex_none()
            .px(px(spacing::S2))
            .py(px(BADGE_Y))
            .rounded_full()
            .border_1()
            .border_color(edge)
            .bg(back)
            .type_(Type::Meta)
            .text_color(ink)
            .child(text.to_owned())
            .into_any_element()
    }

    /// Paseo's `ThreadBlock`: the header (path, badges, a count while
    /// folded), then the root comment and its replies.
    pub(super) fn thread(&self, thread: &Thread<'_>, cx: &mut Context<'_, Shell>) -> AnyElement {
        let neutral = self.tokens.neutral;
        let words = self.shell.words.pulls();
        let hover = SidebarSurfaces::of(self.tokens).hover;
        let key = thread.key();
        let at_first = thread.folded_at_first();
        let folded = self.choices.folded(&key, at_first);
        let path = thread_path(thread.place);
        let label = path.unwrap_or_else(|| words.thread_discussion().to_owned());
        let head = folds(
            div()
                .id(SharedString::from(format!("pr-pane-{key}")))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .px(px(spacing::S3))
                .py(px(spacing::S2))
                .min_h(px(HEADER_MIN))
                .bg(neutral.surface1)
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(
                    div()
                        .type_(Type::Mono)
                        .text_color(neutral.foreground)
                        .min_w_0()
                        .truncate()
                        .child(label),
                )
                .children(
                    thread
                        .place
                        .resolved
                        .then(|| self.badge(words.thread_resolved(), true)),
                )
                .children(
                    thread
                        .place
                        .outdated
                        .then(|| self.badge(words.thread_outdated(), false)),
                )
                .children(folded.then(|| {
                    div()
                        .ml_auto()
                        .child(self.count_chip(thread.comments.len()))
                })),
            key,
            at_first,
            cx,
        );
        let mut block = div().flex().flex_col().child(head);
        if !folded {
            let (root, replies) = match thread.comments.split_first() {
                Some((root, replies)) => (Some(*root), replies),
                None => (None, &[][..]),
            };
            block = block.children(root.map(|root| self.thread_comment(root)));
            // Paseo's `replyRail`: 12 in, 8 below the root, each reply a
            // rounded card.
            if !replies.is_empty() {
                block = block.child(div().flex().flex_col().mx(px(spacing::S3)).mt(px(spacing::S2)).children(
                    replies.iter().map(|reply| {
                        div()
                            .mb(px(spacing::S2))
                            .border_1()
                            .border_color(neutral.border)
                            .rounded(px(spacing::S1_5))
                            .bg(neutral.surface1)
                            .overflow_hidden()
                            .child(self.thread_comment(reply))
                    }),
                ));
            }
            if self.attach {
                block = block.child(self.footer(Adds::Thread(thread.key()), cx));
            }
        }
        block.into_any_element()
    }

    /// Paseo's `ThreadComment`: a line above, the 16 avatar header, then
    /// the words.
    fn thread_comment(&self, item: &TimelineItem) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(self.tokens.neutral.border)
            .pb(px(spacing::S2))
            .child(
                div()
                    .px(px(spacing::S3))
                    .pt(px(spacing::S2))
                    .pb(px(spacing::S1))
                    .min_h(px(spacing::S8))
                    .child(self.header_row(item, 16.0, None)),
            )
            .children(Self::markdown(item).map(|text| div().px(px(spacing::S3)).pt(px(spacing::S2)).child(text)))
            .into_any_element()
    }
}
