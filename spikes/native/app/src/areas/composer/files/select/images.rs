//! Inspect selected images only on the acquisition background executor.
use super::{Incoming, LocalFileProblem};
use std::{io::Read as _, path::Path};
use vessel_app_view::composer::attach::attachment_name;
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::composer::attach::mime_of;
use vessel_proto::AttachmentLimits;

pub(super) fn is_image(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        matches!(
            extension.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff"
        )
    })
}

pub(super) fn normalize(path: &Path, name: &str) -> Result<Incoming, LocalFileProblem> {
    let file = std::fs::File::open(path).map_err(LocalFileProblem::Read)?;
    let mut bytes = Vec::new();
    let _read = file
        .take(AttachmentLimits::DEFAULT.file.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(LocalFileProblem::Read)?;
    let normalized = vessel_app_kit::image_input::normalize(
        bytes,
        vessel_app_kit::image_input::InputLimits {
            source: AttachmentLimits::DEFAULT.file,
            result: AttachmentLimits::DEFAULT.image,
        },
    )
    .map_err(LocalFileProblem::Image)?;
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let name = attachment_name(
        &format!("{stem}.{}", normalized.format.extension()),
        "image.png",
    )
    .ok_or(LocalFileProblem::NoName)?;
    Ok(Incoming {
        name: name.as_str().to_owned(),
        size: u64::try_from(normalized.bytes.len()).unwrap_or(u64::MAX),
        origin: vessel_app_view::composer::tray::Origin::Pasted(std::sync::Arc::from(
            normalized.bytes,
        )),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn store_image(
    store: &vessel_app_link::attachments::local::Store,
    incoming: Incoming,
) -> Result<Incoming, LocalFileProblem> {
    let name = attachment_name(&incoming.name, "image.png").ok_or(LocalFileProblem::NoName)?;
    let mime = mime_of(&incoming.name).ok_or(LocalFileProblem::NoName)?;
    let vessel_app_view::composer::tray::Origin::Pasted(bytes) = incoming.origin else {
        return Err(LocalFileProblem::NoName);
    };
    store
        .put_bytes(name, mime, &bytes)
        .map(Incoming::stored)
        .map_err(LocalFileProblem::Storage)
}
