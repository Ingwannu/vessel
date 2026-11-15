//! Optional host callbacks preserve one markdown renderer and one selection scope.

use std::sync::Arc;

use gpui::{
    AnyElement, App, ClipboardItem, Entity, ImageSource, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, SharedUri, Window, div,
};
pub use gpui_base::text::{InlineLinkKind, RecognizedLink};
use gpui_base::text::{InlineLinkRecognizer, LinkSource};
use gpui_base::text::{SelectionFormat, TextView, TextViewState};

/// An authored href is untrusted text until the host classifies and authorizes it.
#[derive(Debug, Clone)]
pub struct Href {
    value: SharedString,
    source: Source,
}

/// Inferred text keeps literal percent bytes, unlike an authored href.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Original Markdown href.
    Authored,
    /// Recognized text or inline-code token.
    Inferred,
}

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
        &self.value
    }

    /// The host chooses the parser's raw-token or href path without guessing by URL.
    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
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
    inline_links: Option<InlineLinkRecognizer>,
}

impl std::fmt::Debug for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actions")
            .field("image_resolver", &self.image.is_some())
            .field("image_activation", &self.image_click.is_some())
            .field("inline_links", &self.inline_links.is_some())
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
            inline_links: None,
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

    /// Decorate existing text/code runs without replacing their selection/source map.
    #[must_use]
    pub fn with_inline_links(
        mut self,
        recognize: fn(&str, InlineLinkKind) -> Vec<RecognizedLink>,
    ) -> Self {
        self.inline_links = Some(InlineLinkRecognizer::new(recognize));
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
    actions: Option<Actions>,
    copy: CopyStyle,
) -> AnyElement {
    view = view.selection_format(match copy {
        CopyStyle::Plain => SelectionFormat::Plain,
        CopyStyle::MarkdownCodeAware => SelectionFormat::Source,
    });
    if let Some(actions) = actions {
        view = view.on_link_activate(move |target, _event, window, cx| {
            let source = match target.source() {
                LinkSource::Authored => Source::Authored,
                LinkSource::Inferred(InlineLinkKind::Text | InlineLinkKind::Code) => {
                    Source::Inferred
                }
            };
            (actions.link)(
                Href {
                    value: target.href().clone(),
                    source,
                },
                window,
                cx,
            );
        });
        if let Some(recognizer) = actions.inline_links {
            view = view.inline_link_recognizer(recognizer);
        }
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
            let Some(code) = state.read(cx).selected_code_text() else {
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

#[cfg(test)]
mod image_tests;

#[cfg(test)]
mod link_tests;
