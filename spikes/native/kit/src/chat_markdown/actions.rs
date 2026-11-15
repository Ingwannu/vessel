//! Optional host callbacks preserve one markdown renderer and one selection scope.

use std::sync::Arc;

use gpui::{
    AnyElement, App, ClipboardItem, Entity, ImageSource, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, SharedUri, Window, div,
};
use gpui_component::text::{SelectionFormat, TextView, TextViewState};

/// An authored href is untrusted text until the host classifies and authorizes it.
#[derive(Debug, Clone)]
pub struct Href(SharedString);

/// An image URI remains untrusted until the host resolves its authorized resource.
#[derive(Debug, Clone)]
pub struct ImageHref(SharedUri);

impl ImageHref {
    /// Original document image source, independent of the decoded display resource.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl Href {
    /// Raw href supplied by the markdown engine, never automatically opened.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Callbacks are open host extension points: the kit knows no app or node types.
type LinkHandler = dyn Fn(Href, &mut Window, &mut App) + Send + Sync;
type ImageResolver = dyn Fn(&SharedUri) -> ImageSource + Send + Sync;
type ImageHandler = dyn Fn(ImageHref, &mut Window, &mut App) + Send + Sync;

/// Assistant links and image sources are handled by the owning host.
#[derive(Clone)]
pub struct Actions {
    link: Arc<LinkHandler>,
    image: Option<Arc<ImageResolver>>,
    image_click: Option<Arc<ImageHandler>>,
}

impl std::fmt::Debug for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actions")
            .field("image_resolver", &self.image.is_some())
            .field("image_activation", &self.image_click.is_some())
            .finish_non_exhaustive()
    }
}

impl Actions {
    /// Install a link dispatcher so file targets cannot bypass root authorization.
    #[must_use]
    pub fn new(handler: impl Fn(Href, &mut Window, &mut App) + Send + Sync + 'static) -> Self {
        Self {
            link: Arc::new(handler),
            image: None,
            image_click: None,
        }
    }

    /// Every image, including data URLs, uses this retained acquisition snapshot.
    /// Pending/failure sources remain authoritative and never fall back to local I/O.
    #[must_use]
    pub fn with_images(
        mut self,
        resolver: impl Fn(&SharedUri) -> ImageSource + Send + Sync + 'static,
    ) -> Self {
        self.image = Some(Arc::new(resolver));
        self
    }

    /// Primary activation delegates to the host's shared image lightbox.
    #[must_use]
    pub fn on_image_click(
        mut self,
        handler: impl Fn(ImageHref, &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.image_click = Some(Arc::new(handler));
        self
    }
}

/// Existing consumers keep plain copy; assistant hosts can retain semantic markdown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CopyStyle {
    /// Existing plain-text selection behavior.
    #[default]
    Plain,
    /// Markdown across prose, raw code with no trailing linebreak inside a code fence.
    MarkdownCodeAware,
}

pub(super) fn finish(
    mut view: TextView,
    state: Entity<TextViewState>,
    source: Arc<str>,
    actions: Option<Actions>,
    copy: CopyStyle,
) -> AnyElement {
    view = view.selection_format(match copy {
        CopyStyle::Plain => SelectionFormat::Plain,
        CopyStyle::MarkdownCodeAware => SelectionFormat::Source,
    });
    if let Some(actions) = actions {
        view = view.on_link_click(move |uri, _event, window, cx| {
            (actions.link)(Href(uri.clone()), window, cx);
        });
        if let Some(image) = actions.image {
            view = view.image_source(move |uri| image(uri));
        }
        if let Some(handler) = actions.image_click {
            view = view.on_image_click(move |uri, event, window, cx| {
                if !event.is_right_click() && !event.is_middle_click() {
                    handler(ImageHref(uri.clone()), window, cx);
                }
            });
        }
    }
    div()
        .capture_action(move |_: &gpui_base::input::Copy, window, cx| {
            if copy != CopyStyle::MarkdownCodeAware {
                return;
            }
            let selected = state.read(cx).selected_text();
            let range = state.read(cx).selected_source_range();
            let Some(code) = range.and_then(|range| code_selection(&source, range)) else {
                return;
            };
            // A multi-message selection belongs to the window's aggregator. Never
            // replace it with the code fragment selected in only one participant.
            if gpui_base::TextSelection::selected_text(window, cx) == selected {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                cx.stop_propagation();
            }
        })
        .child(view)
        .into_any_element()
}

/// Only a selection wholly within one code body loses Markdown fencing.
fn code_selection(source: &str, selected: std::ops::Range<usize>) -> Option<String> {
    let mut offset: usize = 0;
    let mut open: Option<(char, usize, usize, usize)> = None;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len().saturating_sub(trimmed.len());
        let marker = trimmed.chars().next();
        let count = marker.map_or(0, |marker| {
            trimmed.chars().take_while(|ch| *ch == marker).count()
        });
        if let Some((fence, width, body, opening)) = open {
            if indent <= MAX_FENCE_INDENT
                && marker == Some(fence)
                && count >= width
                && trimmed
                    .get(count..)
                    .is_some_and(|tail| tail.trim().is_empty())
            {
                let end = offset;
                let whole_block = source.get(..opening)?.trim().is_empty()
                    && source
                        .get(offset.saturating_add(line.len())..)?
                        .trim()
                        .is_empty()
                    && selected.start == 0
                    && selected.end == source.len();
                let range = if whole_block {
                    body..end
                } else {
                    selected.clone()
                };
                if range.start >= body && range.end <= end {
                    return source
                        .get(range)
                        .map(|code| code.trim_end_matches(['\r', '\n']).to_owned());
                }
                open = None;
            }
        } else if indent <= MAX_FENCE_INDENT
            && count >= MIN_FENCE_WIDTH
            && matches!(marker, Some('`' | '~'))
            && (marker != Some('`') || trimmed.get(count..).is_some_and(|tail| !tail.contains('`')))
            && let Some(marker) = marker
        {
            open = Some((marker, count, offset.saturating_add(line.len()), offset));
        }
        offset = offset.saturating_add(line.len());
    }
    if let Some((_fence, _width, body, opening)) = open {
        let whole_block = source.get(..opening)?.trim().is_empty()
            && selected.start == 0
            && selected.end == source.len();
        let range = if whole_block {
            body..source.len()
        } else {
            selected
        };
        if range.start >= body && range.end <= source.len() {
            return source
                .get(range)
                .map(|code| code.trim_end_matches(['\r', '\n']).to_owned());
        }
    }
    None
}

const MIN_FENCE_WIDTH: usize = 3;
const MAX_FENCE_INDENT: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_only_selection_copies_raw_without_an_executable_trailing_newline() {
        let source = "prose\n\n```sh\nprintf '한글'\nprintf ok\n```\n";
        let start = source.find("printf").unwrap();
        let end = source.rfind("\n```").unwrap().saturating_add(1);
        assert_eq!(
            code_selection(source, start..end).as_deref(),
            Some("printf '한글'\nprintf ok")
        );
        assert_eq!(
            code_selection(source, 0..end),
            None,
            "mixed prose keeps markdown"
        );
        assert_eq!(
            code_selection(source, start.saturating_add(1)..start.saturating_add(5)).as_deref(),
            Some("rint")
        );
    }

    #[test]
    fn full_single_code_message_differs_from_multi_block_markdown() {
        let source = "~~~rust\nfn main() {}\n~~~";
        assert_eq!(
            code_selection(source, 0..source.len()).as_deref(),
            Some("fn main() {}")
        );
        let source = "```sh\necho one\n```\n\n```sh\necho two\n```";
        assert_eq!(code_selection(source, 0..source.len()), None);
        assert_eq!(code_selection("ordinary **markdown**", 0..8), None);
    }
}

#[cfg(test)]
mod image_tests;
