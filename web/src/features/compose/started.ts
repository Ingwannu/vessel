// What the new-session page does once the session exists: its first
// message on its way (data/openings.ts; with files, data/trays.ts), and
// the start kept for the machine (data/defaults.ts). Divided from
// new-session.tsx, which decides when.

import { rememberStart } from "../../data/defaults";
import type { Fleet } from "../../data/fleet";
import type { Draft, OptionsState } from "../../data/new-session";
import { deliverOpening, returnedDraft } from "../../data/openings";
import { keptChoices } from "../../data/picks";
import { prefer } from "../../data/prefs";
import { handOver, sendWhenReady, trayOf } from "../../data/trays";
import { toast } from "../../ui/toasts";
import type { DraftKey, NodeKey } from "../../wire/bridge";
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
  // shows the prompt meanwhile. Refused, it goes back in the session's
  // box, and the box says why (composer.tsx).
  void deliverOpening(fleet, node, session, prompt).then((state) => {
    if (state.kind === "refused") {
      const key: DraftKey = { kind: "session", session };
      fleet.drafts.type(key, returnedDraft(prompt, fleet.drafts.text(key)));
      toast(
        `The first message was not sent: ${state.message} It is back in the session's box.`,
        "error",
      );
    }
  });
}

/** Keep `agent` and what `draft` started it with as what a new session
 * on `node` starts with: every choice the agent offers (`keptChoices`),
 * or the remembered ones as they were when the agent had not said what
 * it offers. Only the four are kept per machine (`agent` is `null` for
 * an added one). */
export function rememberStarted(
  fleet: Fleet,
  node: NodeId | null,
  agent: HarnessKind | null,
  draft: Pick<Draft, "profile" | "picks">,
  options: OptionsState,
): void {
  if (node === null || agent === null) {
    return;
  }
  const choices = keptChoices(
    options.kind === "ready" ? options.options : null,
    draft.picks,
  );
  for (const kept of rememberStart(node, agent, draft.profile, choices)) {
    void prefer(fleet, kept);
  }
}
