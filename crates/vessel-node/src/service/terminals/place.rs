//! Where a terminal may be opened and by whom (protocol 11, ADR 0031): a
//! place resolved to its folder and what it counts against, an agent's
//! permission (its own profile and the target session's both allow it),
//! the worktree held while it starts (the join of wave 2's first half),
//! and the place checked again once it runs.

use std::path::PathBuf;

use vessel_proto::SessionId;
use vessel_proto::terminals::TerminalPlace;
use vessel_proto::workspace::SessionPlace;
use vessel_session::SessionHandle;
use vessel_session::terminals::TerminalHandle;

use super::super::NodeService;
use crate::principal::Principal;
use crate::terminals::{CLOSE_WAIT, Owner, TerminalsError};

/// Where a terminal resolved to.
pub(super) struct Resolved {
    pub(super) folder: PathBuf,
    pub(super) owner: Owner,
    /// The place as the terminal keeps it: a folder place's path made
    /// canonical, so a list by the same folder finds it.
    pub(super) place: TerminalPlace,
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
        // A terminal beside a session in a project counts against the
        // project too.
        let mut project = place.project();
        let mut kept = place.clone();
        let mut in_folder = None;
        let folder = match place {
            TerminalPlace::Session { session } => {
                let info = self
                    .sessions
                    .get(*session)
                    .await
                    .map_err(|error| TerminalsError::Wire(error.to_wire()))?;
                if info.marks.archived_at.is_some() {
                    return Err(archived());
                }
                // The registry's place for it: the runtime's info carries
                // none (the service fills `workspace` per reply).
                project = self
                    .workspaces
                    .workspace_of(&info.spec)
                    .map(|place| place.project);
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
            // A worktree's own workspace (ADR 0121): its folder, counted
            // against its project.
            TerminalPlace::Worktree { worktree } => {
                let info = self
                    .workspaces
                    .worktree(*worktree)
                    .map_err(|error| TerminalsError::Wire(error.wire()))?;
                project = Some(info.project);
                let placed = self
                    .workspaces
                    .place(&SessionPlace::Worktree {
                        worktree: *worktree,
                    })
                    .await
                    .map_err(|error| TerminalsError::Wire(error.wire()))?;
                placed.cwd
            }
            // An unregistered folder's workspace (ADR 0121): any existing
            // absolute folder, kept canonical. The projects' refused roots
            // do not apply: they guard what a file panel may read, and a
            // home folder is a usual place for a shell.
            TerminalPlace::Folder { path } => {
                let folder = canonical_folder(path).await?;
                kept = TerminalPlace::Folder {
                    path: folder.clone(),
                };
                in_folder = Some(folder.clone());
                folder
            }
        };
        Ok(Resolved {
            folder,
            owner: Owner {
                session: place.session(),
                project,
                agent,
                folder: in_folder,
            },
            place: kept,
        })
    }

    /// An agent may open a terminal only beside its own session or one it
    /// may operate, only where that session's profile turned agent
    /// terminals on, and (an agent of this node) only where its own
    /// session's profile did too: a person allowed terminals for that
    /// harness's sandbox, not for an agent that started a session with
    /// another profile (ADR 0031). Answers the sessions whose end must end
    /// the terminal: the one it is beside, and the agent's own.
    pub(super) async fn agent_may_open(
        &self,
        principal: &Principal,
        place: &TerminalPlace,
    ) -> Result<Vec<SessionHandle>, TerminalsError> {
        let Some((node, own)) = principal.agent() else {
            return Ok(Vec::new());
        };
        let TerminalPlace::Session { session } = place else {
            return Err(TerminalsError::Forbidden(
                "an agent opens terminals only beside a session".to_owned(),
            ));
        };
        let here = self.links.node();
        let mut watch = Vec::new();
        if node == here {
            self.profile_allows(own).await?;
            watch.push(self.live_handle(own, "your session has ended")?);
        }
        if !(node == here && *session == own) {
            self.check_owner(principal, *session)
                .await
                .map_err(TerminalsError::Wire)?;
            self.profile_allows(*session).await?;
            watch.push(self.live_handle(*session, "the session has ended")?);
        }
        Ok(watch)
    }

    /// `session`'s profile sets `agent_terminals = true`, or `forbidden`.
    pub(crate) async fn profile_allows(&self, session: SessionId) -> Result<(), TerminalsError> {
        let info = self
            .sessions
            .get(session)
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
                "agents may not open terminals here: the session's profile does not set \
                 `agent_terminals = true` (profiles.toml), which a person turns on"
                    .to_owned(),
            ))
        }
    }

    /// The runtime's handle of a live session, whose end an agent's
    /// terminal is tied to.
    fn live_handle(
        &self,
        session: SessionId,
        ended: &str,
    ) -> Result<SessionHandle, TerminalsError> {
        self.sessions
            .runtime_handle(session)
            .filter(|handle| !handle.has_ended())
            .ok_or_else(|| TerminalsError::Conflict(ended.to_owned()))
    }

    /// After a start: the place still there, or the terminal ended and
    /// forgotten (its session or project went away meanwhile; deletion
    /// ends what is registered, so one of the two always ends it). A
    /// removal of its worktree cannot have begun meanwhile: the start held
    /// the worktree (`Workspaces::hold_folder`) until it was registered.
    pub(in crate::service) async fn still_placed(
        &self,
        handle: &TerminalHandle,
        place: &TerminalPlace,
    ) -> Result<(), TerminalsError> {
        let refusal = match place {
            // Archived meanwhile, it is put away: its archiving ends what
            // was registered before its mark, this what came after.
            TerminalPlace::Session { session } => match self.sessions.get(*session).await {
                Ok(info) if info.marks.archived_at.is_some() => Some(archived()),
                Ok(_) => None,
                Err(_gone) => Some(TerminalsError::NotFound(handle.id())),
            },
            TerminalPlace::Project { project, .. } => (!self.workspaces.has_project(*project))
                .then(|| TerminalsError::NotFound(handle.id())),
            // The start held the worktree from removal; a folder has no
            // owner that could go away.
            TerminalPlace::Worktree { .. } | TerminalPlace::Folder { .. } => None,
        };
        let Some(refusal) = refusal else {
            return Ok(());
        };
        handle.kill();
        let _ended = tokio::time::timeout(CLOSE_WAIT, handle.ended()).await;
        self.terminals.forget(handle.id());
        Err(refusal)
    }

    /// Hold the worktree `folder` is in, if any, from removal, setup and
    /// teardown until the terminal starting there is registered (ADR 0030's
    /// count, protocol 11): a removal either sees the start, or claimed the
    /// worktree first and the start is refused here, before anything runs.
    pub(in crate::service) fn hold_folder(
        &self,
        folder: &std::path::Path,
    ) -> Result<Option<crate::workspace::Starting>, TerminalsError> {
        self.workspaces
            .hold_folder(folder)
            .map_err(|error| TerminalsError::Wire(error.wire()))
    }
}

/// `path` as a folder place keeps it: absolute, existing, canonical.
pub(super) async fn canonical_folder(path: &std::path::Path) -> Result<PathBuf, TerminalsError> {
    if !path.is_absolute() {
        return Err(TerminalsError::Invalid(
            "a folder place is an absolute path".to_owned(),
        ));
    }
    let asked = path.to_path_buf();
    let found = tokio::task::spawn_blocking(move || {
        std::fs::canonicalize(&asked)
            .ok()
            .filter(|folder| folder.is_dir())
    })
    .await
    .ok()
    .flatten();
    found.ok_or_else(|| TerminalsError::Conflict(format!("the folder is gone: {}", path.display())))
}

/// A start beside an archived session: refused, since archiving ends what
/// runs beside it (the join of wave 2's first half).
pub(in crate::service) fn archived() -> TerminalsError {
    TerminalsError::Conflict(
        "the session is archived; bring it back (`unarchive`) to open terminals or run scripts \
         beside it"
            .to_owned(),
    )
}
