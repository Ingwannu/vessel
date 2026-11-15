// Keep only sanitized accounting replies and request identities. Never
// retain complete frames, credentials, launch URLs or authentication keys.
import type { Page } from "@playwright/test";
import { parse } from "../src/bridge/frames";
import type { RollingUsageScope } from "../src/data/usage/scope";
import type { HarnessRef, UsageReport } from "../src/wire/proto";

interface Selection {
  node: string;
  period: "day" | "week";
}
interface Request extends Selection {
  id: number;
  scope: RollingUsageScope;
}
interface SeenUsage {
  report: UsageReport | null;
  replies: number;
  selection: Selection | null;
  requestedScope: RollingUsageScope | null;
  reportedScope: RollingUsageScope | null;
}

function harnessOf(value: unknown): HarnessRef | null {
  if (
    typeof value === "object" &&
    value !== null &&
    "added" in value &&
    typeof value.added === "string"
  )
    return { added: value.added };
  switch (value) {
    case "claude-code":
    case "codex":
    case "gemini":
    case "open-code":
      return value;
    default:
      return null;
  }
}

function scopeOf(value: unknown): RollingUsageScope | null {
  if (typeof value !== "object" || value === null || !("kind" in value))
    return null;
  if (value.kind === "machine") return { kind: "machine" };
  if (value.kind !== "agent" || !("harness" in value) || !("profile" in value))
    return null;
  const harness = harnessOf(value.harness);
  if (
    harness === null ||
    (value.profile !== null && typeof value.profile !== "string")
  )
    return null;
  return { kind: "agent", harness, profile: value.profile };
}

/** Decode only this observer's non-secret fields at its test boundary. */
function requestOf(payload: string): Request | null {
  const frame: unknown = JSON.parse(payload);
  if (
    typeof frame !== "object" ||
    frame === null ||
    !("type" in frame) ||
    frame.type !== "call" ||
    !("id" in frame) ||
    typeof frame.id !== "number" ||
    !Number.isSafeInteger(frame.id) ||
    !("node" in frame) ||
    typeof frame.node !== "string" ||
    !("call" in frame)
  )
    return null;
  const call = frame.call;
  if (
    typeof call !== "object" ||
    call === null ||
    !("method" in call) ||
    call.method !== "usage.query" ||
    !("params" in call)
  )
    return null;
  const params = call.params;
  if (
    typeof params !== "object" ||
    params === null ||
    !("scope" in params) ||
    !("period" in params)
  )
    return null;
  const scope = scopeOf(params.scope);
  if (scope === null) return null;
  switch (params.period) {
    case "day":
    case "week":
      return { id: frame.id, node: frame.node, period: params.period, scope };
    default:
      return null;
  }
}

/** One pending query per socket and one sanitized result; a reply's
 * scope is recorded only when its id matches the observed request. */
export function watchUsage(page: Page): SeenUsage {
  const seen: SeenUsage = {
    report: null,
    replies: 0,
    selection: null,
    requestedScope: null,
    reportedScope: null,
  };
  page.on("websocket", (socket) => {
    let pending: Request | null = null;
    socket.on("framesent", ({ payload }) => {
      const request = requestOf(payload.toString());
      if (request !== null) {
        pending = request;
        seen.selection = { node: request.node, period: request.period };
        seen.requestedScope = request.scope;
      }
    });
    socket.on("framereceived", ({ payload }) => {
      const frame = parse(payload.toString());
      if (frame.type !== "reply" || frame.id !== pending?.id) return;
      if ("ok" in frame.outcome && frame.outcome.ok.method === "usage.query") {
        seen.report = frame.outcome.ok.result;
        seen.reportedScope = pending.scope;
        seen.replies += 1;
      }
      pending = null;
    });
  });
  return seen;
}
