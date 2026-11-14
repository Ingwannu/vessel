//! The persisted startup switch/prompt through the actual CLI and node.
use crate::support::{Fixture, within};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_settings_keep_the_exact_prompt_when_off_and_survive_a_node_restart() {
    within(async {
        let fixture = Fixture::start().await;
        let shown = fixture.vessel(&["node", "settings"], &[]).await;
        assert!(shown.ok, "{}", shown.stderr);
        assert!(shown.stdout.contains("resume after restart: off"));
        assert!(shown.stdout.contains("resume prompt: \"Resume working.\""));
        let invalid = fixture
            .vessel(&["node", "settings", "--resume-on-startup", "maybe"], &[])
            .await;
        assert!(!invalid.ok);
        assert!(invalid.stderr.contains("choose on or off"));
        let prompt = "계속 작업하세요.\nResume working.";
        let on = fixture
            .vessel(
                &[
                    "node",
                    "settings",
                    "--resume-on-startup",
                    "on",
                    "--resume-prompt",
                    prompt,
                ],
                &[],
            )
            .await;
        assert!(on.ok, "{}", on.stderr);
        assert!(on.stdout.contains("resume after restart: on"));
        assert!(
            on.stdout.contains("계속 작업하세요.\\nResume working."),
            "the CLI quotes a multiline prompt rather than emitting terminal line breaks"
        );
        let saved = vessel_node::DataDir::new(fixture.data())
            .read_settings()
            .unwrap()
            .unwrap();
        let policy = saved.restart_recovery.unwrap().effective();
        assert!(policy.enabled);
        assert_eq!(policy.prompt.as_str(), prompt);
        let off = fixture
            .vessel(&["node", "settings", "--resume-on-startup", "off"], &[])
            .await;
        assert!(off.ok, "{}", off.stderr);
        assert!(off.stdout.contains("resume after restart: off"));
        let fixture = fixture.restart().await;
        let shown = fixture.vessel(&["node", "settings"], &[]).await;
        assert!(shown.ok, "{}", shown.stderr);
        assert!(shown.stdout.contains("resume after restart: off"));
        assert!(shown.stdout.contains("계속 작업하세요.\\nResume working."));
        let empty = fixture
            .vessel(&["node", "settings", "--resume-prompt", ""], &[])
            .await;
        assert!(!empty.ok);
        let saved = vessel_node::DataDir::new(fixture.data())
            .read_settings()
            .unwrap()
            .unwrap()
            .restart_recovery
            .unwrap()
            .effective();
        assert!(!saved.enabled);
        assert_eq!(saved.prompt.as_str(), prompt);
        fixture.stop().await;
    })
    .await;
}
