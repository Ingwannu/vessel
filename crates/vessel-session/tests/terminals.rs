//! Terminals beside sessions (protocol 11, ADR 0031) end to end: a shell in
//! a PTY, a script's run and a service, through the runtime's real
//! processes. Unix only: they run `/bin/sh` (the Windows paths, `cmd.exe`
//! and job objects, are CI's).
//!
//! Tests wait on records and ends, never on time; the timeout only turns a
//! hang into a failure.

#![cfg(unix)]

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vessel_proc::CommandSpec;
use vessel_proto::terminals::{ServiceFailure, ServiceState};
use vessel_proto::{EventSeq, SessionEvent, SessionStatus, TerminalInput};
use vessel_session::terminals::{InputError, Leftover, ScriptStart, Scrollback, start_script};

use support::{
    Fixture, alive, gone, info, output_to_end, read_pid, script_kind, type_in, wait_for_count,
    wait_for_file, wait_for_output, within,
};

#[cfg(test)]
mod support {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use bytes::Bytes;
    use chrono::Utc;
    use tokio::sync::broadcast::error::RecvError;
    use tokio_util::sync::CancellationToken;
    use tokio_util::task::TaskTracker;
    use vessel_proc::CommandSpec;
    use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
    use vessel_proto::{
        EventSeq, SessionEvent, SessionId, SessionStatus, TerminalBytes, TerminalId, TerminalInput,
    };
    use vessel_session::terminals::{
        Leftover, ScriptStart, Scrollback, ShellStart, TerminalHandle, start_script, start_shell,
    };

    pub(crate) const LIMIT: Duration = Duration::from_secs(60);

    pub(crate) async fn within<F: Future>(test: F) -> F::Output {
        vessel_testkit::within(LIMIT, test)
            .await
            .expect("the test hung")
    }

    pub(crate) fn info(kind: TerminalKind, folder: &Path) -> TerminalInfo {
        TerminalInfo::new(
            TerminalId::random(),
            TerminalPlace::Session {
                session: SessionId::random(),
            },
            folder.to_path_buf(),
            kind,
            Utc::now(),
        )
    }

    pub(crate) fn shell_kind() -> TerminalKind {
        TerminalKind::Shell {
            program: PathBuf::from("/bin/sh"),
        }
    }

    pub(crate) fn script_kind(service: bool) -> TerminalKind {
        let script = "test".parse().unwrap();
        let command = "x".parse().unwrap();
        if service {
            TerminalKind::Service { script, command }
        } else {
            TerminalKind::Script { script, command }
        }
    }

    pub(crate) struct Fixture {
        pub(crate) folder: tempfile::TempDir,
        pub(crate) shutdown: CancellationToken,
        pub(crate) tasks: TaskTracker,
    }

    impl Fixture {
        pub(crate) fn new() -> Self {
            Self {
                folder: tempfile::tempdir().unwrap(),
                shutdown: CancellationToken::new(),
                tasks: TaskTracker::new(),
            }
        }

        pub(crate) async fn shell(&self, scrollback: Scrollback) -> TerminalHandle {
            start_shell(ShellStart {
                info: info(shell_kind(), self.folder.path()),
                spec: CommandSpec::new("/bin/sh").cwd(self.folder.path()),
                size: vessel_proto::TerminalSize { cols: 80, rows: 24 },
                scrollback,
                shutdown: self.shutdown.clone(),
                tasks: self.tasks.clone(),
            })
            .await
            .unwrap()
        }

        pub(crate) fn script(
            &self,
            command: &str,
            service: bool,
            approved: impl Fn() -> bool + Send + Sync + 'static,
        ) -> TerminalHandle {
            start_script(ScriptStart {
                info: info(script_kind(service), self.folder.path()),
                spec: CommandSpec::new("/bin/sh")
                    .arg("-c")
                    .arg(command)
                    .cwd(self.folder.path()),
                leftover: Leftover::default(),
                service,
                approved,
                scrollback: Scrollback::DEFAULT,
                shutdown: self.shutdown.clone(),
                tasks: self.tasks.clone(),
            })
        }
    }

    /// Everything a terminal printed, from the start, until it ended.
    pub(crate) async fn output_to_end(handle: &TerminalHandle) -> (String, SessionStatus) {
        let follow = handle.follow(EventSeq::ZERO).unwrap();
        let mut text = Vec::new();
        let mut last = None;
        let mut take = |event: &SessionEvent| match event {
            SessionEvent::Terminal(output) => text.extend_from_slice(output.data.bytes()),
            SessionEvent::Status { status } => last = Some(status.clone()),
            SessionEvent::Agent { event } => {
                if let vessel_proto::AgentEvent::HarnessLog { line } = event {
                    text.extend_from_slice(format!("[{line}]").as_bytes());
                }
            }
            other @ (SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }) => panic!("a terminal logs no {other:?}"),
        };
        for record in &follow.cached {
            take(&record.event);
        }
        let mut live = follow.live;
        loop {
            match live.recv().await {
                Ok(record) => take(&record.event),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(_)) => panic!("lagged"),
            }
        }
        (
            String::from_utf8_lossy(&text).into_owned(),
            last.expect("a final status"),
        )
    }

    /// Wait until the log holds `needle`.
    pub(crate) async fn wait_for_output(handle: &TerminalHandle, needle: &str) {
        wait_for_count(handle, needle, 1).await;
    }

    /// Wait until the log holds `needle` `times` times.
    pub(crate) async fn wait_for_count(handle: &TerminalHandle, needle: &str, times: usize) {
        let mut seen = Vec::new();
        let follow = handle.follow(EventSeq::ZERO).unwrap();
        let mut add = |record: &vessel_proto::EventRecord| {
            if let SessionEvent::Terminal(output) = &record.event {
                seen.extend_from_slice(output.data.bytes());
            }
            String::from_utf8_lossy(&seen).matches(needle).count() >= times
        };
        for record in &follow.cached {
            if add(record) {
                return;
            }
        }
        let mut live = follow.live;
        loop {
            let record = live.recv().await.expect("the terminal ended first");
            if add(&record) {
                return;
            }
        }
    }

    pub(crate) fn type_in(text: &str) -> TerminalInput {
        TerminalInput::Data {
            data: TerminalBytes::new(Bytes::from(text.to_owned())),
        }
    }

    /// Whether `pid` still runs: signal 0 checks it exists without touching
    /// it, and on Linux a zombie (killed, not yet reaped by whoever adopted
    /// it) does not count.
    pub(crate) fn alive(pid: i32) -> bool {
        let exists = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok();
        let zombie = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|stat| {
                stat.rsplit_once(')')
                    .map(|(_, rest)| rest.trim_start().starts_with('Z'))
            })
            .unwrap_or(false);
        exists && !zombie
    }

    /// Whether `pid` is gone within a bounded while (whoever adopted it
    /// reaps it soon after the kill).
    pub(crate) async fn gone(pid: i32) -> bool {
        for _ in 0..200 {
            if !alive(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        false
    }

    pub(crate) fn read_pid(path: &Path) -> i32 {
        std::fs::read_to_string(path)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }

    /// Wait (a bounded while) for a file the terminal writes.
    pub(crate) async fn wait_for_file(path: &Path) {
        for _ in 0..500 {
            if std::fs::read_to_string(path).is_ok_and(|text| text.ends_with('\n')) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{} was never written", path.display());
    }
}

#[tokio::test]
async fn a_shell_runs_what_is_typed_and_ends_when_killed() {
    within(async {
        let fixture = Fixture::new();
        let shell = fixture.shell(Scrollback::DEFAULT).await;
        assert!(shell.pid().is_some());
        shell
            .send(type_in("echo vessel-$((40+2))\n"))
            .await
            .unwrap();
        wait_for_output(&shell, "vessel-42").await;
        shell
            .send(TerminalInput::Resize {
                size: vessel_proto::TerminalSize {
                    cols: 100,
                    rows: 30,
                },
            })
            .await
            .unwrap();
        assert_eq!(shell.info().size.unwrap().cols, 100);
        shell.kill();
        let (_text, status) = output_to_end(&shell).await;
        assert!(matches!(status, SessionStatus::Exited { .. }), "{status:?}");
        assert!(shell.is_ended());
        assert_eq!(shell.pid(), None);
        assert_eq!(
            shell.send(type_in("x")).await,
            Err(InputError::Ended),
            "an ended shell takes nothing"
        );
        // Its output stays readable once it ended.
        let page = shell.page(EventSeq::ZERO, 512, 1 << 20).unwrap();
        assert!(page.at_end);
        assert_eq!(page.records.first().unwrap().seq, EventSeq::ZERO);
    })
    .await;
}

/// A job the shell put in a process group of its own is still in its
/// session: it ends with the terminal (Linux sweeps the session).
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_shells_jobs_end_with_it() {
    within(async {
        let fixture = Fixture::new();
        let pid_file = fixture.folder.path().join("job.pid");
        let shell = fixture.shell(Scrollback::DEFAULT).await;
        // Job control on: the job gets a group of its own, which a signal
        // to the shell's group misses. It ignores the hangup, as a server
        // started with `nohup` or a careless trap would.
        shell
            .send(type_in(&format!(
                "set -m; (trap '' HUP; exec sleep 300) & echo $! > {}\n",
                pid_file.display()
            )))
            .await
            .unwrap();
        wait_for_file(&pid_file).await;
        let job = read_pid(&pid_file);
        assert!(alive(job));
        shell.kill();
        shell.ended().await;
        assert!(gone(job).await, "the job outlived its terminal");
    })
    .await;
}

#[tokio::test]
async fn a_scripts_output_and_exit_are_its_log() {
    within(async {
        let fixture = Fixture::new();
        let run = fixture.script("echo out; echo err >&2; exit 3", false, || true);
        let (text, status) = output_to_end(&run).await;
        assert!(text.contains("out") && text.contains("err"), "{text}");
        assert_eq!(status, SessionStatus::Exited { code: Some(3) });
        assert_eq!(
            run.send(type_in("x")).await,
            Err(InputError::NoInput),
            "a script takes no input"
        );
    })
    .await;
}

/// The brief's test: a script's grandchild ends with it.
#[tokio::test]
async fn a_scripts_grandchild_ends_with_it() {
    within(async {
        let fixture = Fixture::new();
        let pid_file = fixture.folder.path().join("child.pid");
        let run = fixture.script(
            &format!(
                "sh -c 'sleep 300 & echo $! > {}; wait'; sleep 300",
                pid_file.display()
            ),
            false,
            || true,
        );
        wait_for_file(&pid_file).await;
        let grandchild = read_pid(&pid_file);
        assert!(alive(grandchild));
        run.kill();
        run.ended().await;
        assert!(gone(grandchild).await, "the grandchild outlived its script");
        assert!(matches!(run.info().status, SessionStatus::Exited { .. }));
    })
    .await;
}

#[tokio::test]
async fn a_service_that_fails_backs_off_and_stops_when_unapproved() {
    within(async {
        let fixture = Fixture::new();
        let approved = Arc::new(AtomicBool::new(true));
        let asked = Arc::clone(&approved);
        let service = fixture.script("echo up; exit 1", true, move || {
            asked.load(Ordering::Relaxed)
        });
        // Approved for its first start; withdrawn while it runs, before
        // the restart its failure asks for.
        wait_for_output(&service, "up").await;
        approved.store(false, Ordering::Relaxed);
        let (text, status) = output_to_end(&service).await;
        assert!(text.contains("up"), "{text}");
        assert!(text.contains("restarting"), "{text}");
        assert_eq!(
            service.info().service,
            Some(ServiceState::Failed {
                reason: ServiceFailure::NotApproved
            })
        );
        assert!(matches!(status, SessionStatus::Failed { .. }), "{status:?}");
    })
    .await;
}

/// Approval is asked before every start, the first included: a set
/// withdrawn between the node's check and the spawn runs nothing.
#[tokio::test]
async fn an_unapproved_run_starts_nothing() {
    within(async {
        let fixture = Fixture::new();
        let run = fixture.script("echo ran", false, || false);
        let (text, status) = output_to_end(&run).await;
        assert!(!text.contains("ran"), "{text}");
        assert!(matches!(status, SessionStatus::Failed { .. }), "{status:?}");
        assert_eq!(run.info().service, None, "a script has no service state");
    })
    .await;
}

#[tokio::test]
async fn a_service_that_exits_cleanly_is_not_restarted() {
    within(async {
        let fixture = Fixture::new();
        let service = fixture.script("echo done", true, || true);
        let (_text, status) = output_to_end(&service).await;
        assert_eq!(status, SessionStatus::Exited { code: Some(0) });
        assert_eq!(
            service.info().service,
            Some(ServiceState::Exited { code: 0 })
        );
    })
    .await;
}

#[tokio::test]
async fn a_live_service_restarts_in_place_and_stops_when_killed() {
    within(async {
        let fixture = Fixture::new();
        let service = fixture.script("echo service-up; exec sleep 300", true, || true);
        wait_for_output(&service, "service-up").await;
        service.restart().await.unwrap();
        // Started again: a second `service-up` (the watch keeps only the
        // latest state, so the log is what is waited on), with the count
        // from 1.
        wait_for_count(&service, "service-up", 2).await;
        assert!(matches!(
            service.info().service,
            Some(ServiceState::Running { attempt: 1, .. })
        ));
        service.kill();
        let (text, status) = output_to_end(&service).await;
        assert_eq!(text.matches("service-up").count(), 2, "{text}");
        assert!(text.contains("[vessel: restarted]"), "{text}");
        assert_eq!(service.info().service, Some(ServiceState::Stopped));
        assert!(matches!(status, SessionStatus::Exited { .. }), "{status:?}");
    })
    .await;
}

/// The scrollback is bounded by bytes: the oldest output goes, and a reader
/// from the start begins at the oldest kept record.
#[tokio::test]
async fn the_scrollback_drops_its_oldest_output() {
    within(async {
        let fixture = Fixture::new();
        let small = Scrollback {
            records: NonZeroUsize::new(4096).unwrap(),
            bytes: 16 * 1024,
        };
        let run = start_script(ScriptStart {
            info: info(script_kind(false), fixture.folder.path()),
            spec: CommandSpec::new("/bin/sh")
                .arg("-c")
                .arg("i=0; while [ $i -lt 2000 ]; do echo line-$i-0123456789012345678901234567890123456789; i=$((i+1)); done")
                .cwd(fixture.folder.path()),
            leftover: Leftover::default(),
            service: false,
            approved: || true,
            scrollback: small,
            shutdown: fixture.shutdown.clone(),
            tasks: fixture.tasks.clone(),
        });
        run.ended().await;
        let info = run.info();
        assert!(info.oldest_seq > EventSeq::ZERO, "nothing was dropped");
        let page = run.page(EventSeq::ZERO, 512, 1 << 20).unwrap();
        assert_eq!(page.records.first().unwrap().seq, info.oldest_seq);
        let kept: usize = page
            .records
            .iter()
            .map(|record| match &record.event {
                SessionEvent::Terminal(output) => output.data.bytes().len(),
                SessionEvent::Status { .. }
                | SessionEvent::Agent { .. }
                | SessionEvent::ProfileChanged { .. }
                | SessionEvent::ModelCall(_)
                | SessionEvent::OptionsChanged { .. }
                | SessionEvent::Renamed { .. }
                | SessionEvent::QueueChanged { .. }
                | SessionEvent::CommandsChanged { .. }
                | SessionEvent::UsageChanged { .. }
                | SessionEvent::Parked { .. }
                | SessionEvent::Woken
                | SessionEvent::Forked { .. }
                | SessionEvent::Rewound { .. } => 0,
            })
            .sum();
        assert!(kept <= 16 * 1024 + 64 * 4, "kept {kept} bytes");
        assert!(
            run.page(info.next_seq.next().unwrap(), 1, 1).is_err(),
            "past the end"
        );
    })
    .await;
}

/// The node's stop interrupts every terminal.
#[tokio::test]
async fn the_nodes_stop_interrupts_them() {
    within(async {
        let fixture = Fixture::new();
        let shell = fixture.shell(Scrollback::DEFAULT).await;
        let service = fixture.script("exec sleep 300", true, || true);
        fixture.shutdown.cancel();
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        assert_eq!(shell.info().status, SessionStatus::Interrupted);
        assert_eq!(service.info().status, SessionStatus::Interrupted);
        assert_eq!(service.info().service, Some(ServiceState::Stopped));
    })
    .await;
}

#[path = "terminals/lifetime.rs"]
mod lifetime;
