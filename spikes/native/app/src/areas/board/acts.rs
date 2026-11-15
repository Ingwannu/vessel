//! What a card's actions ask of its node (the page's board kill and
//! `use-mark-actions.tsx`): kill, archive and delete behind a confirm,
//! restore at once; the answer put on the board, said in a toast.
//!
//! The board takes the answers to the acts it asked (by machine, session
//! and kind); any other answer goes on to the conversation's.

use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Window, div,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _, breakpoint,
    button, button_with_kind, dialog,
};
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::Card;
use vessel_app_view::session::ENDS_BESIDE_PROTOCOL;
use vessel_app_view::toasts::ToastTone;
use vessel_proto::{Mark, SessionId};

use crate::routes::Route;
use crate::shell::Shell;

/// What a confirm asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// Kill a live session.
    Kill,
    /// Archive an ended one.
    Archive,
    /// Delete an archived one for good.
    Delete,
}

/// A card's session, and what is asked of it.
#[derive(Debug, Clone)]
pub(crate) struct Asking {
    ask: Ask,
    machine: String,
    session: SessionId,
    /// Its id's first four characters.
    short: String,
    /// It is running (archiving stops it first).
    live: bool,
}

impl Asking {
    pub(crate) fn of(ask: Ask, card: &Card) -> Self {
        Self {
            ask,
            machine: card.machine.clone(),
            session: card.session,
            short: card.short_id(),
            live: card.live,
        }
    }
}

/// An act asked from a card, until its answer.
#[derive(Debug, Clone)]
pub(crate) struct Asked {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
    kind: ActKind,
    short: String,
}

impl Shell {
    /// Open the confirm for `asking`.
    pub(crate) fn ask_card(
        &mut self,
        asking: Asking,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.board.confirm = Some(asking);
        self.areas.board.dialog.focus(window, cx);
        cx.notify();
    }

    /// Close the confirm unanswered.
    fn dismiss_card_confirm(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.board.confirm = None;
        self.refocus_archive(window, cx);
        cx.notify();
    }

    /// The confirm was accepted: ask the node.
    fn card_confirmed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(asking) = self.areas.board.confirm.take() else {
            return;
        };
        let act = match asking.ask {
            Ask::Kill => Act::Kill,
            Ask::Archive => Act::Mark(Mark::Archive),
            Ask::Delete => Act::Delete,
        };
        self.card_act(&asking, act);
        self.refocus_archive(window, cx);
        cx.notify();
    }

    /// Restore an archived card: start it again where its agent can pick
    /// its conversation up, else bring it back to its lane.
    pub(crate) fn restore_card(
        &mut self,
        target: &Asking,
        again: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let act = if again {
            Act::StartAgain
        } else {
            Act::Mark(Mark::Unarchive)
        };
        self.card_act(target, act);
        cx.notify();
    }

    fn card_act(&mut self, target: &Asking, act: Act) {
        self.send_act(
            target.machine.clone(),
            target.session,
            target.short.clone(),
            act,
        );
    }

    /// Act on `card`'s session from another screen (the sidebar's row
    /// menu): the answer comes back through [`Self::board_acted`], which
    /// puts the session's new state in the fleet, so the board, the
    /// archive and the sidebar show it at once, without a reload.
    pub(crate) fn act_on_card(&mut self, card: &Card, act: Act) {
        self.send_act(card.machine.clone(), card.session, card.short_id(), act);
    }

    /// Ask before archiving `card`'s session from another screen, with the
    /// board's own question (a live session is stopped first).
    pub(crate) fn ask_archive(
        &mut self,
        card: &Card,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.ask_card(Asking::of(Ask::Archive, card), window, cx);
    }

    /// Send `act` and keep it until its answer (`board_acted`).
    fn send_act(&mut self, machine: String, session: SessionId, short: String, act: Act) {
        self.areas.board.asked.push(Asked {
            machine: machine.clone(),
            session,
            kind: act.kind(),
            short,
        });
        self.command(Command::Act {
            machine,
            session,
            act,
        });
    }

    /// Over the archive, the archive takes the keys back.
    fn refocus_archive(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.route == Route::Archive {
            self.areas.board.query.focus(window, cx);
        }
    }

    /// Take the answer to an act a card asked; any other is given back.
    pub(crate) fn board_acted(
        &mut self,
        answer: ActAnswer,
        cx: &mut Context<'_, Self>,
    ) -> Option<ActAnswer> {
        let asked = &mut self.areas.board.asked;
        let Some(at) = asked.iter().position(|asked| {
            asked.machine == answer.machine
                && asked.session == answer.session
                && asked.kind == answer.kind
        }) else {
            return Some(answer);
        };
        let asked = asked.swap_remove(at);
        let words = self.t.lang.board();
        let said = self.t.lang.conversation();
        match answer.result {
            Ok(Acted::Session(info)) => self.fleet.put_session(&answer.machine, &info),
            Ok(Acted::Forked(info)) => self.fleet.add_session(&answer.machine, &info),
            Ok(Acted::Deleted(deleted)) => {
                self.fleet.drop_session(&answer.machine, answer.session);
                let kept = deleted
                    .worktree
                    .as_ref()
                    .map(|worktree| said.deleted_worktree(&worktree.outcome));
                let text = kept.map_or_else(
                    || said.session_deleted().to_owned(),
                    |kept| format!("{}. {kept}", said.session_deleted()),
                );
                self.toast(text, ToastTone::Ok, cx);
            }
            Ok(Acted::Done | Acted::Conversation(_) | Acted::Log(_)) => {}
            Err(failed) => {
                if answer.kind == ActKind::StartAgain {
                    self.toast(
                        words.again_refused(failed.code, &failed.message),
                        ToastTone::Error,
                        cx,
                    );
                    self.reload();
                } else {
                    self.toast(failed.message, ToastTone::Error, cx);
                }
                return None;
            }
        }
        if answer.kind == ActKind::Kill {
            self.toast(words.killed(&asked.short), ToastTone::Ok, cx);
            self.reload();
        }
        None
    }
}

/// The confirm over the board, when one is asked.
pub(crate) fn confirm(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let asking = shell.areas.board.confirm.as_ref()?;
    let narrow = breakpoint::phone(window);
    let words = shell.t.lang.board();
    let said = shell.t.lang.conversation();
    let (title, body, dismiss, accept) = match asking.ask {
        Ask::Kill => (
            words.kill_title(&asking.short),
            words.kill_body(&asking.machine),
            words.cancel,
            words.kill,
        ),
        Ask::Archive => {
            let beside = shell
                .fleet
                .protocol_of(&asking.machine)
                .is_some_and(|protocol| protocol >= ENDS_BESIDE_PROTOCOL);
            (
                said.archive_title().to_owned(),
                said.archive_words(asking.live, &asking.machine, beside),
                said.no(),
                said.confirm(),
            )
        }
        Ask::Delete => (
            said.delete_title().to_owned(),
            said.delete_words(&asking.machine),
            said.cancel(),
            said.delete(),
        ),
    };
    // Kill and delete cannot be undone: the kit's destructive button.
    let kind = if asking.ask == Ask::Archive {
        ButtonKind::Primary
    } else {
        ButtonKind::Destructive
    };
    let footer = vec![
        button("card-confirm-dismiss", dismiss, false, tokens)
            .on_click(cx.listener(|shell, _event, window, cx| {
                shell.dismiss_card_confirm(window, cx);
            }))
            .into_any_element(),
        button_with_kind(
            "card-confirm-accept",
            accept,
            kind,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.card_confirmed(window, cx)))
        .into_any_element(),
    ];
    Some(dialog(
        &shell.areas.board.dialog,
        DialogContent {
            title: title.into(),
            body: div().type_(Type::Body).child(body).into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::dismiss_card_confirm,
        cx,
    ))
}
