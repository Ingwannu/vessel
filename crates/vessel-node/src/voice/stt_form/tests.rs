use super::*;
use crate::voice::input_buffer::InputBuffer;
use vessel_proto::voice::CapturePcm;

fn utterance() -> Utterance {
    let mut buffer = InputBuffer::default();
    buffer
        .append(&CapturePcm::new(vec![0; 320]).unwrap())
        .unwrap();
    buffer.confirmed().unwrap();
    buffer.seal().unwrap()
}

#[test]
fn provider_wav_is_shared_24k_codec_and_voice_never_inherits_dictation_prompt() {
    let form = Form::new(
        &utterance(),
        &EndpointModel::whisper(),
        &Language::english(),
    )
    .unwrap();
    assert!(!format!("{form:?}").contains("RIFF"));
    let (kind, body) = form.into_parts();
    assert!(
        kind.to_str()
            .unwrap()
            .starts_with("multipart/form-data; boundary=vessel-voice-")
    );
    for field in [
        b"name=\"model\"".as_slice(),
        b"whisper-1",
        b"name=\"language\"",
        b"name=\"response_format\"",
        b"utterance.wav",
    ] {
        assert!(body.windows(field.len()).any(|bytes| bytes == field));
    }
    for forbidden in [b"name=\"prompt\"".as_slice(), b"include[]"] {
        assert!(
            !body
                .windows(forbidden.len())
                .any(|bytes| bytes == forbidden)
        );
    }
    let wav = body.windows(4).position(|bytes| bytes == b"RIFF").unwrap();
    let rate = &body[wav + 24..wav + 28];
    assert_eq!(u32::from_le_bytes(rate.try_into().unwrap()), 24_000);
    let data = &body[wav + 40..wav + 44];
    assert_eq!(u32::from_le_bytes(data.try_into().unwrap()), 478);
}

#[test]
fn only_supported_models_request_scores_and_long_forms_stay_bounded() {
    for name in ["gpt-4o-transcribe", "gpt-4o-mini-transcribe"] {
        let (_, body) = Form::new(&utterance(), &name.parse().unwrap(), &Language::english())
            .unwrap()
            .into_parts();
        assert!(
            body.windows(b"include[]".len())
                .any(|bytes| bytes == b"include[]")
        );
        assert!(
            body.windows(b"logprobs".len())
                .any(|bytes| bytes == b"logprobs")
        );
    }
    let mut buffer = InputBuffer::default();
    buffer.confirmed().unwrap();
    let pcm = CapturePcm::new(vec![0; 3_200]).unwrap();
    for _ in 0..600 {
        buffer.append(&pcm).unwrap();
    }
    let (_, body) = Form::new(
        &buffer.seal().unwrap(),
        &EndpointModel::whisper(),
        &Language::english(),
    )
    .unwrap()
    .into_parts();
    assert!(body.len() <= MAX_FORM_BYTES);
    let mut empty = InputBuffer::default();
    empty.confirmed().unwrap();
    assert!(matches!(
        Form::new(
            &empty.seal().unwrap(),
            &EndpointModel::whisper(),
            &Language::english()
        ),
        Err(FormError::Input)
    ));
}
