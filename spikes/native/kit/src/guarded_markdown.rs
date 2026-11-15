//! Why: ADR 0079 requires plugin Markdown links and images to remain host-gated.
use gpui::{
    App, ClickEvent, ImageCacheError, ImageSource, IntoElement, RenderImage, SharedString,
    SharedUri, Window,
};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

/// The existing `TextView` link callback, with no implicit opener fallback.
pub type MarkdownLinkHandler =
    Arc<dyn Fn(&SharedString, &ClickEvent, &mut Window, &mut App) + Send + Sync>;
/// The existing authoritative `TextView` image resolver, including intrinsic size.
pub type MarkdownImageResolver = Arc<dyn Fn(&SharedUri) -> ImageSource + Send + Sync>;

/// Resource policy is explicit; a failed/refused result never uses the input URI.
#[derive(Clone)]
pub enum ImagePolicy {
    /// Refuse remote, embedded, local and relative document images alike.
    RefuseAll,
    /// A later approved owner may supply its typed resource resolver.
    Resolve(MarkdownImageResolver),
}
impl fmt::Debug for ImagePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RefuseAll => "RefuseAll",
            Self::Resolve(_) => "Resolve(<owner policy>)",
        })
    }
}
impl ImagePolicy {
    fn source(&self, uri: &SharedUri) -> ImageSource {
        match self {
            Self::RefuseAll => refusal_source(),
            Self::Resolve(resolve) => resolve(uri),
        }
    }
}

/// Every guarded document supplies both policies; neither defaults to loading.
#[derive(Clone)]
pub struct MarkdownGuards {
    /// Typed caller admission; the renderer never opens the URI itself.
    pub on_link: MarkdownLinkHandler,
    /// No renderer/data-URI fallback after this policy's result.
    pub images: ImagePolicy,
}
impl fmt::Debug for MarkdownGuards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarkdownGuards")
            .field("images", &self.images)
            .finish_non_exhaustive()
    }
}

/// The existing renderer with authoritative resource guards. Ordinary markdown
/// retains its current behavior; this adds no parser, image loader or style.
#[must_use]
pub fn markdown_guarded(
    id: impl Into<gpui::ElementId>,
    text: impl Into<SharedString>,
    guards: MarkdownGuards,
) -> impl IntoElement {
    let MarkdownGuards { on_link, images } = guards;
    gpui_component::text::TextView::markdown(id, text)
        .on_link_click(move |url, event, window, cx| on_link(url, event, window, cx))
        .image_source(move |uri| images.source(uri))
}

type RefusalLoader = fn(&mut Window, &mut App) -> Option<Result<Arc<RenderImage>, ImageCacheError>>;
const REFUSAL: &str = "document images are refused by the host resource policy";

fn refusal_source() -> ImageSource {
    // All input URIs share one inert loader. It cannot capture a URI, contact
    // a resource service or decode data bytes; the error itself is fixed.
    static LOADER: OnceLock<Arc<RefusalLoader>> = OnceLock::new();
    let loader: Arc<RefusalLoader> = Arc::clone(LOADER.get_or_init(|| {
        let callback: RefusalLoader =
            |_window, _cx| Some(Err(ImageCacheError::Asset(REFUSAL.into())));
        Arc::new(callback)
    }));
    ImageSource::Custom(loader)
}

#[cfg(test)]
mod tests;
