//! Retained shell policy is keyed to the existing draft owner, never the editor.
//! Attachment, lifecycle and provider I/O stay with their established areas.

use std::collections::HashMap;

use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::Surface;

/// State survives presentation remounts without copying or replacing editor text.
#[derive(Debug, Default)]
pub(crate) struct ComposerShellArea {
    surfaces: HashMap<DraftKey, Surface>,
}

impl ComposerShellArea {
    /// Preserve the ordinary chat behavior until a host supplies another policy.
    pub(crate) fn permits_menu(&self, target: DraftKey) -> bool {
        self.surfaces
            .get(&target)
            .copied()
            .unwrap_or_default()
            .autocomplete()
    }
}
