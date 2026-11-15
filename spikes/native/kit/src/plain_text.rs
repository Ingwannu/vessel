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

struct Kept {
    state: Entity<TextViewState>,
    version: u64,
}

/// Render selectable literal prose; the parent supplies its UI face and colour.
/// Stable item identity and version retain selection across unchanged frames.
pub fn plain_text(
    id: impl Into<ElementId>,
    text: &str,
    version: u64,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // A reused list position can have the same revision after a source switch.
    // Content participates so a stale keyed state cannot display prior prose.
    let version = crate::version_of(&(version, text));
    let kept = window.use_keyed_state(id.into(), cx, |_, cx| Kept {
        state: cx.new(|cx| TextViewState::html(&literal_html(text), cx)),
        version,
    });
    let state = kept.update(cx, |kept, cx| {
        if kept.version != version {
            kept.version = version;
            kept.state
                .update(cx, |state, cx| state.set_text(&literal_html(text), cx));
        }
        kept.state.clone()
    });
    TextView::new(&state)
        .selectable(true)
        .selection_format(SelectionFormat::Plain)
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
}
