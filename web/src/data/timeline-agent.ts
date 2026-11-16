// A structured session's agent events folded into the timeline's entries
// (data/timeline.ts keeps the batch; this is what each agent event does
// to it). Protocol 10 adds a prompt's files, messages steered into a
// turn, questions and a turn's tokens.

import type { AgentEvent, PlanEntry } from "../wire/proto";
import * as conversation from "./conversation";
import {
  type Base,
  blockText,
  type Entry,
  entryAt,
  promptParts,
  type TextKind,
  toolOf,
  type ToolState,
  type ToolUpdate,
} from "./timeline-model";

/** What an agent event changes of the batch (`timeline.ts`'s `Work`). */
export interface AgentWork {
  readonly entries: Entry[];
  plan: readonly PlanEntry[] | null;
  turns: number;
  inTurn: boolean;
  turn: number | null;
  attention: number;
  readonly permissions: Map<string, number>;
  readonly questions: Map<string, number>;
  merge: (base: Base, kind: TextKind, text: string) => void;
  findTool: (id: string) => Extract<Entry, { kind: "tool" }> | undefined;
  tool: (base: Base, tool: ToolState) => void;
  update: (base: Base, update: ToolUpdate) => void;
}

/** `event`, recorded at `base`, applied to `work`. */
export function applyAgent(
  work: AgentWork,
  base: Base,
  event: AgentEvent,
): void {
  if (conversation.isConversationAgentEvent(event)) {
    conversationEntry(work, base, event);
    return;
  }
  switch (event.event) {
    case "provider_child":
      // Child activity belongs to its own projection. Work still advances the
      // parent log cursor, without treating child turns or attention as its own.
      return;
    case "ready":
      work.entries.push({
        ...base,
        kind: "ready",
        harnessSession: event.harness_session,
      });
      return;
    case "turn_started": {
      work.turns += 1;
      work.inTurn = true;
      work.turn = base.seq;
      const { text, attachments } = promptParts(event.content);
      work.entries.push({
        ...base,
        kind: "prompt",
        text,
        replayed: false,
        turn: work.turns,
        attachments,
      });
      return;
    }
    case "user_message_chunk":
      work.merge(base, "prompt", blockText(event.content));
      return;
    case "message_chunk":
      work.merge(base, "message", blockText(event.content));
      return;
    case "thought_chunk":
      work.merge(base, "thought", blockText(event.content));
      return;
    case "tool_call":
      work.tool(base, toolOf(event));
      return;
    case "tool_call_update":
      work.update(base, event);
      return;
    case "plan":
      work.plan = event.entries;
      work.entries.push({ ...base, kind: "plan", entries: event.entries });
      return;
    case "permission_requested": {
      const call = work.findTool(event.tool_call);
      work.permissions.set(event.id, work.entries.length);
      work.attention = base.seq;
      work.entries.push({
        ...base,
        key: `p${event.id}`,
        kind: "permission",
        request: {
          id: event.id,
          tool_call: event.tool_call,
          options: event.options,
        },
        title: call?.tool.title ?? "A tool call",
        outcome: null,
      });
      return;
    }
    case "permission_resolved": {
      const index = work.permissions.get(event.id);
      const entry = entryAt(work.entries, index);
      if (index !== undefined && entry?.kind === "permission") {
        work.entries[index] = { ...entry, outcome: event.outcome };
      }
      return;
    }
    case "turn_ended":
      work.inTurn = false;
      work.turn = null;
      work.attention = base.seq;
      work.entries.push({
        ...base,
        kind: "turn_ended",
        reason: event.reason,
        usage: event.usage ?? null,
      });
      return;
    case "turn_failed":
      work.inTurn = false;
      work.turn = null;
      work.attention = base.seq;
      work.entries.push({
        ...base,
        kind: "turn_failed",
        message: event.message,
      });
      return;
    case "harness_log":
      work.entries.push({ ...base, kind: "log", line: event.line });
      return;
  }
}

/** A steered message, or a question asked or settled, as an entry. */
function conversationEntry(
  work: AgentWork,
  base: Base,
  event: conversation.ConversationAgentEvent,
): void {
  switch (event.event) {
    case "steered": {
      const { text, attachments } = promptParts(event.content);
      work.entries.push({ ...base, kind: "steer", text, attachments });
      return;
    }
    case "question_asked":
      work.questions.set(event.id, work.entries.length);
      work.attention = base.seq;
      work.entries.push({
        ...base,
        key: `q${event.id}`,
        kind: "question",
        question: {
          id: event.id,
          message: event.message,
          ...(event.tool_call === undefined
            ? {}
            : { tool_call: event.tool_call }),
          fields: event.fields,
        },
        outcome: null,
      });
      return;
    case "question_resolved": {
      const index = work.questions.get(event.id);
      const entry = entryAt(work.entries, index);
      if (index !== undefined && entry?.kind === "question") {
        work.entries[index] = { ...entry, outcome: event.outcome };
      }
      return;
    }
  }
}
