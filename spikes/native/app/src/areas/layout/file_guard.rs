//! Generic metadata batches cannot dispose of a File owner's retained buffer.
use vessel_app_view::layout::{Layout, LayoutError, Target};

pub(super) fn preserve(before: &Layout, after: &Layout) -> Result<(), LayoutError> {
    if before.tabs().any(|old| {
        matches!(old.target(), Target::File { .. })
            && after
                .tab(old.id())
                .is_none_or(|current| current.target() != old.target())
    }) {
        // Cached Clean is not an owner disposition. This also fences implicit
        // compact-sheet truncation; explicit owner completion is a separate path.
        return Err(LayoutError::UnsupportedHost);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
