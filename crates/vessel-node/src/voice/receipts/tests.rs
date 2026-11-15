use super::*;
use vessel_proto::voice::{PlaybackPcm, SpeechGroupId, VoiceEpoch, VoiceId};

fn lease() -> VoiceLease {
    VoiceLease {
        id: VoiceId::random(),
        epoch: VoiceEpoch::random(),
    }
}
fn playback() -> Playback {
    Playback {
        id: PlaybackId::random(),
        group: SpeechGroupId::random(),
        format: "audio/pcm;rate=24000;bits=16".parse().unwrap(),
        pcm: PlaybackPcm::new(vec![0; 48_000]).unwrap(),
    }
}
fn receipt(lease: VoiceLease, id: PlaybackId, outcome: PlaybackOutcome) -> PlayedVoice {
    PlayedVoice {
        lease,
        playback: id,
        outcome,
    }
}

#[test]
fn played_is_explicit_own_generation_only_and_duplicate_cannot_resettle() {
    let own = lease();
    let mut book = Receipts::new(own);
    let now = Instant::now();
    let audio = playback();
    book.open(&audio, now).unwrap();
    assert_eq!(
        book.complete(receipt(lease(), audio.id, PlaybackOutcome::Played), now),
        Err(ReceiptError::Foreign)
    );
    assert_eq!(book.pending.len(), 1);
    assert_eq!(
        book.complete(
            receipt(own, PlaybackId::random(), PlaybackOutcome::Played),
            now
        ),
        Err(ReceiptError::Unknown)
    );
    assert_eq!(
        book.complete(receipt(own, audio.id, PlaybackOutcome::Played), now),
        Ok(Completion::Settled(PlaybackOutcome::Played))
    );
    assert_eq!(
        book.complete(receipt(own, audio.id, PlaybackOutcome::Played), now),
        Ok(Completion::Duplicate)
    );
    assert_eq!(
        book.complete(receipt(own, audio.id, PlaybackOutcome::Failed), now),
        Err(ReceiptError::Conflict)
    );
    assert_eq!(book.open(&audio, now), Err(ReceiptError::Reused));
}

#[test]
fn exact_duration_deadline_timeout_and_barge_in_cannot_turn_into_played() {
    let own = lease();
    let mut book = Receipts::new(own);
    let now = Instant::now();
    let first = playback();
    let second = playback();
    book.open(&first, now).unwrap();
    book.open(&second, now).unwrap();
    assert_eq!(book.open(&playback(), now), Err(ReceiptError::Capacity));
    let deadline = now.checked_add(Duration::from_secs(31)).unwrap();
    assert_eq!(
        book.complete(receipt(own, first.id, PlaybackOutcome::Played), deadline),
        Err(ReceiptError::TimedOut)
    );
    assert_eq!(book.interrupt(deadline), [second.id]);
    assert_eq!(
        book.complete(receipt(own, second.id, PlaybackOutcome::Played), now),
        Err(ReceiptError::Conflict)
    );
    assert_eq!(
        book.complete(receipt(own, second.id, PlaybackOutcome::Interrupted), now),
        Ok(Completion::Duplicate)
    );
}

#[test]
fn timeout_pruning_and_tombstone_count_bound_do_not_accumulate_or_accept_foreign_ids() {
    let own = lease();
    let mut book = Receipts::new(own);
    let now = Instant::now();
    let audio = playback();
    book.open(&audio, now).unwrap();
    let later = now.checked_add(Duration::from_secs(31)).unwrap();
    assert_eq!(book.expire(later), [audio.id]);
    assert!(book.expire(later).is_empty());
    let gone = later.checked_add(CLOSED_TTL).unwrap();
    assert_eq!(
        book.complete(receipt(own, audio.id, PlaybackOutcome::Failed), gone),
        Err(ReceiptError::Unknown)
    );
    for _ in 0..MAX_CLOSED.saturating_add(3) {
        let audio = playback();
        book.open(&audio, gone).unwrap();
        assert_eq!(
            book.complete(receipt(own, audio.id, PlaybackOutcome::Played), gone)
                .unwrap(),
            Completion::Settled(PlaybackOutcome::Played)
        );
    }
    assert_eq!(book.closed.len(), MAX_CLOSED);
}
