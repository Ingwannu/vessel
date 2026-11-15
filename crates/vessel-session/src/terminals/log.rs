//! [`TermLog`]: a terminal's records, numbered and kept, and its info kept
//! in step with them. Written only by the terminal's actor.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use bytes::Bytes;
use chrono::Utc;
use vessel_proto::terminals::ServiceState;
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, SessionEvent, SessionStatus, TerminalBytes, TerminalOutput,
};

use super::handle::Shared;

/// The actor's side of a terminal's log.
#[derive(Debug)]
pub(super) struct TermLog {
    shared: Arc<Shared>,
    next: EventSeq,
    screen: crate::terminal_screen::ScreenAuthority,
}

impl TermLog {
    pub(super) const fn new(
        shared: Arc<Shared>,
        screen: crate::terminal_screen::ScreenAuthority,
    ) -> Self {
        Self {
            shared,
            next: EventSeq::ZERO,
            screen,
        }
    }

    /// Number `event`, keep it, broadcast it. The info's sequence numbers
    /// move without waking its watchers: output is not a change a waiter
    /// on `revision` cares about (as a session's `next_seq` moves
    /// silently).
    fn publish(&mut self, event: SessionEvent) {
        let seq = self.next;
        // The last `u64` is never reached: a terminal printing a record a
        // nanosecond would need centuries.
        self.next = seq.next().unwrap_or(seq);
        let record = Arc::new(EventRecord {
            seq,
            at: Utc::now(),
            event,
        });
        let oldest = {
            let mut ring = self.shared.ring();
            ring.append(record);
            ring.oldest()
        };
        let next = self.next;
        let _silent = self.shared.info.send_if_modified(|info| {
            info.next_seq = next;
            info.oldest_seq = oldest;
            false
        });
    }

    pub(super) fn poll_capture(&mut self) {
        self.screen.poll_one();
    }

    pub(super) async fn capture_command(
        &mut self,
    ) -> Option<crate::terminal_screen::CaptureCommand> {
        self.screen.next().await
    }

    pub(super) fn reply_capture(&mut self, command: crate::terminal_screen::CaptureCommand) {
        self.screen.command(command);
    }

    /// A chunk of output.
    pub(super) fn output(&mut self, bytes: Bytes) {
        self.screen.output(&bytes);
        self.publish(SessionEvent::Terminal(TerminalOutput {
            data: TerminalBytes::new(bytes),
        }));
    }

    /// One of Vessel's own lines (`vessel: …`), as a terminal session's
    /// notices are.
    pub(super) fn notice(&mut self, line: String) {
        self.publish(SessionEvent::Agent {
            event: AgentEvent::HarnessLog { line },
        });
    }

    /// The process's state. A final one ends the log: its followers see
    /// the end right after it.
    pub(super) fn status(&mut self, status: SessionStatus) {
        let live = status.is_live();
        self.publish(SessionEvent::Status {
            status: status.clone(),
        });
        self.shared.info.send_modify(|info| {
            info.status = status;
            if !live {
                info.ended_at = Some(Utc::now());
            }
            info.revision = info.revision.wrapping_add(1);
        });
        if !live {
            self.screen.finish();
            self.shared.pid.store(0, Ordering::Relaxed);
            drop(
                self.shared
                    .process
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take(),
            );
            self.shared.ring().close();
        }
    }

    /// A service's state: typed on the info (its revision moves), and a
    /// line in the log for a person reading the output (no event of its
    /// own, ADR 0031).
    pub(super) fn service(&mut self, state: ServiceState) {
        self.notice(format!("vessel: {}", state.summary()));
        self.shared.info.send_modify(|info| {
            info.service = Some(state);
            info.revision = info.revision.wrapping_add(1);
        });
    }

    /// The running process's id, for port scans; `None` between runs.
    pub(super) fn running(&self, process: Option<vessel_proc::ProcessLifetime>) {
        let pid = process.as_ref().map_or(0, |process| process.pid().get());
        self.shared.pid.store(pid, Ordering::Relaxed);
        *self
            .shared
            .process
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = process;
    }
}
