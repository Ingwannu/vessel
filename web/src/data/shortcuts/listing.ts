// What Settings › Shortcuts lists (agent-orchestrator's
// KeyboardShortcutsContent): each group's actions with their keys as the
// platform writes them, whether they were changed, what they clash with,
// filtered by a search over names and keys and by "Modified".

import { type Binding, bindingWords, overlaps } from "./keys";
import type { Platform } from "./platform";
import {
  actionsIn,
  bindingsOf,
  definition,
  fixed,
  JUMP_SHOWN,
  modified,
  type Overrides,
  SHORTCUT_ACTIONS,
  type ShortcutAction,
  type ShortcutGroup,
} from "./registry";

/** One row of the list. */
export interface ShortcutRow {
  action: ShortcutAction;
  label: string;
  bindings: readonly Binding[];
  /** Each binding as the platform writes it ("⌘K", "Ctrl+Shift+B"). */
  words: readonly string[];
  modified: boolean;
  fixed: boolean;
  /** Another action holding one of these keys (two tabs, or a newer
   * default): shown on the row, never resolved silently. */
  clashes: readonly ShortcutAction[];
}

/** Which rows are shown. */
export type ShortcutFilter = "all" | "modified";

function clashesOf(
  action: ShortcutAction,
  bindings: readonly Binding[],
  overrides: Overrides,
  platform: Platform,
): ShortcutAction[] {
  return SHORTCUT_ACTIONS.filter(
    (other) =>
      other !== action &&
      !fixed(other) &&
      bindingsOf(other, overrides).some((theirs) =>
        bindings.some((ours) => overlaps(ours, theirs, platform)),
      ),
  );
}

/** `action`'s row. */
export function shortcutRow(
  action: ShortcutAction,
  overrides: Overrides,
  platform: Platform,
): ShortcutRow {
  const bindings = fixed(action) ? [] : bindingsOf(action, overrides);
  const words = fixed(action)
    ? [bindingWords([JUMP_SHOWN], platform)]
    : bindings.map((binding) => bindingWords(binding, platform));
  return {
    action,
    label: definition(action).label,
    bindings,
    words,
    modified: modified(action, overrides),
    fixed: fixed(action),
    clashes: fixed(action)
      ? []
      : clashesOf(action, bindings, overrides, platform),
  };
}

/** Whether `row` answers a search for `query` (by name or keys). */
function found(row: ShortcutRow, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (needle === "") {
    return true;
  }
  return (
    row.label.toLowerCase().includes(needle) ||
    row.words.some((words) => words.toLowerCase().includes(needle))
  );
}

/** The rows of `group` the list shows. */
export function shortcutRows(
  group: ShortcutGroup,
  overrides: Overrides,
  platform: Platform,
  query: string,
  filter: ShortcutFilter,
): ShortcutRow[] {
  return actionsIn(group)
    .map((action) => shortcutRow(action, overrides, platform))
    .filter((row) => filter === "all" || row.modified)
    .filter((row) => found(row, query));
}

/** How many actions differ from their defaults. */
export function modifiedCount(overrides: Overrides): number {
  return SHORTCUT_ACTIONS.filter((action) => modified(action, overrides))
    .length;
}
