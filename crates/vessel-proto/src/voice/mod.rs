//! Spoken-conversation settings metadata only (ADR 0080).
//!
//! These DTOs register no RPC, service, storage field or runtime. Readiness is
//! separate from a selected provider or model; the integrator assigns no number
//! for this values-only carrier. Host UI consumers must use a typed fake until
//! the real node settings field and adapter are present.

mod settings;
pub use settings::{
    LocalTtsModelId, SetVoice, SetVoiceStt, SetVoiceTts, TtsModel, TtsVoice, UnknownTtsModelId,
    VoiceSettings, VoiceSettingsError, VoiceSttSettings, VoiceTtsSettings,
};

/// Unassigned package gate; metadata presence never advertises voice support.
pub const SINCE: Option<crate::ProtocolVersion> = None;
