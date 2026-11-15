// Typed tool display evidence is indexed by scope; native id spelling grants no authority.
import type {
  AgentEvent,
  ToolCallContent,
  ToolCallStatus,
  ToolKind,
  ToolLocation,
  ToolCallScope,
  ToolCallDetail,
  AgentMessageProvenance,
  ToolFailure,
} from "../wire/proto";

/** A tool call as it stands after its updates. */
export interface ToolState {
  id: string;
  scope?: ToolCallScope | null;
  detail?: ToolCallDetail | null;
  failure?: ToolFailure | null;
  agentMessage?: AgentMessageProvenance | null;
  title: string;
  kind: ToolKind;
  status: ToolCallStatus;
  content: ToolCallContent[];
  locations: ToolLocation[];
  input: unknown;
  output: unknown;
}

/** A tool call as the agent announced it. */
export function toolOf(
  call: Extract<AgentEvent, { event: "tool_call" }>,
): ToolState {
  return {
    id: call.id,
    scope: call.scope ?? null,
    detail: call.detail ?? null,
    failure: call.failure ?? null,
    agentMessage: call.agent_message ?? null,
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
    scope: update.scope ?? existing?.scope ?? null,
    detail: retainedDetail(existing?.detail, update.detail),
    failure: update.failure ?? existing?.failure ?? null,
    agentMessage: update.agent_message ?? existing?.agentMessage ?? null,
    title: update.title ?? existing?.title ?? "Tool call",
    kind: update.kind ?? existing?.kind ?? "other",
    status: update.status ?? existing?.status ?? "pending",
    content: update.content ?? existing?.content ?? [],
    locations: update.locations ?? existing?.locations ?? [],
    input: update.raw_input ?? existing?.input ?? null,
    output: update.raw_output ?? existing?.output ?? null,
  };
}

function retainedDetail(
  previous: ToolCallDetail | null | undefined,
  next: ToolCallDetail | null | undefined,
): ToolCallDetail | null {
  if (
    next === undefined ||
    next === null ||
    (next.type === "unknown" &&
      previous?.type !== "unknown" &&
      previous !== undefined &&
      previous !== null)
  )
    return previous ?? null;
  return next;
}

/** Structural tuple tags cannot collide with a malicious native id spelling. */
export function toolKey(id: string, scope?: ToolCallScope | null): string {
  switch (scope?.kind) {
    case undefined:
      return JSON.stringify(["tool", id]);
    case "provider_turn":
      return JSON.stringify(["tool", id, "provider", scope.thread, scope.turn]);
    case "session_turn":
      return JSON.stringify(["tool", id, "session", scope.started]);
    case "record":
      return JSON.stringify(["tool", id, "record", scope.seq]);
  }
}
