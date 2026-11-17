// The view menu's label filter (Paseo `sidebar-labels.ts`): which
// workspaces the sidebar lists when the person picked labels.
//
// [Decision Log]
// - Purpose: show only the workspaces carrying some chosen labels, or
//   those carrying none.
// - Reference: Paseo keeps "Unlabelled" as one more key in the same list,
//   not a flag beside it, and several picks include a workspace carrying
//   any of them; no pick lists everything.
// - Choice: the same, with `UNLABELLED` (the empty key, which no real name
//   folds to) as that key. The filter runs on the page over the registry
//   snapshot; it asks the node nothing (coordinator ruling (a), session
//   key/value labels stay with the lifecycle owner).

import { labelKey, UNLABELLED } from "./names";

/** Whether a workspace carrying `names` passes the picks `selected`. */
export function passesLabels(
  names: readonly string[],
  selected: readonly string[],
): boolean {
  if (selected.length === 0) {
    return true;
  }
  const keys = new Set(names.map(labelKey).filter(Boolean));
  return selected.some((key) =>
    key === UNLABELLED ? keys.size === 0 : keys.has(labelKey(key)),
  );
}

/** `workspaces` that pass `selected`, in their order. */
export function filterByLabels<T extends { labels: readonly string[] }>(
  workspaces: readonly T[],
  selected: readonly string[],
): T[] {
  return workspaces.filter((workspace) =>
    passesLabels(workspace.labels, selected),
  );
}

/** `selected` without the names no catalog has any more, so a deleted
 * label does not keep hiding every workspace. Only call it with
 * catalogs that all loaded: a machine still loading is not evidence a
 * label is gone (Paseo reconciles only on an authoritative catalog). */
export function withoutDeleted(
  selected: readonly string[],
  known: readonly string[],
): string[] {
  const keys = new Set(known.map(labelKey));
  return selected.filter(
    (key) => key === UNLABELLED || keys.has(labelKey(key)),
  );
}
