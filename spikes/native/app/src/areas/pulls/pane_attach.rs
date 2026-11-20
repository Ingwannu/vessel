//! "Add to chat" from the pane: Paseo's `pane.tsx` 349-443 (pinned
//! 0.15.4). A comment, a review or a thread goes to the composer of the
//! session the pane shows; "Add all to chat" adds every card, each as its
//! own item, skipping resolved threads; a failed check first reads its
//! report and log tails (`pr.check`, ADR 0088 part 4) and goes either way.
//!
//! [Decision Log]
//! - Purpose: hand the agent what a reviewer said or why a check failed.
//! - Constraint: the composer takes an item with a window (it focuses its
//!   field); the check's answer comes through the link without one.
//! - Choice: an answer's item is kept and added on the next frame, where
//!   the pane's render has the window (the Add project sheet's pattern).
//!   The texts are `view::workspace::pull::attach`'s. Like Paseo, a check
//!   whose report cannot be read is added with what the pane knows, and
//!   nothing is said.

use std::collections::HashSet;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{Ask, PullAsk};
use vessel_app_view::composer::context::{
    ContextAttachment, ContextId, ContextKind, ContextText, ContextTitle, ContextUrl, Payload,
};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::pull::attach::{
    About, AttachKind, Attachment, activity, can_add, check, thread,
};
use vessel_app_view::workspace::pull::timeline::{Entry, entries, shown};
use vessel_proto::forge::{CheckDetails, CheckRun, GetCheckDetails, PullRequest};

use super::FactsRead;
use super::pane_timeline::TimelineRead;
use crate::shell::Shell;

/// What the pane is adding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Adding {
    /// Failed checks whose report is out, by [`check_key`].
    pub(crate) checks: HashSet<String>,
    /// Made from an answer, added on the next frame.
    pub(crate) pending: Vec<Attachment>,
}

/// A check's identity among the rows (Paseo's `checkIdentity`).
pub(super) fn check_key(run: &CheckRun) -> String {
    run.run.map_or_else(
        || {
            format!(
                "check:{}:{}",
                run.name.as_str(),
                run.url.as_ref().map_or("", |url| url.as_str())
            )
        },
        |id| format!("check-run:{}", id.get()),
    )
}

/// An attachment as the composer's item.
fn item(made: &Attachment) -> Option<ContextAttachment> {
    let kind = match made.kind {
        AttachKind::Comment => ContextKind::PullComment,
        AttachKind::Review => ContextKind::PullReview,
        AttachKind::Check => ContextKind::PullCheck,
    };
    Some(ContextAttachment::Snapshot {
        kind,
        payload: Payload {
            id: ContextId::parse(&made.id).ok()?,
            title: ContextTitle::parse(&made.title).ok()?,
            text: ContextText::parse(&made.text).ok()?,
            url: made
                .url
                .as_deref()
                .and_then(|url| ContextUrl::parse(url).ok()),
        },
    })
}

impl Shell {
    /// Whether the pane can add to a composer now (Paseo's `attachEnabled`).
    pub(super) fn pulls_can_attach(&self) -> bool {
        self.composer_destination().is_some()
    }

    /// The pull request the pane read, whole.
    fn pulls_pane_pull(&self) -> Option<&PullRequest> {
        match &self.areas.pulls.facts.as_ref()?.read {
            FactsRead::Ready(view) => view.pull_request.as_deref(),
            FactsRead::Loading | FactsRead::Failed(_) => None,
        }
    }

    /// Hand `made` to the composer; a refusal is said once.
    fn pulls_attach(
        &mut self,
        made: &[Attachment],
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.composer_destination() else {
            return;
        };
        let Some(scope) = self.composer_context_scope(target) else {
            return;
        };
        for attachment in made {
            let Some(item) = item(attachment) else {
                continue;
            };
            if let Err(problem) = self.composer_attach_context(target, &scope, item, window, cx) {
                self.composer_context_refused(problem, cx);
                return;
            }
        }
    }

    /// The cards' attachments, made by `pick` from the activity as listed.
    fn pulls_activity_attachments(
        &self,
        pick: impl Fn(&About<'_>, &[Entry<'_>]) -> Vec<Attachment>,
    ) -> Vec<Attachment> {
        let Some(facts) = self.areas.pulls.facts.as_ref() else {
            return Vec::new();
        };
        let (Some(pull), TimelineRead::Ready(timeline)) = (self.pulls_pane_pull(), &facts.timeline)
        else {
            return Vec::new();
        };
        let about = About {
            number: pull.summary.number.get(),
            title: pull.summary.title.as_str(),
            url: pull.summary.url.as_str(),
            now: chrono::Utc::now(),
        };
        let items = shown(&timeline.items);
        pick(&about, &entries(&items))
    }

    /// One comment or review, by its id.
    pub(super) fn pulls_add_item(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let made = self.pulls_activity_attachments(|about, listed| {
            listed
                .iter()
                .flat_map(entry_items)
                .find(|item| item.id().as_str() == id)
                .and_then(|item| activity(about, item))
                .into_iter()
                .collect()
        });
        self.pulls_attach(&made, window, cx);
    }

    /// One thread, by its key.
    pub(super) fn pulls_add_thread(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let made = self.pulls_activity_attachments(|about, listed| {
            listed
                .iter()
                .flat_map(entry_threads)
                .find(|found| found.key() == key)
                .and_then(|found| thread(about, found))
                .into_iter()
                .collect()
        });
        self.pulls_attach(&made, window, cx);
    }

    /// Paseo's "Add all to chat": every card in order, a review then its
    /// threads, resolved threads left out.
    pub(super) fn pulls_add_all(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let made = self.pulls_activity_attachments(|about, listed| {
            let mut made = Vec::new();
            for entry in listed {
                match entry {
                    Entry::Single(item) => made.extend(activity(about, item)),
                    Entry::Review { review, threads } => {
                        made.extend(activity(about, review));
                        made.extend(
                            threads
                                .iter()
                                .filter(|found| !found.place.resolved)
                                .filter_map(|found| thread(about, found)),
                        );
                    }
                    Entry::Thread(found) if !found.place.resolved => {
                        made.extend(thread(about, found));
                    }
                    Entry::Thread(_) => {}
                }
            }
            made
        });
        self.pulls_attach(&made, window, cx);
    }

    /// "Add to chat" on a failed check: read its report first when the
    /// node can, else add what the row knows.
    pub(super) fn pulls_add_check(
        &mut self,
        run: &CheckRun,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(facts) = self.areas.pulls.facts.as_ref() else {
            return;
        };
        let (machine, session) = (facts.subject.machine.clone(), facts.subject.session);
        if let Some(id) = run.run.filter(|_| self.pulls_has_pane_reads(&machine)) {
            if let Some(facts) = self.areas.pulls.facts.as_mut() {
                let _new = facts.adding.checks.insert(check_key(run));
            }
            self.ask(
                &machine,
                Ask::Pull(PullAsk::Check(GetCheckDetails { session, run: id })),
            );
            cx.notify();
            return;
        }
        let made = self.pulls_check_attachment(run, None);
        self.pulls_attach(&made, window, cx);
    }

    /// The check's attachment, with its report when one was read.
    fn pulls_check_attachment(
        &self,
        run: &CheckRun,
        report: Option<&vessel_proto::forge::CheckReport>,
    ) -> Vec<Attachment> {
        self.pulls_pane_pull()
            .map(|pull| {
                let about = About {
                    number: pull.summary.number.get(),
                    title: pull.summary.title.as_str(),
                    url: pull.summary.url.as_str(),
                    now: chrono::Utc::now(),
                };
                check(&about, run, report)
            })
            .into_iter()
            .collect()
    }

    /// A check's report arrived: its attachment waits for the next frame.
    pub(super) fn pulls_check_answered(
        &mut self,
        machine: &str,
        ask: &GetCheckDetails,
        result: Result<Box<CheckDetails>, Problem>,
    ) {
        let matches = self.areas.pulls.facts.as_ref().is_some_and(|facts| {
            facts.subject.machine == machine && facts.subject.session == ask.session
        });
        if !matches {
            return;
        }
        let Some(run) = self
            .pulls_pane_pull()
            .and_then(|pull| pull.checks.runs.iter().find(|run| run.run == Some(ask.run)))
            .cloned()
        else {
            return;
        };
        let details = result.ok();
        let report = details
            .as_ref()
            .and_then(|details| details.report.as_deref());
        let made = self.pulls_check_attachment(&run, report);
        if let Some(facts) = self.areas.pulls.facts.as_mut() {
            let _was = facts.adding.checks.remove(&check_key(&run));
            facts.adding.pending.extend(made);
        }
    }

    /// The next frame: add what answers made.
    pub(super) fn pulls_attach_pending(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let made = self
            .areas
            .pulls
            .facts
            .as_mut()
            .map(|facts| std::mem::take(&mut facts.adding.pending))
            .unwrap_or_default();
        self.pulls_attach(&made, window, cx);
        cx.notify();
    }
}

/// An entry's items: the review or comment, then its threads' comments.
fn entry_items<'a>(entry: &'a Entry<'a>) -> Vec<&'a vessel_proto::forge::TimelineItem> {
    match entry {
        Entry::Single(item) => vec![*item],
        Entry::Review { review, threads } => std::iter::once(*review)
            .chain(
                threads
                    .iter()
                    .flat_map(|found| found.comments.iter().copied()),
            )
            .collect(),
        Entry::Thread(found) => found.comments.clone(),
    }
}

/// An entry's threads.
fn entry_threads<'a>(
    entry: &'a Entry<'a>,
) -> Vec<&'a vessel_app_view::workspace::pull::timeline::Thread<'a>> {
    match entry {
        Entry::Single(_) => Vec::new(),
        Entry::Review { threads, .. } => threads.iter().collect(),
        Entry::Thread(found) => vec![found],
    }
}

/// Whether a card's item has words worth adding (Paseo's `canAdd`).
pub(super) fn addable(item: &vessel_proto::forge::TimelineItem) -> bool {
    can_add(item)
}
