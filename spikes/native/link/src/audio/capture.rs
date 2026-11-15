//! The shared microphone API: normalized local PCM, never a speculative node call.
//!
//! Dictation and conversation use the same acquisition identity. Conversation
//! acquisition stays refused until the duplex output owner is connected.

use super::format::Format;
use crate::cancel::Fence;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use vessel_app_view::audio::{CaptureLease, CapturePurpose, CaptureRefusal};

/// A local device failure never retains foreign device/driver diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeviceFailure {
    /// No usable default device or driver.
    #[error("the audio device is unavailable")]
    Unavailable,
    /// The device format is outside the supported contract.
    #[error("the audio device format is unsupported")]
    Format,
    /// A device callback reported interruption.
    #[error("the audio device stopped unexpectedly")]
    Interrupted,
    /// DSP refused a callback frame.
    #[error("audio processing failed")]
    Processing,
    /// Backpressure cannot retain additional microphone audio.
    #[error("the audio consumer fell behind")]
    Overrun,
    /// The dedicated device executor could not start.
    #[error("the audio device thread could not start")]
    Thread,
}

/// Stop closes the callback fence immediately; Released requires stream destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Reserved, before the device has confirmed startup.
    Reserved,
    /// The capture stream started for this exact acquisition.
    Running,
    /// All streams were destroyed and physical ownership retired.
    Released,
    /// Cleanup completed after an explicit device failure.
    Failed(DeviceFailure),
}

/// Acquisition failures precede a usable capture handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AcquireError {
    /// This platform has no native capture backend.
    #[error("native microphone capture is unsupported on this platform")]
    Unsupported,
    /// Voice must not start capture before the duplex implementation is ready.
    #[error("conversation audio is not yet available")]
    ConversationUnavailable,
    /// A poisoned ownership lock cannot prove the device is free.
    #[error("audio ownership cannot be verified")]
    Ownership,
    /// A previous capture or its cleanup still holds the microphone.
    #[error(transparent)]
    Reservation(#[from] CaptureRefusal),
    /// The device worker could not be created.
    #[error(transparent)]
    Device(#[from] DeviceFailure),
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug)]
pub(super) struct Held {
    audio: super::dsp::Captured,
    permit: super::budget::Permit,
    access: super::upload_access::Access,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl Held {
    pub(super) fn new(
        audio: super::dsp::Captured,
        permit: super::budget::Permit,
        access: super::upload_access::Access,
    ) -> Self {
        Self {
            audio,
            permit,
            access,
        }
    }
}

/// One10ms PCM16 mono frame at16kHz, bound to its acquisition identity.
/// Keep the frame (or its owning batch) alive until the upload acknowledges it:
/// its private permit accounts for retained audio across all capture stages.
pub struct Frame {
    lease: CaptureLease,
    pcm: Bytes,
    level: vessel_app_view::voice::Level,
    _permit: super::budget::Permit,
    access: super::upload_access::Access,
}
impl Frame {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub(super) fn new(lease: CaptureLease, held: Held) -> Self {
        let (pcm, level) = held.audio.into_parts();
        Self {
            lease,
            pcm,
            level,
            _permit: held.permit,
            access: held.access,
        }
    }
    /// Routing is determined by this acquisition, never by the current UI route.
    #[must_use]
    pub const fn lease(&self) -> CaptureLease {
        self.lease
    }
    /// The engine's fixed normalized format; frame count is per channel.
    #[must_use]
    pub const fn format(&self) -> Format {
        Format::capture()
    }
    /// Normalized block RMS after capture processing; this is not VAD activity.
    #[must_use]
    pub const fn level(&self) -> vessel_app_view::voice::Level {
        self.level
    }
    /// Whether this block's original capture interval still permits upload.
    /// Recheck immediately before creating an upload; unmute never revives old PCM.
    #[must_use]
    pub fn uploadable(&self) -> bool {
        self.access.live()
    }
    #[cfg(test)]
    pub(crate) fn upload_interval(&self) -> Option<super::upload_access::Interval> {
        self.access.interval()
    }
    /// Borrow audio at the upload/codec boundary; do not put it in diagnostics.
    #[must_use]
    pub fn pcm(&self) -> &[u8] {
        &self.pcm
    }
}
impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureFrame")
            .field("lease", &self.lease)
            .field("bytes", &self.pcm.len())
            .finish_non_exhaustive()
    }
}

/// Cloneable out-of-band controls for a capture whose frame reader lives on another task.
#[derive(Debug, Clone)]
pub struct CaptureControl {
    lease: CaptureLease,
    fence: Arc<Fence>,
    status: watch::Receiver<CaptureState>,
}
impl CaptureControl {
    /// Identity is stable through Stop and physical disposal.
    #[must_use]
    pub const fn lease(&self) -> CaptureLease {
        self.lease
    }
    /// Stop never waits for the frame reader, a queue or a node reply.
    pub fn stop(&self) {
        self.fence.close();
    }
    /// Watch until streams are actually destroyed before treating the slot as free.
    #[must_use]
    pub fn status(&self) -> watch::Receiver<CaptureState> {
        self.status.clone()
    }
}

/// The sole process-wide hardware capture lease, shared by every app window.
#[derive(Debug)]
pub struct Capture {
    lease: CaptureLease,
    fence: Arc<Fence>,
    frames: mpsc::Receiver<Frame>,
    status: watch::Receiver<CaptureState>,
    // The device guard stays on the worker; retaining this handle ensures Drop
    // fences it even if a consumer abandons its next-frame future.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    _owner: super::owner::Handle,
}
impl Capture {
    /// Reserve synchronously, then open the device on its dedicated worker.
    /// Inspect status before treating acquisition as Running.
    ///
    /// # Errors
    /// Busy/uncertain ownership, unsupported platform, unavailable conversation
    /// backend, or failure to start the worker. No node is contacted here.
    pub fn acquire(purpose: CapturePurpose) -> Result<Self, AcquireError> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if purpose == CapturePurpose::Conversation {
                return Err(AcquireError::ConversationUnavailable);
            }
            let (owner, handle) = super::owner::Engine::process().reserve(purpose)?;
            let lease = handle.lease();
            let fence = owner.fence();
            let status = handle.status();
            let frames = super::thread::start(owner)?;
            Ok(Self {
                lease,
                fence,
                frames,
                status,
                _owner: handle,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _purpose = purpose;
            Err(AcquireError::Unsupported)
        }
    }
    /// Keep controls in the UI while another task awaits normalized frames.
    #[must_use]
    pub fn control(&self) -> CaptureControl {
        CaptureControl {
            lease: self.lease,
            fence: Arc::clone(&self.fence),
            status: self.status.clone(),
        }
    }
    /// The same identity appears on every emitted frame.
    #[must_use]
    pub const fn lease(&self) -> CaptureLease {
        self.lease
    }
    /// Observe actual startup/release; requesting Stop alone is not release.
    #[must_use]
    pub fn status(&self) -> watch::Receiver<CaptureState> {
        self.status.clone()
    }
    /// Fence callbacks independently of queues or remote cleanup.
    pub fn stop(&self) {
        self.fence.close();
    }
    /// Cancellation-safe frame read. Stop discards all queued microphone frames.
    pub async fn next(&mut self) -> Option<Frame> {
        #[cfg(not(target_arch = "wasm32"))]
        let received = tokio::select! { biased; () = self.fence.closed() => None, frame = self.frames.recv() => frame };
        #[cfg(target_arch = "wasm32")]
        let received = std::future::ready(None).await;
        if self.fence.is_closed() {
            self.frames.close();
            while self.frames.try_recv().is_ok() {}
            None
        } else {
            received
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests;
