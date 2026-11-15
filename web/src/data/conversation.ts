// What a structured session's log says about its conversation beyond the
// timeline's entries (protocol 10, milestone 11's Track A): the queue, the
// agent's commands and context, what it can do, its questions, and the
// messages steered into a turn. The timeline's entries draw the steers
// and questions in place (timeline-agent.ts); this keeps what the
// composer reads: the queue, the commands, the context, the abilities and
// the questions still waiting.
//
// [Decision Log]
// - Purpose: the page must carry protocol 10's events (an exhaustive
//   switch would otherwise stop the build) and keep what they say, so the
//   next track draws from a model that already holds it.
// - Constraint: `timeline.ts` is at the page's file budget and shared with
//   other tracks; every line there is a merge risk.
// - Choice: a fold of its own over the same records, here; `timeline.ts`
//   only hands these events over (`isConversationEvent`,
//   `isConversationAgentEvent` narrow them out of its switches) and keeps
//   the result on the `Timeline` as `conversation`.
// - Each event replaces what it reports (the queue, the commands, the
//   context are whole each time), so the last record is the truth, as the
//   node's `session.conversation` says it.

import type {
  AgentAbilities,
  AgentCommand,
  AgentEvent,
  ContentBlock,
  ContextUsage,
  EventRecord,
  PromptQueue,
  Question,
  QuestionOutcome,
  SessionEvent,
  TurnUsage,
} from "../wire/proto";

/** A message steered into a running turn. */
export interface Steer {
  seq: number;
  at: string;
  content: readonly ContentBlock[];
}

/** A question the agent asked, and how it was settled (`null` while it
 * waits). */
export interface Asked {
  seq?: number;
  observed?: number;
  snapshotThrough?: number;
  available?: boolean;
  question: Question;
  outcome: QuestionOutcome | null;
}

export interface Conversation {
  /** The queue as its last `queue_changed` left it; `null` before one. */
  queue: PromptQueue | null;
  /** The commands the agent offers; `null` until it says. */
  commands: readonly AgentCommand[] | null;
  /** How full the context is, from the last `usage_changed`. */
  usage: ContextUsage | null;
  /** The last turn's tokens, when the agent reported them. */
  turnUsage: TurnUsage | null;
  /** What the agent can do, from its last `ready`. */
  abilities: AgentAbilities | null;
  questions: readonly Asked[];
  steers: readonly Steer[];
}

export const EMPTY_CONVERSATION: Conversation = {
  queue: null,
  commands: null,
  usage: null,
  turnUsage: null,
  abilities: null,
  questions: [],
  steers: [],
};

type ConversationEventType =
  "queue_changed" | "commands_changed" | "usage_changed";

type ConversationAgentEventType =
  "steered" | "question_asked" | "question_resolved";

/** A session event only the conversation folds. */
export type ConversationEvent = Extract<
  SessionEvent,
  { type: ConversationEventType }
>;

/** An agent event only the conversation folds. */
export type ConversationAgentEvent = Extract<
  AgentEvent,
  { event: ConversationAgentEventType }
>;

export function isConversationEvent(
  event: SessionEvent,
): event is ConversationEvent {
  return (
    event.type === "queue_changed" ||
    event.type === "commands_changed" ||
    event.type === "usage_changed"
  );
}

export function isConversationAgentEvent(
  event: AgentEvent,
): event is ConversationAgentEvent {
  return (
    event.event === "steered" ||
    event.event === "question_asked" ||
    event.event === "question_resolved"
  );
}

/** `conversation` with `records` applied, in order. */
export function foldConversation(
  conversation: Conversation,
  records: readonly EventRecord[],
): Conversation {
  return records.reduce(apply, conversation);
}

function apply(conversation: Conversation, record: EventRecord): Conversation {
  const event = record.event;
  if (isConversationEvent(event)) {
    return session(conversation, event);
  }
  if (event.type !== "agent") {
    return conversation;
  }
  const agent = event.event;
  if (agent.event === "ready") {
    return { ...conversation, abilities: agent.abilities ?? null };
  }
  if (agent.event === "turn_ended") {
    return { ...conversation, turnUsage: agent.usage ?? null };
  }
  if (isConversationAgentEvent(agent)) {
    return turn(conversation, record, agent);
  }
  return conversation;
}

function session(
  conversation: Conversation,
  event: ConversationEvent,
): Conversation {
  switch (event.type) {
    case "queue_changed":
      return { ...conversation, queue: event.queue };
    case "commands_changed":
      return { ...conversation, commands: event.commands };
    case "usage_changed":
      return { ...conversation, usage: event.usage };
  }
}

function turn(
  conversation: Conversation,
  record: EventRecord,
  event: ConversationAgentEvent,
): Conversation {
  switch (event.event) {
    case "steered":
      return {
        ...conversation,
        steers: [
          ...conversation.steers,
          { seq: record.seq, at: record.at, content: event.content },
        ],
      };
    case "question_asked": {
      const question: Question = event;
      const known = conversation.questions.some(
        (asked) => asked.question.id === question.id,
      );
      return {
        ...conversation,
        questions: known
          ? conversation.questions.map((asked) =>
              asked.question.id === question.id && asked.outcome === null
                ? { ...asked, question, observed: record.seq, available: true }
                : asked,
            )
          : [
              ...conversation.questions,
              { seq: record.seq, question, outcome: null },
            ],
      };
    }
    case "question_resolved":
      return {
        ...conversation,
        questions: conversation.questions.map((asked) =>
          asked.question.id === event.id
            ? { ...asked, outcome: event.outcome }
            : asked,
        ),
      };
  }
}

/** The questions still waiting for an answer, oldest first. */
export function waitingQuestions(conversation: Conversation): Asked[] {
  return conversation.questions.filter(
    (asked) => asked.outcome === null && asked.available !== false,
  );
}
