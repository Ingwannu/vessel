// When a running session's composer is drawn again (milestone 11, Track
// A-page's measurement).
//
// [Decision Log]
// - Purpose: the composer takes the whole timeline, and the timeline is a
//   new object for every frame of records, so the composer, its docks,
//   its controls and their tooltips were drawn again for each frame of a
//   long log being read, though nothing they show had changed.
//   `npm run perf` measured the opening of a 5,000-entry session slower
//   than before the composer grew (docs/plans/backend-m11.md, "Track
//   A-page: progress").
// - Constraint: what the composer shows comes from a few of the
//   timeline's fields, each kept as the same object while no record
//   changes it (`data/timeline.ts`, `data/conversation.ts`); the entries
//   are read only for the prompts Up recalls (a replayed prompt is one
//   too, and comes in pieces, so the number of turns does not tell) and
//   for the tool a pending permission names.
// - Alternatives: a narrower prop built in the session page (every
//   caller and every hook under the composer changed for it); leaving it
//   (a cost on every frame of every long log).
// - Choice: `memo` with this comparison: the same session and machine,
//   the same conversation, permission, turn, options and login's
//   record (`binding`, which the limit notice reads again after), the same
//   prompts sent (read once per entries' array and kept beside it, the
//   scan the composer already made for every frame), and the same tool
//   entry for a pending permission.
// - Cost: a field the composer comes to read from the timeline must be
//   added here, or it is drawn late; `composer-props.test.ts` lists what
//   is compared.

import { sentPrompts } from "../../data/recall";
import type { Entry, Timeline } from "../../data/timeline-model";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";

export interface ComposerProps {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  timeline: Timeline;
}

const prompts = new WeakMap<readonly Entry[], readonly string[]>();

/** The prompts sent in `entries`, newest first (`sentPrompts`), read once
 * per array. */
export function promptsOf(entries: readonly Entry[]): readonly string[] {
  const known = prompts.get(entries);
  if (known !== undefined) {
    return known;
  }
  const read = sentPrompts(entries);
  prompts.set(entries, read);
  return read;
}

function sameList(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((text, index) => text === b[index]);
}

/** The tool entry a pending permission names, if drawn. */
export function pendingTool(timeline: Timeline): Entry | undefined {
  const pending = timeline.pending;
  return pending === null
    ? undefined
    : timeline.entries.find(
        (entry) =>
          entry.kind === "tool" && entry.tool.id === pending.request.tool_call,
      );
}

/** Whether the composer would draw `next` as it drew `previous`. */
export function sameComposerProps(
  previous: ComposerProps,
  next: ComposerProps,
): boolean {
  const before = previous.timeline;
  const after = next.timeline;
  return (
    previous.node === next.node &&
    previous.label === next.label &&
    previous.session === next.session &&
    before.conversation === after.conversation &&
    before.pending === after.pending &&
    before.inTurn === after.inTurn &&
    before.turn === after.turn &&
    before.options === after.options &&
    before.binding === after.binding &&
    sameList(promptsOf(before.entries), promptsOf(after.entries)) &&
    (after.pending === null || pendingTool(before) === pendingTool(after))
  );
}
