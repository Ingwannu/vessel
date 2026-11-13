//! `vessel schedule` and `vessel prompt` (protocol 11, ADR 0033) against a
//! node in this process with `fake-acp` as Claude's adapter: a schedule
//! made, listed, run now, changed, paused and deleted, named by its name;
//! a time zone the node does not know refused; a saved prompt saved,
//! replaced under its name, shown, sent into a session's queue and
//! deleted. The scheduler's timing is tested on a clock the node's tests
//! move; nothing here waits for a due time.

use crate::support::{Fixture, Ran, within};

fn ok(ran: &Ran) -> &str {
    assert!(ran.ok, "stdout: {}\nstderr: {}", ran.stdout, ran.stderr);
    &ran.stdout
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_schedule_is_made_run_changed_and_removed_by_its_name() {
    within(async {
        let fixture = Fixture::start().await;
        let made = fixture
            .vessel(
                &[
                    "schedule",
                    "create",
                    "--name",
                    "nightly",
                    "--every",
                    "1h",
                    "--harness",
                    "claude",
                    "--prompt",
                    "plain",
                ],
                b"",
            )
            .await;
        let line = ok(&made);
        assert!(
            line.contains("nightly") && line.contains("every 3600s"),
            "{line}"
        );

        let listed = fixture.vessel(&["schedule", "list"], b"").await;
        assert_eq!(ok(&listed).lines().count(), 1, "{}", listed.stdout);

        let run = fixture.vessel(&["schedule", "run", "nightly"], b"").await;
        let run = ok(&run);
        assert!(run.starts_with("#1 ") && run.contains("started"), "{run}");
        let runs = fixture.vessel(&["schedule", "runs", "nightly"], b"").await;
        assert!(ok(&runs).contains("asked by Owner"), "{}", runs.stdout);

        let edited = fixture
            .vessel(&["schedule", "edit", "nightly", "--every", "2h"], b"")
            .await;
        assert!(ok(&edited).contains("every 7200s"), "{}", edited.stdout);
        let shown = fixture.vessel(&["schedule", "show", "nightly"], b"").await;
        let shown = ok(&shown);
        assert!(
            shown.contains("a new claude") && shown.contains("prompt:\nplain"),
            "{shown}"
        );
        // `--keep` alone changes what follows a run, the rest of the
        // session's spec as it was.
        let kept = fixture
            .vessel(&["schedule", "edit", "nightly", "--keep"], b"")
            .await;
        let _line = ok(&kept);
        let shown = fixture.vessel(&["schedule", "show", "nightly"], b"").await;
        let shown = ok(&shown);
        assert!(shown.contains("session each run (kept)"), "{shown}");

        let paused = fixture.vessel(&["schedule", "pause", "nightly"], b"").await;
        assert!(ok(&paused).contains("paused since"), "{}", paused.stdout);
        let deleted = fixture
            .vessel(&["schedule", "delete", "nightly"], b"")
            .await;
        assert!(ok(&deleted).starts_with("deleted "), "{}", deleted.stdout);
        let gone = fixture.vessel(&["schedule", "show", "nightly"], b"").await;
        assert!(!gone.ok && gone.stderr.contains("no schedule matches"));

        // The node judges the zone.
        let mars = fixture
            .vessel(
                &[
                    "schedule",
                    "create",
                    "--cron",
                    "0 9 * * *",
                    "--tz",
                    "Mars/Olympus",
                    "--harness",
                    "claude",
                    "--prompt",
                    "plain",
                ],
                b"",
            )
            .await;
        assert!(
            !mars.ok && mars.stderr.contains("time zone"),
            "{}",
            mars.stderr
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_prompt_is_saved_replaced_sent_and_removed() {
    within(async {
        let fixture = Fixture::start().await;
        let saved = fixture
            .vessel(&["prompt", "save", "review", "--text", "plain"], b"")
            .await;
        let first = ok(&saved).to_owned();
        assert!(
            first.contains("review") && first.contains("machine"),
            "{first}"
        );
        // Saving under the same name changes that prompt.
        let again = fixture
            .vessel(&["prompt", "save", "review", "--text", "describe"], b"")
            .await;
        assert_eq!(
            ok(&again).split_whitespace().next(),
            first.split_whitespace().next(),
            "the same prompt"
        );
        let listed = fixture.vessel(&["prompt", "list"], b"").await;
        assert_eq!(ok(&listed).lines().count(), 1);
        let shown = fixture.vessel(&["prompt", "show", "review"], b"").await;
        assert_eq!(ok(&shown), "describe\n");

        let cwd = fixture.cwd();
        let run = fixture
            .vessel(
                &["run", "claude", "--structured", "--detach", "--cwd", &cwd],
                b"",
            )
            .await;
        let session = ok(&run).trim().to_owned();
        let sent = fixture
            .vessel(&["prompt", "run", "review", "--session", &session], b"")
            .await;
        assert!(ok(&sent).starts_with(&session), "{}", sent.stdout);
        // A prompt that says nothing about a session to start cannot.
        let new = fixture
            .vessel(&["prompt", "run", "review", "--new"], b"")
            .await;
        assert!(
            !new.ok && new.stderr.contains("--session"),
            "{}",
            new.stderr
        );

        let deleted = fixture.vessel(&["prompt", "delete", "review"], b"").await;
        assert!(ok(&deleted).starts_with("deleted "));
        let none = fixture.vessel(&["prompt", "list"], b"").await;
        assert!(ok(&none).is_empty());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_preview_uses_node_calculation_without_saving() {
    within(async {
        let fixture = Fixture::start().await;
        let preview = fixture
            .vessel(
                &[
                    "schedule",
                    "preview",
                    "--cron",
                    "15 9 * * 1-5",
                    "--tz",
                    "UTC",
                    "--after",
                    "2026-01-05T09:14:30Z",
                    "--count",
                    "2",
                ],
                b"",
            )
            .await;
        assert!(preview.ok, "{}", preview.stderr);
        assert_eq!(
            preview.stdout,
            "2026-01-05T09:15:00+00:00\n2026-01-06T09:15:00+00:00\n"
        );
        let empty = fixture.vessel(&["schedule", "list"], b"").await;
        assert!(empty.ok, "{}", empty.stderr);
        assert_eq!(empty.stdout, "");
        let invalid = fixture
            .vessel(
                &["schedule", "preview", "--every", "1h", "--count", "11"],
                b"",
            )
            .await;
        assert!(!invalid.ok);
        fixture.stop().await;
    })
    .await;
}
