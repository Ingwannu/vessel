//! Protocol 11's terminal calls (track C, ADR 0031): the registry and the
//! calls; who may open one where is `place`; the terminal itself is
//! `vessel_session::terminals`'.

#[cfg(test)]
mod creation_metadata_tests;
mod place;

pub(super) use place::archived;

use chrono::Utc;
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, GetTerminal, KillTerminal, ListTerminals, ReadTerminal,
    RenameTerminal, SendTerminal, SubscribeTerminal, TerminalInfo, TerminalKind, TerminalList,
    TerminalPlace, TerminalSubscribed,
};
use vessel_proto::{Ack, Creator, EventPage, TerminalId, TerminalInput, TerminalSize, WireError};
use vessel_session::FollowError;
use vessel_session::terminals::{InputError, ShellStart, TerminalHandle, start_shell};

use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::terminals::{self, CLOSE_WAIT, Entry, MAX_WAIT_SECS, Shell, Start, TerminalsError};
use crate::waits::Waiter;
use place::Resolved;

/// The most output a page carries.
const PAGE_BYTES: usize = 1024 * 1024;

fn wire(error: TerminalsError) -> WireError {
    error.wire()
}

/// Whether `principal` may see and act on `info`: everyone but an agent;
/// an agent, the terminals it opened.
pub(super) fn may_use(principal: &Principal, info: &TerminalInfo) -> bool {
    match principal.agent() {
        None => true,
        Some((node, session)) => info.created_by == Creator::Agent { node, session },
    }
}

/// A delegated connection carries its delivering hop's version, not the
/// original reader's. New metadata must not cross that unresolved boundary.
fn metadata_route_readable(principal: &Principal, info: &TerminalInfo) -> bool {
    let creation =
        info.place.workspace().is_some() || matches!(info.kind, TerminalKind::Profile { .. });
    !creation || !matches!(principal, Principal::Delegated { .. })
}

impl NodeService {
    /// Refuse metadata the caller cannot decode before any terminal mutation.
    fn terminal_for_connection(
        &self,
        connection: &Connection,
        id: TerminalId,
    ) -> Result<Entry, TerminalsError> {
        let entry = self.terminal_for(connection.principal(), id)?;
        let info = entry.handle.info();
        if !metadata_route_readable(connection.principal(), &info)
            || !info.readable_by(connection.protocol())
        {
            return Err(TerminalsError::Unsupported(
                "this terminal requires creation protocol support".to_owned(),
            ));
        }
        Ok(entry)
    }

    /// A terminal the caller may use.
    pub(super) fn terminal_for(
        &self,
        principal: &Principal,
        id: TerminalId,
    ) -> Result<Entry, TerminalsError> {
        let entry = self.terminals.get(id)?;
        if may_use(principal, &entry.handle.info()) {
            Ok(entry)
        } else {
            Err(TerminalsError::Forbidden(
                "an agent may use only the terminals it opened".to_owned(),
            ))
        }
    }

    /// `terminal.create`.
    pub(crate) async fn create_terminal(
        &self,
        connection: &Connection,
        request: CreateTerminal,
    ) -> Result<TerminalInfo, WireError> {
        let principal = connection.principal();
        let watch = self
            .agent_may_open(principal, &request.place)
            .await
            .map_err(wire)?;
        let Some(shell) = self.terminals.runner.config.shell.clone() else {
            return Err(wire(TerminalsError::Unsupported(
                "this node has no shell for terminals".to_owned(),
            )));
        };
        let Resolved { folder, owner } = self
            .resolve_terminal_place(principal, &request.place)
            .await
            .map_err(wire)?;
        // Before the reservation and the spawn, so nothing starts under a
        // removal's teardown; given back once the terminal is registered.
        let hold = self.hold_folder(&folder).map_err(wire)?;
        let id = TerminalId::random();
        let reservation = self
            .terminals
            .reserve(Start {
                owner,
                id,
                folder: folder.clone(),
                run: None,
            })
            .map_err(wire)?;
        let size = request.size.unwrap_or_default();
        let info = TerminalInfo::new(
            id,
            request.place.clone(),
            folder.clone(),
            TerminalKind::Shell {
                program: shell.program.clone(),
            },
            Utc::now(),
        )
        .with_creator(creator(principal))
        .with_title(request.title);
        let handle = self
            .start_shell(&shell, info, &folder, size)
            .await
            .map_err(wire)?;
        // Registered and watched before anything else is awaited: a caller
        // that stops waiting now leaves a terminal the node still ends.
        self.terminals.register(reservation, handle.clone(), None);
        // Registered, it is one of the live terminals a removal reads.
        drop(hold);
        let here = self.links.node();
        for session in watch {
            self.terminals.watch_for_agents(here, session);
        }
        self.still_placed(&handle, &request.place)
            .await
            .map_err(wire)?;
        Ok(handle.info())
    }

    async fn start_shell(
        &self,
        shell: &Shell,
        info: TerminalInfo,
        folder: &std::path::Path,
        size: TerminalSize,
    ) -> Result<TerminalHandle, TerminalsError> {
        let runner = &self.terminals.runner;
        start_shell(ShellStart {
            info,
            spec: terminals::shell_spec(shell, folder, &runner.config.removed),
            size,
            scrollback: runner.config.scrollback,
            shutdown: runner.shutdown.clone(),
            tasks: runner.tasks.clone(),
        })
        .await
        .map_err(|error| match error {
            vessel_session::terminals::TerminalError::ShuttingDown => TerminalsError::ShuttingDown,
            other @ (vessel_session::terminals::TerminalError::Spawn(_)
            | vessel_session::terminals::TerminalError::Internal(_)) => {
                TerminalsError::Spawn(other.to_string())
            }
        })
    }

    /// `terminal.list`.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn list_terminals(
        &self,
        connection: &Connection,
        request: ListTerminals,
    ) -> Result<TerminalList, WireError> {
        let principal = connection.principal();
        let terminals = self.terminals.list(|info| {
            may_use(principal, info)
                && metadata_route_readable(principal, info)
                && info.readable_by(connection.protocol())
                && request.place.as_ref().is_none_or(|place| match place {
                    TerminalPlace::Session { session } => info.place.session() == Some(*session),
                    TerminalPlace::Project { project, .. } => {
                        info.place.project() == Some(*project)
                    }
                    TerminalPlace::Workspace { workspace } => {
                        info.place.workspace() == Some(*workspace)
                    }
                })
        });
        Ok(TerminalList { terminals })
    }

    /// `terminal.get`: with `changed_from`, a long poll in the caller's
    /// wait shares, answered at once when forwarded.
    pub(crate) async fn get_terminal(
        &self,
        connection: &Connection,
        request: GetTerminal,
    ) -> Result<TerminalInfo, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        let mut wait = request.changed_from.map(|_| MAX_WAIT_SECS);
        let _slot = self.terminal_wait(connection, &mut wait);
        if let (Some(from), Some(seconds)) = (request.changed_from, wait) {
            let mut watch = entry.handle.watch();
            let changed = watch.wait_for(|info| info.revision != from);
            let _timed_out =
                tokio::time::timeout(std::time::Duration::from_secs(u64::from(seconds)), changed)
                    .await;
        }
        Ok(entry.handle.info())
    }

    /// A slot in the caller's wait shares for a long poll, or none (and
    /// then no wait): a forwarded call never waits.
    pub(super) fn terminal_wait(
        &self,
        connection: &Connection,
        wait: &mut Option<u32>,
    ) -> Option<crate::waits::WaitSlot> {
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            *wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        self.wait_slot(connection, shares, wait)
    }

    /// `terminal.read`.
    pub(crate) async fn read_terminal(
        &self,
        connection: &Connection,
        request: ReadTerminal,
    ) -> Result<EventPage, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        let limit = usize::try_from(request.limit.clamp(1, ReadTerminal::MAX_LIMIT)).unwrap_or(1);
        let mut wait = request.wait.map(|seconds| seconds.min(MAX_WAIT_SECS));
        let _slot = self.terminal_wait(connection, &mut wait);
        let read = |handle: &TerminalHandle| {
            handle
                .page(request.from, limit, PAGE_BYTES)
                .map_err(|error| match error {
                    FollowError::Ahead { next } => {
                        TerminalsError::Invalid(format!("the terminal's log ends at {next}"))
                    }
                    FollowError::Gap { .. } => {
                        TerminalsError::Conflict("the scrollback moved; read again".to_owned())
                    }
                })
        };
        let mut page = read(&entry.handle).map_err(wire)?;
        if let Some(seconds) = wait
            && page.records.is_empty()
            && !entry.handle.is_ended()
        {
            // Wait on the live stream for anything at `from`.
            if let Ok(mut follow) = entry.handle.follow(request.from)
                && follow.cached.is_empty()
            {
                let _arrived = tokio::time::timeout(
                    std::time::Duration::from_secs(u64::from(seconds)),
                    follow.live.recv(),
                )
                .await;
            }
            page = read(&entry.handle).map_err(wire)?;
        }
        Ok(EventPage {
            records: page
                .records
                .iter()
                .map(|record| record.as_ref().clone())
                .collect(),
            next: page.next,
            at_end: page.at_end,
        })
    }

    /// `terminal.subscribe`.
    pub(crate) async fn subscribe_terminal(
        &self,
        connection: &Connection,
        request: SubscribeTerminal,
    ) -> Result<TerminalSubscribed, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        terminals::subscribe(
            entry.handle,
            &connection.outbound,
            &self.forwarders,
            request,
        )
        .await
        .map_err(wire)
    }

    /// `terminal.send`.
    pub(crate) async fn send_terminal(
        &self,
        connection: &Connection,
        request: SendTerminal,
    ) -> Result<Ack, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        if let TerminalInput::Data { data } = &request.input
            && data.bytes().len() > SendTerminal::MAX_BYTES
        {
            return Err(wire(TerminalsError::Invalid(
                "at most 64 KiB of input a call".to_owned(),
            )));
        }
        entry
            .handle
            .send(request.input)
            .await
            .map(|()| Ack)
            .map_err(|error| {
                wire(match error {
                    InputError::NoInput => TerminalsError::Unsupported(error.to_string()),
                    InputError::Ended => TerminalsError::Conflict(error.to_string()),
                })
            })
    }

    /// `terminal.rename`.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn rename_terminal(
        &self,
        connection: &Connection,
        request: RenameTerminal,
    ) -> Result<TerminalInfo, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        entry.handle.rename(request.title);
        Ok(entry.handle.info())
    }

    /// `terminal.kill`: ended, and kept with its output.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn kill_terminal(
        &self,
        connection: &Connection,
        request: KillTerminal,
    ) -> Result<Ack, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        entry.handle.kill();
        Ok(Ack)
    }

    /// `terminal.close`: ended, and forgotten.
    pub(crate) async fn close_terminal(
        &self,
        connection: &Connection,
        request: CloseTerminal,
    ) -> Result<Ack, WireError> {
        let entry = self
            .terminal_for_connection(connection, request.terminal)
            .map_err(wire)?;
        entry.handle.kill();
        if tokio::time::timeout(CLOSE_WAIT, entry.handle.ended())
            .await
            .is_err()
        {
            tracing::warn!(terminal = %request.terminal, "a closed terminal did not end in time; forgotten anyway");
        }
        self.terminals.forget(request.terminal);
        Ok(Ack)
    }
}
