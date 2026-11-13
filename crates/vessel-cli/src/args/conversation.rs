//! Protocol 10's command line (milestone 11, Track A): how `vessel send`
//! delivers a prompt and what it attaches, and the `vessel session`
//! commands on a conversation: its queue, answers, starting again, marks
//! and deletion.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use vessel_proto::Delivery;

use crate::selector::{IdPrefix, SessionSelector};

/// `vessel send`.
#[derive(Debug, Args)]
pub(crate) struct Send {
    /// Session id, or a unique prefix of one.
    pub(crate) session: SessionSelector,
    /// What to send.
    pub(crate) text: String,
    /// How a structured session takes it while a turn runs; without one,
    /// it starts a turn, refused while one runs.
    #[command(flatten)]
    pub(crate) delivery: Delivering,
    /// A file to send with the prompt, kept with the session (structured
    /// sessions; repeatable). Images and audio go to the agent inline, for
    /// agents that take them; any other file as a link it reads.
    #[arg(long = "attach", value_name = "FILE")]
    pub(crate) attach: Vec<PathBuf>,
}

/// `--queue`, `--steer` or `--interrupt`: at most one.
#[derive(Debug, Args)]
#[group(multiple = false)]
pub(crate) struct Delivering {
    /// While a turn runs (or prompts wait), wait in the session's queue;
    /// otherwise start now.
    #[arg(long)]
    queue: bool,
    /// Add it to the running turn (agents that take that); with no turn
    /// running, start one.
    #[arg(long)]
    steer: bool,
    /// Cancel the running turn, wait for it to end, then start this one.
    #[arg(long)]
    interrupt: bool,
}

impl Delivering {
    /// The delivery asked for, if one was.
    pub(crate) const fn delivery(&self) -> Option<Delivery> {
        if self.queue {
            Some(Delivery::Queue)
        } else if self.steer {
            Some(Delivery::Steer)
        } else if self.interrupt {
            Some(Delivery::Interrupt)
        } else {
            None
        }
    }
}

/// `vessel session ...` on a conversation (protocol 10).
#[derive(Debug, Subcommand)]
pub(crate) enum ConversationCommand {
    /// Open a side conversation, or ask in one, without changing the
    /// original session. Use this command for each later side question.
    Side {
        /// Original session, or an existing side, by id or unique prefix.
        session: SessionSelector,
        /// Optional question; opening an ended side does not resume it.
        question: Option<vessel_proto::automation::PromptText>,
    },
    /// Show a structured session's queue (the prompts waiting their turn,
    /// next first) and whether it is held; with a change, make it, then
    /// show the queue.
    Queue {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        #[command(subcommand)]
        change: Option<QueueCommand>,
    },
    /// Answer a question the session's agent asks (`field=value` for each
    /// field; a field of several choices once per value) or a permission
    /// it requests (the option's id); or decline either.
    Answer {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// The question's or the request's id, or a unique prefix of one
        /// (`vessel attach` and `vessel log` show them).
        request: IdPrefix,
        /// For a question, `field=value` pairs; for a permission, the
        /// option's id.
        values: Vec<String>,
        /// Decline the question, or refuse the permission.
        #[arg(long, conflicts_with = "values")]
        decline: bool,
    },
    /// Start an ended structured session again: its agent resumes the
    /// conversation under the session's profile, in the same log; prompts
    /// that were waiting come back held.
    Resume {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Stop a live session's idle agent now, keeping the session: its next
    /// prompt starts the agent again with the conversation (protocol 12).
    Park {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Start a parked session's agent again now.
    Wake {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Keep a session at the top of lists, on every device.
    Pin {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// No longer keep it at the top.
    Unpin {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Put a session away (the board's archive), stopping it first if it
    /// runs.
    Archive {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Bring an archived session back to the board.
    Unarchive {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Mark what the session has logged so far as seen.
    Read {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Mark a session unread until it is next read.
    Unread {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Delete an ended session for good: its log, marks, queue and
    /// attachments (the agent's own transcript stays). Asks first.
    Delete {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// Delete without asking (needed when not at a terminal).
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

/// `vessel session queue SESSION ...`: a change to the queue. Entries are
/// named by their id or a unique prefix of it.
#[derive(Debug, Subcommand)]
pub(crate) enum QueueCommand {
    /// Replace a waiting prompt's text; its attachments stay.
    Edit {
        /// The entry.
        entry: IdPrefix,
        /// Its new text.
        text: String,
        /// The revision the edit is made from, as `vessel session queue`
        /// showed it (`r<N>`): refused if someone edited it since. Default:
        /// the entry's current one.
        #[arg(long)]
        revision: Option<u32>,
    },
    /// Put a waiting prompt before another, or last.
    Move {
        /// The entry.
        entry: IdPrefix,
        /// The entry it goes before; default: the end.
        #[arg(long)]
        before: Option<IdPrefix>,
    },
    /// Take a waiting prompt out.
    Remove {
        /// The entry.
        entry: IdPrefix,
    },
    /// Take a waiting prompt out and send it now: start a turn (refused
    /// while one runs), or with --steer or --interrupt as `vessel send`.
    SendNow {
        /// The entry.
        entry: IdPrefix,
        #[command(flatten)]
        how: Now,
    },
    /// Stop the queue from moving on its own.
    Hold,
    /// Let it move again; between turns, its first prompt starts.
    Resume,
    /// Empty it.
    Clear,
}

/// How `send-now` sends: `--steer` or `--interrupt`, at most one.
#[derive(Debug, Args)]
#[group(multiple = false)]
pub(crate) struct Now {
    /// Add it to the running turn (agents that take that).
    #[arg(long)]
    steer: bool,
    /// Cancel the running turn first.
    #[arg(long)]
    interrupt: bool,
}

impl Now {
    /// The delivery: `start` unless a flag says otherwise.
    pub(crate) const fn delivery(&self) -> Delivery {
        if self.steer {
            Delivery::Steer
        } else if self.interrupt {
            Delivery::Interrupt
        } else {
            Delivery::Start
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;
    use crate::args::{Cli, Command, SessionCommand};

    fn conversation(args: &[&str]) -> ConversationCommand {
        let mut full = vec!["vessel", "session"];
        full.extend_from_slice(args);
        let Command::Session {
            command: SessionCommand::Conversation(command),
        } = Cli::try_parse_from(full).unwrap().command
        else {
            panic!("expected a conversation command: {args:?}");
        };
        command
    }

    #[test]
    fn send_takes_one_delivery_and_files() {
        let Command::Send(send) = Cli::try_parse_from([
            "vessel", "send", "3f2a", "next", "--queue", "--attach", "a.png", "--attach", "b.txt",
        ])
        .unwrap()
        .command
        else {
            panic!("expected send");
        };
        assert_eq!(send.delivery.delivery(), Some(Delivery::Queue));
        assert_eq!(
            send.attach,
            [PathBuf::from("a.png"), PathBuf::from("b.txt")]
        );
        let Command::Send(plain) = Cli::try_parse_from(["vessel", "send", "3f2a", "hi"])
            .unwrap()
            .command
        else {
            panic!("expected send");
        };
        assert_eq!(plain.delivery.delivery(), None);
        assert!(
            Cli::try_parse_from(["vessel", "send", "3f2a", "x", "--steer", "--queue"]).is_err()
        );
    }

    #[test]
    fn the_queue_is_shown_or_changed() {
        assert!(matches!(
            conversation(&["queue", "3f2a"]),
            ConversationCommand::Queue { change: None, .. }
        ));
        let ConversationCommand::Queue {
            change: Some(QueueCommand::Edit { text, revision, .. }),
            ..
        } = conversation(&[
            "queue",
            "3f2a",
            "edit",
            "9c",
            "new words",
            "--revision",
            "2",
        ])
        else {
            panic!("expected an edit");
        };
        assert_eq!((text.as_str(), revision), ("new words", Some(2)));
        let ConversationCommand::Queue {
            change: Some(QueueCommand::SendNow { how, .. }),
            ..
        } = conversation(&["queue", "3f2a", "send-now", "9c", "--interrupt"])
        else {
            panic!("expected send-now");
        };
        assert_eq!(how.delivery(), Delivery::Interrupt);
        assert!(matches!(
            conversation(&["queue", "3f2a", "move", "9c", "--before", "7d"]),
            ConversationCommand::Queue {
                change: Some(QueueCommand::Move {
                    before: Some(_),
                    ..
                }),
                ..
            }
        ));
        assert!(
            Cli::try_parse_from(["vessel", "session", "queue", "3f2a", "remove", "not-hex"])
                .is_err()
        );
    }

    #[test]
    fn an_answer_gives_values_or_declines() {
        let ConversationCommand::Answer {
            values, decline, ..
        } = conversation(&["answer", "3f2a", "51c", "way=left", "sure=yes"])
        else {
            panic!("expected answer");
        };
        assert_eq!(values, ["way=left", "sure=yes"]);
        assert!(!decline);
        assert!(matches!(
            conversation(&["answer", "3f2a", "51c", "--decline"]),
            ConversationCommand::Answer { decline: true, .. }
        ));
        assert!(
            Cli::try_parse_from([
                "vessel",
                "session",
                "answer",
                "3f2a",
                "51c",
                "way=left",
                "--decline"
            ])
            .is_err()
        );
    }

    #[test]
    fn marks_resume_and_delete_name_a_session() {
        for mark in [
            "pin",
            "unpin",
            "archive",
            "unarchive",
            "read",
            "unread",
            "resume",
        ] {
            let parsed = Cli::try_parse_from(["vessel", "session", mark, "3f2a"]);
            assert!(parsed.is_ok(), "{mark}: {parsed:?}");
        }
        assert!(matches!(
            conversation(&["delete", "3f2a", "--yes"]),
            ConversationCommand::Delete { yes: true, .. }
        ));
        assert!(matches!(
            conversation(&["delete", "3f2a"]),
            ConversationCommand::Delete { yes: false, .. }
        ));
    }
}
