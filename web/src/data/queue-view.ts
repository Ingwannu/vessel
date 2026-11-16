// The node's queue as the composer draws it (milestone 11, Track
// A-page): each waiting prompt's words and files, who queued it, and,
// when the queue does not move on its own, why.
//
// The queue is the node's (ADR 0024): the page draws what the last
// `queue_changed` says and changes it only by asking the node, naming the
// revision it drew. Nothing here guesses a queue the node has not sent.

import type {
  AttachmentRef,
  ContentBlock,
  Creator,
  EventSeq,
  PromptQueue,
  QueueEntryId,
  QueueHold,
} from "../wire/proto";

/** One waiting prompt, as its row shows it. */
export interface QueueRow {
  id: QueueEntryId;
  revision: number;
  /** Its words (text blocks joined), for the row and the editor. */
  text: string;
  /** Display-only association; editing still requires explicit new words. */
  internalNotice?: EventSeq;
  /** The files it carries, kept as they are by an edit. */
  attachments: readonly AttachmentRef[];
  /** Who queued it, when not the person at this page's machine. */
  by: string | null;
  /** The entry it would go before, if moved up; `null` for the first. */
  before: QueueEntryId | null;
  /** The entry after its next one (moving down puts it before that one);
   * `null` with `last`: it goes to the end. */
  afterNext: QueueEntryId | null;
  first: boolean;
  last: boolean;
}

/** A prompt's words: its text blocks, in order. */
export function promptText(content: readonly ContentBlock[]): string {
  return content
    .map((block) => (block.type === "text" ? block.text : ""))
    .join("");
}

/** A prompt's files. */
export function promptAttachments(
  content: readonly ContentBlock[],
): AttachmentRef[] {
  const found: AttachmentRef[] = [];
  for (const block of content) {
    if (block.type === "attachment") {
      found.push(block.attachment);
    }
  }
  return found;
}

/** `text` with the prompt's files after it, as an edit sends it: the
 * words replaced, the files kept (the CLI's `with_text`). */
export function withText(
  content: readonly ContentBlock[],
  text: string,
): ContentBlock[] {
  const files: ContentBlock[] = content.filter(
    (block) => block.type === "attachment",
  );
  return text === "" ? files : [{ type: "text", text }, ...files];
}

function creatorName(by: Creator): string | null {
  switch (by.kind) {
    case "owner":
      return null;
    case "device":
      return "another device";
    case "agent":
      return "an agent";
  }
}

/** The queue's rows, first to go first. */
export function queueRows(queue: PromptQueue | null): QueueRow[] {
  const entries = queue?.entries ?? [];
  return entries.map((entry, index) => ({
    id: entry.id,
    revision: entry.revision,
    text: entry.provenance == null ? promptText(entry.content) : "",
    attachments:
      entry.provenance == null ? promptAttachments(entry.content) : [],
    ...(entry.provenance == null
      ? {}
      : { internalNotice: entry.provenance.notice }),
    by: creatorName(entry.by),
    before: index === 0 ? null : (entries[index - 1]?.id ?? null),
    afterNext: entries[index + 2]?.id ?? null,
    first: index === 0,
    last: index === entries.length - 1,
  }));
}

/** Why a held queue waits, in the page's words. */
export function holdLine(held: QueueHold): string {
  switch (held) {
    case "cancelled":
      return "Paused after you stopped the agent";
    case "failed":
      return "Paused after the turn failed";
    case "ended":
      return "Paused: the session ended";
    case "asked":
      return "Paused";
  }
}

/** Whether `queue` is held, and why; `null` when it moves on its own. */
export function heldBy(queue: PromptQueue | null): QueueHold | null {
  const held = queue?.held;
  return held === undefined ? null : held;
}

/** The `before` a move of the entry at `from` to place `to` names (both
 * places in the queue as it is; `to` past the end is last): the entry it
 * will go before once taken out, `null` for last, or `undefined` when it
 * would not move. */
export function moveBefore(
  rows: readonly QueueRow[],
  from: number,
  to: number,
): QueueEntryId | null | undefined {
  const place = Math.max(0, Math.min(rows.length - 1, to));
  if (place === from || rows[from] === undefined) {
    return undefined;
  }
  const rest = rows.filter((_, index) => index !== from);
  return rest[place]?.id ?? null;
}
