// Local display overlay for the retained prepared owner, not another send queue.
// Canonical echoes can deduplicate a row but cannot settle an uncertain receipt.
import type { AttachmentRef, ClientMessageId } from "../wire/proto";
import type { Timeline } from "./timeline-model";

export interface PendingPrompt {
  message: ClientMessageId;
  text: string;
  attachments: readonly AttachmentRef[];
  at: string;
  state: "preparing" | "waiting_for_echo" | "uncertain";
}

/** A local key never impersonates an EventSeq, turn identity or record cursor. */
export function pendingPromptKey(pending: PendingPrompt): string {
  return `pending:${pending.message}`;
}

/** Call only within the captured node/session owner. Text equality alone never
 * consumes a local echo, and contradictory same-id input stays visible.
 * Hiding this overlay never releases PreparedSubmissions or acknowledges delivery. */
export function visiblePendingPrompt(
  timeline: Timeline,
  pending: PendingPrompt | undefined,
): PendingPrompt | undefined {
  if (pending === undefined) return undefined;
  const echoed = timeline.entries.some(
    (entry) =>
      (entry.kind === "prompt" || entry.kind === "steer") &&
      entry.clientMessage === pending.message &&
      entry.text === pending.text &&
      entry.attachments.length === pending.attachments.length &&
      entry.attachments.every((attachment, index) =>
        sameAttachment(attachment, pending.attachments.at(index)),
      ),
  );
  return echoed ? undefined : pending;
}

function sameAttachment(
  a: AttachmentRef,
  b: AttachmentRef | undefined,
): boolean {
  return (
    b !== undefined &&
    a.id === b.id &&
    a.name === b.name &&
    a.kind === b.kind &&
    a.mime === b.mime &&
    a.size === b.size
  );
}
