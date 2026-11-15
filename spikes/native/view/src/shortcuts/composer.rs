//! Typed composer adapter contract for actions emitted by the single shortcut
//! interceptor and command center. The composer owns send/queue policy, modes,
//! audio state and validation; this enum carries no inferred provider behavior.

/// The active composer's owner decides whether the action was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComposerAction {
    /// Restore focus to the current composer.
    Focus,
    /// Send using the current send/queue setting.
    Send,
    /// Interrupt the running turn, where the composer allows it.
    Interrupt,
    /// Cycle only the currently offered agent modes.
    ModeCycle,
    /// Start or stop dictation through its existing controller.
    DictationToggle,
    /// Cancel a dictation that is actually active.
    DictationCancel,
    /// Confirm a dictation that is actually active.
    DictationConfirm,
    /// Start or stop realtime voice.
    VoiceToggle,
    /// Mute or unmute an active realtime voice session.
    VoiceMuteToggle,
}
