//! Protocol 11's terminal calls (track C, ADR 0031): who may, where, and
//! the registry; the terminal itself is `vessel_session::terminals`'.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, GetTerminal, KillTerminal, ListTerminals, ReadTerminal,
    RenameTerminal, SendTerminal, SubscribeTerminal, TerminalInfo, TerminalKind, TerminalList,
    TerminalPlace, TerminalSubscribed,
};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    Ack, Creator, EventPage, NodeId, SessionId, TerminalId, TerminalInput, TerminalSize, WireError,
};
use vessel_session::FollowError;
use vessel_session::terminals::{InputError, ShellStart, TerminalHandle, start_shell};

use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::terminals::{self, CLOSE_WAIT, Entry, MAX_WAIT_SECS, Owner, Shell, TerminalsError};
use crate::waits::Waiter;

/// The most output a page carries.
const PAGE_BYTES: usize = 1024 * 1024;

/// Where a terminal resolved to.
pub(super) struct Resolved {
    pub(super) folder: PathBuf,
    pub(super) owner: Owner,
}

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

impl NodeService {
    /// The folder a place resolves to, and what a terminal there counts
    /// against.
    pub(super) async fn resolve_terminal_place(
        &self,
        principal: &Principal,
        place: &TerminalPlace,
    ) -> Result<Resolved, TerminalsError> {
        let agent = principal.agent();
        let folder = match place {
            TerminalPlace::Session { session } => {
                let info = self
                    .sessions
                    .get(*session)
                    .await
                    .map_err(|error| TerminalsError::Wire(error.to_wire()))?;
                if let Some(worktree) = info.workspace.as_ref().and_then(|place| place.worktree)
                    && self.workspaces.busy(worktree)
                {
                    return Err(TerminalsError::Conflict(
                        "the session's worktree is being set up, torn down or removed".to_owned(),
                    ));
                }
                let folder = info
                    .spec
                    .cwd
                    .clone()
                    .unwrap_or_else(|| self.profiles.user_home().to_path_buf());
                let checked = folder.clone();
                let exists = tokio::task::spawn_blocking(move || checked.is_dir())
                    .await
                    .unwrap_or(false);
                if !exists {
                    return Err(TerminalsError::Conflict(format!(
                        "the session's folder is gone: {}",
                        folder.display()
                    )));
                }
                folder
            }
            TerminalPlace::Project { project, dir } => {
                let placed = self
                    .workspaces
                    .place(&SessionPlace::Project {
                        project: *project,
                        dir: dir.clone(),
                    })
                    .await
                    .map_err(|error| TerminalsError::Wire(error.wire()))?;
                placed.cwd
            }
        };
        Ok(Resolved {
            folder,
            owner: Owner {
                session: place.session(),
                project: place.project(),
                agent,
            },
        })
    }

    /// An agent may open a terminal only beside its own session or one it
    /// may operate, and only where that session's profile turned agent
    /// terminals on (ADR 0031).
    async fn agent_may_open(
        &self,
        principal: &Principal,
        place: &TerminalPlace,
    ) -> Result<(), TerminalsError> {
        let Some((node, own)) = principal.agent() else {
            return Ok(());
        };
        let TerminalPlace::Session { session } = place else {
            return Err(TerminalsError::Forbidden(
                "an agent opens terminals only beside a session".to_owned(),
            ));
        };
        let here = self.links.node();
        if !(node == here && *session == own) {
            self.check_owner(principal, *session)
                .await
                .map_err(TerminalsError::Wire)?;
        }
        let info = self
            .sessions
            .get(*session)
            .await
            .map_err(|error| TerminalsError::Wire(error.to_wire()))?;
        let granted = info
            .profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
            .is_some_and(|profile| profile.agent_terminals());
        if granted {
            Ok(())
        } else {
            Err(TerminalsError::Forbidden(
                "agents may not open terminals beside this session: its profile does not set \
                 `agent_terminals = true` (profiles.toml), which a person turns on"
                    .to_owned(),
            ))
        }
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

    /// After a start: the place still there, or the terminal ended and
    /// forgotten (its session or project went away meanwhile; deletion
    /// ends what is registered, so one of the two always ends it).
    pub(super) async fn still_placed(
        &self,
        handle: &TerminalHandle,
        place: &TerminalPlace,
    ) -> Result<(), TerminalsError> {
        let there = match place {
            TerminalPlace::Session { session } => self.sessions.get(*session).await.is_ok(),
            TerminalPlace::Project { project, .. } => self.workspaces.has_project(*project),
        };
        if there {
            return Ok(());
        }
        handle.kill();
        let _ended = tokio::time::timeout(CLOSE_WAIT, handle.ended()).await;
        self.terminals.forget(handle.id());
        Err(TerminalsError::NotFound(handle.id()))
    }

    /// When an agent opened it: end its terminals when its session ends
    /// (its credential dies then). Only this node's agents can be watched;
    /// another node's agent's terminals end with the session they are
    /// beside.
    fn end_with_agent(&self, agent: Option<(NodeId, SessionId)>) {
        let Some((node, session)) = agent else { return };
        if node != self.links.node() {
            return;
        }
        let Some(handle) = self.sessions.runtime_handle(session) else {
            return;
        };
        let terminals = Arc::clone(&self.terminals);
        drop(self.terminals.runner.tasks.spawn(async move {
            let _info = handle.ended().await;
            terminals.end_agent((node, session)).await;
        }));
    }

    /// `terminal.create`.
    pub(crate) async fn create_terminal(
        &self,
        connection: &Connection,
        request: CreateTerminal,
    ) -> Result<TerminalInfo, WireError> {
        let principal = connection.principal();
        self.agent_may_open(principal, &request.place)
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
        let agent = owner.agent;
        let reservation = self.terminals.reserve(owner).map_err(wire)?;
        let size = request.size.unwrap_or_default();
        let info = TerminalInfo::new(
            TerminalId::random(),
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
        self.terminals.register(
            reservation,
            Entry {
                handle: handle.clone(),
                plan: None,
            },
        );
        self.still_placed(&handle, &request.place)
            .await
            .map_err(wire)?;
        self.end_with_agent(agent);
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
                && request.place.as_ref().is_none_or(|place| match place {
                    TerminalPlace::Session { session } => info.place.session() == Some(*session),
                    TerminalPlace::Project { project, .. } => {
                        info.place.project() == Some(*project)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
            .terminal_for(connection.principal(), request.terminal)
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
