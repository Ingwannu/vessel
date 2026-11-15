//! Adversarial ordering fixtures: all audio is synthetic and never printed.

use super::*;

fn input() -> Input {
    Input::new(SampleRate::new(16_000).unwrap()).unwrap()
}
fn full(input: &Input) -> DictationPcm {
    DictationPcm::new(vec![0; input.second]).unwrap()
}
fn tail() -> DictationPcm {
    DictationPcm::new(vec![0; SAMPLE_BYTES]).unwrap()
}
fn seq(value: u32) -> ChunkSeq {
    ChunkSeq::new(value)
}

#[test]
fn contiguous_tail_is_acknowledged_but_not_forwarded_before_matching_finish() {
    let mut input = input();
    assert_eq!(input.accept(seq(0), tail()).unwrap(), Some(seq(0)));
    assert!(input.pop().is_none());
    assert_eq!(input.finish(seq(1)), Err(InputError::BadAudio));
    assert!(input.pop().is_none());
    input.finish(seq(0)).unwrap();
    assert!(input.pop().is_some());
    assert!(input.pop().is_none());
}

#[test]
fn filling_a_gap_acknowledges_all_held_chunks_but_forwards_one_at_a_time() {
    let mut input = input();
    let first = full(&input);
    assert_eq!(input.accept(seq(2), first.clone()).unwrap(), None);
    assert_eq!(input.accept(seq(1), first.clone()).unwrap(), None);
    assert!(input.pop().is_none());
    assert_eq!(input.accept(seq(0), first).unwrap(), Some(seq(2)));
    assert_eq!(input.held.len(), 2);
    for expected in 1..=3 {
        assert!(input.pop().is_some());
        assert_eq!(input.forward, seq(expected));
    }
    assert!(input.pop().is_none());
    assert_eq!(input.held_bytes, 0);
}

#[test]
fn duplicate_audio_shape_is_ignored_but_sequence_overflow_is_checked_first() {
    let mut input = input();
    assert_eq!(input.accept(seq(0), full(&input)).unwrap(), Some(seq(0)));
    let accepted = input.accepted;
    assert_eq!(input.accept(seq(0), tail()).unwrap(), Some(seq(0)));
    assert_eq!(input.accepted, accepted);
    assert_eq!(input.accept(seq(1), tail()).unwrap(), Some(seq(1)));
    let accepted = input.accepted;
    // A second short tail is also a shape error, but the sequence cap wins.
    assert_eq!(
        input.accept(seq(MAX_SEQ.get().saturating_add(1)), tail()),
        Err(InputError::TooLong)
    );
    assert_eq!(input.accepted, accepted);
}

#[test]
fn shape_errors_leave_all_accepted_audio_and_finish_metadata_unchanged() {
    let mut input = input();
    assert_eq!(input.accept(seq(1), full(&input)).unwrap(), None);
    assert_eq!(input.accept(seq(0), tail()), Err(InputError::BadAudio));
    assert_eq!(input.accept(seq(2), tail()).unwrap(), None);
    let accepted = input.accepted;
    assert_eq!(
        input.accept(seq(3), full(&input)),
        Err(InputError::BadAudio)
    );
    assert_eq!(input.finish(seq(1)), Err(InputError::BadAudio));
    assert_eq!(input.finish, None);
    assert_eq!(input.accepted, accepted);
    input.finish(seq(2)).unwrap();
    input.finish(seq(2)).unwrap();
    assert_eq!(input.finish(seq(3)), Err(InputError::BadAudio));
}

#[test]
fn full_gap_hold_still_accepts_the_current_missing_chunk_without_an_extra_gap_slot() {
    let mut input = Input::new(SampleRate::MIN).unwrap();
    let pcm = full(&input);
    for index in 1..=MAX_HELD_CHUNKS {
        assert_eq!(
            input
                .accept(seq(u32::try_from(index).unwrap()), pcm.clone())
                .unwrap(),
            None
        );
    }
    assert_eq!(input.held.len(), MAX_HELD_CHUNKS);
    let accepted = input.accepted;
    assert_eq!(input.accept(seq(1), tail()).unwrap(), None);
    assert_eq!(input.accepted, accepted);
    assert!(input.tail.is_none());
    assert_eq!(
        input.accept(
            seq(u32::try_from(MAX_HELD_CHUNKS + 1).unwrap()),
            pcm.clone()
        ),
        Err(InputError::ReorderOverflow)
    );
    assert_eq!(input.accepted, accepted);
    let through = input.accept(seq(0), pcm).unwrap();
    assert_eq!(through, Some(seq(u32::try_from(MAX_HELD_CHUNKS).unwrap())));
    assert_eq!(input.held.len(), MAX_HELD_CHUNKS);
    assert!(input.pop().is_some());
    assert_eq!(input.held.len(), MAX_HELD_CHUNKS - 1);
}

#[test]
fn byte_bound_can_refuse_before_the_gap_count_bound() {
    let mut input = Input::new(SampleRate::MAX).unwrap();
    let pcm = full(&input);
    let fitting = MAX_HELD_BYTES.div_euclid(input.second);
    assert!(fitting < MAX_HELD_CHUNKS);
    for index in 1..=fitting {
        assert_eq!(
            input
                .accept(seq(u32::try_from(index).unwrap()), pcm.clone())
                .unwrap(),
            None
        );
    }
    assert_eq!(
        input.accept(seq(u32::try_from(fitting + 1).unwrap()), pcm),
        Err(InputError::ReorderOverflow)
    );
    assert_eq!(input.held.len(), fitting);
}

#[test]
fn the_sequence_and_duration_caps_meet_for_full_seconds_without_retaining_the_recording() {
    let mut input = input();
    let pcm = full(&input);
    for value in 0..=MAX_SEQ.get() {
        let position = seq(value);
        assert_eq!(input.accept(position, pcm.clone()).unwrap(), Some(position));
        assert!(input.pop().is_some());
        assert!(input.current.is_none());
        assert!(input.held.is_empty());
    }
    assert_eq!(input.accepted, input.limit);
    assert_eq!(
        input.accept(seq(MAX_SEQ.get().saturating_add(1)), pcm),
        Err(InputError::TooLong)
    );
}

#[test]
fn oversized_full_chunk_is_refused_before_mutation_and_the_input_stays_usable() {
    let mut input = input();
    let oversized = DictationPcm::new(vec![0; input.second + SAMPLE_BYTES]).unwrap();
    assert_eq!(input.accept(seq(0), oversized), Err(InputError::BadAudio));
    assert_eq!(input.accepted, 0);
    assert!(input.current.is_none());
    assert!(input.highest.is_none());
    assert_eq!(input.accept(seq(0), full(&input)).unwrap(), Some(seq(0)));
}

#[test]
fn finish_freezes_the_last_sequence_but_keeps_accepting_missing_chunks() {
    let mut input = input();
    assert_eq!(input.accept(seq(2), full(&input)).unwrap(), None);
    input.finish(seq(2)).unwrap();
    let accepted = input.accepted;
    assert_eq!(
        input.accept(seq(3), full(&input)),
        Err(InputError::BadAudio)
    );
    assert_eq!(input.accepted, accepted);
    assert_eq!(input.finish(seq(1)), Err(InputError::BadAudio));
    assert_eq!(input.finish, Some(seq(2)));
    assert_eq!(input.accept(seq(0), full(&input)).unwrap(), Some(seq(0)));
    assert!(input.pop().is_some());
    assert_eq!(input.accept(seq(1), full(&input)).unwrap(), Some(seq(2)));
    assert!(input.pop().is_some());
    assert!(input.pop().is_some());
    assert!(input.pop().is_none());
    input.finish(seq(2)).unwrap();
}

#[test]
fn finish_without_audio_is_distinct_from_a_sequence_over_the_cap() {
    let mut input = input();
    assert_eq!(input.finish(seq(0)), Err(InputError::NoAudio));
    assert_eq!(
        input.finish(seq(MAX_SEQ.get().saturating_add(1))),
        Err(InputError::TooLong)
    );
    assert!(input.finish.is_none());
    assert!(input.highest.is_none());
}

#[test]
fn a_tail_arriving_after_finish_must_match_the_final_sequence_before_storage() {
    let mut input = input();
    assert_eq!(input.accept(seq(0), full(&input)).unwrap(), Some(seq(0)));
    assert!(input.pop().is_some());
    input.finish(seq(2)).unwrap();
    let accepted = input.accepted;
    assert_eq!(input.accept(seq(1), tail()), Err(InputError::BadAudio));
    assert_eq!(input.accepted, accepted);
    assert!(input.tail.is_none());
    assert_eq!(input.accept(seq(1), full(&input)).unwrap(), Some(seq(1)));
    assert!(input.pop().is_some());
    assert_eq!(input.accept(seq(2), tail()).unwrap(), Some(seq(2)));
    assert!(input.pop().is_some());
    assert!(input.pop().is_none());
    input.finish(seq(2)).unwrap();
}
