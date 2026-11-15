//! The node owns shell startup until registration and placement checks settle.
use super::{NodeService, place};
use crate::{
    sessions::Sessions,
    terminals::{self, Owner, Shell, Terminals, TerminalsError},
    workspace::{StartGuard, Starting, WorkspaceError, Workspaces},
};
use std::sync::Arc;
use vessel_proto::{NodeId, TerminalSize, terminals::TerminalInfo};
use vessel_session::{
    SessionHandle,
    terminals::{ShellStart, TerminalHandle, start_shell},
};

pub(super) struct Opening {
    pub(super) shell: Shell,
    pub(super) info: TerminalInfo,
    pub(super) size: TerminalSize,
    pub(super) owner: Owner,
    pub(super) root: StartGuard,
    pub(super) placement: Option<Starting>,
    pub(super) here: NodeId,
    pub(super) watch: Vec<SessionHandle>,
}
struct Context {
    sessions: Arc<Sessions>,
    workspaces: Arc<Workspaces>,
    terminals: Arc<Terminals>,
}
pub(super) async fn start(
    service: &NodeService,
    opening: Opening,
) -> Result<TerminalInfo, TerminalsError> {
    let context = Context {
        sessions: Arc::clone(&service.sessions),
        workspaces: Arc::clone(&service.workspaces),
        terminals: Arc::clone(&service.terminals),
    };
    service
        .terminals
        .runner
        .tasks
        .spawn(opening.run(context))
        .await
        .map_err(|error| TerminalsError::Wire(WorkspaceError::Task(error).wire()))?
}
impl Opening {
    async fn run(self, context: Context) -> Result<TerminalInfo, TerminalsError> {
        let Self {
            shell,
            info,
            size,
            owner,
            root,
            placement,
            here,
            watch,
        } = self;
        root.revalidate()
            .await
            .map_err(|error| TerminalsError::Wire(error.wire()))?;
        let reservation = context.terminals.reserve(terminals::Start {
            owner,
            id: info.id,
            folder: info.folder.clone(),
            run: None,
        })?;
        let place = info.place.clone();
        let handle =
            shell_start(&context.terminals.runner, &shell, info, root.folder(), size).await?;
        context
            .terminals
            .register(reservation, handle.clone(), None);
        for session in watch {
            context.terminals.watch_for_agents(here, session);
        }
        // Runtime registration now makes this process visible to restore's live
        // check; caller cancellation cannot skip either that or placement cleanup.
        drop(placement);
        let placed = place::check(
            &context.sessions,
            &context.workspaces,
            &context.terminals,
            &handle,
            &place,
        )
        .await;
        if placed.is_err() {
            handle.kill();
            handle.ended().await;
        }
        drop(root);
        placed?;
        Ok(handle.info())
    }
}
async fn shell_start(
    runner: &terminals::Runner,
    shell: &Shell,
    info: TerminalInfo,
    folder: &std::path::Path,
    size: TerminalSize,
) -> Result<TerminalHandle, TerminalsError> {
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
