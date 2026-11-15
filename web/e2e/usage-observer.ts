// Keep only sanitized usage replies and selection fields. Never retain
// complete WebSocket frames, launch URLs, credentials or session keys.
import type { Page } from "@playwright/test";

import { parse } from "../src/bridge/frames";
import type { UsageReport } from "../src/wire/proto";

interface Selection {
  node: string;
  period: "day" | "week";
}

interface SeenUsage {
  report: UsageReport | null;
  replies: number;
  selection: Selection | null;
}

/** Validating test edge for only the non-secret fields of a usage query. */
function selectionOf(payload: string): Selection | null {
  const frame: unknown = JSON.parse(payload);
  if (
    typeof frame !== "object" ||
    frame === null ||
    !("type" in frame) ||
    frame.type !== "call" ||
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
  const scope = params.scope;
  if (
    typeof scope !== "object" ||
    scope === null ||
    !("kind" in scope) ||
    scope.kind !== "machine"
  )
    return null;
  switch (params.period) {
    case "day":
    case "week":
      return { node: frame.node, period: params.period };
    default:
      return null;
  }
}

/** Test observer: memory stays bounded to one sanitized result and selection. */
export function watchUsage(page: Page): SeenUsage {
  const seen: SeenUsage = { report: null, replies: 0, selection: null };
  page.on("websocket", (socket) => {
    socket.on("framesent", ({ payload }) => {
      const selection = selectionOf(payload.toString());
      if (selection !== null) seen.selection = selection;
    });
    socket.on("framereceived", ({ payload }) => {
      const frame = parse(payload.toString());
      if (
        frame.type !== "reply" ||
        !("ok" in frame.outcome) ||
        frame.outcome.ok.method !== "usage.query"
      )
        return;
      seen.report = frame.outcome.ok.result;
      seen.replies += 1;
    });
  });
  return seen;
}
