// What a structured session's timeline is made of (DESIGN.md §6.5): its
// entries, the timeline as a whole, and a content block as text. The
// reducer that builds them from the session's records is data/timeline.ts.

import type {
  AgentEvent,
  AttachmentRef,
  ContentBlock,
  ModelCall,
  PermissionOutcome,
  PermissionRequest,
  PlanEntry,
  Question,
  QuestionOutcome,
  SessionOption,
  SessionStatus,
  StopReason,
  ToolCallContent,
  ToolCallStatus,
  ToolKind,
  ToolLocation,
  TurnUsage,
} from "../wire/proto";
import { type Conversation, EMPTY_CONVERSATION } from "./conversation";
import type { Usage } from "./models";

/** A tool call as it stands after its updates. */
export interface ToolState {
  id: string;
  title: string;
  kind: ToolKind;
  status: ToolCallStatus;
  content: ToolCallContent[];
  locations: ToolLocation[];
  input: unknown;
  output: unknown;
}

/** What every entry has: a stable key, its record, its time. */
export interface Base {
  key: string;
  seq: number;
  at: string;
}

export type Entry =
  | (Base & { kind: "ready"; harnessSession: string })
  | (Base & {
      kind: "prompt";
      text: string;
      replayed: boolean;
      turn: number;
      /** The files it carried (protocol 10). */
      attachments: readonly AttachmentRef[];
      /** A new session's first prompt shown before its log has it
       * (data/openings.ts); never set on a record of the log. */
      sending?: true;
    })
  /** A message added to the running turn (protocol 10's steering). */
  | (Base & {
      kind: "steer";
      text: string;
      attachments: readonly AttachmentRef[];
    })
  /** A question the agent asked, and how it was settled (`null` while it
   * waits); the form itself is docked over the composer. */
  | (Base & {
      kind: "question";
      question: Question;
      outcome: QuestionOutcome | null;
    })
  | (Base & { kind: "message"; text: string })
  | (Base & { kind: "thought"; text: string })
  | (Base & { kind: "tool"; tool: ToolState })
  | (Base & { kind: "plan"; entries: PlanEntry[] })
  | (Base & {
      kind: "permission";
      request: PermissionRequest;
      title: string;
      outcome: PermissionOutcome | null;
    })
  | (Base & {
      kind: "turn_ended";
      reason: StopReason;
      /** The turn's tokens, where the agent reported them. */
      usage: TurnUsage | null;
    })
  | (Base & { kind: "turn_failed"; message: string })
  | (Base & { kind: "log"; line: string })
  | (Base & { kind: "status"; status: SessionStatus })
  | (Base & { kind: "profile"; from: string | null; to: string | null })
  | (Base & { kind: "model"; call: ModelCall });

export interface Timeline {
  entries: readonly Entry[];
  /** The latest plan. */
  plan: readonly PlanEntry[] | null;
  /** Turns started so far. */
  turns: number;
  /** Whether a turn is running. */
  inTurn: boolean;
  /** The running turn's `turn_started` record, which Stop names (protocol
   * 10's `cancel_turn { turn }`); `null` between turns. */
  turn: number | null;
  /** The last record that asks for the person's attention (a turn's end
   * or failure, the session's end, a permission or a question), as the
   * node counts unread; -1 before one. */
  attention: number;
  /** The first permission request still unanswered. */
  pending: Extract<Entry, { kind: "permission" }> | null;
  /** Paths the session's tool calls touched, in first-touched order. */
  files: readonly string[];
  /** The session's status, as its records last said. */
  status: SessionStatus | null;
  /** Its model calls so far; `null` before the first. */
  usage: Usage | null;
  /** What the session runs with, as the agent last reported it (protocol
   * 9): its model, permissions, effort and whatever else it offers.
   * `null` until the node says, which a node older than protocol 9 never
   * does. Not an entry: the composer shows it, the timeline does not. */
  options: readonly SessionOption[] | null;
  /** The last record after which the session's saved login may be another
   * one (its status, profile or options changed, it was parked, woken or
   * rewound; native `usage::binding_epoch`); -1 before one. The session's
   * limit notice is read again after it. */
  binding: number;
  /** The session's title when it changed in this log; `null` when it has
   * not (the session's `SessionInfo.title` is then the one to show). */
  title: string | null;
  /** The next record expected. */
  next: number;
  /** What protocol 10's events say (data/conversation.ts). */
  conversation: Conversation;
  /** Where each tool call's entry is, by the call's id, and each
   * permission request's, by the request's id: entries are only ever
   * appended or replaced in place, never moved or removed, so an index
   * stays right. Without it every tool call searched the whole list, and
   * replaying a long log was quadratic (docs/PERFORMANCE.md: 35 s of long
   * tasks to fold 20,000 entries). */
  tools: ReadonlyMap<string, number>;
  permissions: ReadonlyMap<string, number>;
  /** Where each question's entry is, by the question's id. */
  questions: ReadonlyMap<string, number>;
}

export const EMPTY: Timeline = {
  entries: [],
  plan: null,
  turns: 0,
  inTurn: false,
  turn: null,
  attention: -1,
  pending: null,
  files: [],
  status: null,
  usage: null,
  options: null,
  binding: -1,
  title: null,
  next: 0,
  conversation: EMPTY_CONVERSATION,
  tools: new Map(),
  permissions: new Map(),
  questions: new Map(),
};

/** A content block as text. */
export function blockText(block: ContentBlock): string {
  switch (block.type) {
    case "text":
      return block.text;
    case "resource_link":
      return `[${block.name}](${block.uri})`;
    case "opaque":
      return "";
    case "attachment":
      return `[${block.attachment.name}]`;
  }
}

/** A prompt's words (its text and links) and its files, apart. */
export function promptParts(content: readonly ContentBlock[]): {
  text: string;
  attachments: AttachmentRef[];
} {
  const attachments: AttachmentRef[] = [];
  let text = "";
  for (const block of content) {
    if (block.type === "attachment") {
      attachments.push(block.attachment);
    } else {
      text += blockText(block);
    }
  }
  return { text, attachments };
}

/** A tool call as the agent announced it. */
export function toolOf(
  call: Extract<AgentEvent, { event: "tool_call" }>,
): ToolState {
  return {
    id: call.id,
    title: call.title,
    kind: call.kind,
    status: call.status,
    content: call.content,
    locations: call.locations,
    input: call.raw_input ?? null,
    output: null,
  };
}

/** An update to a tool call, as the agent reports it. */
export type ToolUpdate = Extract<AgentEvent, { event: "tool_call_update" }>;

/** The tool call `existing` (absent when the update came first) with
 * `update` applied: each field the update names replaces the old one. */
export function toolAfter(
  existing: ToolState | undefined,
  update: ToolUpdate,
): ToolState {
  return {
    id: update.id,
    title: update.title ?? existing?.title ?? "Tool call",
    kind: update.kind ?? existing?.kind ?? "other",
    status: update.status ?? existing?.status ?? "pending",
    content: update.content ?? existing?.content ?? [],
    locations: update.locations ?? existing?.locations ?? [],
    input: update.raw_input ?? existing?.input ?? null,
    output: update.raw_output ?? existing?.output ?? null,
  };
}

/** The kinds of entry that grow by chunks of text. */
export type TextKind = "prompt" | "message" | "thought";

/** `text` joined to the last of `entries` when that is a `kind` entry,
 * else a new `kind` entry started with it (a user chunk outside a turn is
 * a replayed prompt, of turn `turn`). `entries` is one batch's own copy
 * (data/timeline.ts) and is changed in place: only the entry that grew is
 * a new object, so only it renders again. */
export function appendText(
  entries: Entry[],
  base: Base,
  kind: TextKind,
  text: string,
  turn: number,
): void {
  const last = entries.at(-1);
  if (last?.kind === kind) {
    entries[entries.length - 1] = { ...last, text: last.text + text };
    return;
  }
  if (kind === "prompt") {
    entries.push({
      ...base,
      kind,
      text,
      replayed: true,
      turn,
      attachments: [],
    });
  } else {
    entries.push({ ...base, kind, text });
  }
}

/** The entry at `index` (an index from `Timeline.tools` or
 * `permissions`), if any. */
export function entryAt(
  entries: readonly Entry[],
  index: number | undefined,
): Entry | undefined {
  return index === undefined ? undefined : entries[index];
}

/** The first permission request among `permissions` still unanswered, in
 * log order: what the composer's dock asks about. */
export function firstPending(
  entries: readonly Entry[],
  permissions: ReadonlyMap<string, number>,
): Extract<Entry, { kind: "permission" }> | null {
  let first: Extract<Entry, { kind: "permission" }> | null = null;
  for (const index of permissions.values()) {
    const entry = entries[index];
    if (
      entry?.kind === "permission" &&
      entry.outcome === null &&
      (first === null || entry.seq < first.seq)
    ) {
      first = entry;
    }
  }
  return first;
}
