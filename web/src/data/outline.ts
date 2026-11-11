// The outline of a session (GOAT's minimap, Paseo's "Chat outline"): one
// mark per prompt, with what the agent last said in reply, to jump
// between turns of a long conversation (DESIGN.md §6.16).
//
// From the conversation's model, as find is: the page holds the whole
// log whether or not the timeline draws all of it.

import { readableText } from "./find";
import type { Entry } from "./timeline-model";

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

/** The outline of `entries`, in order. */
export function outline(entries: readonly Entry[]): OutlineItem[] {
  const items: OutlineItem[] = [];
  let reply: string | null = null;
  const close = () => {
    const last = items.at(-1);
    if (last !== undefined) {
      last.reply = reply;
    }
  };
  for (const entry of entries) {
    if (entry.kind === "prompt") {
      close();
      reply = null;
      items.push({
        key: entry.key,
        prompt: preview(entry.text, PREVIEW),
        reply: null,
      });
    } else if (entry.kind === "message" && items.length > 0) {
      reply = preview(readableText(entry.text), REPLY);
    }
  }
  close();
  return items;
}
