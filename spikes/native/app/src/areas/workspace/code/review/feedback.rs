//! "Send feedback (N)" over the Changes list (Paseo's
//! `review/feedback/surface.tsx` and `use-feedback.ts`): the review sent
//! with "Please address this code review." to the workspace's agent at
//! once, or to the one chosen when there are several; the sent versions
//! cleared, and a toast naming the agent.
//!
//! [Decision Log]
//! - Purpose: plan item 4: the person sends the comments without writing
//!   a prompt, as Paseo's diff pane does.
//! - Constraint: the native composer's send path is the one that turns a
//!   tray's context into prompt blocks (by the node's protocol) and
//!   answers through `Answer::Submitted`; a second path would differ from
//!   it in what an older node reads.
//! - Choice: the review goes as a one-pill context submission on that
//!   path (`Files::Prompt`), its answer taken here before the composer's
//!   (matched by the same `Arc`). Recipients: the live conversations of
//!   this workspace's root on this machine (Paseo: the workspace's agent
//!   tabs). Delivery: for the open session Paseo's rule (steer when the
//!   setting says steer and the agent takes it, else interrupt; a start
//!   when no turn runs); for another session interrupt, as its turn and
//!   abilities are not known here.
//! - Cost: the floating button covers the list's last rows (Paseo pads
//!   the list by the button's clearance; the native list has no padding
//!   of its own).

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, LeadingSize, MenuHandlers, MenuItem, MenuState, menu_with};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Bounds, ClickEvent, Context, IntoElement, ParentElement as _, Pixels, Point,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonContent, ButtonIcon, ButtonKind, ButtonState, ControlSizing, Tokens, anchor_probe,
    breakpoint,
};
use vessel_app_link::Command;
use vessel_app_link::attachments::{Files, SubmitFailure, WithFiles};
use vessel_app_view::agent_mark::AgentMark;
use vessel_app_view::composer::context::{ContextAttachment, ContextTray};
use vessel_app_view::localization::Number;
use vessel_app_view::machine_book::Name;
use vessel_app_view::prefs::TurnDelivery;
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::review::feedback::{FeedbackFacts, Press};
use vessel_app_view::workspace::review::{DraftComment, Offer, ReviewKey};
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{Delivered, Delivery, SessionId};

use crate::shell::Shell;

/// The chooser's width.
const CHOOSER: f32 = 240.0;
/// An agent the review can go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Recipient {
    session: SessionId,
    title: String,
    /// Its provider's mark (Paseo's `WorkspaceTabIcon`).
    mark: Option<AgentMark>,
}

/// A review on its way.
struct Sending {
    prompt: Arc<WithFiles>,
    key: ReviewKey,
    sent: Vec<DraftComment>,
    title: String,
    count: usize,
}

/// The button's state across frames.
pub(in super::super) struct Feedback {
    menu: MenuState,
    /// The button's bounds, for the chooser above it.
    place: Rc<Cell<Option<Bounds<Pixels>>>>,
    choices: Vec<Recipient>,
    sending: Option<Sending>,
    pub(super) failed: bool,
}

impl Feedback {
    pub(in super::super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            menu: MenuState::new(cx),
            place: Rc::default(),
            choices: Vec::new(),
            sending: None,
            failed: false,
        }
    }
}

/// A count as the words take it.
fn count_of(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Whether the card's place is `root`.
fn in_root(card: &vessel_app_view::board::Card, root: &WorkspaceRoot) -> bool {
    match root {
        WorkspaceRoot::Project { project } => {
            card.detail.project == Some(*project) && card.detail.worktree.is_none()
        }
        WorkspaceRoot::Worktree { worktree } => card.detail.worktree == Some(*worktree),
    }
}

impl Shell {
    pub(super) fn review_feedback(&self) -> &Feedback {
        &self.areas.workspace.code.changes.review.feedback
    }

    /// The live conversations of the diff's root on its machine.
    fn review_recipients(&self) -> Vec<Recipient> {
        let Some((machine, root)) = self.areas.workspace.code.root() else {
            return Vec::new();
        };
        self.fleet
            .machines
            .iter()
            .filter(|held| held.name == machine)
            .flat_map(|held| held.sessions.iter())
            .filter(|card| card.structured && card.live && card.lane.is_some())
            .filter(|card| in_root(card, root))
            .map(|card| Recipient {
                session: card.session,
                title: card.title.clone(),
                mark: card.detail.harness.as_ref().map(AgentMark::of),
            })
            .collect()
    }

    fn review_machine(&self) -> Option<Name> {
        let (machine, _) = self.areas.workspace.code.root()?;
        Name::parse(machine).ok()
    }

    fn review_feedback_facts(&self, offer: Option<&Offer>, recipients: usize) -> FeedbackFacts {
        let comments = self.review_key().map_or(0, |key| {
            self.areas.composer.drafts.reviews().comments(&key).len()
        });
        FeedbackFacts {
            comments,
            sendable: offer.map(|offer| offer.snapshot.review.comments.len()),
            recipients,
            connected: self
                .review_machine()
                .is_some_and(|machine| self.attachment_context(&machine).is_some()),
            sending: self.review_feedback().sending.is_some(),
        }
    }

    fn review_feedback_pressed(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let recipients = self.review_recipients();
        let offer = self.review_offer();
        match self
            .review_feedback_facts(offer.as_ref(), recipients.len())
            .press()
        {
            Press::Nothing => {}
            Press::Send => {
                if let Some(recipient) = recipients.first().cloned() {
                    self.review_feedback_send(&recipient, cx);
                }
            }
            Press::Choose => {
                let feedback = &mut self.areas.workspace.code.changes.review.feedback;
                feedback.choices = recipients;
                match feedback.place.get() {
                    Some(bounds) => feedback.menu.open_above(bounds, px(CHOOSER), window, cx),
                    None => feedback.menu.open(at, window, cx),
                }
            }
        }
        cx.notify();
    }

    /// Paseo's delivery for `session`.
    fn review_delivery(&self, session: SessionId) -> Delivery {
        let open = self
            .areas
            .session
            .open
            .as_ref()
            .is_some_and(|open| open.session == session);
        if !open {
            return Delivery::Interrupt;
        }
        // A permission request waiting holds the turn: Paseo's interrupt
        // (or steer) applies, not a queue behind it.
        if !self.areas.session.conversation.turn_running()
            && !self.areas.conversation.permission_waits()
        {
            Delivery::Queue
        } else if self.prefs.settings.composer.delivery == TurnDelivery::Steer && self.steers() {
            Delivery::Steer
        } else {
            Delivery::Interrupt
        }
    }

    fn review_feedback_send(&mut self, recipient: &Recipient, cx: &mut Context<'_, Self>) {
        let (Some(offer), Some(machine)) = (self.review_offer(), self.review_machine()) else {
            return;
        };
        let Some(connection) = self.attachment_context(&machine) else {
            return;
        };
        let (key, sent, count) = (
            offer.key.clone(),
            offer.snapshot.sent.clone(),
            offer.snapshot.review.comments.len(),
        );
        let mut tray = ContextTray::default();
        let Ok(Some(context)) = tray
            .add(ContextAttachment::Review(Box::new(offer)))
            .and_then(|()| tray.prepare())
        else {
            return;
        };
        let prompt = Arc::new(WithFiles {
            expected_node: connection.node(),
            connection,
            message: vessel_proto::timeline::ClientMessageId::random(),
            machine,
            session: recipient.session,
            text: self.words.review().feedback_prompt().to_owned(),
            context: Some(context),
            attachments: Vec::new(),
            delivery: self.review_delivery(recipient.session),
        });
        let feedback = &mut self.areas.workspace.code.changes.review.feedback;
        feedback.failed = false;
        feedback.sending = Some(Sending {
            prompt: Arc::clone(&prompt),
            key,
            sent,
            title: recipient.title.clone(),
            count,
        });
        self.command(Command::Files(Files::Prompt(prompt)));
        cx.notify();
    }

    /// A feedback send's answer (`true`: it was this one, not a
    /// composer's): the sent versions cleared and the agent named, or the
    /// failure shown.
    pub(crate) fn review_feedback_answered(
        &mut self,
        prompt: &Arc<WithFiles>,
        result: &Result<Delivered, SubmitFailure>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let feedback = &mut self.areas.workspace.code.changes.review.feedback;
        if !feedback
            .sending
            .as_ref()
            .is_some_and(|sending| Arc::ptr_eq(&sending.prompt, prompt))
        {
            return false;
        }
        let Some(sending) = feedback.sending.take() else {
            return false;
        };
        if result.is_ok() {
            let _cleared = self
                .areas
                .composer
                .drafts
                .reviews_mut()
                .clear_sent(&sending.key, &sending.sent);
            self.drafts_changed(cx);
            self.review_offer_sync();
            let words = self.words.review().feedback_sent(&sending.title);
            self.toast(words.unwrap_or_default(), ToastTone::Ok, cx);
        } else {
            self.areas.workspace.code.changes.review.feedback.failed = true;
        }
        cx.notify();
        true
    }
}

/// The button, its notice and its chooser, over the Changes list; nothing
/// while no comment is kept.
pub(in super::super) fn surface(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let recipients = shell.review_recipients();
    let offer = shell.review_offer();
    let facts = shell.review_feedback_facts(offer.as_ref(), recipients.len());
    if facts.comments == 0 {
        return Vec::new();
    }
    let state = facts.state();
    let words = shell.words.review();
    let label = match &shell.review_feedback().sending {
        Some(sending) => words.feedback_sending(Number::count(count_of(sending.count))),
        None => words.feedback_send(Number::count(count_of(facts.sendable.unwrap_or(0)))),
    }
    .unwrap_or_default();
    let narrow = breakpoint::phone(window);
    let place = Rc::clone(&shell.review_feedback().place);
    let button = if narrow {
        super::feedback_draw::fab(label, state.can_send, tokens)
    } else {
        vessel_app_kit::button_with_content(
            "review-feedback",
            ButtonContent::IconAndLabel {
                icon: ButtonIcon::new(Glyph::Send),
                label: label.into(),
            },
            ButtonKind::Primary,
            if state.can_send {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            ControlSizing::from_compact(false),
            tokens,
        )
    };
    let button = if state.can_send {
        button.on_click(cx.listener(|shell, event: &ClickEvent, window, cx| {
            shell.review_feedback_pressed(event.position(), window, cx);
        }))
    } else {
        button
    };
    let button = div().relative().child(button).child(anchor_probe(&place));
    let mut drawn = vec![
        if narrow {
            div()
                .absolute()
                .bottom(px(spacing::S4))
                .right(px(spacing::S4))
                .child(button)
        } else {
            div()
                .absolute()
                .bottom(px(spacing::S4))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(button)
        }
        .into_any_element(),
    ];
    drawn.extend(
        super::feedback_draw::notice(shell, state.unavailable, tokens)
            .map(IntoElement::into_any_element),
    );
    drawn.extend(chooser(shell, tokens, window, cx));
    drawn
}

/// The chooser of agents (Paseo's `DropdownMenu`, a sheet titled
/// "Choose an agent" on a phone), each with its provider's mark.
fn chooser(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let feedback = shell.review_feedback();
    let items: Vec<MenuItem> = feedback
        .choices
        .iter()
        .map(|choice| {
            let item = MenuItem::new(choice.title.clone());
            match choice.mark.and_then(crate::agent_mark::artwork) {
                Some(artwork) => item.with_mark(artwork, LeadingSize::Regular),
                None => item.with_leading_at(Glyph::Bot, LeadingSize::Regular),
            }
        })
        .collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.workspace.code.changes.review.feedback.menu,
        chose: Rc::new(|shell: &mut Shell, index, _window, cx| {
            let chosen = shell
                .areas
                .workspace
                .code
                .changes
                .review
                .feedback
                .choices
                .get(index)
                .cloned();
            if let Some(recipient) = chosen {
                shell.review_feedback_send(&recipient, cx);
            }
        }),
    };
    let content = Content::new("review-feedback-chooser", &items)
        .with_sheet(shell.words.review().feedback_choose().to_owned())
        .with_width(px(CHOOSER));
    menu_with(
        content,
        &feedback.menu,
        &handlers,
        tokens,
        breakpoint::phone(window),
        cx,
    )
}
