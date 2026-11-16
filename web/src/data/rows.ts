// What a row of the timeline's window is (features/session/use-window.ts),
// once "Collapse tool calls" (tool-runs.ts) has folded some entries away.
//
// [Decision Log] (milestone 11: tracks P and W together)
// - Purpose: three things count the timeline's items. The window (track P)
//   lays out rows by index and measures them by key; the reducer (track P,
//   timeline.ts) finds a tool call's entry by its index in `entries`; find
//   and the outline (track W) name an entry by its key. Folding a run of
//   tool calls into one row (track W) makes rows and entries differ.
// - Choice: a row is an entry that is drawn: every entry, less the members
//   of a closed run other than its lead, which draws the run's summary
//   (and itself, once the run is opened). Indices into `entries` stay in
//   the reducer; the window's indices are into these rows; only keys cross
//   between the two, and `rowIndex` turns any entry's key into the row
//   that shows it (a folded member's is its run's lead).
// - Alternatives: every entry a row, a folded one drawing nothing (track W
//   before the merge): thousands of empty rows the window would measure
//   and lay out at the guessed height until drawn, so a far key's
//   estimate drifts by the guess times the folded entries.
// - Cost: the rows are worked out again when the entries, the folds or the
//   open runs change: one pass over the entries. With nothing folded they
//   are the entries themselves (the same array).

import type { Entry } from "./timeline-model";
import type { Folds } from "./tool-runs";
import { isInternalInput } from "./timeline-input";

/** The entries drawn as the window's rows, in order. */
export function rowsOf(
  entries: readonly Entry[],
  folds: Folds | null,
  open: ReadonlySet<string>,
): readonly Entry[] {
  if (folds === null && !entries.some(isInternalInput)) {
    return entries;
  }
  return entries.filter((entry) => {
    if (isInternalInput(entry)) return false;
    const run = folds?.get(entry.key);
    return run === undefined || run.lead === entry.key || open.has(run.lead);
  });
}

/** Where in `rows` the row that shows entry `key` is: its own, or for an
 * entry folded away, its run's lead; -1 when it is not in the log. */
export function rowIndex(
  rows: readonly Entry[],
  key: string,
  folds: Folds | null,
): number {
  const own = rows.findIndex((row) => row.key === key);
  if (own >= 0 || folds === null) {
    return own;
  }
  const lead = folds.get(key)?.lead;
  return lead === undefined ? -1 : rows.findIndex((row) => row.key === lead);
}
