//! Selectable literal prose over the same retained text engine as Markdown.
//!
//! [Decision Log]
//! - Purpose: notices must be selectable without interpreting provider text as
//!   Markdown, links, images or HTML actions.
//! - Existing asset: the component text engine's HTML paragraph parser and global
//!   selection adapter. A bare pre element preserves whitespace without a code mark.
//! - Choice: escape the complete payload and retain one HTML state per item/version.
//! - Why: selection/copy stay in the shared engine; no parallel selection manager
//!   or provider-content renderer is introduced.
//! - Cost: ordinary HTML line-ending normalization applies. The sentinel line feed
//!   is consumed by the parser, preserving the payload's own leading line feed.
//!   Plain selection retains the shared engine's paragraph-ending line feed.

use gpui::{AnyElement, App, AppContext as _, ElementId, Entity, IntoElement as _, Window};
use gpui_component::text::{SelectionFormat, TextView, TextViewState};

use crate::{FindMarks, FindPalette, Scheme, Tokens};

struct Kept {
    state: Entity<TextViewState>,
    version: u64,
    marked: Option<(u64, Scheme)>,
}

impl Kept {
    fn sync_marks(&mut self, marking: Option<(&FindMarks<'_>, &Tokens)>, cx: &mut App) {
        match marking {
            Some((marks, tokens)) if self.marked != Some((marks.key, tokens.scheme)) => {
                self.marked = self.state.update(cx, |state, cx| {
                    crate::chat_markdown::mark(state, marks, FindPalette::of(tokens), cx)
                        .map(|key| (key, tokens.scheme))
                });
            }
            None if self.marked.is_some() => {
                self.state.update(cx, TextViewState::clear_range_highlights);
                self.marked = None;
            }
            Some(_) | None => {}
        }
    }
}

/// Render selectable literal prose; the parent supplies its UI face and colour.
/// Stable item identity and version retain selection across unchanged frames.
#[must_use]
pub fn plain_text(
    id: impl Into<ElementId>,
    text: &str,
    version: u64,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    render(id.into(), text, version, None, window, cx)
}

/// The same selectable literal body, with the existing chat-find marks/reveal.
/// The source is still escaped as text; no Markdown or action parser is enabled.
#[must_use]
pub fn plain_text_with_marks(
    id: impl Into<ElementId>,
    text: &str,
    version: u64,
    marks: &FindMarks<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    render(id.into(), text, version, Some((marks, tokens)), window, cx)
}

fn render(
    id: ElementId,
    text: &str,
    version: u64,
    marking: Option<(&FindMarks<'_>, &Tokens)>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // A reused list position can have the same revision after a source switch.
    // Content participates so a stale keyed state cannot display prior prose.
    let version = crate::version_of(&(version, text));
    let kept = window.use_keyed_state(id, cx, |_, cx| Kept {
        state: cx.new(|cx| TextViewState::html(&literal_html(text), cx)),
        version,
        marked: None,
    });
    let state = kept.update(cx, |kept, cx| {
        if kept.version != version {
            kept.version = version;
            kept.state
                .update(cx, |state, cx| state.set_text(&literal_html(text), cx));
            kept.marked = None;
        }
        kept.sync_marks(marking, cx);
        kept.state.clone()
    });
    let list = marking.map(|(marks, _)| (marks.list.clone(), marks.clear_top));
    TextView::new(&state)
        .selectable(true)
        .selection_format(SelectionFormat::Plain)
        .on_reveal(move |bounds, _, _| {
            if let Some((list, clear_top)) = &list {
                list.reveal_bounds(bounds, *clear_top);
            }
        })
        .into_any_element()
}

fn literal_html(text: &str) -> String {
    let mut html = String::from("<pre>\n");
    for character in text.chars() {
        match character {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            other => html.push(other),
        }
    }
    html.push_str("</pre>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn the_actual_engine_keeps_markup_literal_and_uses_plain_paragraph_copy(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
        });
        let text = "\n  **literal** <img src='https://example.invalid/private'>\n\t& 한글  ";
        let state = cx.new(|cx| TextViewState::html(&literal_html(text), cx));
        cx.run_until_parked();
        state.update(cx, |state, cx| {
            // The engine appends its one structural paragraph separator. Preserve
            // the payload's intentional leading/trailing whitespace before it.
            let expected = format!("{text}\n");
            assert_eq!(state.rendered_text().as_str(), expected);
            state.set_selection_format(SelectionFormat::Plain, cx);
            state.select_all(cx);
            assert_eq!(state.selected_text(), expected);
        });
    }

    #[gpui::test]
    fn shared_find_marks_preserve_literal_selection_and_refresh_on_scheme_or_close(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        });
        let text = "fix **literal** <example>한글</example> fix";
        let state = cx.new(|cx| TextViewState::html(&literal_html(text), cx));
        cx.run_until_parked();
        cx.update(|cx| {
            let mut kept = Kept {
                state,
                version: 0,
                marked: None,
            };
            let list = crate::VirtualList::new(false);
            let spans = |rendered: &str| {
                rendered
                    .match_indices("fix")
                    .map(|(at, matched)| at..at.saturating_add(matched.len()))
                    .collect()
            };
            let marks = FindMarks {
                list: &list,
                clear_top: gpui::px(0.0),
                key: 1,
                spans: &spans,
                current: Some(1),
            };
            let dark = Tokens::of(Scheme::Dark);
            let light = Tokens::of(Scheme::Light);
            kept.state.update(cx, |state, cx| {
                let rendered = state.rendered_text();
                let ranges = (marks.spans)(rendered.as_str()).into_iter().map(|range| {
                    gpui_component::text::RangeHighlight::new(range, FindPalette::of(&dark).found)
                });
                assert!(
                    state.set_range_highlights(ranges, cx).is_ok(),
                    "draft HOLD: the pinned HTML format rejects highlights until the canonical opt-in seam exists"
                );
            });
            kept.sync_marks(Some((&marks, &dark)), cx);
            assert_eq!(kept.marked, Some((1, Scheme::Dark)));
            kept.state.update(cx, |state, cx| {
                assert_eq!(state.rendered_text().as_str(), format!("{text}\n"));
                state.set_selection_format(SelectionFormat::Plain, cx);
                state.select_all(cx);
                assert_eq!(state.selected_text(), format!("{text}\n"));
            });
            kept.sync_marks(Some((&marks, &light)), cx);
            assert_eq!(kept.marked, Some((1, Scheme::Light)));
            kept.sync_marks(None, cx);
            assert_eq!(kept.marked, None);
            assert_eq!(kept.state.read(cx).selected_text(), format!("{text}\n"));
        });
    }
}
