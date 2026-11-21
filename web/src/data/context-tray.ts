// The context attached to one session's next message (TRACKER 833):
// Paseo's context-attachment pills (`attachments/types.ts` 58-82,
// `workspace-attachments-store.ts` 100-118, pinned 0.15.4): a pull
// request's comment, review or check logs, or a workspace file, each a
// pill in the composer's strip, sent as text with the prompt.
//
// [Decision Log]
// - Purpose: the pull request pane's and the explorer's "Add to chat"
//   put a pill in the composer, as Paseo's, instead of writing text into
//   the draft (P5b wrote text; the coordinator's TRACKER 833).
// - Constraint: the node takes a prompt as content blocks and has no
//   attachment kind for context; the native app sends each pill as one
//   more text block after the words and the files
//   (link/attachments/submit.rs), and the page sends what the native
//   sends. The bridge keeps a draft's text only (ADR 0028).
// - Alternatives: Paseo's scope, one tray per workspace shown in every
//   composer of it; text in the draft (P5b's, which the person had to
//   read past and could not take back as one piece).
// - Choice: one tray per session (the page's composer is per session and
//   the native app keys its trays the same way), the last 16 kept, in
//   memory for the tab's life, as the page's file tray (data/trays.ts).
//   A second add of the same kind and id replaces the first and moves to
//   the end (Paseo's `appendWorkspaceAttachment`); a prompt that went
//   takes only the pills it carried (one added meanwhile stays); a prompt
//   the node refused leaves them all.
// - Cost: a reload forgets the pills (Paseo keeps a workspace file pill
//   with its draft); a pill is text the page already holds, so nothing
//   is fetched on Send.

import type { AttachmentRef, ContentBlock } from "../wire/proto";
import { Store } from "./store";

/** What a pill is: Paseo's `forge.change_request_{comment,review,check}`
 * and `workspace_file`. */
export type ContextKind = "comment" | "review" | "check" | "file";

/** One pill. */
export interface ContextPill {
  readonly kind: ContextKind;
  /** The producer's identity; with the kind, the pill's key. */
  readonly id: string;
  /** The pill's first line: an author, a thread's place, a check, or a
   * file's name. */
  readonly title: string;
  /** A file's path under its name; null for a pull request's items,
   * whose second line is their kind's word. */
  readonly detail: string | null;
  /** What the agent reads. */
  readonly text: string;
}

/** A pill's key: one per kind and id. */
export function pillKey(pill: Pick<ContextPill, "kind" | "id">): string {
  return `${pill.kind}:${pill.id}`;
}

/** `current` with `added` in, each replacing a pill with its key and
 * going to the end (Paseo's order). */
export function appendPills(
  current: readonly ContextPill[],
  added: readonly ContextPill[],
): readonly ContextPill[] {
  const keys = new Set(added.map(pillKey));
  const kept = current.filter((pill) => !keys.has(pillKey(pill)));
  const unique = new Map(added.map((pill) => [pillKey(pill), pill]));
  return [...kept, ...unique.values()];
}

/** The text blocks a prompt carries for `pills`, in their order. */
export function pillTexts(pills: readonly ContextPill[]): string[] {
  return pills.map((pill) => pill.text).filter((text) => text.trim() !== "");
}

/** A prompt's content: the words, the uploaded files, then one text block
 * per pill (the native app's order, link/attachments/submit.rs). */
export function promptContent(
  words: string,
  files: readonly AttachmentRef[],
  pills: readonly ContextPill[],
): ContentBlock[] {
  const said = words.trim();
  return [
    ...(said === "" ? [] : [{ type: "text" as const, text: said }]),
    ...files.map((attachment) => ({
      type: "attachment" as const,
      attachment,
    })),
    ...pillTexts(pills).map((text) => ({ type: "text" as const, text })),
  ];
}

/** A prompt for a machine before the conversation calls, which takes
 * text only: the pills' texts after the words, a blank line apart. */
export function textPrompt(
  words: string,
  pills: readonly ContextPill[],
): string {
  return [words.trim(), ...pillTexts(pills)]
    .filter((part) => part !== "")
    .join("\n\n");
}

/** A workspace file, whole (Paseo's `workspaceFileAttachmentToAgentAttachment`:
 * the agent reads the path, not the file's content). */
export function filePill(path: string): ContextPill {
  const clean = path.trim().replace(/^\.\//, "");
  const name = clean.split("/").pop() ?? clean;
  return {
    kind: "file",
    id: `${clean}:whole_file`,
    title: name === "" ? clean : name,
    detail: clean,
    text: `Workspace file: ${clean}`,
  };
}

/** One session's pills. */
export class ContextTray {
  readonly pills = new Store<readonly ContextPill[]>([]);

  add(added: readonly ContextPill[]): void {
    if (added.length === 0) return;
    this.pills.update((current) => appendPills(current, added));
  }

  remove(key: string): void {
    this.pills.update((current) =>
      current.filter((pill) => pillKey(pill) !== key),
    );
  }

  /** Take out the pills a prompt carried. */
  sent(carried: readonly ContextPill[]): void {
    const keys = new Set(carried.map(pillKey));
    this.pills.update((current) =>
      current.filter((pill) => !keys.has(pillKey(pill))),
    );
  }
}

/** The trays of this tab, by session. */
const trays = new Map<string, ContextTray>();

/** As many as the file trays (data/trays.ts): the oldest goes beyond. */
const TRAYS = 16;

/** The pills of `session`'s composer, made when first asked. */
export function contextOf(session: string): ContextTray {
  const found = trays.get(session);
  if (found !== undefined) {
    // Most recent last, so the first is the one to drop.
    trays.delete(session);
    trays.set(session, found);
    return found;
  }
  const made = new ContextTray();
  trays.set(session, made);
  if (trays.size > TRAYS) {
    const [oldest] = trays.keys();
    if (oldest !== undefined) trays.delete(oldest);
  }
  return made;
}
