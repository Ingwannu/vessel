//! One project in Settings > Projects, under the list (the page's
//! `project-detail.tsx`, `project-commands.tsx`, `project-snapshots.tsx`):
//! its folder and default branch; the commands its `vessel.toml` names,
//! and whether that set is approved, with Review (the approval, every
//! command shown) or Withdraw; its worktrees, each removable (by the node's
//! rule, then the person's choice when work would be lost) and set up
//! again; its snapshots, each dropped only after a second press. What the
//! node refuses is said in words above the lists until dismissed.

use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{Tokens, button};
use vessel_app_link::workspace::{Ask, WorktreeAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::{
    DropSnapshot, IfLost, ListWorktrees, ProjectInfo, RemoveWorktree, RemoveWorktreeOutcome,
    SetupState, Snapshot, SnapshotList, WorktreeInfo, WorktreeList,
};
use vessel_proto::{ProjectId, WorktreeId};

use super::approval;
use super::remove::{self, Removing, branch_name};
use super::widgets::{banner, divided, empty, fact, row, rows, section, title};
use crate::shell::Shell;

/// An open project's lists and what is being done to them.
pub(crate) struct Detail {
    /// Which project.
    pub(crate) project: ProjectId,
    worktrees: Vec<WorktreeInfo>,
    snapshots: Vec<Snapshot>,
    /// Whether the approval is open.
    pub(super) reviewing: bool,
    /// A removal the node answered `kept`.
    pub(super) removing: Option<Removing>,
    /// The snapshot whose Drop was pressed once.
    armed: Option<WorktreeId>,
    /// The snapshot being dropped.
    dropping: Option<WorktreeId>,
    /// What the node refused, until dismissed.
    failure: Option<String>,
}

impl Detail {
    pub(crate) const fn new(project: ProjectId) -> Self {
        Self {
            project,
            worktrees: Vec::new(),
            snapshots: Vec::new(),
            reviewing: false,
            removing: None,
            armed: None,
            dropping: None,
            failure: None,
        }
    }
}

impl Shell {
    /// The open detail of `project`, if it is the one open.
    fn detail_of(&mut self, project: ProjectId) -> Option<&mut Detail> {
        self.areas
            .workspace
            .page
            .detail
            .as_mut()
            .filter(|detail| detail.project == project)
    }

    /// The open detail of the project `worktree` belongs to.
    fn detail_with(&mut self, worktree: WorktreeId) -> Option<&mut Detail> {
        self.areas.workspace.page.detail.as_mut().filter(|detail| {
            detail.worktrees.iter().any(|known| known.id == worktree)
                || detail
                    .removing
                    .as_ref()
                    .is_some_and(|removing| removing.worktree.id == worktree)
        })
    }

    pub(super) fn project_worktrees(
        &mut self,
        _machine: &str,
        project: Option<ProjectId>,
        result: Result<WorktreeList, Problem>,
    ) {
        let Some(detail) = project.and_then(|project| self.detail_of(project)) else {
            return;
        };
        if let Ok(list) = result {
            detail.worktrees = list.worktrees;
        }
    }

    pub(super) fn project_snapshots(
        &mut self,
        _machine: &str,
        project: ProjectId,
        result: Result<SnapshotList, Problem>,
    ) {
        if let Some(detail) = self.detail_of(project)
            && let Ok(list) = result
        {
            detail.snapshots = list.snapshots;
        }
    }

    pub(super) fn snapshot_dropped(
        &mut self,
        machine: &str,
        ask: &DropSnapshot,
        result: Result<(), Problem>,
    ) {
        let words = self.words();
        let Some(detail) = self.detail_of(ask.project) else {
            return;
        };
        detail.dropping = None;
        detail.armed = None;
        if let Err(problem) = result {
            let name = detail
                .snapshots
                .iter()
                .find(|snapshot| snapshot.worktree == ask.worktree)
                .map_or_else(String::new, snapshot_name);
            detail.failure = Some(words.snapshot_not_dropped(&name, &words.problem(&problem)));
        }
        self.ask(machine, Ask::Worktree(WorktreeAsk::Snapshots(ask.project)));
    }

    pub(super) fn setup_started(
        &mut self,
        machine: &str,
        worktree: WorktreeId,
        result: Result<WorktreeInfo, Problem>,
    ) {
        let words = self.words();
        let Some(detail) = self.detail_with(worktree) else {
            return;
        };
        let project = detail.project;
        match result {
            Ok(info) => {
                if let Some(known) = detail
                    .worktrees
                    .iter_mut()
                    .find(|known| known.id == info.id)
                {
                    *known = info;
                }
            }
            Err(problem) => {
                detail.failure = Some(words.setup_not_started(&words.problem(&problem)));
            }
        }
        self.ask(
            machine,
            Ask::Worktree(WorktreeAsk::List(ListWorktrees {
                project: Some(project),
            })),
        );
    }

    pub(super) fn worktree_removed(
        &mut self,
        machine: &str,
        ask: &RemoveWorktree,
        result: Result<RemoveWorktreeOutcome, Problem>,
    ) {
        let words = self.words();
        let labels = words.labels();
        let Some(detail) = self.detail_with(ask.worktree) else {
            return;
        };
        let project = detail.project;
        let info = detail
            .worktrees
            .iter()
            .find(|known| known.id == ask.worktree)
            .cloned()
            .or_else(|| {
                detail
                    .removing
                    .as_ref()
                    .map(|removing| removing.worktree.clone())
            });
        let branch = info
            .as_ref()
            .map_or_else(String::new, |info| branch_name(info, labels.detached));
        let mut notice = None;
        match result {
            Ok(RemoveWorktreeOutcome::Removed { snapshot, .. }) => {
                detail.removing = None;
                let short = snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.commit.as_str().chars().take(7).collect::<String>());
                notice = Some(words.removed(&branch, short.as_deref()));
            }
            Ok(RemoveWorktreeOutcome::Kept { reasons, .. }) => {
                let error = match ask.if_lost {
                    IfLost::Keep => None,
                    IfLost::Snapshot => Some(labels.snapshot_refused.to_owned()),
                    IfLost::Discard => Some(labels.nothing_removed.to_owned()),
                };
                if let Some(worktree) = info {
                    detail.removing = Some(Removing {
                        worktree,
                        reasons,
                        busy: None,
                        error,
                    });
                }
            }
            Err(problem) => {
                let why = words.problem(&problem);
                match detail.removing.as_mut() {
                    Some(removing) => {
                        removing.busy = None;
                        removing.error = Some(why);
                    }
                    None => detail.failure = Some(words.not_removed(&branch, &why)),
                }
            }
        }
        if notice.is_some() {
            self.notice = notice;
        }
        self.ask(
            machine,
            Ask::Worktree(WorktreeAsk::List(ListWorktrees {
                project: Some(project),
            })),
        );
        self.ask(machine, Ask::Worktree(WorktreeAsk::Snapshots(project)));
    }

    fn worktree_action(&mut self, ask: WorktreeAsk, cx: &mut Context<'_, Self>) {
        if let Some(machine) = self.projects_machine() {
            self.ask(&machine, Ask::Worktree(ask));
        }
        cx.notify();
    }
}

/// A snapshot's name: its branch, else its commit's first seven.
fn snapshot_name(snapshot: &Snapshot) -> String {
    snapshot.branch.as_ref().map_or_else(
        || snapshot.commit.as_str().chars().take(7).collect(),
        ToString::to_string,
    )
}

/// The detail of `project`.
pub(super) fn render(
    shell: &Shell,
    machine: &str,
    project: &ProjectInfo,
    detail: &Detail,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let local = approval::local();
    let added = local.when(project.created_at).1;
    let branch = if project.git {
        project
            .default_branch
            .as_ref()
            .map_or_else(|| labels.none_yet.to_owned(), ToString::to_string)
    } else {
        labels.not_git_repository.to_owned()
    };
    let facts = rows(tokens)
        .child(divided(
            row(
                title(labels.folder, tokens),
                &[],
                Some(div().child(fact(project.root.display().to_string(), true, tokens))),
                tokens,
            ),
            0,
            tokens,
        ))
        .child(divided(
            row(
                title(labels.default_branch, tokens),
                &[],
                Some(div().child(fact(branch, project.git, tokens))),
                tokens,
            ),
            1,
            tokens,
        ))
        .child(divided(
            row(
                title(labels.added, tokens),
                &[],
                Some(div().child(fact(added, false, tokens))),
                tokens,
            ),
            2,
            tokens,
        ));
    let id = project.id;
    let remove = div().child(
        button("project-remove", labels.remove_project, false, tokens)
            .text_color(tokens.neutral.destructive)
            .on_click(cx.listener(move |this, _event, _window, cx| this.forget_project(id, cx))),
    );
    let mut column = div().flex().flex_col().child(section(
        project.name.to_string(),
        Some(remove),
        facts,
        tokens,
    ));
    if let Some(failure) = &detail.failure {
        column = column.child(banner(
            failure.clone(),
            true,
            Some(div().child(
                button("detail-dismiss", labels.dismiss, false, tokens).on_click(cx.listener(
                    |this, _event, _window, cx| {
                        if let Some(detail) = this.areas.workspace.page.detail.as_mut() {
                            detail.failure = None;
                        }
                        cx.notify();
                    },
                )),
            )),
            tokens,
        ));
    }
    column = column.child(section(
        labels.commands,
        None,
        super::detail_commands::render(shell, machine, project, detail, tokens, cx),
        tokens,
    ));
    column = column.child(super::scripts::render(
        shell, machine, project.id, tokens, cx,
    ));
    if project.git {
        column = column.child(section(
            labels.worktrees,
            None,
            worktrees(shell, detail, tokens, cx),
            tokens,
        ));
    }
    if !detail.snapshots.is_empty() {
        column = column.child(section(
            labels.snapshots,
            None,
            snapshots(shell, project.id, detail, tokens, cx),
            tokens,
        ));
    }
    column
}

fn worktrees(shell: &Shell, detail: &Detail, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words();
    let labels = words.labels();
    if detail.worktrees.is_empty() {
        return empty(labels.no_worktrees, tokens);
    }
    let mut list = rows(tokens);
    for (index, worktree) in detail.worktrees.iter().enumerate() {
        let mut facts = vec![
            words.setup_state(&worktree.setup).to_owned(),
            if worktree.sessions.is_empty() {
                labels.no_live_session.to_owned()
            } else {
                words.live(worktree.sessions.len())
            },
        ];
        if let Some(kept) = worktree.kept {
            facts.push(words.kept(&words.kept_reason(kept)));
        }
        let id = worktree.id;
        let mut controls = div();
        if matches!(
            worktree.setup,
            SetupState::Failed { .. } | SetupState::Skipped
        ) {
            controls = controls.child(
                button(
                    ElementId::Name(format!("run-setup-{index}").into()),
                    labels.run_setup,
                    false,
                    tokens,
                )
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.worktree_action(WorktreeAsk::RunSetup(id), cx);
                })),
            );
        }
        controls = controls.child(
            button(
                ElementId::Name(format!("remove-worktree-{index}").into()),
                labels.remove,
                false,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.worktree_action(
                    WorktreeAsk::Remove(RemoveWorktree {
                        worktree: id,
                        if_lost: IfLost::Keep,
                    }),
                    cx,
                );
            })),
        );
        list = list.child(divided(
            row(
                title(branch_name(worktree, labels.detached), tokens),
                &facts,
                Some(controls),
                tokens,
            ),
            index,
            tokens,
        ));
    }
    let mut column = div().flex().flex_col().gap(px(10.0)).child(list);
    if let Some(removing) = &detail.removing {
        column = column.child(remove::render(shell, removing, tokens, cx));
    }
    column
}

fn snapshots(
    shell: &Shell,
    project: ProjectId,
    detail: &Detail,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let local = approval::local();
    let mut list = rows(tokens);
    for (index, snapshot) in detail.snapshots.iter().enumerate() {
        let worktree = snapshot.worktree;
        let facts = vec![
            snapshot.commit.as_str().chars().take(7).collect(),
            local.when(snapshot.created_at).1,
        ];
        let busy = detail.dropping.is_some();
        let controls = if detail.armed == Some(worktree) {
            div()
                .child(
                    button(
                        ElementId::Name(format!("snapshot-keep-{index}").into()),
                        labels.keep,
                        false,
                        tokens,
                    )
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        if let Some(detail) = this.areas.workspace.page.detail.as_mut() {
                            detail.armed = None;
                        }
                        cx.notify();
                    })),
                )
                .child(
                    button(
                        ElementId::Name(format!("snapshot-drop-{index}").into()),
                        labels.drop_for_good,
                        true,
                        tokens,
                    )
                    .bg(tokens.neutral.destructive)
                    .opacity(if busy { 0.5 } else { 1.0 })
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        if let Some(detail) = this.areas.workspace.page.detail.as_mut()
                            && detail.dropping.is_none()
                        {
                            detail.dropping = Some(worktree);
                            this.worktree_action(
                                WorktreeAsk::DropSnapshot(DropSnapshot { project, worktree }),
                                cx,
                            );
                        }
                    })),
                )
        } else {
            div().child(
                button(
                    ElementId::Name(format!("snapshot-arm-{index}").into()),
                    labels.drop,
                    false,
                    tokens,
                )
                .text_color(tokens.neutral.destructive)
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    if let Some(detail) = this.areas.workspace.page.detail.as_mut()
                        && detail.dropping.is_none()
                    {
                        detail.armed = Some(worktree);
                    }
                    cx.notify();
                })),
            )
        };
        list = list.child(divided(
            row(
                title(snapshot_name(snapshot), tokens),
                &facts,
                Some(controls),
                tokens,
            ),
            index,
            tokens,
        ));
    }
    list
}
