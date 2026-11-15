//! Register before allowing any script attempt to acquire physical admission.
use super::{NodeService, remove_leftover};
use crate::{
    sessions::Sessions,
    terminals::{self, Owner, Plan, Start, Terminals, TerminalsError},
    workspace::{Starting, WorkspaceError, Workspaces},
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::watch;
use vessel_proc::CommandSpec;
use vessel_proto::{
    Creator, TerminalId,
    scripts::{ScriptRefusal, ScriptRunOutcome},
    terminals::TerminalInfo,
};
use vessel_session::terminals::{Leftover, ScriptStart, start_script_admitted};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Registration {
    Pending,
    Ready,
}

pub(super) struct Opening {
    pub(super) spec: CommandSpec,
    pub(super) leftover: Leftover,
    pub(super) plan: Arc<Plan>,
    pub(super) hold: Option<Starting>,
    pub(super) creator: Creator,
}
struct Context {
    sessions: Arc<Sessions>,
    workspaces: Arc<Workspaces>,
    terminals: Arc<Terminals>,
}
pub(super) async fn start(
    service: &NodeService,
    opening: Opening,
) -> Result<ScriptRunOutcome, TerminalsError> {
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
    async fn run(self, context: Context) -> Result<ScriptRunOutcome, TerminalsError> {
        let Self {
            spec,
            leftover,
            plan,
            hold,
            creator,
        } = self;
        let folder = spec
            .cwd
            .clone()
            .unwrap_or_else(|| plan.script_root.root.clone());
        let id = TerminalId::random();
        let reservation = match context.terminals.reserve(Start {
            owner: Owner {
                session: plan.place.session(),
                project: Some(plan.script_root.project),
                agent: None,
            },
            id,
            folder: folder.clone(),
            run: Some((plan.root.clone(), plan.entry.name.clone())),
        }) {
            Ok(reservation) => reservation,
            Err(error) => {
                remove_leftover(leftover.0.as_deref()).await;
                return match error {
                    TerminalsError::Running(terminal) => Ok(ScriptRunOutcome::Refused {
                        reason: ScriptRefusal::AlreadyRunning { terminal },
                    }),
                    other @ (TerminalsError::NotFound(_)
                    | TerminalsError::Limit(_)
                    | TerminalsError::Forbidden(_)
                    | TerminalsError::Conflict(_)
                    | TerminalsError::Unsupported(_)
                    | TerminalsError::Invalid(_)
                    | TerminalsError::Spawn(_)
                    | TerminalsError::ShuttingDown
                    | TerminalsError::Wire(_)) => Err(other),
                };
            }
        };
        let command = Command {
            spec,
            leftover,
            plan: Arc::clone(&plan),
            creator,
        };
        let (registered, registration) = watch::channel(Registration::Pending);
        let handle = launch(
            &context.terminals.runner,
            &context.workspaces,
            command,
            id,
            folder,
            registration,
        );
        context
            .terminals
            .register(reservation, handle.clone(), Some(Arc::clone(&plan)));
        super::super::terminals::check_place(
            &context.sessions,
            &context.workspaces,
            &context.terminals,
            &handle,
            &plan.place,
        )
        .await?;
        // Only a registered, still-placed run can acquire admission and spawn.
        let _previous = registered.send_replace(Registration::Ready);
        drop(hold);
        Ok(ScriptRunOutcome::Started {
            terminal: Box::new(handle.info()),
        })
    }
}

struct Command {
    spec: CommandSpec,
    leftover: Leftover,
    plan: Arc<Plan>,
    creator: Creator,
}
fn launch(
    runner: &terminals::Runner,
    workspaces: &Arc<Workspaces>,
    command: Command,
    id: TerminalId,
    folder: std::path::PathBuf,
    registration: watch::Receiver<Registration>,
) -> vessel_session::terminals::TerminalHandle {
    let Command {
        spec,
        leftover,
        plan,
        creator,
    } = command;
    let info = TerminalInfo::new(
        id,
        plan.place.clone(),
        folder.clone(),
        plan.kind(),
        Utc::now(),
    )
    .with_creator(creator);
    let workspaces = Arc::clone(workspaces);
    let approval = Arc::clone(&workspaces);
    let (project, digest) = (plan.script_root.project, plan.digest.clone());
    start_script_admitted(
        ScriptStart {
            info,
            spec: terminals::scrubbed(spec, &runner.config.removed),
            leftover,
            service: matches!(
                plan.entry.kind(),
                vessel_proto::scripts::ScriptKind::Service
            ),
            approved: move || approval.is_approved(project, &digest),
            scrollback: runner.config.scrollback,
            shutdown: runner.shutdown.clone(),
            tasks: runner.tasks.clone(),
        },
        move || {
            let mut registration = registration.clone();
            let workspaces = Arc::clone(&workspaces);
            let folder = folder.clone();
            async move {
                {
                    let _ready = registration
                        .wait_for(|state| *state == Registration::Ready)
                        .await
                        .map_err(|_closed| WorkspaceError::Stopping)?;
                }
                let root = workspaces.admit_start(&folder).await?;
                root.revalidate().await?;
                Ok::<_, WorkspaceError>(root)
            }
        },
    )
}
