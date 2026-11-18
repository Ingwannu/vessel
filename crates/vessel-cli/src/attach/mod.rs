//! `vessel attach`: join a running (or finished) session; and `vessel
//! shell`'s terminals beside sessions (protocol 11, ADR 0031), attached
//! as a terminal session is (`attach_terminal`).
//!
//! A terminal session is a byte pipe to the harness's own TUI; a structured
//! session is a conversation. Both replay history from `--from` first (a
//! TUI needs its earlier output to draw the current screen), and both
//! leave the session running when they detach.
//!
//! Both survive a lost connection (milestone 2): they follow the session
//! through a `ResumingStream`, which dials the node again and continues
//! from its cursor, so nothing is shown twice or skipped. The user sees
//! `[vessel] connection lost; reconnecting…` and `[vessel] reconnected`.
//! Input goes to whichever connection is current; input in flight when the
//! connection dropped is reported as possibly not applied and is never
//! sent again.

mod structured;
pub(crate) mod terminal;

use std::sync::Arc;

use vessel_client::{
    CallError, Log, NodeClient, ResumingStream, SessionLog, StreamError, follow_resuming,
    follow_terminal_resuming,
};
use vessel_proto::terminals::{SendTerminal, TerminalInfo};
use vessel_proto::{
    EventSeq, SessionEvent, SessionId, SessionInfo, SessionInput, SessionMode, SessionStatus,
    TerminalId, TerminalInput,
};

use crate::connect::{Connected, NodeConnector, client_info};
use crate::error::CliError;
use crate::io;
use crate::render;

/// Attach to `session` until it ends or the user detaches.
pub(crate) async fn attach(
    connected: &Connected,
    session: &SessionInfo,
    from: EventSeq,
) -> Result<(), CliError> {
    let stream = follow_resuming(
        Arc::clone(&connected.connector),
        client_info(),
        session.id,
        from,
    )
    .reusing(connected.client.clone());
    let ended = match session.spec.mode {
        SessionMode::Terminal { .. } => terminal::attach(stream, Keys::Session(session.id)).await?,
        SessionMode::Structured { .. } => structured::attach(stream, session).await?,
    };
    let message = match ended {
        Ended::Detached => format!("[vessel] detached from {}\n", session.id),
        Ended::Session(Some(status)) => format!("[vessel] session {}\n", render::status(&status)),
        Ended::Session(None) => "[vessel] session ended\n".to_owned(),
    };
    io::note(&message).await?;
    Ok(())
}

/// How an attach finished.
#[derive(Debug)]
enum Ended {
    /// The user detached; the session goes on.
    Detached,
    /// The session ended, with its final status if it was seen.
    Session(Option<SessionStatus>),
}

/// The follow the attach modes read: a session's, or a terminal's.
type Stream<L = SessionLog> = ResumingStream<Arc<NodeConnector>, L>;

/// Attach to `terminal` (protocol 11): a shell as a terminal session is,
/// keys in and its screen out; a script's run (which takes no input) by
/// printing its output until it ends or the user interrupts.
pub(crate) async fn attach_terminal(
    connected: &Connected,
    terminal: &TerminalInfo,
    from: EventSeq,
) -> Result<(), CliError> {
    let stream = follow_terminal_resuming(
        Arc::clone(&connected.connector),
        client_info(),
        terminal.id,
        from,
    )
    .reusing(connected.client.clone());
    let ended = if terminal.kind.takes_input() {
        terminal::attach(stream, Keys::Terminal(terminal.id)).await?
    } else {
        print_output(stream).await?
    };
    let message = match ended {
        Ended::Detached => format!("[vessel] detached from {}\n", terminal.id),
        Ended::Session(Some(status)) => {
            format!("[vessel] terminal {}\n", render::status(&status))
        }
        Ended::Session(None) => "[vessel] terminal ended\n".to_owned(),
    };
    io::note(&message).await?;
    Ok(())
}

/// A log's output printed as it comes, with Vessel's own lines, until it
/// ends; Ctrl-C leaves it running.
async fn print_output<L: Log>(mut stream: Stream<L>) -> Result<Ended, CliError> {
    let mut last: Option<SessionStatus> = None;
    // Retain the receiver while output awaits; an interrupt must stay pending.
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    loop {
        let item = tokio::select! {
            biased;
            interrupted = &mut interrupt => {
                interrupted?;
                return Ok(Ended::Detached);
            }
            item = stream.next() => item,
        };
        match item {
            None => return Ok(Ended::Session(last)),
            Some(Ok(record)) => match record.event {
                SessionEvent::Terminal(output) => io::out(output.data.bytes()).await?,
                SessionEvent::Status { status } => {
                    if !status.is_live() {
                        last = Some(status);
                    }
                }
                SessionEvent::Agent {
                    event: vessel_proto::AgentEvent::HarnessLog { line },
                } => io::note(&format!("[{line}]\n")).await?,
                SessionEvent::Agent { .. }
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
                | SessionEvent::Rewound { .. }
                | SessionEvent::Reloaded { .. } => {}
            },
            Some(Err(error)) if !error.is_fatal() => notice(&error, "\n").await?,
            Some(Err(error)) => return Err(error.into()),
        }
    }
}

/// Where typed keys go: a terminal session, or a terminal beside one.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Keys {
    /// A session in terminal mode (`session.send`).
    Session(SessionId),
    /// A terminal (`terminal.send`, protocol 11).
    Terminal(TerminalId),
}

/// What became of input sent while attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sent {
    /// The node took it.
    Accepted,
    /// The node refused it (shown).
    Refused,
    /// The connection dropped during the send (shown): it may or may not
    /// have arrived.
    Lost,
}

/// Send input while attached.
///
/// - A refusal (the session just ended, or another client answered a
///   permission request first) is shown and attaching goes on; the
///   timeline says what happened.
/// - A connection lost during the send is shown as such: the input may or
///   may not have arrived, and it is not sent again (milestone 2 plan,
///   C1). Attaching goes on; the stream reconnects.
async fn send(
    client: &NodeClient,
    session: SessionId,
    input: SessionInput,
) -> Result<Sent, CliError> {
    reported(client.send(session, input).await).await
}

/// Send keys or a resize to `keys`, as [`send`] does.
async fn send_keys(
    client: &NodeClient,
    keys: Keys,
    input: TerminalInput,
) -> Result<Sent, CliError> {
    let sent = match keys {
        Keys::Session(session) => client.send(session, SessionInput::Terminal { input }).await,
        Keys::Terminal(terminal) => client
            .call(SendTerminal { terminal, input })
            .await
            .map(drop),
    };
    reported(sent).await
}

/// What became of a send, shown when it was refused or lost.
async fn reported(sent: Result<(), CallError>) -> Result<Sent, CliError> {
    match sent {
        Ok(()) => Ok(Sent::Accepted),
        Err(CallError::Remote(error)) => {
            io::note(&format!("\r\n[vessel] {}\r\n", error.message)).await?;
            Ok(Sent::Refused)
        }
        Err(CallError::Disconnected) => {
            io::note(
                "\r\n[vessel] the connection dropped while sending input; \
                 it may or may not have been applied\r\n",
            )
            .await?;
            Ok(Sent::Lost)
        }
        Err(error) => Err(error.into()),
    }
}

/// Show a non-fatal stream notice: the first loss, every fifth attempt
/// after it (so a long outage is visibly still being retried), the
/// resume, and pruned records.
async fn notice(error: &StreamError, line_end: &str) -> Result<(), CliError> {
    let show = match error {
        StreamError::Reconnecting { attempt, .. } => *attempt == 1 || attempt % 5 == 0,
        StreamError::Resumed { .. } => {
            io::note(&format!("{line_end}[vessel] reconnected{line_end}")).await?;
            return Ok(());
        }
        StreamError::Missing { .. }
        | StreamError::Call(_)
        | StreamError::Unsubscribed
        | StreamError::Unreachable { .. } => true,
    };
    if show {
        io::note(&format!("{line_end}[vessel] {error}{line_end}")).await?;
    } else {
        tracing::debug!(%error, "still reconnecting");
    }
    Ok(())
}
