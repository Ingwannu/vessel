//! The binary command refuses files before touching node resolution or stdout.
use super::support::{vessel_in, within};

#[tokio::test]
async fn invalid_audio_is_a_private_input_refusal_before_invalid_node_resolution() {
    within(async {
        const MARKER: &str = "private-file-speech-8dc46930";
        let root = tempfile::tempdir().unwrap();
        tokio::fs::write(root.path().join("recording.wav"), MARKER)
            .await
            .unwrap();
        let result = vessel_in(
            root.path(),
            &["--node", "not a node address", "dictate", "recording.wav"],
            b"",
        )
        .await;
        assert!(!result.ok);
        assert!(result.stdout.is_empty());
        assert!(
            result
                .stderr
                .contains("dictation input must be a complete little-endian RIFF/WAVE container")
        );
        assert!(!result.stderr.contains(MARKER));
        assert!(!root.path().join("config").exists());
    })
    .await;
}

#[tokio::test]
async fn dictate_help_names_a_file_and_refuses_a_missing_argument() {
    within(async {
        let root = tempfile::tempdir().unwrap();
        let help = vessel_in(root.path(), &["dictate", "--help"], b"").await;
        assert!(help.ok);
        assert!(help.stdout.contains("<FILE>"));
        assert!(help.stdout.contains("PCM16"));
        let missing = vessel_in(root.path(), &["dictate"], b"").await;
        assert!(!missing.ok);
        assert!(missing.stdout.is_empty());
        assert!(missing.stderr.contains("<FILE>"));
    })
    .await;
}

#[tokio::test]
async fn unconsumed_json_is_refused_before_file_or_node_work() {
    within(async {
        let root = tempfile::tempdir().unwrap();
        let result = vessel_in(
            root.path(),
            &[
                "--node",
                "not a node address",
                "dictate",
                "missing.wav",
                "--json",
            ],
            b"",
        )
        .await;
        assert!(!result.ok);
        assert!(result.stdout.is_empty());
        assert!(
            result
                .stderr
                .contains("does not support explicit output selection")
        );
        assert!(!root.path().join("config").exists());
    })
    .await;
}
