//! `vessel schedule` and `vessel prompt` (protocol 11, ADR 0033) against a
//! node in this process with `fake-acp` as Claude's adapter: a schedule
//! made, listed, run now, changed, paused and deleted, named by its name;
//! a time zone the node does not know refused; a saved prompt saved,
//! replaced under its name, shown, sent into a session's queue and
//! deleted. The scheduler's timing is tested on a clock the node's tests
//! move; nothing here waits for a due time.

use crate::support::{Fixture, Ran, within};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScheduleRow {
    id: vessel_proto::ScheduleId,
    name: Option<String>,
    cadence: String,
    status: String,
}
#[derive(Deserialize)]
struct Deleted {
    id: vessel_proto::ScheduleId,
    status: DeletionStatus,
}
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeletionStatus {
    Deleted,
}

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
                    "--json",
                ],
                b"",
            )
            .await;
        let made: ScheduleRow = serde_json::from_str(ok(&made)).unwrap();
        assert_eq!(made.name.as_deref(), Some("nightly"));
        assert_eq!(made.cadence, "cron:0 * * * * (UTC)");

        let listed = fixture.vessel(&["schedule", "ls", "--json"], b"").await;
        let rows: Vec<ScheduleRow> = serde_json::from_str(ok(&listed)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, made.id);

        let run = fixture
            .vessel(&["schedule", "run-once", "nightly", "--json"], b"")
            .await;
        let run: ScheduleRow = serde_json::from_str(ok(&run)).unwrap();
        assert_eq!(run.id, made.id);
        let runs = fixture
            .vessel(&["schedule", "logs", "nightly", "--json"], b"")
            .await;
        let runs: Vec<vessel_proto::automation::ScheduleRun> =
            serde_json::from_str(ok(&runs)).unwrap();
        assert_eq!(runs.len(), 1);
        assert!(matches!(
            runs[0].trigger,
            vessel_proto::automation::RunTrigger::Asked { .. }
        ));

        let edited = fixture
            .vessel(
                &["schedule", "update", "nightly", "--every", "2h", "--json"],
                b"",
            )
            .await;
        let edited: ScheduleRow = serde_json::from_str(ok(&edited)).unwrap();
        assert_eq!(edited.cadence, "cron:0 */2 * * * (UTC)");
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

        let paused = fixture
            .vessel(&["schedule", "pause", "nightly", "--json"], b"")
            .await;
        let paused: ScheduleRow = serde_json::from_str(ok(&paused)).unwrap();
        assert_eq!(paused.status, "paused");
        let deleted = fixture
            .vessel(&["schedule", "delete", "nightly", "--json"], b"")
            .await;
        let deleted: Deleted = serde_json::from_str(ok(&deleted)).unwrap();
        assert_eq!(deleted.id, made.id);
        assert_eq!(deleted.status, DeletionStatus::Deleted);
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
