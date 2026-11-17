// Workspace label names as a node compares them (ADR 0086): the display
// spelling a node keeps, and the folded key two spellings share.
//
// [Decision Log]
// - Purpose: the page matches a workspace's assigned names against a
//   node's catalog, merges catalogs across machines, and tells the person
//   whether a typed name is already a label. Each comparison has to agree
//   with the node, or a picker shows "Blocked" unassigned on a workspace
//   the node says carries "blocked".
// - Constraint: the node's grammar (`vessel-proto::workspace_labels`)
//   collapses ECMAScript whitespace, trims, and folds case for identity.
//   JavaScript's `\s` is that same reference set, so the page can follow
//   it exactly here rather than ask the node.
// - Choice: one normalizer and one key, used by every label module; the
//   key is never shown, only compared.

import type { LabelColor } from "../../wire/proto";

/** The filter key that means "no label" (Paseo's Unlabelled row). No real
 * name normalizes to it, because an empty name is refused. */
export const UNLABELLED = "";

/** `raw` as a node would keep it: inner whitespace runs collapsed to one
 * space, the edges trimmed. Empty when nothing visible is left. */
export function normalizeLabelName(raw: string): string {
  return raw.split(/\s+/u).filter(Boolean).join(" ");
}

/** The identity two spellings of one label share. */
export function labelKey(name: string): string {
  return normalizeLabelName(name).toLowerCase();
}

/** Sort names as the catalogs list them: by name, ignoring case. */
export function byLabelName(left: string, right: string): number {
  return left.localeCompare(right, undefined, { sensitivity: "base" });
}

/** The colours a label can take, in the order the swatches show them
 * (the node's ten, Paseo's order; the first is a new label's). */
export const LABEL_COLORS = [
  "violet",
  "sky",
  "emerald",
  "orange",
  "pink",
  "indigo",
  "teal",
  "red",
  "amber",
  "blue",
] as const satisfies readonly LabelColor[];

/** A colour's name, for its swatch. */
export function colorName(color: LabelColor): string {
  return color.charAt(0).toUpperCase() + color.slice(1);
}
