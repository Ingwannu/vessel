//! A session's pull request in the app: "Create PR" and "View PR" in the
//! git menu (Paseo's `git/policy.ts` pull request action), the form that
//! opens one, the link at the head of the Changes tab (Paseo's
//! `ChangesPullRequestLink`) and its facts (Paseo's
//! `git/pull-request-panel/pane.tsx`). The rules are
//! `view::workspace::pull`'s; this area keeps state, wires and draws.
//!
//! [Decision Log]
//! - Purpose: open a pull request from the session's branch and read its
//!   checks, reviews and mergeability without leaving the session.
//! - Constraints: the summary rides on the session's card (the node keeps
//!   it fresh); the whole pull request is `pr.get`; `pr.open` is the
//!   node's (it pushes the branch for a person first). Paseo writes the
//!   title with a model; this node takes the person's, so there is a form.
//! - Alternatives: the facts as a tab of the code panel, as Paseo's pane
//!   (a tab in track F's panel and its tab bar); a page of every pull
//!   request (`pr.list` covers watched sessions only, so it would claim a
//!   whole the node does not send).
//! - Choice: the form and the facts as dialogs of this area, opened from
//!   the git row; the answers come on the workspace pair (`Answer::Pull`).
//! - Cost: the facts cover the Changes tab while open, where Paseo's pane
//!   sits beside it; a tab can come when the code panel offers one.

mod card;
mod facts;
mod feedback;
mod form;
mod link;
mod merge;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, DialogState, TextField};
use vessel_app_link::workspace::{Ask, PullAnswer, PullAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::pull::{
    OpenDraft, OpenProblem, Opened, PullEntry, open_outcome, open_request, prefill, pull_entry,
};
use vessel_proto::SessionId;
use vessel_proto::forge::{
    GetPullRequest, OpenPullRequest, OpenRefusal, PullRequestView, SessionPullRequest,
};
use vessel_proto::workspace::{CommitPage, GitStatus, ReadLog, RefName, WorkspaceRoot};

use crate::shell::Shell;

pub(crate) use card::card_items;
pub(crate) use facts::dialog as facts_dialog;
pub(crate) use form::{base_list, dialog as form_dialog};
pub(crate) use link::link;

/// The session a dialog is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Subject {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
    pub(crate) root: WorkspaceRoot,
}

/// Why the form did not send, or what the node said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FormIssue {
    /// A rule of the form.
    Form(OpenProblem),
    /// The node made nothing.
    Refused(OpenRefusal),
    /// The call did not answer.
    Call(Problem),
}

/// The open form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Form {
    pub(crate) subject: Subject,
    pub(crate) branch: String,
    pub(crate) draft: OpenDraft,
    /// The base the node measures the folder against, as it names it
    /// (`origin/main` for a project whose remote HEAD names it); the form
    /// starts from it (`form::form_base`).
    pub(crate) proposed: Option<RefName>,
    /// Whether the person chose a base; until then `draft.base` is unused.
    pub(crate) base_chosen: bool,
    /// The log read for the defaults, while it is out.
    pub(crate) prefilling: Option<ReadLog>,
    /// Defaults read and not yet put in the fields (the answer comes
    /// without a window; the next frame puts them in).
    pub(crate) fill: Option<OpenDraft>,
    /// The request sent, while it is out.
    pub(crate) sending: Option<Box<OpenPullRequest>>,
    pub(crate) issue: Option<FormIssue>,
}

/// What the facts dialog shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FactsRead {
    Loading,
    Ready(Box<PullRequestView>),
    Failed(Problem),
}

/// The open facts dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Facts {
    pub(crate) subject: Subject,
    pub(crate) read: FactsRead,
    /// A read from the forge is out (the refresh button spins).
    pub(crate) refreshing: bool,
}

pub(crate) struct PullsArea {
    pub(crate) form: Option<Form>,
    pub(crate) facts: Option<Facts>,
    pub(crate) dialog: DialogState,
    pub(crate) title: TextField,
    pub(crate) body: TextField,
    pub(crate) bases: ComboboxState,
    pub(crate) base_anchor: vessel_app_kit::select::Anchor,
    /// The draft switch's changes (the kit slides its thumb on a new one).
    pub(crate) draft_turn: u64,
    pub(crate) tips: vessel_app_kit::Tooltips,
    pub(crate) link_tip: vessel_app_kit::tooltip::Place,
    /// The whole pull request last read (its merge methods).
    pub(crate) whole: Option<merge::Whole>,
    /// A read of it for the menu is out, for that session.
    pub(crate) reading_whole: Option<SessionId>,
    /// The merge sent, while it is out.
    pub(crate) merging: Option<Box<vessel_proto::forge::MergePullRequest>>,
    /// The feedback change sent, while it is out.
    pub(crate) feedback_sending: Option<vessel_proto::forge::SetFeedback>,
    /// The feedback switch's changes.
    pub(crate) feedback_turn: u64,
}

impl PullsArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            form: None,
            facts: None,
            dialog: DialogState::new(cx),
            title: TextField::new(
                "",
                |shell: &mut Shell, _text, window, cx| shell.pulls_submit(window, cx),
                window,
                cx,
            ),
            body: TextField::form_multiline(
                "",
                |shell: &mut Shell, _text, window, cx| shell.pulls_submit(window, cx),
                window,
                cx,
            ),
            bases: ComboboxState::new(window, cx),
            base_anchor: vessel_app_kit::select::Anchor::default(),
            draft_turn: 0,
            tips: vessel_app_kit::Tooltips::new(),
            link_tip: vessel_app_kit::tooltip::Place::default(),
            whole: None,
            reading_whole: None,
            merging: None,
            feedback_sending: None,
            feedback_turn: 0,
        }
    }
}

impl Shell {
    /// The session the git row shows, and what its card says of its pull
    /// request.
    pub(crate) fn pulls_subject(&self) -> Option<(Subject, Option<&SessionPullRequest>)> {
        let open = self.areas.session.open.as_ref()?;
        let target = self.areas.git.target.as_ref()?;
        if target.machine != open.machine {
            return None;
        }
        let card = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == open.machine)?
            .sessions
            .iter()
            .find(|card| card.session == open.session)?;
        let subject = Subject {
            machine: open.machine.clone(),
            session: open.session,
            root: target.root,
        };
        Some((subject, card.detail.pull_request.as_deref()))
    }

    /// The git menu's pull request entry for `status`.
    pub(crate) fn pulls_entry(&self, status: &GitStatus) -> PullEntry {
        self.pulls_subject()
            .map_or(PullEntry::Hidden, |(_, pull)| pull_entry(pull, status))
    }

    /// The legacy toolbar cannot prove the origin of its displayed facts.
    /// Its new scoped opener belongs to C4; never revive alias-only prefilling
    /// or external navigation from this unverified callback.
    pub(crate) fn pulls_chosen(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.git_unverified_view(cx);
    }

    /// The log asked for the defaults arrived: keep the defaults for the
    /// next frame (`pulls_fill`).
    pub(crate) fn pulls_log_read(
        &mut self,
        machine: &str,
        ask: &ReadLog,
        result: Result<CommitPage, Problem>,
    ) {
        let Some(form) = self.areas.pulls.form.as_mut().filter(|form| {
            form.subject.machine == machine && form.prefilling.as_ref() == Some(ask)
        }) else {
            return;
        };
        form.prefilling = None;
        // A log that could not be read leaves the form empty: the person
        // writes the title.
        if let Ok(page) = result {
            form.fill = Some(prefill(
                &page.commits,
                &form.branch,
                form.draft.base.clone(),
            ));
        }
    }

    /// Put the defaults read into the fields the person has not typed in.
    pub(crate) fn pulls_fill(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let pulls = &mut self.areas.pulls;
        let Some(start) = pulls.form.as_mut().and_then(|form| form.fill.take()) else {
            return;
        };
        if pulls.title.text(cx).is_empty() {
            pulls.title.set_text(&start.title, window, cx);
        }
        if pulls.body.text(cx).is_empty() {
            pulls.body.set_text(&start.body, window, cx);
        }
        cx.notify();
    }

    /// Enter in the title, Mod+Enter in the description, or Create PR.
    pub(crate) fn pulls_submit(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let base = self
            .areas
            .pulls
            .form
            .as_ref()
            .map(|form| form::form_base(self, form));
        let pulls = &mut self.areas.pulls;
        let Some(form) = pulls.form.as_mut().filter(|form| form.sending.is_none()) else {
            return;
        };
        form.draft.base = base.flatten();
        form.draft.title = pulls.title.text(cx);
        form.draft.body = pulls.body.text(cx);
        match open_request(form.subject.session, &form.draft) {
            Ok(request) => {
                form.issue = None;
                form.sending = Some(Box::new(request.clone()));
                let machine = form.subject.machine.clone();
                self.ask(&machine, Ask::Pull(PullAsk::Open(request)));
            }
            Err(problem) => form.issue = Some(FormIssue::Form(problem)),
        }
        cx.notify();
    }

    /// The link or a toast's View: open the facts and read them whole.
    pub(crate) fn pulls_open_facts(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some((subject, _)) = self.pulls_subject() else {
            return;
        };
        let machine = subject.machine.clone();
        let session = subject.session;
        self.areas.pulls.facts = Some(Facts {
            subject,
            read: FactsRead::Loading,
            refreshing: false,
        });
        self.areas.pulls.dialog.focus(window, cx);
        self.ask(
            &machine,
            Ask::Pull(PullAsk::Get(GetPullRequest {
                session,
                refresh: false,
            })),
        );
        cx.notify();
    }

    /// The refresh button: read it from the forge now.
    pub(crate) fn pulls_refresh(&mut self, cx: &mut Context<'_, Self>) {
        let Some(facts) = self
            .areas
            .pulls
            .facts
            .as_mut()
            .filter(|facts| !facts.refreshing)
        else {
            return;
        };
        facts.refreshing = true;
        let machine = facts.subject.machine.clone();
        let session = facts.subject.session;
        self.ask(
            &machine,
            Ask::Pull(PullAsk::Get(GetPullRequest {
                session,
                refresh: true,
            })),
        );
        cx.notify();
    }

    /// Close whichever dialog is open.
    pub(crate) fn pulls_close(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.pulls.form = None;
        self.areas.pulls.facts = None;
        cx.notify();
    }

    /// What the node answered.
    pub(crate) fn pulls_answered(
        &mut self,
        machine: &str,
        answer: PullAnswer,
        cx: &mut Context<'_, Self>,
    ) {
        match answer {
            PullAnswer::Read { session, result } => self.pulls_read(machine, session, result),
            PullAnswer::Opened { ask, result } => self.pulls_opened(machine, &ask, result, cx),
            PullAnswer::Merged { ask, result } => self.pulls_merged(&ask, result, cx),
            PullAnswer::Feedback { ask, result } => {
                self.pulls_feedback_answered(&ask, result, cx);
            }
        }
        cx.notify();
    }

    fn pulls_read(
        &mut self,
        machine: &str,
        session: SessionId,
        result: Result<Box<PullRequestView>, Problem>,
    ) {
        // Whoever asked, the whole pull request read is the menu's too
        // (its merge methods).
        let pulls = &mut self.areas.pulls;
        if pulls.reading_whole == Some(session) {
            pulls.reading_whole = None;
        }
        if let Some(pull) = result
            .as_ref()
            .ok()
            .and_then(|view| view.pull_request.clone())
        {
            pulls.whole = Some(merge::Whole { session, pull });
        }
        let Some(facts) =
            self.areas.pulls.facts.as_mut().filter(|facts| {
                facts.subject.machine == machine && facts.subject.session == session
            })
        else {
            return;
        };
        facts.refreshing = false;
        facts.read = match result {
            Ok(view) => FactsRead::Ready(view),
            // A failed refresh keeps what was read; a first read says why.
            Err(problem) => match &facts.read {
                FactsRead::Ready(_) => return,
                FactsRead::Loading | FactsRead::Failed(_) => FactsRead::Failed(problem),
            },
        };
    }

    fn pulls_opened(
        &mut self,
        machine: &str,
        ask: &OpenPullRequest,
        result: Result<vessel_proto::forge::OpenOutcome, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let pulls = &mut self.areas.pulls;
        let Some(form) = pulls
            .form
            .as_mut()
            .filter(|form| form.subject.machine == machine && form.sending.as_deref() == Some(ask))
        else {
            return;
        };
        form.sending = None;
        let words = self.words.pulls();
        match result.map(open_outcome) {
            Ok(Opened::Made(_)) => {
                pulls.form = None;
                let done = words.created().to_owned();
                self.toast(done, ToastTone::Ok, cx);
            }
            Ok(Opened::Existing(_)) => {
                pulls.form = None;
                let done = words.existing().to_owned();
                self.toast(done, ToastTone::Info, cx);
            }
            Ok(Opened::Refused(refusal)) => form.issue = Some(FormIssue::Refused(refusal)),
            Err(problem) => form.issue = Some(FormIssue::Call(problem)),
        }
    }
}
