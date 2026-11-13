//! `pr.open` and `pr.merge` (ADR 0029): run in the caller's call, never in
//! the watcher, against the forge directly and as a person's requests;
//! the watcher is told to look again once done.
//!
//! [Decision Log]
//! - Purpose: a person (or, for opening, an agent) acts on the pull
//!   request of a session's branch, with what the forge says now, not
//!   what the watcher last read.
//! - Opening: a person's call pushes the branch first (`--set-upstream`,
//!   never forced, the repository's hooks running as for `git.push`); an
//!   agent's never pushes, since the node would run the repository's hooks
//!   and the user's credentials for it: the branch must be on the remote
//!   at the folder's commit already (its remote-tracking ref says so).
//!   An open pull request of that head is `exists`; nothing new is made.
//! - Merging: exact and never automatic. The pull request is read fresh;
//!   it must be the session's, open, not a draft and at the head given;
//!   then the forge merges it pinned to that head, so a push between the
//!   read and the merge is refused by the forge too.

use std::path::PathBuf;
use std::sync::Arc;

use vessel_forge::{Forge, ForgeError, Fresh, Merged, Opened, Opening, Priority, Read};
use vessel_proto::forge::{
    ForgeAccess, MergeOutcome, MergePullRequest, MergeRefusal, OpenOutcome, OpenPullRequest,
    OpenRefusal, PullBody, PullNumber, PullRequest, PullState, RepoRef,
};
use vessel_proto::workspace::PushOutcome;
use vessel_proto::{ErrorCode, ProjectId, SessionId, WireError};

use super::folder::{self, Spot};
use super::hosts::denied;
use super::poll::pick;
use super::{Command, Forges};
use crate::sessions::{Found as SessionFound, Sessions};

/// The session's branch on its forge, ready to act on.
struct Target {
    folder: PathBuf,
    project: ProjectId,
    spot: Spot,
    forge: Arc<dyn Forge>,
}

impl Forges {
    /// Where `session`'s pull request is, or why it cannot be acted on.
    async fn target(
        &self,
        sessions: &Sessions,
        session: SessionId,
    ) -> Result<Result<Target, ForgeAccess>, WireError> {
        let info = match sessions
            .find(session)
            .await
            .map_err(|error| error.to_wire())?
        {
            SessionFound::Live { handle, .. } if !handle.has_ended() => handle.info(),
            SessionFound::Live { .. } | SessionFound::Stored(_) => {
                return Ok(Err(ForgeAccess::Ended));
            }
        };
        let place = info.spec.place.as_deref();
        let project = place.and_then(|place| self.workspaces.project_of(place));
        let (Some(project), Some(folder)) = (project, info.spec.cwd.clone()) else {
            return Ok(Err(ForgeAccess::NotPlaced));
        };
        let spot = match folder::read(&self.workspaces, &folder, project, None)
            .await
            .0
        {
            Ok(spot) => spot,
            Err(why) => return Ok(Err(why)),
        };
        let access = self.hosts.access(&spot.repo.host).await;
        if !matches!(access, ForgeAccess::Ready { .. }) {
            return Ok(Err(access));
        }
        let Some(forge) = self.hosts.forge() else {
            return Ok(Err(ForgeAccess::NotSupported {
                host: spot.repo.host.clone(),
            }));
        };
        Ok(Ok(Target {
            folder,
            project,
            spot,
            forge,
        }))
    }

    fn denied(&self, repo: &RepoRef, error: &ForgeError) -> ForgeAccess {
        denied(&repo.host, error, self.hosts.login(&repo.host))
    }

    /// `pr.open` for a person (`push`) or an agent.
    pub(crate) async fn open(
        &self,
        sessions: &Sessions,
        push: bool,
        request: OpenPullRequest,
    ) -> Result<OpenOutcome, WireError> {
        let refused = |refusal| Ok(OpenOutcome::Refused { refusal });
        let target = match self.target(sessions, request.session).await? {
            Ok(target) => target,
            Err(access) => return refused(OpenRefusal::Access { access }),
        };
        let Target {
            folder,
            project,
            spot,
            forge,
        } = target;
        if push {
            match self.workspaces.push_branch(&folder, &spot.remote).await {
                Ok(PushOutcome::Pushed { .. }) => {}
                Ok(PushOutcome::Refused { refusal }) => {
                    return refused(OpenRefusal::Push { refusal });
                }
                Err(error) => return Err(error.wire()),
            }
        } else {
            let tracked = self
                .workspaces
                .tracking(&folder, &spot.remote, &spot.branch)
                .await
                .map_err(|error| error.wire())?;
            if tracked.as_ref() != Some(&spot.head) {
                return refused(OpenRefusal::NotPushed);
            }
        }
        let repo = &spot.repo;
        let outcome = match self.existing(forge.as_ref(), &spot).await {
            Err(error) => OpenOutcome::Refused {
                refusal: OpenRefusal::Access {
                    access: self.denied(repo, &error),
                },
            },
            Ok(Some(pull)) => OpenOutcome::Exists {
                pull_request: Box::new(pull),
            },
            Ok(None) => {
                let base = request
                    .base
                    .or_else(|| self.workspaces.base_branch(project));
                match base {
                    Some(base) => {
                        let opening = Opening {
                            head: spot.branch.clone(),
                            base,
                            title: request.title,
                            body: request.body.unwrap_or_else(PullBody::empty),
                            draft: request.draft,
                        };
                        self.create(forge.as_ref(), &spot, &opening).await
                    }
                    None => OpenOutcome::Refused {
                        refusal: OpenRefusal::Rejected {
                            message: "the project has no default branch; name the base".to_owned(),
                        },
                    },
                }
            }
        };
        self.tell(Command::Refresh {
            session: request.session,
            person: true,
            reply: None,
        });
        Ok(outcome)
    }

    /// An open pull request of the folder's commit, read whole.
    async fn existing(
        &self,
        forge: &dyn Forge,
        spot: &Spot,
    ) -> Result<Option<PullRequest>, ForgeError> {
        let listed = match forge
            .find(&spot.repo, &spot.branch, Priority::Person)
            .await?
        {
            Fresh::Changed(listed) => listed,
            Fresh::Same => Vec::new(),
        };
        let open = listed
            .iter()
            .filter(|found| found.state == PullState::Open && found.head == spot.head)
            .max_by_key(|found| found.updated_at);
        match open {
            Some(found) => Ok(Some(
                forge
                    .read(&spot.repo, found.number, Priority::Person)
                    .await?
                    .pull,
            )),
            None => Ok(None),
        }
    }

    async fn create(&self, forge: &dyn Forge, spot: &Spot, opening: &Opening) -> OpenOutcome {
        let refused = |refusal| OpenOutcome::Refused { refusal };
        let number = match forge.open(&spot.repo, opening).await {
            Ok(Opened::Created(number)) => number,
            Ok(Opened::Exists) => {
                return match self.existing(forge, spot).await {
                    Ok(Some(pull)) => OpenOutcome::Exists {
                        pull_request: Box::new(pull),
                    },
                    Ok(None) => refused(OpenRefusal::Rejected {
                        message: "the forge says a pull request of this branch exists".to_owned(),
                    }),
                    Err(error) => refused(OpenRefusal::Access {
                        access: self.denied(&spot.repo, &error),
                    }),
                };
            }
            Ok(Opened::NoCommits) => return refused(OpenRefusal::NoCommits),
            Ok(Opened::Rejected(message)) => return refused(OpenRefusal::Rejected { message }),
            Err(error) => {
                return refused(OpenRefusal::Access {
                    access: self.denied(&spot.repo, &error),
                });
            }
        };
        match forge.read(&spot.repo, number, Priority::Person).await {
            Ok(read) => OpenOutcome::Opened {
                pull_request: Box::new(read.pull),
            },
            Err(error) => {
                tracing::debug!(%error, "an opened pull request could not be read back");
                refused(OpenRefusal::Rejected {
                    message: format!(
                        "pull request #{number} was opened, but reading it back failed; \
                         it shows once the node reads it"
                    ),
                })
            }
        }
    }

    /// `pr.merge` (a person's).
    pub(crate) async fn merge(
        &self,
        sessions: &Sessions,
        request: MergePullRequest,
    ) -> Result<MergeOutcome, WireError> {
        if !matches!(request.head.as_str().len(), 40 | 64) {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                "`head` must be a full commit id",
            ));
        }
        let refused = |refusal| Ok(MergeOutcome::Refused { refusal });
        let target = match self.target(sessions, request.session).await? {
            Ok(target) => target,
            Err(access) => return refused(MergeRefusal::Access { access }),
        };
        let Target { spot, forge, .. } = target;
        let read = match self.current(forge.as_ref(), &spot, request.number).await {
            Ok(Ok(read)) => read,
            Ok(Err(refusal)) => return refused(refusal),
            Err(error) => {
                return refused(MergeRefusal::Access {
                    access: self.denied(&spot.repo, &error),
                });
            }
        };
        let summary = &read.pull.summary;
        if summary.state != PullState::Open {
            return refused(MergeRefusal::NotOpen {
                state: summary.state,
            });
        }
        if summary.draft {
            return refused(MergeRefusal::Draft);
        }
        if summary.head_commit != request.head {
            return refused(MergeRefusal::HeadMoved {
                head: summary.head_commit.clone(),
            });
        }
        if !forge.pins_head(request.method) {
            return refused(MergeRefusal::CannotPin);
        }
        if !read.pull.merge_methods.contains(&request.method) {
            return refused(MergeRefusal::MethodNotAllowed);
        }
        let merged = forge
            .merge(&spot.repo, request.number, request.method, &request.head)
            .await;
        let outcome = match merged {
            Ok(Merged::Done(commit)) => MergeOutcome::Merged { commit },
            Ok(Merged::HeadMoved) => {
                let head = match forge
                    .read(&spot.repo, request.number, Priority::Person)
                    .await
                {
                    Ok(read) => read.pull.summary.head_commit,
                    Err(_) => request.head.clone(),
                };
                MergeOutcome::Refused {
                    refusal: MergeRefusal::HeadMoved { head },
                }
            }
            Ok(Merged::NotMergeable(message)) => MergeOutcome::Refused {
                refusal: MergeRefusal::NotMergeable { message },
            },
            Ok(Merged::MethodNotAllowed) => MergeOutcome::Refused {
                refusal: MergeRefusal::MethodNotAllowed,
            },
            Ok(Merged::MergeQueue) => MergeOutcome::Refused {
                refusal: MergeRefusal::MergeQueue,
            },
            Ok(Merged::Rejected(message)) => MergeOutcome::Refused {
                refusal: MergeRefusal::Rejected { message },
            },
            Err(error) => MergeOutcome::Refused {
                refusal: MergeRefusal::Access {
                    access: self.denied(&spot.repo, &error),
                },
            },
        };
        self.tell(Command::Refresh {
            session: request.session,
            person: true,
            reply: None,
        });
        Ok(outcome)
    }

    /// The session's pull request read fresh, when it is `number`.
    async fn current(
        &self,
        forge: &dyn Forge,
        spot: &Spot,
        number: PullNumber,
    ) -> Result<Result<Read, MergeRefusal>, ForgeError> {
        let listed = match forge
            .find(&spot.repo, &spot.branch, Priority::Person)
            .await?
        {
            Fresh::Changed(listed) => listed,
            Fresh::Same => Vec::new(),
        };
        let Some(found) = pick(&listed, &spot.head) else {
            return Ok(Err(MergeRefusal::NoPullRequest));
        };
        if found.number != number {
            return Ok(Err(MergeRefusal::OtherPullRequest {
                number: found.number,
            }));
        }
        forge
            .read(&spot.repo, number, Priority::Person)
            .await
            .map(Ok)
    }
}
