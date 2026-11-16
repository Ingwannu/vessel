//! Protocol 11's script and port calls (track C, ADR 0031): a project's
//! scripts run where a person approved them (ADR 0027), as terminals.

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::ports::{ListPorts, PortList, PortOwner};
use vessel_proto::scripts::{
    ListScripts, RestartScript, RunScript, ScriptList, ScriptPlace, ScriptRefusal, ScriptRunOutcome,
};
use vessel_proto::terminals::{TerminalInfo, TerminalPlace};
use vessel_proto::{Creator, TerminalId, WireError};
use vessel_session::terminals::{Leftover, RestartError, ScriptStart, start_script};

use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::terminals::{self, MAX_WAIT_SECS, Owner, Plan, Start, TerminalsError};
use crate::workspace::{DirError, ScriptRoot};

fn wire(error: TerminalsError) -> WireError {
    error.wire()
}

impl NodeService {
    /// The checkout of a script's place.
    async fn script_place(&self, place: &ScriptPlace) -> Result<ScriptRoot, ScriptLookup> {
        let (project, worktree) = match place {
            ScriptPlace::Project { project } => (*project, None),
            ScriptPlace::Session { session } => {
                let info =
                    self.sessions.get(*session).await.map_err(|error| {
                        ScriptLookup::Failed(TerminalsError::Wire(error.to_wire()))
                    })?;
                if info.marks.archived_at.is_some() {
                    return Err(ScriptLookup::Failed(super::terminals::archived()));
                }
                // The session's place as the registry knows it: the info
                // a session's runtime holds carries none (the service
                // fills `workspace` per reply, `with_workspace`).
                let Some(workspace) = self.workspaces.workspace_of(&info.spec) else {
                    return Err(ScriptLookup::NoProject);
                };
                (workspace.project, workspace.worktree)
            }
        };
        self.workspaces
            .script_root(project, worktree)
            .map_err(|error| ScriptLookup::Failed(TerminalsError::Wire(error.wire())))
    }

    /// `script.list`: agents are refused before this runs (`guard`).
    pub(crate) async fn list_scripts(
        &self,
        _: &Connection,
        request: ListScripts,
    ) -> Result<ScriptList, WireError> {
        let root = match self.script_place(&request.place).await {
            Ok(root) => root,
            Err(ScriptLookup::NoProject) => {
                return Err(wire(TerminalsError::Invalid(
                    "the session runs in no project, so it has no project file".to_owned(),
                )));
            }
            Err(ScriptLookup::Failed(error)) => return Err(wire(error)),
        };
        let (commands, _parsed) = self.workspaces.scripts(&root).await;
        let runs = self.terminals.runs_in(&root.root);
        Ok(ScriptList {
            root: root.root,
            commands,
            runs,
        })
    }

    /// `script.run`: agents are refused before this runs (`guard`).
    pub(crate) async fn run_script(
        &self,
        connection: &Connection,
        request: RunScript,
    ) -> Result<ScriptRunOutcome, WireError> {
        let root = match self.script_place(&request.place).await {
            Ok(root) => root,
            Err(ScriptLookup::NoProject) => return Ok(refused(ScriptRefusal::NoProject)),
            Err(ScriptLookup::Failed(error)) => return Err(wire(error)),
        };
        let (commands, parsed) = self.workspaces.scripts(&root).await;
        let file = match parsed {
            Ok(Some(file)) => file,
            Ok(None) => return Ok(refused(ScriptRefusal::NoSuchScript)),
            Err(error) => {
                return Ok(refused(ScriptRefusal::InvalidFile {
                    message: error.to_string(),
                }));
            }
        };
        let Some(entry) = file.script(&request.script).cloned() else {
            return Ok(refused(ScriptRefusal::NoSuchScript));
        };
        // The digest of the very parse whose command runs: never a second
        // read (ADR 0027).
        let digest = file.digest();
        if !self.workspaces.is_approved(root.project, &digest) {
            return Ok(ScriptRunOutcome::NeedsApproval { commands });
        }
        if let ScriptPlace::Project { project } = &request.place {
            // ADR 0106: work starting at a project root opens its workspace;
            // a session place was registered when the session was.
            self.workspaces.register_root(*project).await;
        }
        let place = match request.place {
            ScriptPlace::Session { session } => TerminalPlace::Session { session },
            ScriptPlace::Project { project } => TerminalPlace::Project {
                project,
                dir: entry.dir.clone(),
            },
        };
        let plan = Arc::new(Plan {
            place,
            root: root.root.clone(),
            script_root: root,
            entry,
            digest,
        });
        self.start_run(connection.principal(), plan).await
    }

    /// Start `plan`'s run in a new terminal.
    async fn start_run(
        &self,
        principal: &Principal,
        plan: Arc<Plan>,
    ) -> Result<ScriptRunOutcome, WireError> {
        let (spec, leftover) = match self
            .workspaces
            .script_command(&plan.script_root, &plan.entry)
            .await
        {
            Ok(command) => command,
            Err(DirError::Missing) => return Ok(refused(ScriptRefusal::NoSuchDir)),
            Err(DirError::Workspace(error)) => return Err(error.wire()),
        };
        let folder = spec
            .cwd
            .clone()
            .unwrap_or_else(|| plan.script_root.root.clone());
        // A run in a worktree holds it from removal, setup and teardown until
        // it is registered (as a shell does); refused while they hold it.
        let hold = match self.hold_folder(&folder) {
            Ok(hold) => hold,
            Err(refusal) => {
                remove_leftover(leftover.as_deref()).await;
                return Err(wire(refusal));
            }
        };
        let id = TerminalId::random();
        // One lock decides the limits and that the script does not run
        // already, so two runs at once cannot both start.
        let reserved = self.terminals.reserve(Start {
            owner: Owner {
                session: plan.place.session(),
                project: Some(plan.script_root.project),
                agent: None,
            },
            id,
            folder: folder.clone(),
            run: Some((plan.root.clone(), plan.entry.name.clone())),
        });
        let reservation = match reserved {
            Ok(reservation) => reservation,
            Err(refusal) => {
                remove_leftover(leftover.as_deref()).await;
                return match refusal {
                    TerminalsError::Running(terminal) => {
                        Ok(refused(ScriptRefusal::AlreadyRunning { terminal }))
                    }
                    other @ (TerminalsError::NotFound(_)
                    | TerminalsError::Limit(_)
                    | TerminalsError::Forbidden(_)
                    | TerminalsError::Conflict(_)
                    | TerminalsError::Unsupported(_)
                    | TerminalsError::Invalid(_)
                    | TerminalsError::Spawn(_)
                    | TerminalsError::ShuttingDown
                    | TerminalsError::Wire(_)) => Err(wire(other)),
                };
            }
        };
        let runner = &self.terminals.runner;
        let spec = terminals::scrubbed(spec, &runner.config.removed);
        let info = TerminalInfo::new(id, plan.place.clone(), folder, plan.kind(), Utc::now())
            .with_creator(creator(principal));
        let workspaces = Arc::clone(&self.workspaces);
        let (project, digest) = (plan.script_root.project, plan.digest.clone());
        let handle = start_script(ScriptStart {
            info,
            spec,
            leftover: Leftover(leftover),
            service: matches!(
                plan.entry.kind(),
                vessel_proto::scripts::ScriptKind::Service
            ),
            approved: move || workspaces.is_approved(project, &digest),
            scrollback: runner.config.scrollback,
            shutdown: runner.shutdown.clone(),
            tasks: runner.tasks.clone(),
        });
        self.terminals
            .register(reservation, handle.clone(), Some(Arc::clone(&plan)));
        drop(hold);
        self.still_placed(&handle, &plan.place)
            .await
            .map_err(wire)?;
        Ok(ScriptRunOutcome::Started {
            terminal: Box::new(handle.info()),
        })
    }

    /// `script.restart`: in place while it runs, in a new terminal once it
    /// ended; only while its set is still approved.
    pub(crate) async fn restart_script(
        &self,
        connection: &Connection,
        request: RestartScript,
    ) -> Result<ScriptRunOutcome, WireError> {
        let entry = self.terminals.get(request.terminal).map_err(wire)?;
        let Some(plan) = entry.plan.clone() else {
            return Err(wire(TerminalsError::Unsupported(
                "only a script's run can be restarted".to_owned(),
            )));
        };
        if !self
            .workspaces
            .is_approved(plan.script_root.project, &plan.digest)
        {
            let (commands, _parsed) = self.workspaces.scripts(&plan.script_root).await;
            return Ok(ScriptRunOutcome::NeedsApproval { commands });
        }
        match entry.handle.restart().await {
            Ok(()) => Ok(ScriptRunOutcome::Started {
                terminal: Box::new(entry.handle.info()),
            }),
            Err(RestartError::Ended) => self.start_run(connection.principal(), plan).await,
            Err(RestartError::NotARun) => Err(wire(TerminalsError::Unsupported(
                "only a script's run can be restarted".to_owned(),
            ))),
        }
    }

    /// `port.list`: what the caller may see of the last scan; a long poll
    /// with `changed_from`, in the wait shares, at once when forwarded.
    pub(crate) async fn list_ports(
        &self,
        connection: &Connection,
        request: ListPorts,
    ) -> Result<PortList, WireError> {
        let mut wait = request.changed_from.map(|_| MAX_WAIT_SECS);
        let _slot = self.terminal_wait(connection, &mut wait);
        let principal = connection.principal();
        let mine = principal
            .agent()
            .map(|(node, session)| (session, Creator::Agent { node, session }));
        // An agent's own are picked out before the reply's bound, so the
        // node's other ports never cut them off.
        let list = self
            .ports
            .list(request.changed_from, wait.is_some(), |port| match &mine {
                None => true,
                Some((session, mine)) => match &port.owner {
                    PortOwner::Session { session: owner } => owner == session,
                    PortOwner::Terminal { terminal } => self
                        .terminals
                        .get(*terminal)
                        .is_ok_and(|entry| entry.handle.info().created_by == *mine),
                },
            })
            .await;
        Ok(list)
    }
}

/// Why a script's place has no checkout.
enum ScriptLookup {
    /// A session with no project.
    NoProject,
    /// The place is not there.
    Failed(TerminalsError),
}

/// A run that did not start leaves no command file (Windows' `.cmd`).
async fn remove_leftover(leftover: Option<&std::path::Path>) {
    if let Some(path) = leftover
        && let Err(error) = tokio::fs::remove_file(path).await
    {
        tracing::debug!(%error, "could not remove a run's command file");
    }
}

fn refused(reason: ScriptRefusal) -> ScriptRunOutcome {
    ScriptRunOutcome::Refused { reason }
}
