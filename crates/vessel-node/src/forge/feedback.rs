//! Pull request feedback sent to a session (ADR 0029, "Feedback to the
//! session"): which events a read holds, which were sent already, what
//! is quoted, and the one prompt that carries them through the queue.
//!
//! [Decision Log]
//! - Purpose: with feedback on, an agent learns of its pull request's
//!   failing checks, requested changes, unresolved comments and conflicts
//!   without a person relaying them; each once.
//! - Constraint: on a public repository anyone may review or comment, and
//!   what is sent becomes the agent's prompt; a person's queue must not
//!   fill with feedback, a running turn must not be interrupted, and a
//!   session waiting on its owner must not be pushed on.
//! - Choice: an event is named by a signature (repository, number, kind,
//!   the forge's id or the head); those sent are kept in the store and
//!   skipped. A review's or comment's text is quoted only when its author
//!   may write to the repository (asked once an hour per login), stripped
//!   of control and format characters and set apart as quoted data;
//!   others' are counted and linked. One prompt, through the queue as the
//!   owner (it starts a turn only when the session is idle); while it
//!   waits, newer events are added to it by an edit. Nothing is sent while
//!   the session waits on a permission or a question; the next look tries
//!   again. A signature is recorded only once the prompt was accepted.
//! - A conflict's signature is forgotten once the pull request can merge
//!   again, so a later conflict is sent (agent-orchestrator's re-arming).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use chrono::Utc;
use vessel_forge::{Author, Forge, Read, ReviewVerdict};
use vessel_proto::forge::{ForgeLogin, Mergeable, RepoRef};
use vessel_proto::{Activity, ContentBlock, Creator, Delivery, QueueChange};
use vessel_session::SessionHandle;
use vessel_store::{MAX_FEEDBACK_SENT, Store};

mod text;

use text::Note;

/// The first words of every feedback prompt: how the node finds its own
/// waiting prompt, and how a person tells it apart.
pub(super) const HEADER: &str = "Vessel: pull request feedback";
/// The most events one prompt lists.
const MAX_EVENTS: usize = 20;
/// The most a feedback prompt weighs.
pub(super) const MAX_BYTES: usize = 16 * 1024;
/// How long whether a login may write stays known.
const KEEP_WRITER: Duration = Duration::from_secs(3600);
/// The most logins kept.
const MAX_WRITERS: usize = 1024;

/// Whether logins may write to repositories, as last asked.
#[derive(Debug, Default)]
pub(crate) struct Writers(Mutex<HashMap<(RepoRef, ForgeLogin), (bool, Instant)>>);

impl Writers {
    async fn may_write(&self, forge: &dyn Forge, repo: &RepoRef, login: &ForgeLogin) -> bool {
        let key = (repo.clone(), login.clone());
        if let Some((writes, at)) = self.lock().get(&key)
            && at.elapsed() < KEEP_WRITER
        {
            return *writes;
        }
        // A failure to ask quotes nothing (it is not remembered).
        let Ok(writes) = forge.can_write(repo, login).await else {
            return false;
        };
        let mut known = self.lock();
        if known.len() >= MAX_WRITERS {
            known.retain(|_, (_, at)| at.elapsed() < KEEP_WRITER);
        }
        if known.len() < MAX_WRITERS {
            let _previous = known.insert(key, (writes, Instant::now()));
        }
        writes
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(RepoRef, ForgeLogin), (bool, Instant)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a feedback job needs.
#[derive(Debug)]
pub(super) struct Feed {
    pub(super) handle: SessionHandle,
    pub(super) forge: Arc<dyn Forge>,
    pub(super) store: Store,
    pub(super) read: Arc<Read>,
    pub(super) sent: HashSet<String>,
    pub(super) writers: Arc<Writers>,
}

/// One event of a read.
#[derive(Debug)]
struct Event {
    signature: String,
    note: Note,
    /// Whose words it carries, to be asked whether they may write.
    author: Option<Author>,
}

/// Send what is new; answers the signatures sent, as now recorded.
pub(super) async fn feed(feed: Feed) -> HashSet<String> {
    let Feed {
        handle,
        forge,
        store,
        read,
        mut sent,
        writers,
    } = feed;
    let session = handle.id();
    let prefix = prefix(&read);
    let all = events(&read, &prefix);
    let gone = gone(&read, &prefix, &all, &sent);
    if !gone.is_empty()
        && store
            .forget_feedback_sent(session, gone.clone())
            .await
            .is_ok()
    {
        for signature in &gone {
            let _was = sent.remove(signature);
        }
    }
    let new: Vec<Event> = all
        .into_iter()
        .filter(|event| !sent.contains(&event.signature))
        .collect();
    if new.is_empty() || waits_on_owner(&handle) {
        return sent;
    }
    let more = new.len().saturating_sub(MAX_EVENTS);
    let mut notes = Vec::with_capacity(new.len().min(MAX_EVENTS));
    let mut signatures = Vec::with_capacity(new.len());
    for event in new.into_iter().take(MAX_EVENTS) {
        let quoted = match &event.author {
            Some(Author {
                login: Some(login),
                bot: false,
            }) => {
                writers
                    .may_write(forge.as_ref(), &read.pull.summary.repo, login)
                    .await
            }
            Some(_) | None => false,
        };
        notes.push(event.note.quoted(quoted));
        signatures.push(event.signature);
    }
    // Turned off while this was being written: nothing is sent.
    if !store.feedback(session).await.is_ok_and(|stored| stored.on) {
        return sent;
    }
    let url = read.pull.summary.url.as_str();
    if deliver(&handle, &notes, more, url).await {
        if let Err(error) = store
            .feedback_sent(session, signatures.clone(), Utc::now())
            .await
        {
            tracing::warn!(%session, %error, "sent feedback could not be recorded");
        }
        sent.extend(signatures);
    }
    sent
}

/// The signatures to forget before this read's events are weighed: a
/// conflict's, once the forge knows the pull request does not conflict
/// (so a later conflict is sent again); and near the bound of what is
/// kept, every one this read no longer has (the store then drops the
/// oldest only past that).
fn gone(read: &Read, prefix: &str, all: &[Event], sent: &HashSet<String>) -> Vec<String> {
    let resolved = matches!(
        read.pull.summary.mergeable,
        Mergeable::Mergeable | Mergeable::Blocked | Mergeable::Unstable | Mergeable::Behind
    );
    let conflict = format!("{prefix}conflict:");
    let full = sent.len().saturating_add(MAX_EVENTS) > MAX_FEEDBACK_SENT;
    let current: HashSet<&str> = all.iter().map(|event| event.signature.as_str()).collect();
    let mut gone: Vec<String> = sent
        .iter()
        .filter(|signature| {
            (resolved && signature.starts_with(&conflict))
                || (full && !current.contains(signature.as_str()))
        })
        .cloned()
        .collect();
    gone.sort();
    gone
}

/// `<host>/<owner>/<name>#<number>:`, every signature's start.
fn prefix(read: &Read) -> String {
    let summary = &read.pull.summary;
    format!(
        "{}/{}#{}:",
        summary.repo.host, summary.repo.path, summary.number
    )
}

/// Every event `read` holds, oldest kinds first: failing checks, then
/// requested changes, unresolved comments, a conflict.
fn events(read: &Read, prefix: &str) -> Vec<Event> {
    let head = &read.pull.summary.head_commit;
    let mut events = Vec::new();
    for check in &read.discussion.failed {
        events.push(Event {
            signature: format!("{prefix}check:{head}:{}", check.name),
            note: Note::Check {
                name: check.name.as_str().to_owned(),
                url: check.url.as_ref().map(|url| url.as_str().to_owned()),
            },
            author: None,
        });
    }
    for review in &read.discussion.reviews {
        if review.verdict == ReviewVerdict::ChangesRequested {
            events.push(Event {
                signature: format!("{prefix}review:{}", review.id.as_str()),
                note: Note::Changes {
                    by: login_of(&review.author),
                    text: review.body.clone(),
                    url: review.url.as_ref().map(|url| url.as_str().to_owned()),
                },
                author: Some(review.author.clone()),
            });
        }
    }
    for thread in read
        .discussion
        .threads
        .iter()
        .filter(|thread| !thread.resolved)
    {
        for comment in &thread.comments {
            events.push(Event {
                signature: format!("{prefix}comment:{}", comment.id.as_str()),
                note: Note::Comment {
                    by: login_of(&comment.author),
                    place: thread.path.as_ref().map(|path| match thread.line {
                        Some(line) => format!("{path}:{line}"),
                        None => path.clone(),
                    }),
                    text: comment.body.clone(),
                    url: comment.url.as_ref().map(|url| url.as_str().to_owned()),
                },
                author: Some(comment.author.clone()),
            });
        }
    }
    if read.pull.summary.mergeable == Mergeable::Conflicting {
        events.push(Event {
            signature: format!("{prefix}conflict:{head}"),
            note: Note::Conflict,
            author: None,
        });
    }
    events
}

fn login_of(author: &Author) -> String {
    match (&author.login, author.bot) {
        (Some(login), _) => login.as_str().to_owned(),
        (None, true) => "a bot".to_owned(),
        (None, false) => "someone".to_owned(),
    }
}

/// The session waits on its owner (a permission or a question).
fn waits_on_owner(handle: &SessionHandle) -> bool {
    handle
        .info()
        .board
        .is_some_and(|board| board.activity == Activity::WaitingInput)
}

/// Add `notes` to the feedback prompt waiting in the queue, or queue a
/// new one; whether it was accepted.
async fn deliver(handle: &SessionHandle, notes: &[String], more: usize, url: &str) -> bool {
    let waiting = handle.conversation().ok().and_then(|conversation| {
        conversation.queue.entries.into_iter().find(|entry| {
            entry.by == Creator::Owner
                && matches!(entry.content.first(), Some(ContentBlock::Text { text }) if text.starts_with(HEADER))
        })
    });
    if let Some(entry) = waiting {
        let earlier = match entry.content.first() {
            Some(ContentBlock::Text { text }) => text.clone(),
            Some(_) | None => String::new(),
        };
        // At most one waits: one too full to take these is sent first,
        // and the next look adds them to a new one.
        let Some(text) = text::add(&earlier, notes, more, url) else {
            return false;
        };
        let change = QueueChange::Edit {
            entry: entry.id,
            revision: entry.revision,
            content: vec![ContentBlock::Text { text }],
        };
        return handle.change_queue(change).await.is_ok();
    }
    let text = text::prompt(notes, more, url);
    handle
        .prompt(
            vec![ContentBlock::Text { text }],
            Delivery::Queue,
            Creator::Owner,
        )
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use vessel_proto::forge::{ForgeKind, PullNumber};

    use super::*;

    fn read(mergeable: Mergeable) -> Read {
        let repo = RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "owner/name".parse().unwrap(),
        };
        let head = "a".repeat(40).parse().unwrap();
        let branch = "topic".parse().unwrap();
        let mut read =
            vessel_forge::fake::pull(&repo, PullNumber::new(3).unwrap(), &branch, &head).unwrap();
        read.pull.summary.mergeable = mergeable;
        read
    }

    /// A conflict is sent again once the forge knows it was resolved (not
    /// while it does not know yet); at the bound, what a read no longer has
    /// goes first.
    #[test]
    fn a_resolved_conflict_re_arms_and_the_bound_drops_the_unseen_first() {
        let conflicting = read(Mergeable::Conflicting);
        let prefix = prefix(&conflicting);
        let all = events(&conflicting, &prefix);
        assert_eq!(all.len(), 1);
        let sent: HashSet<String> = all.iter().map(|event| event.signature.clone()).collect();
        assert!(gone(&conflicting, &prefix, &all, &sent).is_empty());
        let unknown = read(Mergeable::Unknown);
        let none = events(&unknown, &prefix);
        assert!(
            gone(&unknown, &prefix, &none, &sent).is_empty(),
            "not known yet"
        );
        let clean = read(Mergeable::Mergeable);
        let none = events(&clean, &prefix);
        let rearmed: Vec<String> = sent.iter().cloned().collect();
        assert_eq!(gone(&clean, &prefix, &none, &sent), rearmed);
        let mut many: HashSet<String> =
            (0..MAX_FEEDBACK_SENT).map(|n| format!("old:{n}")).collect();
        many.extend(sent.iter().cloned());
        let dropped = gone(&conflicting, &prefix, &all, &many);
        assert_eq!(dropped.len(), MAX_FEEDBACK_SENT);
        assert!(!dropped.iter().any(|signature| sent.contains(signature)));
    }
}
