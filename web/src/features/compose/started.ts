// What the new-session page does once the session exists: its first
// message on its way (data/openings.ts; with files, data/trays.ts), and
// the start kept for the machine (data/defaults.ts). Divided from
// new-session.tsx, which decides when.

import { rememberStart } from "../../data/defaults";
import type { Fleet } from "../../data/fleet";
import { deliverOpening } from "../../data/openings";
import { prefer } from "../../data/prefs";
import { handOver, sendWhenReady, trayOf } from "../../data/trays";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  HarnessKind,
  NodeId,
  SessionId,
  SessionSpec,
} from "../../wire/proto";

/** Send `prompt` to the new session `session` without waiting on it. */
export function sendFirst(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  spec: SessionSpec,
  prompt: string,
): void {
  if (spec.mode !== "structured") {
    return;
  }
  if (trayOf("new").items.get().length > 0) {
    // Files go once the session's agent can take them: its composer
    // uploads them and sends this prompt.
    handOver("new", session);
    fleet.drafts.type({ kind: "session", session }, prompt);
    sendWhenReady(session);
    return;
  }
  if (prompt === "") {
    return;
  }
  // The node answers once the agent has started; the session's page
  // shows the prompt meanwhile.
  void deliverOpening(fleet, node, session, prompt).then((state) => {
    if (state.kind === "refused") {
      toast(
        `The session started; the prompt did not: ${state.message}`,
        "error",
      );
    }
  });
}

/** Keep what `spec` started `agent` with on `node`. */
export function rememberStarted(
  fleet: Fleet,
  node: NodeId,
  agent: HarnessKind,
  profile: string,
  spec: SessionSpec,
): void {
  const chosen = spec.mode === "structured" ? (spec.options ?? []) : [];
  for (const kept of rememberStart(node, agent, profile, chosen)) {
    void prefer(fleet, kept);
  }
}
