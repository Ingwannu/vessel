//! The wire edge rejects invalid shapes before any stream owns audio.

use super::*;

#[test]
fn ids_are_bounded_log_safe_tokens() {
    for good in ["a", "msg_123-456", &"A".repeat(MAX_ID_CHARS)] {
        let id: DictationId = good.parse().unwrap();
        let encoded = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<DictationId>(&encoded).unwrap(), id);
    }
    for bad in [
        "",
        "two words",
        "한글",
        "line\nbreak",
        &"x".repeat(MAX_ID_CHARS + 1),
    ] {
        assert!(matches!(
            bad.parse::<DictationId>(),
            Err(DictationDecodeError::Id)
        ));
    }
}

#[test]
fn format_decodes_once_and_rejects_ambiguous_fields() {
    for rate in [MIN_RATE, 16_000, 24_000, MAX_RATE] {
        let wire = format!("audio/pcm;rate={rate};bits=16");
        let format: Pcm16Mono = wire.parse().unwrap();
        assert_eq!(format.rate().get(), rate);
        assert_eq!(format.to_string(), wire);
        let wire = format!("\"{wire}\"");
        assert_eq!(serde_json::from_str::<Pcm16Mono>(&wire).unwrap(), format);
    }
    assert!(
        "audio/pcm;bits=16;channels=1;rate=16000"
            .parse::<Pcm16Mono>()
            .is_ok()
    );
    for bad in [
        "audio/wav;rate=16000;bits=16",
        "audio/pcm;rate=16000",
        "audio/pcm;bits=16",
        "audio/pcm;rate=oops;bits=16",
        "audio/pcm;rate=16000;bits=8",
        "audio/pcm;rate=16000;bits=16;channels=2",
        "audio/pcm;rate=16000;bits=16;rate=24000",
        "audio/pcm;rate=16000;bits=16;bits=16",
        "audio/pcm;rate=16000;bits=16;channels=1;channels=1",
        "audio/pcm;rate=16000;bits=16;extra=1",
    ] {
        assert!(bad.parse::<Pcm16Mono>().is_err());
    }
    for rate in [MIN_RATE - 1, MAX_RATE + 1] {
        let wire = format!("audio/pcm;rate={rate};bits=16");
        assert!(wire.parse::<Pcm16Mono>().is_err());
    }
    assert!(SampleRate::new(0).is_err());
}

#[test]
fn pcm_round_trips_at_both_caps_and_refuses_incomplete_samples() {
    for size in [PCM_SAMPLE_BYTES, MAX_CHUNK_BYTES] {
        let pcm = DictationPcm::new(vec![42; size]).unwrap();
        let wire = serde_json::to_string(&pcm).unwrap();
        assert_eq!(serde_json::from_str::<DictationPcm>(&wire).unwrap(), pcm);
        assert_eq!(pcm.into_bytes().len(), size);
    }
    for size in [
        0,
        PCM_SAMPLE_BYTES - 1,
        PCM_SAMPLE_BYTES + 1,
        MAX_CHUNK_BYTES + PCM_SAMPLE_BYTES,
    ] {
        assert!(matches!(
            DictationPcm::new(vec![0; size]),
            Err(DictationDecodeError::Pcm)
        ));
    }
    for wire in ["\"\"", "\"AQ==\"", "\"private-invalid-audio\""] {
        let error = serde_json::from_str::<DictationPcm>(wire).unwrap_err();
        assert!(!error.to_string().contains("private-invalid-audio"));
    }
    let oversized = format!("\"{}\"", "A".repeat(MAX_CHUNK_BYTES.div_ceil(3) * 4 + 4));
    assert!(serde_json::from_str::<DictationPcm>(&oversized).is_err());
}

#[test]
fn transcripts_are_bounded_in_utf8_bytes_and_debug_is_redacted() {
    let character = "가";
    let max_characters = MAX_TRANSCRIPT_BYTES / character.len();
    let text = character.repeat(max_characters);
    let transcript = Transcript::try_from(text.clone()).unwrap();
    assert!(transcript.as_str() == text, "transcript bytes changed");
    assert!(transcript.to_string() == text, "transcript display changed");
    let wire = serde_json::to_string(&transcript).unwrap();
    assert_eq!(
        serde_json::from_str::<Transcript>(&wire).unwrap(),
        transcript
    );
    assert!(Transcript::try_from(character.repeat(max_characters + 1)).is_err());
    assert!(Transcript::try_from(String::new()).is_ok());
    assert!(Transcript::try_from("x".repeat(MAX_TRANSCRIPT_BYTES)).is_ok());
    let oversized = format!("\"{}\"", "x".repeat(MAX_TRANSCRIPT_BYTES + 1));
    assert!(serde_json::from_str::<Transcript>(&oversized).is_err());
    assert_eq!(format!("{transcript:?}"), "Transcript([redacted])");
    let pcm = DictationPcm::new(vec![42; PCM_SAMPLE_BYTES]).unwrap();
    assert_eq!(format!("{pcm:?}"), "DictationPcm([redacted])");
}

#[test]
fn sequence_and_finish_budget_have_distinct_boundary_policies() {
    // Oversized sequences must reach the node to get the typed TooLong refusal.
    let seq: ChunkSeq = serde_json::from_str(&(MAX_SEQ.get() + 1).to_string()).unwrap();
    assert_eq!(seq, ChunkSeq::new(MAX_SEQ.get() + 1));
    assert!(serde_json::from_str::<ChunkSeq>("-1").is_err());
    assert!(serde_json::from_str::<ChunkSeq>(&(u64::from(u32::MAX) + 1).to_string()).is_err());
    assert_eq!(WithinMs::new(u32::MAX).get(), MAX_FINISH_MS);
    assert_eq!(
        serde_json::from_str::<WithinMs>(&MAX_FINISH_MS.to_string())
            .unwrap()
            .get(),
        MAX_FINISH_MS
    );
    assert!(serde_json::from_str::<WithinMs>(&(MAX_FINISH_MS + 1).to_string()).is_err());
    assert!(SINCE <= crate::PROTOCOL_VERSION);
}
