//! The wire edge rejects invalid shapes before any stream owns audio.

use super::*;

#[test]
fn ids_are_bounded_log_safe_tokens() {
    for good in ["a", "msg_123-456", &"A".repeat(64)] {
        let id: DictationId = good.parse().unwrap();
        let encoded = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<DictationId>(&encoded).unwrap(), id);
    }
    for bad in ["", "two words", "한글", "line\nbreak", &"x".repeat(65)] {
        assert!(matches!(
            bad.parse::<DictationId>(),
            Err(DictationDecodeError::Id)
        ));
    }
}

#[test]
fn format_decodes_once_and_rejects_ambiguous_fields() {
    for rate in [8_000, 16_000, 24_000, 48_000] {
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
        "audio/pcm;rate=7999;bits=16",
        "audio/pcm;rate=48001;bits=16",
        "audio/pcm;rate=oops;bits=16",
        "audio/pcm;rate=16000;bits=8",
        "audio/pcm;rate=16000;bits=16;channels=2",
        "audio/pcm;rate=16000;bits=16;rate=8000",
        "audio/pcm;rate=16000;bits=16;bits=16",
        "audio/pcm;rate=16000;bits=16;channels=1;channels=1",
        "audio/pcm;rate=16000;bits=16;extra=1",
    ] {
        assert!(bad.parse::<Pcm16Mono>().is_err());
    }
    assert!(SampleRate::new(0).is_err());
}

#[test]
fn pcm_round_trips_at_both_caps_and_refuses_incomplete_samples() {
    for size in [2, MAX_CHUNK_BYTES] {
        let pcm = DictationPcm::new(vec![42; size]).unwrap();
        let wire = serde_json::to_string(&pcm).unwrap();
        assert_eq!(serde_json::from_str::<DictationPcm>(&wire).unwrap(), pcm);
        assert_eq!(pcm.into_bytes().len(), size);
    }
    for size in [0, 1, 3, MAX_CHUNK_BYTES + 2] {
        assert!(matches!(
            DictationPcm::new(vec![0; size]),
            Err(DictationDecodeError::Pcm)
        ));
    }
    for wire in ["\"\"", "\"AQ==\"", "\"private-invalid-audio\""] {
        let error = serde_json::from_str::<DictationPcm>(wire).unwrap_err();
        assert!(!error.to_string().contains("private-invalid-audio"));
    }
    let oversized = format!("\"{}\"", "A".repeat(400_000));
    assert!(serde_json::from_str::<DictationPcm>(&oversized).is_err());
}

#[test]
fn transcripts_are_bounded_in_utf8_bytes_and_debug_is_redacted() {
    let text = "가".repeat(21_845);
    let transcript = Transcript::try_from(text.clone()).unwrap();
    assert!(transcript.as_str() == text, "transcript bytes changed");
    assert!(transcript.to_string() == text, "transcript display changed");
    let wire = serde_json::to_string(&transcript).unwrap();
    assert_eq!(
        serde_json::from_str::<Transcript>(&wire).unwrap(),
        transcript
    );
    assert!(Transcript::try_from("가".repeat(21_846)).is_err());
    assert!(Transcript::try_from(String::new()).is_ok());
    assert!(Transcript::try_from("x".repeat(MAX_TRANSCRIPT_BYTES)).is_ok());
    let oversized = format!("\"{}\"", "x".repeat(MAX_TRANSCRIPT_BYTES + 1));
    assert!(serde_json::from_str::<Transcript>(&oversized).is_err());
    assert_eq!(format!("{transcript:?}"), "Transcript([redacted])");
    let pcm = DictationPcm::new(vec![42; 2]).unwrap();
    assert_eq!(format!("{pcm:?}"), "DictationPcm([redacted])");
}

#[test]
fn sequence_and_finish_budget_have_distinct_boundary_policies() {
    // Oversized sequences must reach the node to get the typed TooLong refusal.
    let seq: ChunkSeq = serde_json::from_str("1800").unwrap();
    assert_eq!(seq, ChunkSeq::new(MAX_SEQ.get() + 1));
    assert!(serde_json::from_str::<ChunkSeq>("-1").is_err());
    assert!(serde_json::from_str::<ChunkSeq>("4294967296").is_err());
    assert_eq!(WithinMs::new(u32::MAX).get(), 300_000);
    assert_eq!(
        serde_json::from_str::<WithinMs>("300000").unwrap().get(),
        300_000
    );
    assert!(serde_json::from_str::<WithinMs>("300001").is_err());
    assert!(SINCE <= crate::PROTOCOL_VERSION.get());
}
