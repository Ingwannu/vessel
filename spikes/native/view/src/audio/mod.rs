//! Shared microphone ownership transitions, with no devices, executor or I/O.
//!
//! The link's one process-wide device actor drives this model. Voice and
//! dictation reserve the same slot; requesting Stop does not release it.
//! Only confirmed stream destruction permits another owner (ADR 0080).

/// Capture purpose is explicit, so dictation cannot acquire a second voice microphone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePurpose {
    /// Composer dictation receives normalized PCM from the shared engine.
    Dictation,
    /// Spoken conversation shares the same device and echo reference.
    Conversation,
}

/// A unique local acquisition, unrelated to a node's voice lease or session event sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureTicket(u64);

/// Both purpose and generation must match before device events can change ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureLease {
    ticket: CaptureTicket,
    purpose: CapturePurpose,
}

impl CaptureLease {
    /// Consumers route normalized PCM without inferring purpose from a UI route.
    #[must_use]
    pub const fn purpose(self) -> CapturePurpose {
        self.purpose
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Reserved(CaptureLease),
    Running(CaptureLease),
    Stopping(CaptureLease),
}

/// Physical ownership is retained while cleanup is pending or uncertain.
#[derive(Debug, Default)]
pub struct CaptureSlot {
    next: u64,
    phase: Phase,
}

/// Refusals never open devices or silently release another consumer's slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptureRefusal {
    /// A reservation counts as ownership even before a microphone starts.
    #[error("the microphone is owned by {0:?}")]
    Busy(CapturePurpose),
    /// Never wrap an old callback identity into a new acquisition.
    #[error("microphone acquisition identities exhausted")]
    Exhausted,
    /// Device completion arrived at the wrong transition.
    #[error("microphone completion is out of order")]
    Order,
}

/// Retired callbacks are explicit so the link can dispose any resource they created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureCompletion {
    /// The current acquisition advanced.
    Current,
    /// No current owner may be changed by this older completion.
    Stale,
}

impl CaptureSlot {
    /// Reserve before a remote voice enable, without opening a physical device yet.
    ///
    /// # Errors
    /// A busy slot or exhausted acquisition counter cannot reserve again.
    pub fn reserve(&mut self, purpose: CapturePurpose) -> Result<CaptureLease, CaptureRefusal> {
        if let Some(lease) = self.owner() {
            return Err(CaptureRefusal::Busy(lease.purpose));
        }
        // Identity reuse would let a delayed device callback release another lease.
        let next = self.next.checked_add(1).ok_or(CaptureRefusal::Exhausted)?;
        let lease = CaptureLease {
            ticket: CaptureTicket(next),
            purpose,
        };
        self.next = next;
        self.phase = Phase::Reserved(lease);
        Ok(lease)
    }

    /// Start only the reserved owner; a success arriving after Stop must be torn down.
    ///
    /// # Errors
    /// A repeated matching start cannot invent another live stream.
    pub fn started(&mut self, lease: CaptureLease) -> Result<CaptureCompletion, CaptureRefusal> {
        if self.owner() != Some(lease) {
            return Ok(CaptureCompletion::Stale);
        }
        match self.phase {
            Phase::Reserved(_) => {
                self.phase = Phase::Running(lease);
                Ok(CaptureCompletion::Current)
            }
            Phase::Stopping(_) => Ok(CaptureCompletion::Stale),
            Phase::Idle | Phase::Running(_) => Err(CaptureRefusal::Order),
        }
    }

    /// Gate callbacks immediately while retaining physical ownership through cleanup.
    ///
    /// # Errors
    /// A matching Stop twice must be reconciled rather than starting another teardown.
    pub fn stopping(&mut self, lease: CaptureLease) -> Result<CaptureCompletion, CaptureRefusal> {
        if self.owner() != Some(lease) {
            return Ok(CaptureCompletion::Stale);
        }
        match self.phase {
            Phase::Reserved(_) | Phase::Running(_) => {
                self.phase = Phase::Stopping(lease);
                Ok(CaptureCompletion::Current)
            }
            Phase::Idle | Phase::Stopping(_) => Err(CaptureRefusal::Order),
        }
    }

    /// Call only after the device actor confirms streams were actually stopped/dropped.
    ///
    /// # Errors
    /// Queueing a Stop, release failure or a running stream is not release evidence.
    pub fn released(&mut self, lease: CaptureLease) -> Result<CaptureCompletion, CaptureRefusal> {
        if self.owner() != Some(lease) {
            return Ok(CaptureCompletion::Stale);
        }
        match self.phase {
            Phase::Stopping(_) => {
                self.phase = Phase::Idle;
                Ok(CaptureCompletion::Current)
            }
            Phase::Idle | Phase::Reserved(_) | Phase::Running(_) => Err(CaptureRefusal::Order),
        }
    }

    /// Capture/upload permission never follows merely from holding a reservation.
    #[must_use]
    pub fn accepts_frames(&self, lease: CaptureLease) -> bool {
        matches!(self.phase, Phase::Running(current) if current == lease)
    }

    /// A pending or failed release remains visible to every prospective consumer.
    #[must_use]
    pub const fn owner(&self) -> Option<CaptureLease> {
        match self.phase {
            Phase::Idle => None,
            Phase::Reserved(lease) | Phase::Running(lease) | Phase::Stopping(lease) => Some(lease),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_and_dictation_are_exclusive_until_physical_release_confirmation() {
        let mut slot = CaptureSlot::default();
        let voice = slot.reserve(CapturePurpose::Conversation).unwrap();
        assert!(!slot.accepts_frames(voice));
        assert_eq!(
            slot.reserve(CapturePurpose::Dictation),
            Err(CaptureRefusal::Busy(CapturePurpose::Conversation))
        );
        assert_eq!(slot.started(voice), Ok(CaptureCompletion::Current));
        assert!(slot.accepts_frames(voice));
        assert_eq!(slot.released(voice), Err(CaptureRefusal::Order));
        assert_eq!(slot.stopping(voice), Ok(CaptureCompletion::Current));
        assert!(!slot.accepts_frames(voice));
        // No release callback means cleanup is still uncertain; retain the slot.
        assert_eq!(
            slot.reserve(CapturePurpose::Dictation),
            Err(CaptureRefusal::Busy(CapturePurpose::Conversation))
        );
        assert_eq!(slot.released(voice), Ok(CaptureCompletion::Current));
        let dictation = slot.reserve(CapturePurpose::Dictation).unwrap();
        assert_eq!(dictation.purpose(), CapturePurpose::Dictation);
        assert_ne!(voice, dictation);
        assert_eq!(slot.released(voice), Ok(CaptureCompletion::Stale));
        assert_eq!(slot.started(voice), Ok(CaptureCompletion::Stale));
        assert_eq!(slot.owner(), Some(dictation));
    }

    #[test]
    fn stop_during_preparation_retires_late_start_without_releasing_ownership() {
        let mut slot = CaptureSlot::default();
        let lease = slot.reserve(CapturePurpose::Dictation).unwrap();
        assert_eq!(slot.stopping(lease), Ok(CaptureCompletion::Current));
        assert_eq!(slot.started(lease), Ok(CaptureCompletion::Stale));
        assert_eq!(slot.owner(), Some(lease));
        assert!(!slot.accepts_frames(lease));
        assert_eq!(slot.released(lease), Ok(CaptureCompletion::Current));
        assert_eq!(slot.owner(), None);
    }

    #[test]
    fn exhausted_identity_never_wraps_into_an_earlier_callback() {
        let mut slot = CaptureSlot {
            next: u64::MAX,
            phase: Phase::Idle,
        };
        assert_eq!(
            slot.reserve(CapturePurpose::Dictation),
            Err(CaptureRefusal::Exhausted)
        );
        assert_eq!(slot.owner(), None);
    }
}
