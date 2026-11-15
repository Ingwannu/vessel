//! `vessel project …`, `vessel worktree …`, and where `vessel run` starts
//! a session (protocol 10, ADR 0027). Projects and worktrees are named by
//! id, a unique prefix of one, or (projects) their name.

use std::fmt::Write as _;
use std::time::Duration;

use vessel_client::NodeClient;
use vessel_proto::workspace::{
    AddProject, ApproveCommands, CloneProject, CreateWorktree, CreateWorktreeOutcome, GetProject,
    GetProjectCommands, GitSupport, InitProject, ListProjects, ListSnapshots, ListWorktrees,
    ProjectInfo, ProjectState, ReadSetupOutput, RemoveProject, RemoveWorktree, RunSetup,
    SessionPlace, SetupChoice, SetupState, WorktreeBase, WorktreeBranch, WorktreeInfo,
};
use vessel_proto::{ProjectId, WorktreeId};

use crate::args::{ProjectCommand, Target, WorktreeCommand};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;
/// Preserve the shared CLI rendering entry point while its implementation has its own module.
pub(crate) use crate::project_commands::text as commands_text;

/// One long poll's wait, under common 60 s idle limits.
const POLL_SECS: u32 = 30;

pub(crate) async fn project(target: &Target, command: ProjectCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        ProjectCommand::Create { .. }
        | ProjectCommand::Rename { .. }
        | ProjectCommand::Delete { .. } => Err(CliError::Usage(
            "registry commands are handled by the typed project adapter",
        )),
        ProjectCommand::List => {
            let list = client.call(ListProjects {}).await?;
            let mut text = format!("git: {}\n", git_line(list.git));
            for project in &list.projects {
                text.push_str(&project_line(project));
            }
            if list.more {
                text.push_str("(more not shown)\n");
            }
            out(&text).await
        }
        ProjectCommand::Add { path, name } => {
            let name = name.map(|name| parse(&name, "name")).transpose()?;
            let added = client.call(AddProject { path, name }).await?;
            out(&project_line(&added)).await
        }
        ProjectCommand::Clone {
            url,
            parent,
            folder,
            no_wait,
        } => {
            let request = CloneProject {
                url: parse(&url, "url")?,
                parent,
                folder: folder.map(|folder| parse(&folder, "folder")).transpose()?,
                name: None,
            };
            let mut project = client.call(request).await?;
            out(&project_line(&project)).await?;
            if no_wait {
                return Ok(());
            }
            while matches!(project.state, ProjectState::Cloning { .. }) {
                project = client
                    .call(GetProject {
                        project: project.id,
                        changed_from: Some(project.revision),
                    })
                    .await?;
                io::note(&format!("[vessel] {}\n", state_text(&project.state))).await?;
            }
            out(&project_line(&project)).await
        }
        ProjectCommand::Init { path } => {
            let made = client.call(InitProject { path, name: None }).await?;
            out(&project_line(&made)).await
        }
        ProjectCommand::Remove { project } => {
            let project = find_project(&client, &project).await?;
            let _ack = client
                .call(RemoveProject {
                    project: project.id,
                })
                .await?;
            out(&format!("forgot {} (its files stay)\n", project.name)).await
        }
        ProjectCommand::Commands { project } => {
            let project = find_project(&client, &project).await?;
            let commands = client
                .call(GetProjectCommands {
                    project: project.id,
                    base: None,
                })
                .await?;
            out(&commands_text(&commands)).await
        }
        ProjectCommand::Approve {
            project,
            digest,
            withdraw,
        } => {
            let project = find_project(&client, &project).await?;
            let commands = client
                .call(ApproveCommands {
                    project: project.id,
                    digest: parse(&digest, "digest")?,
                    approved: !withdraw,
                })
                .await?;
            out(&commands_text(&commands)).await
        }
    }
}

pub(crate) async fn worktree(target: &Target, command: WorktreeCommand) -> Result<(), CliError> {
    command.require_contract()?;
    let client = connect(target).await?.client;
    match command {
        WorktreeCommand::Ls { .. }
        | WorktreeCommand::Create(_)
        | WorktreeCommand::Archive { .. } => Err(CliError::ParityBlocked(
            "worktree source/archival awaits the canonical workspace registry",
        )),
        WorktreeCommand::List { project, cwd: _cwd } => {
            let project = match project {
                Some(project) => Some(find_project(&client, &project).await?.id),
                None => None,
            };
            let list = client.call(ListWorktrees { project }).await?;
            let text: String = list.worktrees.iter().map(worktree_line).collect();
            out(&text).await
        }
        WorktreeCommand::New {
            project,
            branch,
            existing,
            base,
            skip_setup,
            no_wait,
        } => {
            let project = find_project(&client, &project).await?;
            let branch = if existing {
                WorktreeBranch::Existing {
                    name: parse(branch.as_deref().unwrap_or_default(), "branch")?,
                }
            } else {
                WorktreeBranch::New {
                    name: branch.map(|name| parse(&name, "branch")).transpose()?,
                    base: base
                        .map(|base| parse(&base, "base").map(|name| WorktreeBase::Ref { name }))
                        .transpose()?,
                }
            };
            let setup = if skip_setup {
                SetupChoice::Skip
            } else {
                SetupChoice::Run
            };
            let worktree = new_worktree(&client, project.id, branch, setup).await?;
            out(&worktree_line(&worktree)).await?;
            if no_wait {
                return Ok(());
            }
            let state = follow(&client, worktree.id).await?;
            out(&format!("setup: {}\n", setup_text(&state))).await
        }
        WorktreeCommand::Output {
            worktree,
            follow: keep,
        } => {
            let worktree = find_worktree(&client, &worktree).await?;
            if keep {
                let state = follow(&client, worktree.id).await?;
                return out(&format!("setup: {}\n", setup_text(&state))).await;
            }
            let mut from = 0;
            loop {
                let page = client
                    .call(ReadSetupOutput {
                        worktree: worktree.id,
                        from,
                        wait: None,
                    })
                    .await?;
                io::out(page.bytes.bytes()).await?;
                if page.next == from {
                    return Ok(());
                }
                from = page.next;
            }
        }
        WorktreeCommand::Setup { worktree } => {
            let worktree = find_worktree(&client, &worktree).await?;
            let _running = client
                .call(RunSetup {
                    worktree: worktree.id,
                })
                .await?;
            let state = follow(&client, worktree.id).await?;
            out(&format!("setup: {}\n", setup_text(&state))).await
        }
        WorktreeCommand::Remove {
            worktree,
            snapshot,
            discard,
        } => remove(&client, &worktree, snapshot, discard).await,
        WorktreeCommand::Snapshots { project } => snapshots(&client, &project).await,
    }
}

/// `vessel run`'s place: `--project` (with `--dir`), `--worktree`, or
/// `--project --new-worktree` (made and set up first, as the page and the
/// agent tool do: the three steps of ADR 0027).
pub(crate) async fn place(
    client: &NodeClient,
    project: Option<&str>,
    dir: Option<&str>,
    new_worktree: bool,
    worktree: Option<&str>,
) -> Result<Option<SessionPlace>, CliError> {
    if let Some(worktree) = worktree {
        let worktree = find_worktree(client, worktree).await?;
        return Ok(Some(SessionPlace::Worktree {
            worktree: worktree.id,
        }));
    }
    let Some(project) = project else {
        return Ok(None);
    };
    let project = find_project(client, project).await?;
    if new_worktree {
        let branch = WorktreeBranch::New {
            name: None,
            base: None,
        };
        let worktree = new_worktree_for(client, project.id, branch).await?;
        let state = follow(client, worktree.id).await?;
        io::note(&format!("[vessel] setup: {}\n", setup_text(&state))).await?;
        return Ok(Some(SessionPlace::Worktree {
            worktree: worktree.id,
        }));
    }
    Ok(Some(SessionPlace::Project {
        project: project.id,
        dir: dir.map(|dir| parse(dir, "dir")).transpose()?,
    }))
}

async fn remove(
    client: &NodeClient,
    worktree: &str,
    snapshot: bool,
    discard: bool,
) -> Result<(), CliError> {
    let worktree = find_worktree(client, worktree).await?;
    let if_lost = match (snapshot, discard) {
        (true, _) => vessel_proto::workspace::IfLost::Snapshot,
        (false, true) => vessel_proto::workspace::IfLost::Discard,
        (false, false) => vessel_proto::workspace::IfLost::Keep,
    };
    let outcome = client
        .call(RemoveWorktree {
            worktree: worktree.id,
            if_lost,
        })
        .await?;
    out(&crate::render::removal(&outcome)).await
}

async fn snapshots(client: &NodeClient, project: &str) -> Result<(), CliError> {
    let project = find_project(client, project).await?;
    let list = client
        .call(ListSnapshots {
            project: project.id,
        })
        .await?;
    let mut text = String::new();
    for snapshot in &list.snapshots {
        let _written = writeln!(
            text,
            "{}  {}  {}  {}",
            snapshot.worktree,
            snapshot.commit,
            snapshot
                .branch
                .as_ref()
                .map_or("-", |branch| branch.as_str()),
            snapshot.created_at.format("%Y-%m-%d %H:%M")
        );
    }
    out(&text).await
}

async fn new_worktree_for(
    client: &NodeClient,
    project: ProjectId,
    branch: WorktreeBranch,
) -> Result<WorktreeInfo, CliError> {
    new_worktree(client, project, branch, SetupChoice::Run).await
}

async fn new_worktree(
    client: &NodeClient,
    project: ProjectId,
    branch: WorktreeBranch,
    setup: SetupChoice,
) -> Result<WorktreeInfo, CliError> {
    let outcome = client
        .call(CreateWorktree {
            project,
            branch,
            setup,
        })
        .await?;
    match outcome {
        CreateWorktreeOutcome::Created { worktree } => Ok(worktree),
        CreateWorktreeOutcome::NeedsApproval { commands } => Err(CliError::Workspace(format!(
            "the project's setup commands are not approved:\n{}\
             approve them with `vessel project approve <project> <digest>`, or pass \
             --skip-setup",
            commands_text(&commands)
        ))),
        CreateWorktreeOutcome::Refused { reason } => Err(CliError::Workspace(format!(
            "no worktree: {}",
            crate::render::refusal(&reason)
        ))),
    }
}

/// Print a worktree's setup output until setup ends; its state then.
async fn follow(client: &NodeClient, worktree: WorktreeId) -> Result<SetupState, CliError> {
    let mut from = 0;
    loop {
        let page = client
            .call(ReadSetupOutput {
                worktree,
                from,
                wait: Some(POLL_SECS),
            })
            .await?;
        io::note_bytes(page.bytes.bytes()).await?;
        let running = matches!(page.setup, SetupState::Running { .. });
        if !running && page.next == from {
            return Ok(page.setup);
        }
        if running && page.next == from {
            // The node had no room for this wait: pause before the next.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        from = page.next;
    }
}

/// The project `given` names: its id, a unique prefix of one, or its name.
pub(crate) async fn project_id(client: &NodeClient, given: &str) -> Result<ProjectId, CliError> {
    find_project(client, given).await.map(|project| project.id)
}

/// The project `given` names, whole (see [`project_id`]).
pub(crate) async fn find_project(
    client: &NodeClient,
    given: &str,
) -> Result<ProjectInfo, CliError> {
    let projects = client.call(ListProjects {}).await?.projects;
    let matching: Vec<ProjectInfo> = projects
        .into_iter()
        .filter(|project| {
            project.name.as_str() == given || project.id.to_string().starts_with(given)
        })
        .collect();
    one(matching, given, "project")
}

async fn find_worktree(client: &NodeClient, given: &str) -> Result<WorktreeInfo, CliError> {
    let worktrees = client
        .call(ListWorktrees { project: None })
        .await?
        .worktrees;
    let matching: Vec<WorktreeInfo> = worktrees
        .into_iter()
        .filter(|worktree| worktree.id.to_string().starts_with(given))
        .collect();
    one(matching, given, "worktree")
}

pub(crate) fn one<T>(mut matching: Vec<T>, given: &str, what: &str) -> Result<T, CliError> {
    match matching.len() {
        1 => matching
            .pop()
            .ok_or_else(|| CliError::Workspace(format!("no {what} matches `{given}`"))),
        0 => Err(CliError::Workspace(format!("no {what} matches `{given}`"))),
        count => Err(CliError::Workspace(format!(
            "`{given}` matches {count} {what}s; type more of the id"
        ))),
    }
}

pub(crate) fn parse<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, CliError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| CliError::Workspace(format!("{what}: {error}")))
}

pub(crate) async fn out(text: &str) -> Result<(), CliError> {
    Ok(io::out(text.as_bytes()).await?)
}

fn git_line(git: GitSupport) -> String {
    match git {
        GitSupport::Found { version } => format!("{version}"),
        GitSupport::TooOld { version, .. } => {
            format!("{version} (too old: 2.25 or newer is needed)")
        }
        GitSupport::Missing => "not found".to_owned(),
        GitSupport::Unknown => "unknown".to_owned(),
    }
}

fn project_line(project: &ProjectInfo) -> String {
    format!(
        "{}  {}  {}  {}\n",
        project.id,
        project.name,
        state_text(&project.state),
        project.root.display()
    )
}

fn state_text(state: &ProjectState) -> String {
    match state {
        ProjectState::Ready => "ready".to_owned(),
        ProjectState::Cloning { stage, percent } => format!(
            "cloning ({}{})",
            crate::render::clone_stage(*stage),
            percent
                .map(|percent| format!(" {percent}%"))
                .unwrap_or_default()
        ),
        ProjectState::CloneFailed { message, .. } => format!("clone failed: {message}"),
        ProjectState::Missing => "missing".to_owned(),
        ProjectState::Unknown => "unknown".to_owned(),
    }
}

fn worktree_line(worktree: &WorktreeInfo) -> String {
    format!(
        "{}  {}  setup {}  {}{}\n",
        worktree.id,
        worktree
            .branch
            .as_ref()
            .map_or("(detached)", |branch| branch.as_str()),
        setup_text(&worktree.setup),
        worktree.path.display(),
        if worktree.sessions.is_empty() {
            String::new()
        } else {
            format!("  {} session(s)", worktree.sessions.len())
        }
    )
}

fn setup_text(state: &SetupState) -> String {
    match state {
        SetupState::None => "none".to_owned(),
        SetupState::Skipped => "skipped".to_owned(),
        SetupState::Running { .. } => "running".to_owned(),
        SetupState::Succeeded { .. } => "succeeded".to_owned(),
        SetupState::Failed { failure, .. } => {
            format!("failed ({})", crate::render::setup_failure(*failure))
        }
        SetupState::Unknown => "unknown".to_owned(),
    }
}
