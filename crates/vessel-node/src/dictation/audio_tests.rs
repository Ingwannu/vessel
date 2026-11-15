//! Synthetic arithmetic fixtures report lengths and metadata, never sample bytes.

use super::*;

fn pcm(samples: usize) -> DictationPcm {
    DictationPcm::new(1_000_i16.to_le_bytes().repeat(samples)).unwrap()
}

#[test]
fn chunk_boundaries_preserve_integer_phase_and_carry() {
    for hz in [
        SampleRate::MIN.get(),
        16_000,
        22_050,
        PROVIDER_HZ,
        44_100,
        SampleRate::MAX.get(),
    ] {
        let rate = SampleRate::new(hz).unwrap();
        let second = usize::try_from(hz).unwrap();
        let mut once = Resampler::new(rate);
        let joined = once.push(&pcm(second * 2)).unwrap();
        let mut chunked = Resampler::new(rate);
        let first = chunked.push(&pcm(second)).unwrap();
        let last = chunked.push(&pcm(second)).unwrap();
        let mut split = first.0.to_vec();
        split.extend_from_slice(&last.0);
        assert!(joined.0 == split, "splitting input changed resampled audio");
        assert_eq!(format!("{joined:?}"), "ResampledPcm([redacted])");
    }
}

#[test]
fn synthetic_paseo_comparison_counts_pin_the_stated_rounding_difference() {
    // Values from the independently executed source comparison, not a claim
    // about arbitrary audio or recognition quality.
    for (hz, expected) in [
        (SampleRate::MIN.get(), 96_036),
        (16_000, 96_018),
        (22_050, 96_014),
        (PROVIDER_HZ, 96_013),
        (44_100, 96_007),
        (SampleRate::MAX.get(), 96_006),
    ] {
        let mut resampler = Resampler::new(SampleRate::new(hz).unwrap());
        let second = usize::try_from(hz).unwrap();
        let mut count = 0;
        for samples in [second, second, second, second, 13] {
            count += resampler
                .push(&pcm(samples))
                .unwrap()
                .0
                .len()
                .div_euclid(SAMPLE_BYTES);
        }
        assert_eq!(count, expected);
    }
}

#[test]
fn commit_threshold_follows_whole_resampled_chunks() {
    let mut resampler = Resampler::new(SampleRate::new(16_000).unwrap());
    let mut segment = SegmentBuffer::new().unwrap();
    for _ in 0..COMMIT_SECONDS {
        segment
            .append(&resampler.push(&pcm(16_000)).unwrap())
            .unwrap();
    }
    assert!(!segment.ready());
    segment
        .append(&resampler.push(&pcm(16_000)).unwrap())
        .unwrap();
    assert!(segment.ready());
    let committed = segment.seal().unwrap();
    assert!(committed.as_slice().len() < segment.bytes_per_second * MAX_SEGMENT_SECONDS);
    assert_eq!(format!("{committed:?}"), "Segment([redacted])");
    assert!(!segment.ready());
    assert_eq!(segment.duration().unwrap(), Duration::ZERO);
}

#[test]
fn silence_is_discarded_and_a_loud_tail_has_a_provider_rate_wav() {
    let quiet = i16::try_from(SILENCE_PEAK.saturating_sub(1)).unwrap();
    let audible = i16::try_from(SILENCE_PEAK).unwrap();
    for (sample, audible) in [
        (0_i16, false),
        (quiet, false),
        (audible, true),
        (i16::MIN, true),
    ] {
        let mut segment = SegmentBuffer::new().unwrap();
        segment
            .append(&ResampledPcm(Bytes::copy_from_slice(&sample.to_le_bytes())))
            .unwrap();
        let committed = segment.seal();
        assert_eq!(committed.is_some(), audible);
        assert!(segment.seal().is_none());
        if let Some(committed) = committed {
            let mut wave = Vec::new();
            committed.append_wav(&mut wave).unwrap();
            assert!(wave.starts_with(b"RIFF"));
            assert_eq!(wave.len(), WAV_HEADER_BYTES + SAMPLE_BYTES);
            assert_eq!(
                u16::from_le_bytes(wave[20..22].try_into().unwrap()),
                PCM_FORMAT
            );
            assert_eq!(
                u16::from_le_bytes(wave[22..24].try_into().unwrap()),
                MONO_CHANNELS
            );
            assert_eq!(
                u32::from_le_bytes(wave[24..28].try_into().unwrap()),
                PROVIDER_HZ
            );
            assert_eq!(
                u32::from(u16::from_le_bytes(wave[34..36].try_into().unwrap())),
                i16::BITS
            );
        }
    }
}

#[test]
fn oversized_segments_are_refused_before_mutation() {
    let mut segment = SegmentBuffer::new().unwrap();
    segment
        .append(&ResampledPcm(Bytes::from_static(&[0, 0])))
        .unwrap();
    let length = segment.pcm.len();
    let oversized = ResampledPcm(vec![0; segment.bytes_per_second * MAX_SEGMENT_SECONDS].into());
    assert!(matches!(
        segment.append(&oversized),
        Err(AudioError::SegmentTooLong)
    ));
    assert_eq!(segment.pcm.len(), length);
    assert_eq!(segment.peak, 0);
}

#[test]
fn interpolated_samples_match_the_small_source_fixture_within_one_lsb() {
    // Independently evaluated Paseo fixture: two chunks, signed extremes and
    // a zero crossing. Comparison is restricted to the shared prefix because
    // floating phase can emit an additional endpoint sample.
    let fixtures: &[(u32, &[i16])] = &[
        (
            SampleRate::MIN.get(),
            &[
                -32767, -22178, -11589, -1000, -667, -333, 0, 333, 667, 1000, 11589, 22177, 32766,
            ],
        ),
        (16_000, &[-32767, -11589, -667, 0, 667, 11589]),
        (22_050, &[-32767, -3581, -162, 756, 22442]),
        (PROVIDER_HZ, &[-32768, -1000, 0, 1000, 32767]),
        (44_100, &[-32767, -162, 22442]),
        (SampleRate::MAX.get(), &[-32767, 0]),
    ];
    for (hz, reference) in fixtures {
        let mut converter = Resampler::new(SampleRate::new(*hz).unwrap());
        let mut actual = Vec::new();
        for chunk in [&[i16::MIN, -1000, 0][..], &[1000, i16::MAX][..]] {
            let bytes: Vec<_> = chunk
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect();
            let output = converter.push(&DictationPcm::new(bytes).unwrap()).unwrap();
            actual.extend(
                output
                    .0
                    .chunks_exact(SAMPLE_BYTES)
                    .map(|pair| i16::from_le_bytes(pair.try_into().unwrap())),
            );
        }
        assert!(!actual.is_empty());
        assert!(actual.len().abs_diff(reference.len()) <= 1);
        for (actual, expected) in actual.iter().zip(reference.iter()) {
            assert!(
                actual.abs_diff(*expected) <= 1,
                "source fixture exceeded the stated rounding difference"
            );
        }
    }
}
