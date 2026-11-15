//! Independent fixed-form framing checks; no assertion prints body/audio bytes.

use super::*;
use vessel_proto::dictation::DictationPcm;

fn segment(bytes: Vec<u8>) -> Segment {
    Segment::from_test_pcm(DictationPcm::new(bytes).unwrap())
}

fn assert_wav(value: &[u8], pcm: &Segment) {
    assert!(value.starts_with(b"RIFF"));
    assert!(value.ends_with(pcm.as_slice()));
    assert_eq!(
        u32::from_le_bytes(value[4..8].try_into().unwrap()),
        u32::try_from(value.len().checked_sub(8).unwrap()).unwrap()
    );
    let channels = u16::from_le_bytes(value[22..24].try_into().unwrap());
    let hz = u32::from_le_bytes(value[24..28].try_into().unwrap());
    let bits = u16::from_le_bytes(value[34..36].try_into().unwrap());
    let width = channels
        .checked_mul(bits.div_euclid(u16::try_from(u8::BITS).unwrap()))
        .unwrap();
    assert_eq!(
        u32::from_le_bytes(value[28..32].try_into().unwrap()),
        hz.checked_mul(u32::from(width)).unwrap()
    );
    assert_eq!(u16::from_le_bytes(value[32..34].try_into().unwrap()), width);
    assert_eq!(
        u32::from_le_bytes(value[40..44].try_into().unwrap()),
        u32::try_from(pcm.as_slice().len()).unwrap()
    );
}

#[test]
fn the_form_has_matching_framing_utf8_text_and_one_binary_wav() {
    let model: EndpointModel = "módèl\"x".parse().unwrap();
    let language = Language::english();
    let pcm = segment(1000_i16.to_le_bytes().to_vec());
    let form = Multipart::new(
        &pcm,
        &Fields {
            model: &model,
            language: &language,
        },
    )
    .unwrap();
    let header = form.content_type.to_str().unwrap();
    let boundary = header
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_eq!(boundary.len(), DELIMITER_DIGITS);
    assert!(boundary.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let delimiter = format!("--{boundary}");
    let body = &form.bytes;
    let starts: Vec<_> = body
        .windows(delimiter.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == delimiter.as_bytes()).then_some(index))
        .collect();
    let names = ["model", "language", "response_format", "prompt", "file"];
    assert_eq!(starts.len(), names.len() + 1);
    assert_eq!(starts[0], 0);
    assert!(&body[starts[names.len()] + delimiter.len()..] == b"--\r\n");
    for (index, pair) in starts.windows(2).enumerate() {
        assert!(&body[pair[0] + delimiter.len()..pair[0] + delimiter.len() + 2] == b"\r\n");
        assert!(&body[pair[1] - 2..pair[1]] == b"\r\n");
        let part = &body[pair[0] + delimiter.len() + 2..pair[1] - 2];
        let separator = part
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let headers = std::str::from_utf8(&part[..separator]).unwrap();
        assert!(headers.starts_with(&format!(
            "content-disposition: form-data; name=\"{}\"",
            names[index]
        )));
        let value = &part[separator + 4..];
        match index {
            0 => assert!(value == model.as_str().as_bytes(), "model bytes changed"),
            1 => assert!(
                value == language.as_str().as_bytes(),
                "language bytes changed"
            ),
            2 => assert!(value == b"json", "response format changed"),
            3 => assert!(value == PROMPT.as_bytes(), "fixed dictation prompt changed"),
            4 => {
                assert!(headers.contains("filename=\"audio.wav\"\r\ncontent-type: audio/wav"));
                assert_wav(value, &pcm);
            }
            _ => panic!("unexpected form part count"),
        }
    }
}

#[test]
fn entropy_failures_and_delimiter_collisions_are_bounded_typed_refusals() {
    let marker = "0".repeat(DELIMITER_DIGITS);
    let model: EndpointModel = marker.parse().unwrap();
    let language = Language::english();
    let fields = Fields {
        model: &model,
        language: &language,
    };
    let pcm = segment(1000_i16.to_le_bytes().to_vec());
    let mut calls = 0;
    let failure = Boundary::with_entropy(&pcm, &fields, || {
        calls += 1;
        Ok([0; DELIMITER_BYTES])
    });
    assert!(matches!(failure, Err(FormError::Delimiter)));
    assert_eq!(calls, MAX_DELIMITER_ATTEMPTS);
    let error = Boundary::with_entropy(&pcm, &fields, || Err(getrandom::Error::UNSUPPORTED))
        .err()
        .unwrap();
    assert!(matches!(error, FormError::Entropy(_)));
    assert!(std::error::Error::source(&error).is_some());
    let raw = "a".repeat(DELIMITER_DIGITS);
    let binary = segment(raw.as_bytes().to_vec());
    assert!(Boundary(raw).collides(&binary, &fields));
}
