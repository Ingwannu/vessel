//! Synthetic arithmetic fixtures report lengths and metadata, never sample bytes.

use super::*;

fn pcm(samples: usize) -> DictationPcm {
    DictationPcm::new(1_000_i16.to_le_bytes().repeat(samples)).unwrap()
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
        let input = pcm_sample(sample);
        let mut converter = Resampler::new(crate::speech_audio::PROVIDER_RATE);
        segment.append(&converter.push(&input).unwrap()).unwrap();
        let committed = segment.seal();
        assert_eq!(committed.is_some(), audible);
        assert!(segment.seal().is_none());
        if let Some(committed) = committed {
            let mut wave = Vec::new();
            committed.append_wav(&mut wave).unwrap();
            assert!(wave.starts_with(b"RIFF"));
            assert!(wave.ends_with(committed.as_slice()));
        }
    }
}

#[test]
fn oversized_segments_are_refused_before_mutation() {
    let mut segment = SegmentBuffer::new().unwrap();
    let input = pcm_sample(0);
    let mut converter = Resampler::new(crate::speech_audio::PROVIDER_RATE);
    segment.append(&converter.push(&input).unwrap()).unwrap();
    let length = segment.pcm.len();
    let oversized_input = vec![0; segment.bytes_per_second * MAX_SEGMENT_SECONDS];
    let mut converter = crate::speech_audio::Resampler::new(crate::speech_audio::PROVIDER_RATE);
    let oversized = converter
        .push(Pcm16::new(&oversized_input).unwrap())
        .unwrap();
    assert!(matches!(
        segment.append(&oversized),
        Err(AudioError::SegmentTooLong)
    ));
    assert_eq!(segment.pcm.len(), length);
    assert_eq!(segment.peak, 0);
}

fn pcm_sample(sample: i16) -> DictationPcm {
    DictationPcm::new(sample.to_le_bytes().to_vec()).unwrap()
}
