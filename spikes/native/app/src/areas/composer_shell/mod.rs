//! Retained shell policy is keyed to the existing draft owner, never the editor.
//! Attachment, lifecycle and provider I/O stay with their established areas.

use std::collections::HashMap;

use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::Surface;

mod fullscreen;
#[cfg(test)]
pub(crate) mod retained;
mod target;
pub(crate) use fullscreen::render as fullscreen;
#[cfg(test)]
pub(crate) use retained::ResidentComposer;

/// State survives presentation remounts without copying or replacing editor text.
#[derive(Debug, Default)]
pub(crate) struct ComposerShellArea {
    surfaces: HashMap<DraftKey, Surface>,
}

impl ComposerShellArea {
    pub(crate) fn surface(&self, target: DraftKey) -> Surface {
        self.surfaces.get(&target).copied().unwrap_or_default()
    }

    pub(crate) fn collapse(&mut self, target: DraftKey) {
        if let Some(surface) = self.surfaces.get_mut(&target) {
            surface.collapse();
        }
    }

    pub(super) fn surface_mut(&mut self, target: DraftKey) -> &mut Surface {
        self.surfaces.entry(target).or_default()
    }
}
