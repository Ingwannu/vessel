//! Synthetic fixtures pin counts, sample tolerance and format without logging audio.
use super::*;

const FIXTURE_TOLERANCE: u16 = 1;
fn pcm(samples: usize) -> Vec<u8> {
    1000_i16.to_le_bytes().repeat(samples)
}
fn rates() -> [Rate; 6] {
    [
        Rate::MIN,
        Rate::new(16_000).unwrap(),
        Rate::new(22_050).unwrap(),
        PROVIDER_RATE,
        Rate::new(44_100).unwrap(),
        Rate::MAX,
    ]
}

#[test]
fn chunk_boundaries_keep_phase_and_carry_at_every_fixture_rate() {
    for rate in rates() {
        let second = usize::try_from(rate.get()).unwrap();
        let whole = pcm(second.checked_mul(2).unwrap());
        let chunk = pcm(second);
        let mut once = Resampler::new(rate);
        let joined = once.push(Pcm16::new(&whole).unwrap()).unwrap();
        let mut split = Resampler::new(rate);
        let first = split.push(Pcm16::new(&chunk).unwrap()).unwrap();
        let last = split.push(Pcm16::new(&chunk).unwrap()).unwrap();
        let mut combined = first.as_slice().to_vec();
        combined.extend_from_slice(last.as_slice());
        assert!(
            joined.as_slice() == combined,
            "chunking changed the resampled samples"
        );
        assert_eq!(format!("{joined:?}"), "ResampledPcm([redacted])");
    }
}

#[test]
fn independently_recorded_paseo_fixture_counts_are_preserved() {
    for (rate, expected) in rates()
        .into_iter()
        .zip([96_036, 96_018, 96_014, 96_013, 96_007, 96_006])
    {
        let second = usize::try_from(rate.get()).unwrap();
        let mut resampler = Resampler::new(rate);
        let mut count = 0_usize;
        for samples in [second, second, second, second, 13] {
            let input = pcm(samples);
            let output = resampler.push(Pcm16::new(&input).unwrap()).unwrap();
            count = count.saturating_add(output.as_slice().len().div_euclid(SAMPLE_BYTES));
        }
        assert_eq!(count, expected);
    }
}

#[test]
fn signed_extremes_and_zero_crossing_keep_the_stated_fixture_tolerance() {
    let expected: &[&[i16]] = &[
        &[
            -32767, -22178, -11589, -1000, -667, -333, 0, 333, 667, 1000, 11589, 22177, 32766,
        ],
        &[-32767, -11589, -667, 0, 667, 11589],
        &[-32767, -3581, -162, 756, 22442],
        &[-32768, -1000, 0, 1000, 32767],
        &[-32767, -162, 22442],
        &[-32767, 0],
    ];
    for (rate, reference) in rates().into_iter().zip(expected) {
        let mut converter = Resampler::new(rate);
        let mut actual = Vec::new();
        for samples in [&[i16::MIN, -1000, 0][..], &[1000, i16::MAX][..]] {
            let input: Vec<_> = samples
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect();
            let output = converter.push(Pcm16::new(&input).unwrap()).unwrap();
            actual.extend(
                output
                    .as_slice()
                    .chunks_exact(SAMPLE_BYTES)
                    .map(|pair| i16::from_le_bytes(pair.try_into().unwrap())),
            );
        }
        assert!(!actual.is_empty());
        assert!(actual.len().abs_diff(reference.len()) <= usize::from(FIXTURE_TOLERANCE));
        for (actual, expected) in actual.iter().zip(reference.iter()) {
            assert!(
                actual.abs_diff(*expected) <= FIXTURE_TOLERANCE,
                "sample fixture tolerance exceeded"
            );
        }
    }
}

#[test]
fn aligned_views_and_the_matching_rate_bypass_do_not_copy_capture() {
    let input = pcm(3);
    let view = Pcm16::new(&input).unwrap();
    assert_eq!(view.as_slice().as_ptr(), input.as_ptr());
    let mut converter = Resampler::new(PROVIDER_RATE);
    let output = converter.push(view).unwrap();
    assert_eq!(output.as_slice().as_ptr(), input.as_ptr());
    assert!(output.as_slice() == input);
    assert_eq!(format!("{view:?}"), "Pcm16([redacted])");
    assert!(matches!(Pcm16::new(&[0]), Err(CodecError::Alignment)));
    assert_eq!(Rate::new(PROVIDER_RATE.get()).unwrap(), PROVIDER_RATE);
}

#[test]
fn wav_is_appended_after_existing_metadata_with_exact_declared_format() {
    for rate in [Rate::new(16_000).unwrap(), PROVIDER_RATE] {
        let input = pcm(3);
        let wave = Wav::new(rate, Pcm16::new(&input).unwrap()).unwrap();
        let prefix = b"existing-body-prefix";
        let mut body = prefix.to_vec();
        wave.append_to(&mut body).unwrap();
        let encoded = body.strip_prefix(prefix).unwrap();
        assert!(encoded.starts_with(b"RIFF"));
        assert!(encoded.get(8..16) == Some(b"WAVEfmt ".as_slice()));
        assert!(encoded.get(36..40) == Some(b"data".as_slice()));
        assert_eq!(
            u32::from_le_bytes(encoded[4..8].try_into().unwrap()),
            u32::try_from(encoded.len().checked_sub(wav::RIFF_PREFIX_BYTES).unwrap()).unwrap()
        );
        assert_eq!(
            u32::from_le_bytes(encoded[16..20].try_into().unwrap()),
            wav::FMT_BYTES
        );
        assert_eq!(
            u16::from_le_bytes(encoded[20..22].try_into().unwrap()),
            wav::PCM_FORMAT
        );
        assert_eq!(
            u16::from_le_bytes(encoded[22..24].try_into().unwrap()),
            wav::MONO
        );
        assert_eq!(
            u32::from_le_bytes(encoded[24..28].try_into().unwrap()),
            rate.get()
        );
        assert_eq!(
            u32::from_le_bytes(encoded[28..32].try_into().unwrap()),
            rate.get()
                .saturating_mul(u32::try_from(SAMPLE_BYTES).unwrap())
        );
        assert_eq!(
            u16::from_le_bytes(encoded[32..34].try_into().unwrap()),
            u16::try_from(SAMPLE_BYTES).unwrap()
        );
        assert_eq!(
            u32::from(u16::from_le_bytes(encoded[34..36].try_into().unwrap())),
            i16::BITS
        );
        assert_eq!(
            u32::from_le_bytes(encoded[40..wav::HEADER_BYTES].try_into().unwrap()),
            u32::try_from(input.len()).unwrap()
        );
        assert!(encoded.get(wav::HEADER_BYTES..) == Some(input.as_slice()));
        assert_eq!(format!("{wave:?}"), "Wav([redacted])");
    }
}

#[test]
fn empty_codec_output_is_not_a_capture_or_silence_policy() {
    let empty = Pcm16::new(&[]).unwrap();
    let output = Resampler::new(PROVIDER_RATE).push(empty).unwrap();
    assert!(output.as_slice().is_empty());
    let wave = Wav::new(PROVIDER_RATE, output.as_pcm()).unwrap();
    let mut encoded = Vec::new();
    wave.append_to(&mut encoded).unwrap();
    assert_eq!(
        u32::from_le_bytes(encoded[40..wav::HEADER_BYTES].try_into().unwrap()),
        0
    );
}

#[test]
fn allocation_refusal_keeps_the_existing_body_prefix_and_typed_cause() {
    let mut body = b"prefix".to_vec();
    let error = reserve(&mut body, usize::MAX).unwrap_err();
    assert!(matches!(error, CodecError::Allocation(_)));
    assert!(std::error::Error::source(&error).is_some());
    assert!(body == b"prefix");
}
