//! A session started in a new worktree, walked to its end on its own task
//! (the page's `features/compose/start-worktree.ts`, made detached as the
//! references are: the session's screen opens at once and shows these
//! steps; the session is created when the worktree is ready).
//!
//! [Decision Log]
//! - Purpose: the owner's complaint about the page was that Send held the
//!   person on the new-session screen through the worktree, the approval
//!   and setup; GOAT shows the thread at once and prepares the worktree in
//!   it. The node's calls allow that as they are: `worktree.create`,
//!   `project.approve`, `worktree.output` and `session.create` are separate
//!   calls, and nothing ties them to one screen.
//! - Constraint: an approval is a person's choice, never automatic; a
//!   session in a worktree whose setup runs is refused (`conflict`).
//! - Choice: one task per start, keyed ([`StartKey`]), sending each step
//!   as an [`Answer::Start`] to whoever shows it and waiting on a channel
//!   for the person's answers ([`StartAnswer`]). Dropping the channel's
//!   sender stops the walk at its next step: nothing more is asked and no
//!   session is started; a worktree already made stays, with no session in
//!   it (Settings > Projects lists it).
//! - Cost: the session's id exists only at the end, so the screen that
//!   shows a start keys it by the start, not by a session.

use std::time::Instant;

use tokio::sync::mpsc;
use vessel_app_view::Card;
use vessel_app_view::workspace::command_changes::{Listed, compare_approved, needs_folder};
use vessel_app_view::workspace::pace::{QUIET, quiet_left};
use vessel_app_view::workspace::place::{Place, ProjectPlace, place_spec, worktree_branch};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::setup_log::OutputDecoder;
use vessel_app_view::workspace::start::{
    ApprovalAnswer, FailedAnswer, Refusal, Step, TRIES, branch_of, setup_ended,
};
use vessel_client::NodeClient;
use vessel_proto::workspace::{
    ApproveCommands, CreateWorktree, CreateWorktreeOutcome, GetProjectCommands, ProjectCommands,
    ReadSetupOutput, RunSetup, SessionPlace, SetupChoice, SetupState, WorktreeInfo,
};
use vessel_proto::{ContentBlock, Delivery, SessionSpec};

use super::calls::{call, problem};
use super::{Answer, Reply};

/// The longest a read of setup's output waits for more, in seconds (the
/// node's own bound).
const OUTPUT_WAIT_S: u32 = 30;

/// Which start a step belongs to (the composer's count; one per Send).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StartKey(pub u64);

/// A start, as the composer hands it over.
#[derive(Debug)]
pub struct StartAsk {
    /// Its key.
    pub key: StartKey,
    /// Where: a project, in its folder as it is or in a new worktree.
    pub place: ProjectPlace,
    /// The session's spec, but for its place (filled here).
    pub spec: SessionSpec,
    /// The first prompt.
    pub prompt: String,
    /// The person's answers; dropping the sender stops the walk.
    pub answers: mpsc::Receiver<StartAnswer>,
}

/// A person's answer to a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartAnswer {
    /// To an approval.
    Approval(ApprovalAnswer),
    /// To a failed setup.
    Failed(FailedAnswer),
}

/// How a start ended.
#[derive(Debug)]
pub enum StartEnded {
    /// The session runs: open it (boxed: a card is large beside the other
    /// endings).
    Started(Box<Card>),
    /// Stopped by the person, by leaving, or after a refusal shown.
    Stopped,
    /// A call failed.
    Failed(Problem),
}

/// The walk's own state.
struct Walk<'a> {
    client: &'a NodeClient,
    reply: &'a Reply,
    key: StartKey,
    answers: mpsc::Receiver<StartAnswer>,
}

impl Walk<'_> {
    async fn show(&self, step: Step) {
        self.reply
            .send(Answer::Start {
                key: self.key,
                step,
            })
            .await;
    }

    /// The person's answer to an approval (`None`: they left).
    async fn approval(&mut self) -> Option<ApprovalAnswer> {
        loop {
            match self.answers.recv().await? {
                StartAnswer::Approval(answer) => return Some(answer),
                StartAnswer::Failed(_) => {}
            }
        }
    }

    /// The person's answer to a failed setup (`None`: they left).
    async fn failed(&mut self) -> Option<FailedAnswer> {
        loop {
            match self.answers.recv().await? {
                StartAnswer::Failed(answer) => return Some(answer),
                StartAnswer::Approval(_) => {}
            }
        }
    }

    fn left(&self) -> bool {
        self.answers.is_closed()
    }
}

/// Walk `start` to its end, then send how it ended.
pub(super) async fn walk(client: &NodeClient, reply: &Reply, start: StartAsk) {
    let key = start.key;
    let mut walk = Walk {
        client,
        reply,
        key,
        answers: start.answers,
    };
    // A project's folder as it is needs no worktree: the session at once.
    let place = Place::Project(start.place.clone());
    let made = if let Some(part) = place_spec(&place) {
        Ok(part.applied(start.spec))
    } else {
        match make(&mut walk, &start.place).await {
            Made::Ready(worktree) => {
                let mut spec = start.spec;
                spec.cwd = None;
                Ok(spec.with_place(SessionPlace::Worktree { worktree }))
            }
            Made::Stopped => Err(StartEnded::Stopped),
            Made::Failed(problem) => Err(StartEnded::Failed(problem)),
        }
    };
    let ended = match made {
        Ok(spec) => begin(client, &reply.machine, spec, start.prompt)
            .await
            .map_or_else(StartEnded::Failed, |card| {
                StartEnded::Started(Box::new(card))
            }),
        Err(ended) => ended,
    };
    reply.send(Answer::StartEnded { key, ended }).await;
}

use vessel_app_view::workspace::start::Made;

/// Make the worktree and set it up, asking the person where they decide.
async fn make(walk: &mut Walk<'_>, place: &ProjectPlace) -> Made {
    let Ok(Some(branch)) = worktree_branch(&Place::Project(place.clone())) else {
        return Made::Stopped;
    };
    let mut setup = place.setup;
    let mut commands: Vec<String> = Vec::new();
    loop {
        if walk.left() {
            return Made::Stopped;
        }
        walk.show(Step::Making).await;
        let made = call(
            walk.client,
            CreateWorktree {
                project: place.project,
                branch: branch.clone(),
                setup,
            },
        )
        .await;
        let outcome = match made {
            Ok(outcome) => outcome,
            Err(problem) => return Made::Failed(problem),
        };
        let listed = match outcome {
            CreateWorktreeOutcome::Created { worktree, .. } => {
                return set_up(walk, worktree, commands).await;
            }
            CreateWorktreeOutcome::Refused { reason, .. } => {
                walk.show(Step::Refused(Refusal::Worktree(reason))).await;
                return Made::Stopped;
            }
            // No `..`: what a person approves is named whole (see
            // `Listed::of`); a field the node adds must reach the screen.
            CreateWorktreeOutcome::NeedsApproval { commands } => commands,
        };
        let Some(asked) = Listed::of(&listed) else {
            // Only a listed set needs an approval; anything else is a
            // refusal the person cannot act on here.
            walk.show(Step::Refused(Refusal::CommandsUnreadable)).await;
            return Made::Stopped;
        };
        // The folder's set is read only from a node that does not say
        // which set was approved last.
        let folder: Option<ProjectCommands> = if needs_folder(&asked) {
            call(
                walk.client,
                GetProjectCommands {
                    project: place.project,
                    base: None,
                },
            )
            .await
            .ok()
        } else {
            None
        };
        let compared = compare_approved(&asked, folder.as_ref());
        let digest = asked.digest.clone();
        let setup_commands: Vec<String> = asked.setup.iter().map(ToString::to_string).collect();
        walk.show(Step::Approval {
            commands: listed.clone(),
            compared: compared.clone(),
            busy: false,
        })
        .await;
        match walk.approval().await {
            None => return Made::Stopped,
            Some(ApprovalAnswer::Cancel) => {
                walk.show(Step::Idle).await;
                return Made::Stopped;
            }
            Some(ApprovalAnswer::Skip) => {
                setup = SetupChoice::Skip;
            }
            Some(ApprovalAnswer::Approve) => {
                walk.show(Step::Approval {
                    commands: listed,
                    compared,
                    busy: true,
                })
                .await;
                let approved = call(
                    walk.client,
                    ApproveCommands {
                        project: place.project,
                        digest,
                        approved: true,
                    },
                )
                .await;
                if let Err(problem) = approved {
                    return Made::Failed(problem);
                }
                commands = setup_commands;
            }
        }
    }
}

/// Follow a made worktree's setup to its end; on a failure, ask.
async fn set_up(walk: &mut Walk<'_>, mut info: WorktreeInfo, commands: Vec<String>) -> Made {
    loop {
        let ended = match output(walk, &info, &commands).await {
            Followed::Left => return Made::Stopped,
            Followed::Lost(problem) => {
                walk.show(Step::Refused(Refusal::NotFollowed {
                    branch: branch_of(&info),
                    problem,
                }))
                .await;
                return Made::Stopped;
            }
            Followed::Ended(setup) => setup,
        };
        if !matches!(ended, SetupState::Failed { .. }) {
            return Made::Ready(info.id);
        }
        match walk.failed().await {
            None => return Made::Stopped,
            Some(FailedAnswer::Start) => return Made::Ready(info.id),
            Some(FailedAnswer::Stop) => {
                walk.show(Step::Idle).await;
                return Made::Stopped;
            }
            Some(FailedAnswer::Retry) => {
                match call(walk.client, RunSetup { worktree: info.id }).await {
                    Ok(again) => info = again,
                    Err(problem) => return Made::Failed(problem),
                }
            }
        }
    }
}

/// How following setup's output ended.
enum Followed {
    /// Setup no longer runs, and every byte was read.
    Ended(SetupState),
    /// The person left.
    Left,
    /// Reads failed [`TRIES`] times in a row.
    Lost(Problem),
}

/// Follow `info`'s setup output to its end, sending each page.
async fn output(walk: &Walk<'_>, info: &WorktreeInfo, commands: &[String]) -> Followed {
    let mut decoder = OutputDecoder::default();
    let mut from = 0_u64;
    let mut setup = info.setup.clone();
    let mut failures = 0_u32;
    let branch = branch_of(info);
    let shown = |setup: SetupState| Step::Setup {
        branch: branch.clone(),
        setup,
        commands: commands.to_vec(),
    };
    walk.show(shown(setup.clone())).await;
    loop {
        if walk.left() {
            return Followed::Left;
        }
        let asked = Instant::now();
        let page = call(
            walk.client,
            ReadSetupOutput {
                worktree: info.id,
                from,
                wait: Some(OUTPUT_WAIT_S),
            },
        )
        .await;
        let page = match page {
            Ok(page) => page,
            Err(problem) => {
                failures = failures.saturating_add(1);
                if failures >= TRIES {
                    return Followed::Lost(problem);
                }
                tokio::time::sleep(QUIET).await;
                continue;
            }
        };
        failures = 0;
        let bytes = page.bytes.bytes();
        let empty = bytes.is_empty();
        if !empty {
            let text = decoder.text(bytes);
            if !text.is_empty() {
                walk.show(Step::Output(text)).await;
            }
        }
        from = page.next;
        if page.setup != setup {
            setup = page.setup;
            walk.show(shown(setup.clone())).await;
        }
        if setup_ended(&setup, empty) {
            return Followed::Ended(setup);
        }
        if empty && let Some(left) = quiet_left(asked, Instant::now()) {
            tokio::time::sleep(left).await;
        }
    }
}

/// Create the session in `worktree` and send its first prompt.
///
/// The lead's `State::start` does the same for a session without a place;
/// a worktree needs protocol 10, so `session.prompt` is always there.
async fn begin(
    client: &NodeClient,
    machine: &str,
    spec: SessionSpec,
    prompt: String,
) -> Result<Card, Problem> {
    let info = client
        .create_session(spec)
        .await
        .map_err(|error| problem(&error))?;
    let content = vec![ContentBlock::Text { text: prompt }];
    let _delivered = client
        .prompt(info.id, content, Delivery::Start)
        .await
        .map_err(|error| problem(&error))?;
    Ok(Card::from_session(machine, &info))
}
