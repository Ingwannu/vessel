use super::*;
use vessel_proto::voice::MAX_CAPTURE_BYTES;

fn chunk(value: u8) -> CapturePcm {
    CapturePcm::new(vec![value; MAX_CAPTURE_BYTES]).unwrap()
}

#[test]
fn long_idle_is_bounded_and_confirmation_preserves_the_recent_prefix_once() {
    let mut input = InputBuffer::default();
    for _ in 0..1_000 {
        input.append(&chunk(42)).unwrap();
    }
    assert_eq!(input.pre_roll.len(), PRE_ROLL_BYTES);
    assert!(input.seal().is_none());
    input.append(&chunk(43)).unwrap();
    input.confirmed().unwrap();
    assert_eq!(input.confirmed(), Err(BufferError::Order));
    input.append(&chunk(44)).unwrap();
    let audio = input.seal().unwrap();
    assert_eq!(
        audio.pcm16().len(),
        PRE_ROLL_BYTES.saturating_add(MAX_CAPTURE_BYTES)
    );
    assert!(audio.pcm16().starts_with(&[42, 42]));
    let tail = audio
        .pcm16()
        .get(PRE_ROLL_BYTES.saturating_sub(MAX_CAPTURE_BYTES)..)
        .unwrap();
    assert!(tail.starts_with(&[43, 43]));
    assert!(tail.ends_with(&[44, 44]));
    assert!(!format!("{audio:?}").contains("[42"));
    assert!(input.seal().is_none());
}

#[test]
fn sixty_second_limit_refuses_before_mutation_and_stop_never_seals_old_pcm() {
    let mut input = InputBuffer::default();
    input.confirmed().unwrap();
    for _ in 0..MAX_UTTERANCE_BYTES.div_euclid(MAX_CAPTURE_BYTES) {
        input.append(&chunk(1)).unwrap();
    }
    assert_eq!(input.append(&chunk(2)), Err(BufferError::TooLong));
    let audio = input.seal().unwrap();
    assert_eq!(audio.pcm16().len(), MAX_UTTERANCE_BYTES);
    assert!(audio.pcm16().ends_with(&[1, 1]));
    input.append(&chunk(7)).unwrap();
    input.confirmed().unwrap();
    input.discard();
    assert!(input.seal().is_none());
    assert!(input.pre_roll.is_empty());
    input.append(&chunk(8)).unwrap();
    input.confirmed().unwrap();
    assert!(input.seal().unwrap().pcm16().iter().all(|byte| *byte == 8));
}
