// The outline of a session (GOAT's minimap, Paseo's "Chat outline"): one
// mark per prompt, with what the agent last said in reply, to jump
// between turns of a long conversation (DESIGN.md §6.16).
//
// From the conversation's model, as find is: the page holds the whole
// log whether or not the timeline draws all of it.

import { readableText } from "./find";
import type { Entry } from "./timeline-model";
import { isInternalInput } from "./timeline-input";

/** One prompt in the outline. */
export interface OutlineItem {
  /** The prompt entry's key, to reveal it. */
  key: string;
  /** Its text on one line, cut at `PREVIEW` characters (Paseo's 120). */
  prompt: string;
  /** The agent's last message of that turn, cut at `REPLY` (GOAT shows
   * three lines of it); `null` before it has answered. */
  reply: string | null;
}

/** The longest prompt preview, ellipsis included. */
export const PREVIEW = 120;

/** The longest reply preview. */
export const REPLY = 240;

/** The fewest prompts an outline is shown for (GOAT's and Paseo's). */
export const OUTLINE_MIN = 2;

/** `text` on one line, cut to `max` characters with an ellipsis. */
export function preview(text: string, max: number): string {
  const line = text.replace(/\s+/g, " ").trim();
  const characters = Array.from(line);
  return characters.length <= max
    ? line
    : `${characters.slice(0, max - 1).join("")}…`;
}

/** Each prompt's and each reply's preview, made once per entry object:
 * the outline is worked out again for every batch of records, and a
 * reply's Markdown is read through every pass of `readableText`. An
 * entry that did not change keeps its object (timeline.ts), so a batch
 * reads only what it changed. */
const previews = new WeakMap<Entry, string>();

function previewOf(entry: Entry, max: number, text: () => string): string {
  const known = previews.get(entry);
  if (known !== undefined) {
    return known;
  }
  const made = preview(text(), max);
  previews.set(entry, made);
  return made;
}

/** The outline of `entries`, in order. Only the last message of each
 * turn is read, once per entry object. */
export function outline(entries: readonly Entry[]): OutlineItem[] {
  const items: OutlineItem[] = [];
  let current: OutlineItem | null = null;
  let last: Entry | null = null;
  const close = () => {
    if (current !== null && last !== null && last.kind === "message") {
      const reply = last;
      current.reply = previewOf(reply, REPLY, () => readableText(reply.text));
    }
  };
  for (const entry of entries) {
    if (entry.kind === "prompt") {
      close();
      last = null;
      current = null;
      if (isInternalInput(entry)) continue;
      current = {
        key: entry.key,
        prompt: previewOf(entry, PREVIEW, () => entry.text),
        reply: null,
      };
      items.push(current);
    } else if (entry.kind === "message" && current !== null) {
      last = entry;
    }
  }
  close();
  return items;
}

/** Whether two outlines say the same: the page keeps the older one then,
 * so the outline draws nothing for a batch that changed no prompt or
 * reply. */
export function sameOutline(
  a: readonly OutlineItem[],
  b: readonly OutlineItem[],
): boolean {
  return (
    a.length === b.length &&
    a.every((item, index) => {
      const other = b[index];
      return (
        item.key === other?.key &&
        item.prompt === other.prompt &&
        item.reply === other.reply
      );
    })
  );
}
