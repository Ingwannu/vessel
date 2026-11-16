// Local notice associations hide only the associated input presentation. The
// prompt entry remains a real turn boundary for folding, outline and lifecycle.
import type { AgentEvent, EventSeq } from "../wire/proto";
import { promptParts, type Entry } from "./timeline-model";

type InputEvent = Extract<AgentEvent, { event: "turn_started" | "steered" }>;

/** Only the typed association selects this projection, never text or an id prefix. */
export function inputParts(
  event: InputEvent,
): ReturnType<typeof promptParts> & { internalNotice?: EventSeq } {
  const notice = event.provenance?.notice;
  return notice === undefined
    ? promptParts(event.content)
    : { text: "", attachments: [], internalNotice: notice };
}

/** Display metadata is not admission or permission to execute a notice. */
export function isInternalInput(entry: Entry): boolean {
  return (
    (entry.kind === "prompt" || entry.kind === "steer") &&
    entry.internalNotice !== undefined
  );
}
