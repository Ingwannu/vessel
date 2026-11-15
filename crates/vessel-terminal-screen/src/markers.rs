//! Physical viewport identity belongs to one authority, named buffer and reset epoch.
use crate::{BufferKind, LineIndex, Screen, ScreenError, Snapshot};
use std::sync::Arc;

/// Finite marker slots per buffer; evicted handles remain owned until released/replaced.
pub const MAX_VIEWPORT_MARKERS: usize = vt100::MAX_MARKERS;
#[derive(Debug, Clone)]
pub(crate) struct ScreenIdentity(Arc<()>);
impl Default for ScreenIdentity {
    fn default() -> Self {
        Self(Arc::new(()))
    }
}
impl PartialEq for ScreenIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ScreenIdentity {}

/// Reset identity within one authority/buffer, never transport authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferEpoch(u64);
impl BufferEpoch {
    /// A node may project the epoch explicitly alongside its independent actor identity.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    fn next(self) -> Result<Self, ScreenError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(ScreenError::MarkerIdentity)
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Epochs {
    primary: BufferEpoch,
    alternate: BufferEpoch,
}
impl Epochs {
    pub(crate) const fn of(self, buffer: BufferKind) -> BufferEpoch {
        match buffer {
            BufferKind::Primary => self.primary,
            BufferKind::Alternate => self.alternate,
        }
    }
    pub(crate) fn advance(&mut self, buffer: BufferKind) -> Result<(), ScreenError> {
        match buffer {
            BufferKind::Primary => self.primary = self.primary.next()?,
            BufferKind::Alternate => self.alternate = self.alternate.next()?,
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self) -> Result<(), ScreenError> {
        let primary = self.primary.next()?;
        let alternate = self.alternate.next()?;
        *self = Self { primary, alternate };
        Ok(())
    }
}

/// Opaque caller-owned marker; wire IDs alone cannot reconstruct or authorize it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportAnchor {
    identity: ScreenIdentity,
    buffer: BufferKind,
    epoch: BufferEpoch,
    marker: vt100::Marker,
}
impl ViewportAnchor {
    /// The primary anchor is never silently applied to alternate rows.
    #[must_use]
    pub const fn buffer(&self) -> BufferKind {
        self.buffer
    }
    /// Clear/reset invalidates the previous epoch even if a local marker number repeats.
    #[must_use]
    pub const fn epoch(&self) -> BufferEpoch {
        self.epoch
    }
}
/// Buffer activity is independent from whether the retained marker still exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPresence {
    /// Its buffer currently supplies the displayed cells.
    Active,
    /// Retain the anchor while displaying the other buffer's live rows.
    Inactive,
}
/// An inspected row can disappear without turning its address into a live-tail offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorState {
    /// Current physical row, adjusted by the same row edits as the grid.
    Retained(LineIndex),
    /// Row genuinely removed; replacing at this oldest row is an explicit clamp.
    Evicted {
        /// Oldest retained physical row; the caller explicitly adopts a new marker there.
        oldest: LineIndex,
    },
    /// Authority allocation or buffer reset changed; never revive another opening's row.
    Reset,
    /// Its caller already released or replaced this local handle.
    Released,
}
/// One read-only cut of marker availability, without any node route or pane identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorProjection {
    /// A hidden primary anchor remains distinguishable from the active alternate screen.
    pub presence: BufferPresence,
    /// Exact retention/reset/release disposition.
    pub state: AnchorState,
}
/// Cleanup does not allocate or admit a stale marker into a new epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorRelease {
    /// Owned slot released.
    Released,
    /// Handle had already been released or replaced.
    AlreadyGone,
    /// The buffer/authority was reset; nothing in the current table was touched.
    Reset,
}

impl Screen {
    /// Pin an active retained physical row. Node callers own its bounded lifetime.
    ///
    /// # Errors
    /// Refuses an unavailable row, full table or exhausted identity before replacing a marker.
    pub fn create_marker(&mut self, row: LineIndex) -> Result<ViewportAnchor, ScreenError> {
        let buffer = self.buffer_kind();
        let marker = self
            .parser
            .screen_mut()
            .create_marker(index(row)?)
            .map_err(marker_error)?;
        Ok(ViewportAnchor {
            identity: self.identity.clone(),
            buffer,
            epoch: self.parser.callbacks().epochs.of(buffer),
            marker,
        })
    }
    /// Replace a caller's active marker without needing a temporary extra table slot.
    ///
    /// # Errors
    /// Refuses reset/inactive/released handles and invalid rows without retiring the old marker.
    pub fn replace_marker(
        &mut self,
        old: &ViewportAnchor,
        row: LineIndex,
    ) -> Result<ViewportAnchor, ScreenError> {
        if old.identity != self.identity
            || old.epoch != self.parser.callbacks().epochs.of(old.buffer)
        {
            return Err(ScreenError::MarkerReset);
        }
        if old.buffer != self.buffer_kind() {
            return Err(ScreenError::MarkerInactive);
        }
        let marker = self
            .parser
            .screen_mut()
            .replace_marker(old.marker, index(row)?)
            .map_err(marker_error)?;
        Ok(ViewportAnchor {
            marker,
            ..old.clone()
        })
    }
    /// Resolve inactive-buffer markers too; a reset cannot fall back to a new local ID.
    ///
    /// # Errors
    /// Refuses an engine row outside the validated facade's physical-row range.
    pub fn resolve_marker(&self, anchor: &ViewportAnchor) -> Result<AnchorProjection, ScreenError> {
        resolve(
            self.parser.screen(),
            &self.identity,
            self.parser.callbacks().epochs,
            anchor,
        )
    }
    /// Release or retire a stale handle without touching another buffer or authority.
    pub fn release_marker(&mut self, anchor: &ViewportAnchor) -> AnchorRelease {
        if anchor.identity != self.identity
            || anchor.epoch != self.parser.callbacks().epochs.of(anchor.buffer)
        {
            return AnchorRelease::Reset;
        }
        match self
            .parser
            .screen_mut()
            .release_marker(buffer(anchor.buffer), anchor.marker)
        {
            vt100::MarkerRelease::Released => AnchorRelease::Released,
            vt100::MarkerRelease::AlreadyGone => AnchorRelease::AlreadyGone,
        }
    }
}
impl Snapshot {
    /// Resolve against the same immutable marker/cell/mode cut, never future row edits.
    ///
    /// # Errors
    /// Refuses an engine row outside the validated physical-row range.
    pub fn resolve_marker(&self, anchor: &ViewportAnchor) -> Result<AnchorProjection, ScreenError> {
        resolve(&self.state, &self.identity, self.epochs, anchor)
    }
}
fn resolve(
    state: &vt100::Screen,
    identity: &ScreenIdentity,
    epochs: Epochs,
    anchor: &ViewportAnchor,
) -> Result<AnchorProjection, ScreenError> {
    let active = if state.alternate_screen() {
        BufferKind::Alternate
    } else {
        BufferKind::Primary
    };
    let presence = if active == anchor.buffer {
        BufferPresence::Active
    } else {
        BufferPresence::Inactive
    };
    let projected = if anchor.identity != *identity || anchor.epoch != epochs.of(anchor.buffer) {
        AnchorState::Reset
    } else {
        match state.resolve_marker(buffer(anchor.buffer), anchor.marker) {
            vt100::MarkerPosition::Retained(row) => AnchorState::Retained(LineIndex(
                u32::try_from(row).map_err(ScreenError::MarkerIndex)?,
            )),
            vt100::MarkerPosition::Evicted => AnchorState::Evicted {
                oldest: LineIndex(0),
            },
            vt100::MarkerPosition::Released => AnchorState::Released,
        }
    };
    Ok(AnchorProjection {
        presence,
        state: projected,
    })
}
fn index(row: LineIndex) -> Result<usize, ScreenError> {
    usize::try_from(row.0).map_err(ScreenError::MarkerIndex)
}
fn buffer(kind: BufferKind) -> vt100::Buffer {
    match kind {
        BufferKind::Primary => vt100::Buffer::Primary,
        BufferKind::Alternate => vt100::Buffer::Alternate,
    }
}
fn marker_error(error: vt100::MarkerError) -> ScreenError {
    match error {
        vt100::MarkerError::Row => ScreenError::MarkerRow,
        vt100::MarkerError::Capacity => ScreenError::MarkerCapacity,
        vt100::MarkerError::Exhausted => ScreenError::MarkerIdentity,
        vt100::MarkerError::Released => ScreenError::MarkerReleased,
    }
}

#[cfg(test)]
mod tests;
