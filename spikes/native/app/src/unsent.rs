//! A command the link did not take ([`Unsent`]): what waited on its
//! answer settles, in the app's words, as it does when a machine refuses.
//!
//! [Decision Log]
//! - Purpose: `Link::send` dropped a command its queue did not take and
//!   only logged it, so a card's act, a pairing, a start or a workspace
//!   ask stayed pending with nothing coming (track G1's review,
//!   2026-11-13).
//! - Constraint: the shell sends from `&self` in many places (F's and
//!   G1's screens through `Shell::ask`, the conversation's `act_at`), and
//!   some callers set what waits only after sending (a resume, a start).
//! - Alternatives: settle at each call site (most have no `&mut`, and a
//!   settle there would run before the caller set what waits); wait for
//!   room in the queue (the UI thread would block).
//! - Choice: `Shell::command` submits; a command given back goes on the
//!   shell's own queue ([`receive`]) and settles on the next update,
//!   after the caller's code, through the handler a machine's refusal
//!   reaches, with the answer [`settle`] makes for it. Every caller that
//!   waits on an answer settles without a change of its own, the
//!   workspace screens of tracks F and G1 included.
//! - Cost: the settle comes one update after the send. An act not sent
//!   reads as the node's `unavailable` with the app's sentence as its
//!   words (`ActFailed` keeps its shape, so the board's and the
//!   conversation's matches stay); the devices page already says a full
//!   queue that way.

use tokio::sync::mpsc;
use vessel_app_kit::ui::{AsyncApp, WeakEntity};
use vessel_app_link::agents::AgentsError;
use vessel_app_link::attachments::{Files, Step, WithFiles};
use vessel_app_link::{ActAnswer, ActFailed, Command, Event, Rejected, Unsent, devices};
use vessel_app_view::composer::tray::{self, Again};
use vessel_app_view::idle_release;
use vessel_app_view::machine_book::Failure;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::{ErrorCode, SessionId};

use crate::shell::Shell;

/// How a command the link did not take settles.
#[derive(Debug)]
pub(crate) enum Settle {
    /// Actor bookkeeping has no UI request or result to fabricate.
    Ignore,
    /// The answer a refusal would bring: what waits on it settles as it
    /// does for that.
    Answer(Box<Event>),
    /// A prompt the composer had emptied: its text goes back.
    Prompt {
        /// The session it was for.
        session: SessionId,
        /// What was typed.
        text: String,
    },
    /// Nothing waits on an answer: the words are said once.
    Say,
}

/// How `unsent` settles; `words`, the app's sentence for it, in the
/// person's language.
pub(crate) fn settle(unsent: Unsent, words: &str) -> Settle {
    let said = || words.to_owned();
    let event = match *unsent.command {
        Command::RetireConnection(_tag) => return Settle::Ignore,

        Command::PromptIndex(ask) => {
            Event::PromptIndex(vessel_app_link::prompt_index::IndexAnswer {
                ask,
                delivery: None,
                result: Err(vessel_app_link::prompt_index::rejected(unsent.why)),
            })
        }
        // As the devices page: a full queue is the page's own "busy, try
        // again", a stopped link the machine's lost connection.
        Command::Agents {
            machine, request, ..
        } => Event::Agents {
            machine,
            request,
            answer: Err(match unsent.why {
                Rejected::Full => AgentsError::Busy,
                Rejected::Closed => AgentsError::Disconnected,
            }),
        },
        Command::Machines(ask) => Event::Machines(ask.refused(Failure::Unsent)),
        Command::Sync { ticket, .. } => Event::Sync(vessel_app_link::sync::Answer {
            ticket,
            result: Err(vessel_app_link::sync::rejected(unsent.why)),
        }),
        Command::Devices {
            context,
            ticket,
            ask,
        } => Event::Devices(devices::Answer {
            context,
            outcome: ask.refused(ticket, devices::rejected(unsent.why)),
        }),
        Command::NodeSettings(ask) => Event::NodeSettings(ask.refused(match unsent.why {
            Rejected::Full => idle_release::Problem::Refused(ErrorCode::Unavailable),
            Rejected::Closed => idle_release::Problem::Offline,
        })),
        Command::Start { .. } | Command::Files(Files::Create(_)) => Event::Failed(said()),
        Command::Options { machine, harness } => Event::Options {
            machine,
            harness,
            outcome: Err(said()),
        },
        Command::Resume { session, .. } => Event::ResumeFailed {
            session,
            why: said(),
        },
        Command::Act {
            machine,
            session,
            act,
        } => Event::Acted(ActAnswer {
            terminal: act.terminal_opening().cloned(),
            machine,
            session,
            kind: act.kind(),
            result: Err(ActFailed {
                code: Some(ErrorCode::Unavailable),
            }),
        }),
        Command::Pair { .. } => Event::PairFailed(vessel_app_view::pairing::PairProblem::Unsent),
        Command::Workspace { machine, ask } => Event::Workspace {
            machine,
            answer: ask.refused(&Problem::Unsent),
        },
        // The machine's page never asked: said under the Messages group as
        // that machine's failure, in the app's words.
        Command::Search(ask) => Event::Searched(vessel_app_link::search::Answer {
            machine: ask.machine,
            connection: ask.connection,
            delivery: None,
            ticket: ask.ticket,
            result: Err(vessel_app_link::search::Refusal::Failed {
                code: Some(ErrorCode::Unavailable),
            }),
        }),
        Command::Prompt { session, text, .. }
        | Command::Files(Files::Prompt(WithFiles { session, text, .. })) => {
            return Settle::Prompt { session, text };
        }
        // A file the link never took: failed, to be tried again.
        Command::Files(Files::Send(attach)) => Event::Upload {
            session: attach.session,
            key: attach.key,
            step: Step::Failed {
                problem: tray::Problem::Node(said()),
                again: Again::Restart,
            },
        },
        Command::Open { .. }
        | Command::Stop { .. }
        | Command::Answer { .. }
        | Command::Reload
        | Command::SavePrefs(_)
        | Command::KeepCommands { .. }
        | Command::SaveDrafts(_) => return Settle::Say,
    };
    Settle::Answer(Box::new(event))
}

/// The commands the link gave back, into the shell, until it closes.
pub(crate) async fn receive(
    this: WeakEntity<Shell>,
    mut returned: mpsc::UnboundedReceiver<Unsent>,
    cx: &mut AsyncApp,
) {
    while let Some(unsent) = returned.recv().await {
        if this
            .update(cx, |shell, cx| shell.unsent(unsent, cx))
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_app_link::workspace::{Answer, Ask, ProjectAsk};
    use vessel_app_link::{Act, ActKind, Command, Event, Link, Rejected};
    use vessel_app_view::Lang;
    use vessel_app_view::workspace::projects::ProjectsView;
    use vessel_proto::{ErrorCode, SessionId};

    use super::{Settle, settle};

    const WORDS: &str = "Could not send. Try again.";

    fn list() -> Command {
        Command::Workspace {
            machine: "laptop".to_owned(),
            ask: Ask::Project(ProjectAsk::List),
        }
    }

    /// The queue full: the command comes back, and the projects it asked
    /// for stop loading and say why in the app's words.
    #[test]
    fn a_full_queue_gives_the_ask_back_and_its_row_settles() {
        let (link, _inbox) = Link::detached(1).expect("a runtime");
        assert!(link.submit(list()).is_ok(), "the first fills the queue");
        let unsent = link.submit(list()).expect_err("a full queue gives it back");
        assert_eq!(unsent.why, Rejected::Full);
        let Settle::Answer(event) = settle(unsent, WORDS) else {
            panic!("an ask settles as its answer");
        };
        let Event::Workspace {
            machine,
            answer: Answer::Projects(result),
        } = *event
        else {
            panic!("a project list settles as a project list");
        };
        assert_eq!(machine, "laptop");
        let ProjectsView::Failed(problem) = ProjectsView::listed(result) else {
            panic!("the list no longer loads");
        };
        assert_eq!(Lang::En.workspace().problem(&problem), WORDS);
        assert_eq!(
            Lang::Ko.workspace().problem(&problem),
            "보내지 못했습니다. 다시 시도하세요."
        );
    }

    /// The link stopped: an act comes back as the answer its row waits
    /// for (the same machine, session and kind), refused with the words.
    #[test]
    fn a_stopped_link_settles_the_act_it_was_asked() {
        let (link, inbox) = Link::detached(1).expect("a runtime");
        drop(inbox);
        let session = SessionId::random();
        let unsent = link
            .submit(Command::Act {
                machine: "laptop".to_owned(),
                session,
                act: Act::StartAgain,
            })
            .expect_err("a stopped link gives it back");
        assert_eq!(unsent.why, Rejected::Closed);
        let Settle::Answer(event) = settle(unsent, WORDS) else {
            panic!("an act settles as its answer");
        };
        let Event::Acted(answer) = *event else {
            panic!("an act settles as an act's answer");
        };
        assert_eq!(
            (answer.machine.as_str(), answer.session, answer.kind),
            ("laptop", session, ActKind::StartAgain)
        );
        let failed = answer.result.expect_err("refused");
        assert_eq!(failed.code, Some(ErrorCode::Unavailable));
        assert_eq!(failed.message, WORDS);
    }

    /// A terminal read retains ownership even before a node can receive it.
    #[test]
    fn terminal_history_queue_refusal_keeps_its_opening_identity() {
        let (link, inbox) = Link::detached(1).expect("a runtime");
        drop(inbox);
        let session = SessionId::random();
        let opening = vessel_app_link::TerminalOpening::new("laptop".parse().unwrap(), session);
        let act = Act::TerminalLog {
            opening: opening.clone(),
            until: vessel_app_view::timeline::Until::Time(chrono::DateTime::UNIX_EPOCH),
        };
        let unsent = link
            .submit(Command::Act {
                machine: "laptop".into(),
                session,
                act,
            })
            .unwrap_err();
        let Settle::Answer(event) = settle(unsent, WORDS) else {
            panic!("an act receipt");
        };
        let Event::Acted(answer) = *event else {
            panic!("an act receipt");
        };
        assert_eq!(answer.terminal, Some(opening));
        assert!(answer.result.is_err());
    }

    #[test]
    fn internal_retirement_never_fabricates_an_answer_or_user_notice() {
        const COMMAND_CAPACITY: usize = 1;
        let (link, inbox) = Link::detached(COMMAND_CAPACITY).unwrap();
        drop(inbox);
        let context = vessel_app_view::connection::ConnectionEpochs::default()
            .admit(
                vessel_app_view::connection::MachineKey::from_book("laptop"),
                vessel_proto::NodeId::random(),
            )
            .unwrap();
        let unsent = link
            .submit(Command::RetireConnection(context.tag()))
            .unwrap_err();
        assert!(matches!(settle(unsent, WORDS), Settle::Ignore));
    }

    /// A prompt keeps its text; a command nothing waits on is said.
    #[test]
    fn a_prompt_comes_back_with_its_text() {
        let (link, inbox) = Link::detached(1).expect("a runtime");
        drop(inbox);
        let session = SessionId::random();
        let unsent = link
            .submit(Command::Prompt {
                machine: "laptop".to_owned(),
                session,
                text: "fix the test".to_owned(),
                delivery: vessel_proto::Delivery::Queue,
            })
            .expect_err("given back");
        let Settle::Prompt {
            session: for_session,
            text,
        } = settle(unsent, WORDS)
        else {
            panic!("a prompt settles as a prompt");
        };
        assert_eq!((for_session, text.as_str()), (session, "fix the test"));
        let reload = link.submit(Command::Reload).expect_err("given back");
        assert!(matches!(settle(reload, WORDS), Settle::Say));
    }
}
