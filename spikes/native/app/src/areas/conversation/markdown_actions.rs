//! Temporary refusal policy while the scoped Markdown host is unavailable.
//! None would retain the engine's OS URL and image URI/data fallback.
use gpui::{ImageCacheError, ImageSource};
use std::sync::Arc;
use vessel_app_kit::chat_markdown::actions::Actions;
use vessel_app_kit::ui::SharedString;

/// Intercept every link and reject every image without interpreting its URI.
/// This grants no external/file opening, image acquisition or decoding authority.
#[must_use]
pub(super) fn unavailable(reason: &str) -> Actions {
    let reason: SharedString = reason.to_owned().into();
    Actions::new(|_, _, _| {}).with_images(move |_| denied_image(reason.clone()))
}

fn denied_image(reason: SharedString) -> ImageSource {
    #[cfg(test)]
    tests::denied();
    ImageSource::Custom(Arc::new(move |_, _| {
        Some(Err(ImageCacheError::Asset(reason.clone())))
    }))
}

#[cfg(test)]
mod tests;
