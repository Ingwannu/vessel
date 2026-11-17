// A structured session's timeline: its records, folded into entries with
// stable keys (DESIGN.md §6.5; ADR 0017).
//
// [Decision Log]
// - Purpose: the session view renders a log, not the raw records: chunks
//   of one message merged, a tool call updated in place, a permission
//   request with its answer, the latest plan.
// - Choice: a pure reducer over `EventRecord`s in sequence order, fed in
//   batches. It copies the entry list once per batch and replaces only the
//   entries that change, so React re-renders those alone. Tool calls and
//   permission requests are found through the timeline's index by id,
//   never by a search, so replaying a long log is linear (milestone 11,
//   docs/PERFORMANCE.md). Keys come from sequence numbers and the
//   protocol's ids, so an entry keeps its key while it grows.
// - Model calls (protocol 7) are entries too, and summed into `usage` as
//   they arrive, so the inspector's totals cost nothing per render. They
//   carry metadata only; the record holds no content to show.
// - It is plain TypeScript with no React, tested on its own, including
//   against records a real node produced with `fake-acp`.

import type {
  AgentEvent,
  EventRecord,
  PlanEntry,
  SessionOption,
  SessionStatus,
  ToolLocation,
} from "../wire/proto";
import * as conversation from "./conversation";
import { addCall, type Usage } from "./models";
import { applyAgent } from "./timeline-agent";
import {
  appendText,
  type Base,
  type Entry,
  entryAt,
  firstPending,
  type TextKind,
  type Timeline,
  toolAfter,
  type ToolState,
  type ToolUpdate,
} from "./timeline-model";

/** `timeline` with `records` applied (records already seen are skipped). */
export function reduce(
  timeline: Timeline,
  records: readonly EventRecord[],
): Timeline {
  const fresh = records.filter((record) => record.seq >= timeline.next);
  if (fresh.length === 0) {
    return timeline;
  }
  const work = new Work(timeline);
  for (const record of fresh) {
    work.apply(record);
  }
  const folded = conversation.foldConversation(timeline.conversation, fresh);
  return { ...work.finish(), conversation: folded };
}

/** The records after which the session may run under another saved
 * login, so its limit notice is read again (native view/src/usage/session.rs
 * `binding_epoch`, the same list). */
const BINDING: ReadonlySet<string> = new Set([
  "status",
  "profile_changed",
  "options_changed",
  "parked",
  "woken",
  "rewound",
]);

/** One batch's mutable copy. */
class Work {
  readonly entries: Entry[];
  plan: readonly PlanEntry[] | null;
  turns: number;
  inTurn: boolean;
  turn: number | null;
  attention: number;
  readonly files: string[];
  /** `files` as a set, for the membership test. */
  readonly touched: Set<string>;
  readonly tools: Map<string, number>;
  readonly permissions: Map<string, number>;
  readonly questions: Map<string, number>;
  status: SessionStatus | null;
  usage: Usage | null;
  options: readonly SessionOption[] | null;
  binding: number;
  title: string | null;
  next: number;

  constructor(timeline: Timeline) {
    this.entries = [...timeline.entries];
    this.plan = timeline.plan;
    this.turns = timeline.turns;
    this.inTurn = timeline.inTurn;
    this.turn = timeline.turn;
    this.attention = timeline.attention;
    this.files = [...timeline.files];
    this.touched = new Set(timeline.files);
    this.tools = new Map(timeline.tools);
    this.permissions = new Map(timeline.permissions);
    this.questions = new Map(timeline.questions);
    this.status = timeline.status;
    this.usage = timeline.usage;
    this.options = timeline.options;
    this.binding = timeline.binding;
    this.title = timeline.title;
    this.next = timeline.next;
  }

  finish(): Omit<Timeline, "conversation"> {
    return {
      entries: this.entries,
      plan: this.plan,
      turns: this.turns,
      inTurn: this.inTurn,
      turn: this.turn,
      attention: this.attention,
      pending: firstPending(this.entries, this.permissions),
      files: this.files,
      status: this.status,
      usage: this.usage,
      options: this.options,
      binding: this.binding,
      title: this.title,
      next: this.next,
      tools: this.tools,
      permissions: this.permissions,
      questions: this.questions,
    };
  }

  apply(record: EventRecord): void {
    this.next = record.seq + 1;
    const base = { key: `r${record.seq}`, seq: record.seq, at: record.at };
    const event = record.event;
    if (conversation.isConversationEvent(event)) return;
    if (BINDING.has(event.type)) this.binding = record.seq;
    switch (event.type) {
      case "status":
        this.status = event.status;
        if (
          event.status.state !== "starting" &&
          event.status.state !== "running"
        ) {
          this.inTurn = false;
          this.turn = null;
          this.attention = record.seq;
        }
        this.entries.push({ ...base, kind: "status", status: event.status });
        return;
      case "terminal":
        return;
      case "profile_changed":
        this.entries.push({
          ...base,
          kind: "profile",
          from: event.from ?? null,
          to: event.to ?? null,
        });
        return;
      case "agent":
        this.agent(base, event.event);
        return;
      case "model_call":
        // The record itself: it is metadata only (route, model, tokens,
        // how it ended, how long).
        this.entries.push({ ...base, kind: "model", call: event });
        this.usage = addCall(this.usage, event);
        return;
      case "options_changed":
        // The whole set each time, so the last one is the truth.
        this.options = event.options;
        return;
      case "renamed":
        this.title = event.title;
        return;
      case "parked":
      case "woken":
        // Protocol 12 (ADR 0038): the agent's process was stopped while
        // idle, or started again. Nothing in the conversation changes and
        // the status stays running; `SessionInfo.parked` carries the state
        // for whoever shows it.
        return;
      case "forked":
      case "rewound":
        // Protocol 14 (ADR 0052): where a fork's conversation comes from,
        // and a rewind's turn. Drawing them (the source's history above a
        // fork, the left-behind turns folded) is the conversation track's.
        return;
    }
  }

  agent(base: Base, event: AgentEvent): void {
    applyAgent(this, base, event);
  }

  /** A chunk of text joined to the entry it continues, or starting one. */
  merge(base: Base, kind: TextKind, text: string): void {
    appendText(this.entries, base, kind, text, this.turns);
  }

  findTool(id: string): Extract<Entry, { kind: "tool" }> | undefined {
    const entry = entryAt(this.entries, this.tools.get(id));
    return entry?.kind === "tool" ? entry : undefined;
  }

  tool(base: Base, tool: ToolState): void {
    this.touch(tool.locations);
    const index = this.tools.get(tool.id);
    const existing = entryAt(this.entries, index);
    if (index !== undefined && existing?.kind === "tool") {
      this.entries[index] = { ...existing, tool };
      return;
    }
    this.tools.set(tool.id, this.entries.length);
    this.entries.push({ ...base, key: `t${tool.id}`, kind: "tool", tool });
  }

  update(base: Base, update: ToolUpdate): void {
    this.tool(base, toolAfter(this.findTool(update.id)?.tool, update));
  }

  touch(locations: readonly ToolLocation[]): void {
    for (const location of locations) {
      if (!this.touched.has(location.path)) {
        this.touched.add(location.path);
        this.files.push(location.path);
      }
    }
  }
}
