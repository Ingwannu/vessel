//! Pending voice endpoints remain fail-closed while reload/provider prerequisites land.
//!
//! The method table registers the typed boundary before the implementation.
//! `SINCE=None` refuses every RPC before these handlers. No capability is
//! advertised, device/provider opened or prompt changed by this skeleton.

use vessel_proto::voice::{
    AbortVoice, CaptureAck, CaptureVoice, NextVoice, PlaybackOutcome, PlayedVoice, ReadyVoice,
    SpeakVoice, StartVoice, StopVoice, StopVoiceResult, VoiceFailure, VoiceUpdate,
};
use vessel_proto::{Ack, ErrorCode, ErrorDetail, WireError};

use super::{Connection, NodeService};
use crate::principal::Principal;

fn unavailable<T>() -> Result<T, WireError> {
    Err(WireError {
        code: ErrorCode::Unsupported,
        message: "The voice runtime is not ready".to_owned(),
        detail: Some(ErrorDetail::Voice {
            reason: VoiceFailure::LocalNotBuilt,
        }),
    })
}

impl NodeService {
    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn start_voice(
        &self,
        _: &Connection,
        _: StartVoice,
    ) -> Result<ReadyVoice, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn capture_voice(
        &self,
        _: &Connection,
        _: CaptureVoice,
    ) -> Result<CaptureAck, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn next_voice(
        &self,
        _: &Connection,
        _: NextVoice,
    ) -> Result<VoiceUpdate, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn played_voice(
        &self,
        _: &Connection,
        _: PlayedVoice,
    ) -> Result<Ack, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn abort_voice(
        &self,
        _: &Connection,
        _: AbortVoice,
    ) -> Result<Ack, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn stop_voice(
        &self,
        _: &Connection,
        _: StopVoice,
    ) -> Result<StopVoiceResult, WireError> {
        unavailable()
    }

    #[expect(
        clippy::unused_async,
        reason = "pending generated async handler before provider/reload wiring"
    )]
    pub(crate) async fn speak_voice(
        &self,
        connection: &Connection,
        _: SpeakVoice,
    ) -> Result<PlaybackOutcome, WireError> {
        match connection.principal() {
            Principal::Agent { .. } => unavailable(),
            Principal::Owner | Principal::Device { .. } | Principal::Delegated { .. } => {
                Err(WireError::new(
                    ErrorCode::Forbidden,
                    "Only the active voice session's agent can speak",
                ))
            }
        }
    }
}
