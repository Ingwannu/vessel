//! A structured session's timeline as plain text, and what a typed line
//! means in that conversation.
//!
//! Kept free of I/O: [`Conversation::show`] turns one record into text to
//! print, and [`Conversation::interpret`] turns one typed line into an
//! action, so both are unit-tested.

use std::fmt::Write as _;

use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, PermissionId, PermissionOptionId, PermissionOutcome,
    PermissionRequest, PlanStatus, SessionEvent, StopReason, ToolCallContent, ToolCallStatus,
};

use crate::render;

/// What a typed line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Start a turn with this text.
    Prompt(String),
    /// Cancel the turn in progress.
    Cancel,
    /// Answer a pending permission request.
    Answer {
        request: PermissionId,
        option: PermissionOptionId,
    },
    /// Leave the conversation (the session keeps running).
    Detach,
    /// Tell the user something instead.
    Explain(String),
    /// Nothing (an empty line).
    Nothing,
}

/// Help shown when attaching. Ctrl-D is described as what it is, the end
/// of input: the attach then waits for the turns it started to finish
/// (`attach::structured`), so it does not leave at once during a turn the
/// way `/detach` does.
pub(crate) const HELP: &str = "type a message and press Enter; while a permission request is \
open, a number answers it; /cancel stops the turn; /detach leaves now; Ctrl-D ends input and \
leaves once the turn in progress has finished\n";

/// Which streamed message, if any, the current output line belongs to.
/// Chunks of the same message continue the line; anything else starts a
/// new one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Streaming {
    /// At the start of a line.
    #[default]
    Nothing,
    /// The agent's reply.
    Reply,
    /// A user message the harness replays on resume.
    UserReplay,
}

/// State needed to render a timeline: what is streaming on the current
/// line, and the permission requests still open (oldest first).
#[derive(Debug, Default)]
pub(crate) struct Conversation {
    streaming: Streaming,
    pending: Vec<PermissionRequest>,
}

impl Conversation {
    /// A selected gap must not join chunks from different messages on one line.
    pub(crate) fn finish_line(&mut self) -> &'static str {
        let ending = match self.streaming {
            Streaming::Nothing => "",
            Streaming::Reply | Streaming::UserReplay => "\n",
        };
        self.streaming = Streaming::Nothing;
        ending
    }

    /// The text for one record.
    pub(crate) fn show(&mut self, record: &EventRecord) -> String {
        match &record.event {
            SessionEvent::Status { status } => {
                self.line(format!("[session {}]", render::status(status)))
            }
            // A structured session has no terminal output; nothing to show.
            SessionEvent::Terminal(_) => String::new(),
            // A turn's end a fork or a rewind can start at says its record
            // (protocol 14): `vessel session fork|rewind ID --after N`.
            SessionEvent::Agent {
                event:
                    AgentEvent::TurnEnded {
                        reason,
                        point: Some(_),
                        ..
                    },
            } => self.line(format!(
                "[turn ended: {}; fork or rewind --after {}]",
                stop_reason(*reason),
                record.seq.get()
            )),
            SessionEvent::Agent { event } => self.agent(event),
            SessionEvent::ProfileChanged { from, to } => self.line(format!(
                "[profile {} -> {}]",
                render::profile(from.as_ref()),
                render::profile(to.as_ref())
            )),
            SessionEvent::ModelCall(call) => self.line(render::model_call(call)),
            SessionEvent::OptionsChanged { options } => {
                self.line(format!("[options: {}]", render::options(options)))
            }
            SessionEvent::Renamed { title, .. } => self.line(format!("[renamed: {title}]")),
            SessionEvent::QueueChanged { queue } => self.line(render::queue(queue)),
            SessionEvent::CommandsChanged { commands } => {
                self.line(format!("[commands: {}]", render::commands(commands)))
            }
            SessionEvent::UsageChanged { usage } => self.line(render::usage(usage)),
            SessionEvent::Parked { reason } => {
                self.line(format!("[agent parked: {}]", reason.describe()))
            }
            SessionEvent::Woken => self.line("[agent started again]".to_owned()),
            SessionEvent::Forked { from, after } => self.line(match after {
                Some(after) => format!(
                    "[a fork of session {from}: its conversation up to record {} (vessel log {from})]",
                    after.get()
                ),
                None => format!("[a fork of session {from} (vessel log {from})]"),
            }),
            SessionEvent::Rewound {
                after,
                files_changed,
                ..
            } => self.line(format!(
                "[rewound to record {}: what came after is no longer the conversation{}]",
                after.get(),
                if *files_changed > 0 {
                    format!("; {files_changed} file changes were not put back")
                } else {
                    String::new()
                }
            )),
        }
    }

    fn agent(&mut self, event: &AgentEvent) -> String {
        match event {
            AgentEvent::Ready {
                harness_session, ..
            } => self.line(format!("[ready: harness session {harness_session}]")),
            AgentEvent::TurnStarted { content, .. } => self.line(format!("> {}", text(content))),
            AgentEvent::UserMessageChunk { content } => {
                self.stream(Streaming::UserReplay, "> ", content)
            }
            AgentEvent::MessageChunk { content } => self.stream(Streaming::Reply, "", content),
            AgentEvent::ThoughtChunk { content } => {
                self.line(format!("(thinking) {}", block(content)))
            }
            AgentEvent::ToolCall(call) => self.line(format!(
                "[tool] {} ({:?}, {})",
                call.title,
                call.kind,
                tool_status(call.status)
            )),
            AgentEvent::ToolCallUpdate(update) => {
                let mut text = String::new();
                if let Some(status) = update.status {
                    let _infallible = write!(text, "[tool {}] {}", update.id, tool_status(status));
                }
                for content in update.content.iter().flatten() {
                    if let ToolCallContent::Diff { path, .. } = content {
                        let _infallible = write!(text, "\n  edits {}", path.display());
                    }
                }
                if text.is_empty() {
                    String::new()
                } else {
                    self.line(text)
                }
            }
            AgentEvent::Plan { entries } => {
                let mut text = "[plan]".to_owned();
                for entry in entries {
                    let mark = match entry.status {
                        PlanStatus::Pending => "[ ]",
                        PlanStatus::InProgress => "[~]",
                        PlanStatus::Completed => "[x]",
                    };
                    let _infallible = write!(text, "\n  {mark} {}", entry.content);
                }
                self.line(text)
            }
            AgentEvent::PermissionRequested(request) => {
                let mut text = format!(
                    "[permission] {} asks for approval:",
                    request.tool_call.as_ref().map_or("agent", |id| id.as_str())
                );
                for (number, option) in (1_u32..).zip(&request.options) {
                    let _infallible =
                        write!(text, "\n  {number}) {} ({:?})", option.name, option.kind);
                }
                text.push_str("\n  answer with a number");
                self.pending.push(request.clone());
                self.line(text)
            }
            AgentEvent::PermissionResolved { id, outcome } => {
                self.pending.retain(|request| request.id != *id);
                let outcome = match outcome {
                    PermissionOutcome::Selected { option } => format!("answered: {option}"),
                    PermissionOutcome::Cancelled => "withdrawn".to_owned(),
                };
                self.line(format!("[permission {outcome}]"))
            }
            AgentEvent::TurnEnded { reason, .. } => {
                self.line(format!("[turn ended: {}]", stop_reason(*reason)))
            }
            AgentEvent::TurnFailed { message } => self.line(format!("[turn failed: {message}]")),
            AgentEvent::Error { message } => self.line(format!("[error] {message}")),
            AgentEvent::Notification(notice) => {
                self.line(format!("[{:?}] {}", notice.level, notice.message))
            }
            AgentEvent::Compaction(compaction) => {
                self.line(format!("[context compaction {:?}]", compaction.status))
            }
            AgentEvent::Plugin(item) => self.line(format!("[plugin {}: {}]", item.plugin, item.id)),
            AgentEvent::HarnessLog { line } => self.line(format!("[harness] {line}")),
            AgentEvent::Steered { content, .. } => self.line(format!(">> {}", text(content))),
            AgentEvent::QuestionAsked(question) => self.line(render::question(question)),
            AgentEvent::QuestionResolved { id, outcome } => self.line(format!(
                "[question {id} {}]",
                render::question_outcome(outcome)
            )),
        }
    }

    /// `text` on a line of its own, ending any streamed message first.
    fn line(&mut self, mut text: String) -> String {
        if self.streaming != Streaming::Nothing {
            text.insert(0, '\n');
        }
        self.streaming = Streaming::Nothing;
        text.push('\n');
        text
    }

    /// One chunk of a streamed message: continues the line if the same
    /// message is streaming, otherwise starts a new line with `prefix`.
    fn stream(&mut self, message: Streaming, prefix: &str, content: &ContentBlock) -> String {
        let mut text = String::new();
        if self.streaming != message {
            if self.streaming != Streaming::Nothing {
                text.push('\n');
            }
            text.push_str(prefix);
        }
        self.streaming = message;
        text.push_str(&block(content));
        text
    }

    /// What `line` asks for, given the requests now open.
    pub(crate) fn interpret(&self, line: &str) -> Action {
        let line = line.trim();
        match line {
            "" => return Action::Nothing,
            "/cancel" => return Action::Cancel,
            "/detach" | "/quit" => return Action::Detach,
            _command if line.starts_with('/') => {
                return Action::Explain(format!("unknown command {line}; {HELP}"));
            }
            _text => {}
        }
        let Some(request) = self.pending.first() else {
            return Action::Prompt(line.to_owned());
        };
        let chosen = line
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|index| request.options.get(index));
        match chosen {
            Some(option) => Action::Answer {
                request: request.id,
                option: option.id.clone(),
            },
            None => Action::Explain(format!(
                "a permission request is open; answer with 1 to {}\n",
                request.options.len()
            )),
        }
    }
}

fn text(content: &[ContentBlock]) -> String {
    content.iter().map(block).collect()
}

fn block(content: &ContentBlock) -> String {
    match content {
        ContentBlock::Text { text } => text.clone(),
        ContentBlock::ResourceLink { uri, name } => format!("[{name}]({uri})"),
        ContentBlock::Opaque { .. } => "[attachment]".to_owned(),
        ContentBlock::Attachment { attachment } => format!("[attachment: {}]", attachment.name),
    }
}

const fn tool_status(status: ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "pending",
        ToolCallStatus::InProgress => "running",
        ToolCallStatus::Completed => "done",
        ToolCallStatus::Failed => "failed",
        ToolCallStatus::Canceled => "canceled",
    }
}

const fn stop_reason(reason: StopReason) -> &'static str {
    match reason {
        StopReason::EndTurn => "done",
        StopReason::MaxTokens => "token limit",
        StopReason::MaxTurnRequests => "request limit",
        StopReason::Refusal => "refused",
        StopReason::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use vessel_proto::{EventSeq, PermissionOption, PermissionOptionKind, ToolCallId};

    use super::*;

    fn agent(event: AgentEvent) -> EventRecord {
        EventRecord {
            seq: EventSeq::ZERO,
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent { event },
        }
    }

    fn chunk(text: &str) -> EventRecord {
        agent(AgentEvent::MessageChunk {
            content: ContentBlock::Text {
                text: text.to_owned(),
            },
        })
    }

    fn request() -> PermissionRequest {
        PermissionRequest {
            plan_proposal: None,
            delivery_state: Default::default(),
            tool_scope: None,
            id: PermissionId::random(),
            tool_call: Some(ToolCallId::new("call_1")),
            options: vec![
                PermissionOption {
                    id: PermissionOptionId::new("allow"),
                    name: "Allow".to_owned(),
                    kind: PermissionOptionKind::AllowOnce,

                    action: None,
                },
                PermissionOption {
                    id: PermissionOptionId::new("deny"),
                    name: "Deny".to_owned(),
                    kind: PermissionOptionKind::RejectOnce,

                    action: None,
                },
            ],

            presentation: None,
        }
    }

    #[test]
    fn replies_stream_inline_and_other_events_start_new_lines() {
        let mut view = Conversation::default();
        assert_eq!(view.show(&chunk("Hel")), "Hel");
        assert_eq!(view.show(&chunk("lo")), "lo");
        let ended = agent(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        });
        assert_eq!(view.show(&ended), "\n[turn ended: done]\n");
        assert_eq!(view.show(&ended), "[turn ended: done]\n");
        let pointed = agent(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: Some(vessel_proto::TurnPoint {
                message: vessel_proto::AgentMessageId::new("msg-1"),
                conversation: vessel_proto::HarnessSessionId::new("conversation"),
            }),
        });
        assert_eq!(
            view.show(&pointed),
            format!(
                "[turn ended: done; fork or rewind --after {}]\n",
                pointed.seq.get()
            )
        );
    }

    #[test]
    fn replayed_user_messages_get_their_own_lines() {
        let mut view = Conversation::default();
        let user = |text: &str| {
            agent(AgentEvent::UserMessageChunk {
                content: ContentBlock::Text {
                    text: text.to_owned(),
                },
            })
        };
        assert_eq!(view.show(&user("fix ")), "> fix ");
        assert_eq!(view.show(&user("it")), "it");
        assert_eq!(view.show(&chunk("On it.")), "\nOn it.");
        assert_eq!(view.show(&user("thanks")), "\n> thanks");
    }

    #[test]
    fn numbers_answer_the_oldest_open_permission_request() {
        let mut view = Conversation::default();
        assert_eq!(
            view.interpret("hi there"),
            Action::Prompt("hi there".to_owned())
        );
        let open = request();
        let shown = view.show(&agent(AgentEvent::PermissionRequested(open.clone())));
        assert!(shown.contains("1) Allow"));
        assert!(shown.contains("2) Deny"));
        assert_eq!(
            view.interpret("2"),
            Action::Answer {
                request: open.id,
                option: PermissionOptionId::new("deny"),
            }
        );
        assert!(matches!(view.interpret("3"), Action::Explain(_)));
        assert!(matches!(view.interpret("go on"), Action::Explain(_)));
        let resolved = agent(AgentEvent::PermissionResolved {
            id: open.id,
            outcome: PermissionOutcome::Cancelled,
        });
        assert_eq!(view.show(&resolved), "[permission withdrawn]\n");
        assert_eq!(view.interpret("2"), Action::Prompt("2".to_owned()));
    }

    #[test]
    fn slash_commands() {
        let view = Conversation::default();
        assert_eq!(view.interpret(" /cancel "), Action::Cancel);
        assert_eq!(view.interpret("/detach"), Action::Detach);
        assert_eq!(view.interpret(""), Action::Nothing);
        assert!(matches!(view.interpret("/frobnicate"), Action::Explain(_)));
    }
}
