//! Each ask run to its answer, on its own task.

use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::is_cloning;
use vessel_app_view::workspace::save::save_text;
use vessel_client::NodeClient;
use vessel_proto::workspace::{
    GetStatus, ListBranches, ListProjects, ListSnapshots, ListStashes, ProjectInfo, ReadFile,
    RemoveProject, RunSetup, WriteExpect, WriteFile,
};

use super::calls::call;
use super::{
    Answer, Ask, FileAsk, Form, GitAsk, GitWrite, GitWritten, Hub, PathChange, ProjectAsk, Reply,
    Save, WorktreeAsk, follow, start,
};

/// Run one ask to its answer.
pub(super) async fn run(hub: &Hub, client: &NodeClient, reply: &Reply, ask: Ask) {
    let answer = match ask {
        Ask::CommandAccess(ask) => Answer::CommandAccess(ask.refused(&Problem::Unsent)),
        // Script operations require the actor's SelectedConnection preflight.
        // A raw-client Hub route must never bypass that admission boundary.
        Ask::Scripts(ask) => Answer::Scripts(super::scripts::refused(ask, &Problem::Unsent)),
        Ask::Layout(ask) => Answer::Layout(super::layout::run(client, ask).await),
        Ask::Project(ask) => project(hub, client, reply, ask).await,
        Ask::Worktree(ask) => worktree(client, ask).await,
        Ask::Git(ask) => git_ask(client, ask).await,
        Ask::File(ask) => file(client, ask).await,
        Ask::Start(start) => {
            start::walk(client, reply, *start).await;
            return;
        }
        Ask::Pull(ask) => Answer::Pull(super::pulls::run(client, ask).await),
        Ask::Session(session) => Answer::Session {
            session,
            result: call(client, vessel_proto::GetSession { session })
                .await
                .map(Box::new),
        },
    };
    reply.send(answer).await;
}

async fn project(hub: &Hub, client: &NodeClient, reply: &Reply, ask: ProjectAsk) -> Answer {
    match ask {
        ProjectAsk::List => {
            let result = call(client, ListProjects {}).await;
            if let Ok(list) = &result {
                for cloning in list.projects.iter().filter(|project| is_cloning(project)) {
                    follow::clone_of(hub, client, reply, cloning.clone());
                }
            }
            Answer::Projects(result)
        }
        ProjectAsk::Add(ask) => took(hub, client, reply, Form::Folder, call(client, ask).await),
        ProjectAsk::Clone(ask) => took(hub, client, reply, Form::Clone, call(client, ask).await),
        ProjectAsk::Init(ask) => took(hub, client, reply, Form::New, call(client, ask).await),
        ProjectAsk::Remove(project) => Answer::Forgot {
            project,
            result: call(client, RemoveProject { project }).await.map(drop),
        },
        ProjectAsk::Commands(ask) => Answer::Commands {
            project: ask.project,
            approved: None,
            result: Err(Problem::Unsent),
        },
        ProjectAsk::Approve(ask) => Answer::Commands {
            project: ask.project,
            approved: Some((ask.digest.clone(), ask.approved)),
            result: Err(Problem::Unsent),
        },
    }
}

async fn worktree(client: &NodeClient, ask: WorktreeAsk) -> Answer {
    match ask {
        WorktreeAsk::List(ask) => Answer::Worktrees {
            project: ask.project,
            result: call(client, ask).await,
        },
        WorktreeAsk::Snapshots(project) => Answer::Snapshots {
            project,
            result: call(client, ListSnapshots { project }).await,
        },
        WorktreeAsk::DropSnapshot(ask) => Answer::Dropped {
            result: call(client, ask.clone()).await.map(drop),
            ask,
        },
        WorktreeAsk::RunSetup(worktree) => Answer::SetupStarted {
            worktree,
            result: call(client, RunSetup { worktree }).await,
        },
        WorktreeAsk::Remove(ask) => Answer::Removed {
            result: call(client, ask.clone()).await,
            ask,
        },
    }
}

async fn git_ask(client: &NodeClient, ask: GitAsk) -> Answer {
    match ask {
        GitAsk::Status(root) => Answer::Status {
            root,
            result: call(client, GetStatus { root }).await,
        },
        GitAsk::Changes(ask) => Answer::Changes {
            result: call(client, ask.clone()).await,
            ask,
        },
        GitAsk::Diff(ask) => Answer::Diff {
            result: call(client, ask.clone()).await,
            ask,
        },
        GitAsk::Branches(root) => Answer::Branches {
            root,
            result: call(client, ListBranches { root }).await,
        },
        GitAsk::Log(ask) => Answer::Log {
            result: call(client, ask.clone()).await,
            ask,
        },
        GitAsk::Stashes(root) => Answer::Stashes {
            root,
            result: call(client, ListStashes { root }).await,
        },
        GitAsk::Write(write) => Answer::Git {
            result: git(client, write.clone()).await,
            write,
        },
    }
}

async fn file(client: &NodeClient, ask: FileAsk) -> Answer {
    match ask {
        FileAsk::List(ask) => Answer::Listed {
            result: call(client, ask.clone()).await,
            ask,
        },
        FileAsk::Read(ask) => Answer::Read {
            result: call(client, ask.clone()).await,
            ask,
        },
        FileAsk::Save(save) => saved(client, save).await,
        FileAsk::Find(ask) => Answer::Found {
            result: call(client, ask.clone()).await,
            ask,
        },
        FileAsk::Grep(ask) => Answer::Grepped {
            result: call(client, ask.clone()).await,
            ask,
        },
        FileAsk::Path(change) => Answer::PathChanged {
            result: path_change(client, change.clone()).await,
            change,
        },
    }
}

/// A guarded save, at most one write.
async fn saved(client: &NodeClient, save: Save) -> Answer {
    let saved = save_text(
        |expect, text| {
            call(
                client,
                WriteFile {
                    root: save.root,
                    path: save.path.clone(),
                    text,
                    expect,
                },
            )
        },
        || {
            call(
                client,
                ReadFile {
                    root: save.root,
                    path: save.path.clone(),
                    offset: 0,
                    limit: None,
                },
            )
        },
        save.text,
        save.base.as_ref(),
    )
    .await;
    Answer::Saved {
        root: save.root,
        path: save.path,
        saved,
    }
}

/// A project the node answered with, into an answer; followed when it is
/// being cloned.
fn took(
    hub: &Hub,
    client: &NodeClient,
    reply: &Reply,
    form: Form,
    result: Result<ProjectInfo, Problem>,
) -> Answer {
    if let Ok(project) = &result
        && is_cloning(project)
    {
        follow::clone_of(hub, client, reply, project.clone());
    }
    Answer::Took { form, result }
}

async fn git(client: &NodeClient, write: GitWrite) -> Result<GitWritten, Problem> {
    Ok(match write {
        GitWrite::Stage(ask) => GitWritten::Status(call(client, ask).await?),
        GitWrite::Unstage(ask) => GitWritten::Status(call(client, ask).await?),
        GitWrite::Discard(ask) => GitWritten::Status(call(client, ask).await?),
        GitWrite::Commit(ask) => GitWritten::Commit(call(client, ask).await?),
        GitWrite::Fetch(ask) => GitWritten::Fetch(call(client, ask).await?),
        GitWrite::Push(ask) => GitWritten::Push(call(client, ask).await?),
        GitWrite::Pull(ask) => GitWritten::Pull(call(client, ask).await?),
        GitWrite::Switch(ask) => GitWritten::Switch(call(client, ask).await?),
        GitWrite::RenameBranch(ask) => GitWritten::Rename(call(client, ask).await?),
        GitWrite::Stash(ask) => GitWritten::Stash(call(client, ask).await?),
    })
}

async fn path_change(client: &NodeClient, change: PathChange) -> Result<(), Problem> {
    match change {
        PathChange::CreateDir(ask) => call(client, ask).await.map(drop),
        PathChange::Rename(ask) => call(client, ask).await.map(drop),
        PathChange::Delete(ask) => call(client, ask).await.map(drop),
        PathChange::CreateFile { root, path } => call(
            client,
            WriteFile {
                root,
                path,
                text: String::new(),
                expect: WriteExpect::Absent,
            },
        )
        .await
        .map(drop),
    }
}
