use super::*;

fn pcm(samples: impl IntoIterator<Item = i16>) -> CapturePcm {
    CapturePcm::new(
        samples
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn ten_ms_chunks_carry_exact_reference_windows_without_padded_samples() {
    let mut windows = Windows::default();
    let token = CancellationToken::new();
    let mut observed = Vec::new();
    for _ in 0..3 {
        windows
            .push(&pcm([8_192; 160]), &token, |window| {
                observed.push(window.samples().to_vec())
            })
            .unwrap();
    }
    assert!(observed.is_empty());
    assert_eq!(windows.filled, 480);
    windows
        .push(&pcm([-8_192; 160]), &token, |window| {
            observed.push(window.samples().to_vec())
        })
        .unwrap();
    assert_eq!(observed.len(), 1);
    assert!(
        observed[0][..480]
            .iter()
            .all(|value| value.to_bits() == 0.25_f32.to_bits())
    );
    assert!(
        observed[0][480..]
            .iter()
            .all(|value| value.to_bits() == (-0.25_f32).to_bits())
    );
    assert_eq!(windows.filled, 128);
    assert!(!format!("{windows:?}{:?}", windows.partial).contains("0.25"));
    windows.clear();
    assert_eq!(windows.filled, 0);
}

#[test]
fn capture_chunk_partition_does_not_change_inference_samples() {
    let source: Vec<_> = (0_i32..3_072)
        .map(|index| i16::try_from(index.saturating_sub(1_536)).unwrap())
        .collect();
    let token = CancellationToken::new();
    let mut expected = Vec::new();
    let mut windows = Windows::default();
    for chunk in source.chunks(1_536) {
        windows
            .push(&pcm(chunk.iter().copied()), &token, |window| {
                expected.extend(window.samples().iter().map(|value| value.to_bits()))
            })
            .unwrap();
    }
    let mut actual = Vec::new();
    let mut windows = Windows::default();
    for chunk in source.chunks(160) {
        windows
            .push(&pcm(chunk.iter().copied()), &token, |window| {
                actual.extend(window.samples().iter().map(|value| value.to_bits()))
            })
            .unwrap();
    }
    assert_eq!(actual, expected);
    // The exact six-window tail retains the sixth window as reference lookahead.
    assert_eq!(actual.len(), 2_560);
    assert_eq!(windows.filled, WINDOW_SAMPLES);
}

#[test]
fn cancellation_during_a_chunk_fences_later_inference_and_discards_the_partial() {
    let token = CancellationToken::new();
    let mut windows = Windows::default();
    let mut observed = 0_usize;
    let result = windows.push(&pcm([0; 1_536]), &token, |_window| {
        observed = observed.saturating_add(1);
        token.cancel();
    });
    assert!(matches!(result, Err(InputError::Cancelled)));
    assert_eq!(observed, 1);
    assert_eq!(windows.filled, 0);
    assert!(matches!(
        windows.push(&pcm([0; 160]), &token, |_window| panic!(
            "late detector call"
        )),
        Err(InputError::Cancelled)
    ));
}

#[test]
fn an_exact_full_window_waits_for_a_real_following_sample() {
    let mut windows = Windows::default();
    let token = CancellationToken::new();
    let mut observed = Vec::new();
    windows
        .push(&pcm([8_192; 512]), &token, |window| {
            observed.push(window.samples().to_vec())
        })
        .unwrap();
    assert!(observed.is_empty());
    assert_eq!(windows.filled, 512);
    windows
        .push(&pcm([-8_192; 1]), &token, |window| {
            observed.push(window.samples().to_vec())
        })
        .unwrap();
    assert_eq!(observed.len(), 1);
    assert!(
        observed[0]
            .iter()
            .all(|sample| sample.to_bits() == 0.25_f32.to_bits())
    );
    assert_eq!(windows.filled, 1);
    assert_eq!(
        windows.partial.samples()[0].to_bits(),
        (-0.25_f32).to_bits()
    );
}
