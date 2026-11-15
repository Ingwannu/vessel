//! Decoded image sources follow the lifetime of their bounded byte owners.
use crate::shell::Shell;
use gpui::{Image, ImageFormat, ImageSource};
use std::sync::Arc;
use vessel_app_view::composer::tray::Origin;

/// An image's picture: a file on this device read by GPUI's image cache, a
/// pasted one decoded once and kept while a tray holds it.
pub(super) fn picture(
    shell: &Shell,
    origin: &Origin,
    mime: &vessel_proto::conversation::MimeType,
) -> ImageSource {
    match origin {
        Origin::File(path) => ImageSource::from(path.clone()),
        Origin::Stored(local) => ImageSource::from(local.path.clone()),
        Origin::Pasted(bytes) => shell.areas.composer.pictures.of(bytes, mime.as_str()),
    }
}

/// Pasted and sent images' pictures, kept while a tray or the sent store
/// holds their bytes.
#[derive(Default)]
pub(crate) struct Pictures(std::cell::RefCell<Vec<(Arc<[u8]>, ImageSource)>>);

impl std::fmt::Debug for Pictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pictures").finish_non_exhaustive()
    }
}

impl Pictures {
    pub(super) fn of(&self, bytes: &Arc<[u8]>, mime: &str) -> ImageSource {
        let mut kept = self.0.borrow_mut();
        // A picture whose bytes no tray, and no sent store, holds any more
        // goes.
        kept.retain(|(held, _)| Arc::strong_count(held) > 1);
        if let Some((_, source)) = kept.iter().find(|(held, _)| Arc::ptr_eq(held, bytes)) {
            return source.clone();
        }
        let format = match mime {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/gif" => ImageFormat::Gif,
            "image/webp" => ImageFormat::Webp,
            _ => ImageFormat::Png,
        };
        let image = Arc::new(Image::from_bytes(format, bytes.to_vec()));
        let source = ImageSource::from(image);
        kept.push((Arc::clone(bytes), source.clone()));
        source
    }
}
