// What a session is called on the page (protocol 9, ADR 0022), and what
// a person may rename it to.
//
// The node keeps the one truth (`SessionInfo.title`) and parses a title
// on the way in (`SessionTitle`): trimmed, one line, no control
// characters, at most 120 characters typed or 80 taken from a prompt.
// The page checks the same rules before it asks, so a refusal is said
// beside the field as the person types, not after a round trip; the
// node still decides.

import type { SessionInfo } from "../wire/proto";
import { folderName } from "./format";

/** The most characters a typed title may have. */
export const TITLE_MAX = 120;

/** The most characters a title taken from a prompt keeps. */
export const PROMPT_TITLE_MAX = 80;

/** Characters, as the node counts them (Unicode scalar values). */
function characters(text: string): string[] {
  return Array.from(text);
}

// eslint's `no-control-regex` stays on; the class is built from code
// points so the source holds no raw control character.
const CONTROL = new RegExp(
  `[${String.fromCharCode(0)}-${String.fromCharCode(0x1f)}${String.fromCharCode(0x7f)}-${String.fromCharCode(0x9f)}]`,
  "u",
);

/** What an untitled session is called when it has no folder either
 * (agent-orchestrator's "Untitled session"; milestone 10 showed "~"). */
export const UNTITLED = "Untitled session";

/** What the page calls `session`: the title its log last reported
 * (`renamed`, fresher than the polled list), else the node's, else its
 * folder's name, as a session of an older node is called, else
 * "Untitled session". */
export function sessionName(
  session: SessionInfo,
  renamed: string | null = null,
): string {
  return (
    renamed ?? session.title ?? folderName(session.spec.cwd) ?? UNTITLED
  );
}

/** Whether `session` has a title of its own (not its folder's name). */
export function hasTitle(session: SessionInfo): boolean {
  return session.title !== undefined && session.title !== null;
}

/** `text` as a title, or why it cannot be one. */
export function checkTitle(
  text: string,
): { ok: true; title: string } | { ok: false; reason: string } {
  const title = text.trim();
  if (title === "") {
    return { ok: false, reason: "A title needs some text." };
  }
  if (CONTROL.test(title)) {
    return { ok: false, reason: "A title is one line of text." };
  }
  if (characters(title).length > TITLE_MAX) {
    return {
      ok: false,
      reason: `A title is at most ${String(TITLE_MAX)} characters.`,
    };
  }
  return { ok: true, title };
}

/** The title a prompt gives a session, as the node takes one from a first
 * prompt: its first line with text, cut at 80 characters. `null` when it
 * has no text. */
export function titleFromPrompt(text: string): string | null {
  const line = text
    .split(/\r\n|\r|\n/)
    .map((part) => part.trim())
    .find((part) => part !== "");
  if (line === undefined) {
    return null;
  }
  const kept = characters(line).slice(0, PROMPT_TITLE_MAX).join("").trim();
  return CONTROL.test(kept) ? null : kept;
}
