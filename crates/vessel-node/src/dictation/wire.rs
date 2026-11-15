//! One value-free conversion from private dictation causes to protocol refusals.
use super::{
    input::InputError, prepare::PrepareError, registry::RegistryError, stream_error::StreamError,
    updates::ReadRefusal,
};
use vessel_proto::{ErrorCode, ErrorDetail, WireError, dictation::DictationReason};

pub(super) fn refusal(error: &RegistryError) -> WireError {
    let (code, reason) = match error {
        RegistryError::Taken | RegistryError::Conflict | RegistryError::Mailbox(_) => {
            (ErrorCode::Conflict, None)
        }
        RegistryError::Forbidden => (ErrorCode::Forbidden, None),
        RegistryError::Gone => (ErrorCode::NotFound, None),
        RegistryError::Quota(_) => (ErrorCode::Unavailable, Some(DictationReason::TooMany)),
        RegistryError::Input(source) => (ErrorCode::InvalidRequest, Some(input_reason(source))),
        RegistryError::Prepare(source) => preparation(source),
        RegistryError::Stream(source) => stream(source),
        RegistryError::Worker(_) | RegistryError::Reply(_) => {
            (ErrorCode::Internal, Some(DictationReason::Upstream))
        }
    };
    let mut wire = WireError::new(code, error.to_string());
    wire.detail = reason.map(|reason| ErrorDetail::Dictation { reason });
    wire
}

fn preparation(error: &PrepareError) -> (ErrorCode, Option<DictationReason>) {
    match error {
        PrepareError::Configuration(_) => (ErrorCode::Unsupported, None),
        PrepareError::Disabled => (ErrorCode::Unsupported, Some(DictationReason::Disabled)),
        PrepareError::LocalNotBuilt { model } => (
            ErrorCode::Unsupported,
            Some(DictationReason::LocalNotBuilt {
                model: model.clone(),
            }),
        ),
        PrepareError::KeyAbsent => (ErrorCode::Unavailable, Some(DictationReason::KeyAbsent)),
        PrepareError::Secret(_) | PrepareError::Endpoint(_) => {
            (ErrorCode::Unavailable, Some(DictationReason::Upstream))
        }
    }
}

fn input_reason(error: &InputError) -> DictationReason {
    match error {
        InputError::BadAudio => DictationReason::BadAudio,
        InputError::TooLong => DictationReason::TooLong,
        InputError::ReorderOverflow => DictationReason::ReorderOverflow,
        InputError::NoAudio => DictationReason::NoAudio,
        InputError::Count(_) => DictationReason::Upstream,
    }
}

fn stream(error: &StreamError) -> (ErrorCode, Option<DictationReason>) {
    if let Some(reason) = error.reason() {
        let code = match &reason {
            DictationReason::BadAudio
            | DictationReason::TooLong
            | DictationReason::ReorderOverflow
            | DictationReason::NoAudio => ErrorCode::InvalidRequest,
            DictationReason::Replaced | DictationReason::Cancelled => ErrorCode::Conflict,
            DictationReason::Disabled
            | DictationReason::LocalNotBuilt { .. }
            | DictationReason::Unknown => ErrorCode::Unsupported,
            DictationReason::KeyAbsent
            | DictationReason::TranscriptTooLong
            | DictationReason::TimedOut
            | DictationReason::Upstream
            | DictationReason::TooMany => ErrorCode::Unavailable,
        };
        return (code, Some(reason));
    }
    // These are the only non-terminal actor refusals. A second reader must
    // conflict without consuming an update; transport loss names no stream.
    match error {
        StreamError::Read(ReadRefusal::Waiting) => (ErrorCode::Conflict, None),
        StreamError::Read(ReadRefusal::Gone)
        | StreamError::Gone
        | StreamError::Send(_)
        | StreamError::Reply(_) => (ErrorCode::NotFound, None),
        StreamError::Input(_)
        | StreamError::Audio(_)
        | StreamError::Text(_)
        | StreamError::Budget(_)
        | StreamError::Provider(_)
        | StreamError::Clock
        | StreamError::Capacity
        | StreamError::Partial(_)
        | StreamError::Terminal(_)
        | StreamError::Transcript(_)
        | StreamError::TimedOut
        | StreamError::Replaced
        | StreamError::Cancelled => (ErrorCode::Internal, Some(DictationReason::Upstream)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictation::{endpoint::ProviderError, roots::RootsError};
    use std::{error::Error, sync::Arc};
    use vessel_proto::dictation::LocalModelId;

    #[test]
    fn a_private_foreign_cause_cannot_escape_through_wire_display_debug_or_sources() {
        const MARKER: &str = "foreign-private-marker-65f41d4a";
        let error = RegistryError::Prepare(PrepareError::Endpoint(ProviderError::Roots(Arc::new(
            RootsError::Tls(rustls::Error::General(MARKER.to_owned())),
        ))));
        let mut source: Option<&(dyn Error + 'static)> = Some(&error);
        let mut private_marker_present = false;
        while let Some(error) = source {
            private_marker_present |= error.to_string().contains(MARKER);
            source = error.source();
        }
        assert!(
            private_marker_present,
            "the fixture must retain a real payload-bearing cause"
        );
        let wire = refusal(&error);
        assert_eq!(wire.code, ErrorCode::Unavailable);
        assert_eq!(
            wire.detail,
            Some(ErrorDetail::Dictation {
                reason: DictationReason::Upstream
            })
        );
        assert!(!wire.to_string().contains(MARKER));
        assert!(!format!("{wire:?}").contains(MARKER));
        assert!(wire.source().is_none());
    }

    #[test]
    fn disabled_and_missing_key_use_the_adrs_distinct_refusal_classes() {
        for (source, code, reason) in [
            (
                PrepareError::Disabled,
                ErrorCode::Unsupported,
                DictationReason::Disabled,
            ),
            (
                PrepareError::KeyAbsent,
                ErrorCode::Unavailable,
                DictationReason::KeyAbsent,
            ),
        ] {
            let wire = refusal(&RegistryError::Prepare(source));
            assert_eq!(wire.code, code);
            assert_eq!(wire.detail, Some(ErrorDetail::Dictation { reason }));
        }
    }

    #[test]
    fn local_refusals_keep_the_required_model_and_pending_reads_conflict_without_ending_audio() {
        let model = LocalModelId::default_model();
        let wire = refusal(&RegistryError::Prepare(PrepareError::LocalNotBuilt {
            model: model.clone(),
        }));
        assert_eq!(wire.code, ErrorCode::Unsupported);
        assert_eq!(
            wire.detail,
            Some(ErrorDetail::Dictation {
                reason: DictationReason::LocalNotBuilt { model }
            })
        );
        assert_eq!(
            refusal(&RegistryError::Stream(Arc::new(StreamError::Read(
                ReadRefusal::Waiting
            ))))
            .code,
            ErrorCode::Conflict
        );
        let no_audio = refusal(&RegistryError::Stream(Arc::new(StreamError::Input(
            InputError::NoAudio,
        ))));
        assert_eq!(no_audio.code, ErrorCode::InvalidRequest);
        assert_eq!(
            no_audio.detail,
            Some(ErrorDetail::Dictation {
                reason: DictationReason::NoAudio
            })
        );
    }
}
