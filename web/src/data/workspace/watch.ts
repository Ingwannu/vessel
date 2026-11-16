// The file a page shows follows the disk while it is shown (Paseo's
// `file-panels/file/handle.ts`, `setObserving` and `receive`): one
// finite `file.observe` at a time (the file panels' calls, ADR 0105),
// anchored on its last answer, decides when the page reads the file
// again. The same rule as the native panel's (`view::workspace::watch`;
// ADR 0036: one behaviour, checked on both sides).
//
// [Decision Log]
// - Purpose: a file changed by an agent, git or another editor shows its
//   new content without a reload by hand.
// - Constraint: the node's observation answers when the file differs
//   from `after`, at its heartbeat with the same state, or at once (a
//   forwarded call, no wait share left); a loop must not ask back to back.
// - Choice: Paseo's rule that a version the page already shows is no
//   change (a save of this page comes back as its own version); anything
//   else that differs from the anchor, or from the content shown on the
//   first answer, is a change. `pace.ts`'s quiet time is waited out after
//   every answer. A failed observation (an older node, a refusal, the
//   socket gone) ends the watch; the page then behaves as before.
// - Cost: one node observer and one wait share per shown file.

import type { ContentHash, ObservedFile } from "../../wire/proto";

/** Whether `now` is a change the page must read: `before` is the last
 * answer (null: the first), `shown` the hash of what the page shows
 * (null: not known, as for a file over 8 MiB). */
export function changed(
  before: ObservedFile | null,
  now: ObservedFile,
  shown: ContentHash | null,
): boolean {
  if (now.kind === "ready" && shown !== null && now.revision.hash === shown) {
    return false;
  }
  if (before === null) {
    return now.kind === "missing" || shown !== null;
  }
  return !sameObserved(before, now);
}

/** Two answers of one state. */
export function sameObserved(a: ObservedFile, b: ObservedFile): boolean {
  if (a.kind === "missing" || b.kind === "missing") {
    return a.kind === b.kind;
  }
  return (
    a.revision.hash === b.revision.hash && a.revision.size === b.revision.size
  );
}
