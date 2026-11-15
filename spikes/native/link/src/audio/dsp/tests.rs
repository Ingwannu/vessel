use super::*;

fn format(rate: u32, channels: Channels) -> Format {
    Format::new(RateHz::new(rate).unwrap(), channels).unwrap()
}

fn decode(frame: &Captured) -> Vec<i16> {
    frame
        .pcm()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| i16::from_le_bytes(*bytes))
        .collect()
}

#[test]
fn malformed_callback_frames_are_refused_before_processing() {
    assert!(RateHz::new(22_050).is_err());
    assert!(RateHz::new(0).is_err());
    assert!(Channels::try_from(8).is_err());
    let mut processor = Processor::new(
        format(48_000, Channels::Mono),
        format(48_000, Channels::Stereo),
    )
    .unwrap();
    assert!(matches!(
        processor.capture(&[0.0; 479]),
        Err(AudioError::Frame)
    ));
    assert!(matches!(
        processor.capture(&[f32::NAN; 480]),
        Err(AudioError::NonFinite)
    ));
    assert!(matches!(
        processor.capture(&[1.1; 480]),
        Err(AudioError::Range)
    ));
    assert!(matches!(
        processor.prepare_render(&[-1.1; 240], RateHz::new(24_000).unwrap()),
        Err(AudioError::Range)
    ));
    assert!(matches!(
        processor.prepare_render(&[f32::INFINITY; 240], RateHz::new(24_000).unwrap()),
        Err(AudioError::NonFinite)
    ));
}

#[test]
fn stereo_capture_downmixes_and_mono_levels_keep_the_correct_sign() {
    let stereo = format(16_000, Channels::Stereo);
    let mut processor = Processor::configured(stereo, stereo, Config::default()).unwrap();
    let alternating: Vec<_> = [0.5, -0.5].into_iter().cycle().take(320).collect();
    let capture = processor.capture(&alternating).unwrap();
    assert!(decode(&capture).into_iter().all(|sample| sample == 0));
    let mono = format(16_000, Channels::Mono);
    let mut processor = Processor::configured(mono, mono, Config::default()).unwrap();
    let capture = processor.capture(&[0.25; 160]).unwrap();
    assert!(decode(&capture).into_iter().all(|sample| sample == 8_192));
    assert!(!format!("{capture:?}").contains("8192"));
}

#[test]
fn conversion_frames_preserve_mono_in_both_speaker_channels() {
    let mut processor = Processor::configured(
        format(48_000, Channels::Mono),
        format(48_000, Channels::Stereo),
        Config::default(),
    )
    .unwrap();
    for _ in 0..4 {
        let rendered = processor
            .prepare_render(&[0.25; 240], RateHz::new(24_000).unwrap())
            .unwrap();
        assert_eq!(rendered.samples().len(), 960);
        for pair in rendered.samples().as_chunks::<2>().0 {
            let [left, right] = pair;
            assert_eq!(left.to_bits(), right.to_bits());
        }
        processor.reference_render(rendered.samples()).unwrap();
        assert!(!format!("{rendered:?}").contains("0.25"));
    }
}
