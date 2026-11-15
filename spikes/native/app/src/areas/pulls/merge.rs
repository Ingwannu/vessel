//! Merging from the git menu: Paseo's direct merge rows ("Merge PR
//! (squash)" and its kin, `git/policy.ts`), offered from the whole pull
//! request (`pr.get`: the methods the repository allows) with the card's
//! fresher summary, and merged on one choice as Paseo does; the request
//! pins the number and the head the person saw (`view::pull::merge`).

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, PullAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::pull::{Merged, merge_methods, merge_outcome, merge_refusal_words};
use vessel_proto::SessionId;
use vessel_proto::forge::{
    GetPullRequest, MergeMethod, MergeOutcome, MergePullRequest, MergeRefusal, PullRequest,
};
use vessel_proto::workspace::GitStatus;

use crate::shell::Shell;

/// The whole pull request last read for a session (its merge methods).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Whole {
    pub(crate) session: SessionId,
    pub(crate) pull: Box<PullRequest>,
}

impl Shell {
    /// The whole pull request as the person sees it now: the one read, with
    /// the card's summary (the node's watcher keeps that one fresher).
    fn pulls_seen(&self) -> Option<(SessionId, PullRequest)> {
        let (subject, pull) = self.pulls_subject()?;
        let summary = pull?.summary.as_ref()?;
        let whole = self.areas.pulls.whole.as_ref()?;
        if whole.session != subject.session || whole.pull.summary.number != summary.number {
            return None;
        }
        let mut seen = (*whole.pull).clone();
        seen.summary = summary.clone();
        Some((subject.session, seen))
    }

    /// The merge rows for the menu while the folder is `status`.
    pub(crate) fn pulls_merges(&self, status: &GitStatus) -> Vec<MergeMethod> {
        self.pulls_seen()
            .map(|(_, seen)| merge_methods(&seen, status))
            .unwrap_or_default()
    }

    /// The git menu opened: read the whole pull request when there is one
    /// and the card's summary moved since the last read (the rows join
    /// the open menu when it answers).
    pub(crate) fn pulls_menu_opened(&mut self) {
        let Some((subject, Some(pull))) = self.pulls_subject() else {
            return;
        };
        let Some(summary) = pull.summary.as_ref() else {
            return;
        };
        let pulls = &self.areas.pulls;
        let fresh = pulls.whole.as_ref().is_some_and(|whole| {
            whole.session == subject.session
                && whole.pull.summary.number == summary.number
                && whole.pull.summary.updated_at == summary.updated_at
        });
        if fresh || pulls.reading_whole == Some(subject.session) {
            return;
        }
        self.areas.pulls.reading_whole = Some(subject.session);
        self.ask(
            &subject.machine,
            Ask::Pull(PullAsk::Get(GetPullRequest {
                session: subject.session,
                refresh: false,
            })),
        );
    }

    /// The legacy menu has no original drawn physical receipt. Refuse here,
    /// rather than only at one caller, without changing an already sent merge.
    pub(crate) fn pulls_merge(&mut self, _method: MergeMethod, cx: &mut Context<'_, Self>) {
        self.git_unverified_view(cx);
    }

    /// What `pr.merge` answered: Paseo's toasts ("PR merged", or why not;
    /// the forge's own words on a line under the sentence).
    pub(super) fn pulls_merged(
        &mut self,
        ask: &MergePullRequest,
        result: Result<MergeOutcome, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.pulls.merging.as_deref() != Some(ask) {
            return;
        }
        self.areas.pulls.merging = None;
        let words = self.words;
        match result.map(merge_outcome) {
            Ok(Merged::Done) => {
                let done = words.pulls().merged().to_owned();
                self.toast(done, ToastTone::Ok, cx);
            }
            Ok(Merged::Refused(refusal)) => {
                let mut text = merge_refusal_words(&refusal, words).into_owned();
                if let MergeRefusal::Rejected { message, .. }
                | MergeRefusal::NotMergeable {
                    message: Some(message),
                    ..
                } = &refusal
                {
                    text.push('\n');
                    text.push_str(message);
                }
                self.toast(text, ToastTone::Error, cx);
            }
            Err(problem) => {
                let failure = self.t.lang.git_call_failure(&problem);
                let text = format!("{}\n{}", words.pulls().merge_failed(), failure.title);
                self.toast(text, ToastTone::Error, cx);
            }
        }
    }
}
