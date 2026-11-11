// Every action the page's keys can run, with its label, its group and its
// default keys (DESIGN.md §7.1): the one list the listener matches, the
// Settings section lists and the palette shows keys from.
//
// [Decision Log]
// - Purpose: milestone 10 had three listeners (the palette's `⌘K` and
//   `/`, `n`, and `g` then a letter), fixed, each in its own component.
//   The owner asked for keys that can be rebound, as all three references
//   allow.
// - Shape: agent-orchestrator's `shortcuts.ts` (an id, a label, a group of
//   General, Navigation, Session or Panels; at most two keys per action;
//   "jump to session 1–9" fixed), with Vessel's sequences (`G S`) kept,
//   which Paseo's chords also allow.
// - Defaults avoid the keys a browser keeps for itself (`Mod+N`,
//   `Mod+T`, `Mod+W`, `Mod+1` to `Mod+9`), as Paseo's browser build does
//   with Alt: Vessel is a page, not a desktop app.
// - The overrides are the person's, kept by the bridge (`data/prefs.ts`);
//   this list is code. An id is never reused for another action: a kept
//   override names it.

import { type Binding, type Chord, parseBinding } from "./keys";

/** The actions, in the order Settings lists them. */
export const SHORTCUT_ACTIONS = [
  "palette",
  "new-session",
  "settings",
  "go-board",
  "go-machines",
  "go-sync",
  "go-devices",
  "previous-session",
  "next-session",
  "jump-session",
  "find",
  "focus-composer",
  "toggle-sidebar",
  "toggle-inspector",
] as const;

/** One action the keys can run. */
export type ShortcutAction = (typeof SHORTCUT_ACTIONS)[number];

/** The groups, in Settings' order (agent-orchestrator's). */
export const SHORTCUT_GROUPS = [
  "General",
  "Navigation",
  "Session",
  "Panels",
] as const;

/** One group of actions. */
export type ShortcutGroup = (typeof SHORTCUT_GROUPS)[number];

/** What Settings says of an action, and its keys before any override. */
export interface ShortcutDefinition {
  label: string;
  group: ShortcutGroup;
  /** Written as the bridge keeps them (`keys.ts`). */
  defaults: readonly string[];
}

/** The most keys one action holds (agent-orchestrator's). */
export const MAX_KEYS = 2;

/** `action`'s label, group and default keys. */
export function definition(action: ShortcutAction): ShortcutDefinition {
  switch (action) {
    case "palette":
      return {
        label: "Open the command palette",
        group: "General",
        defaults: ["Mod+K", "/"],
      };
    case "new-session":
      return { label: "New session", group: "General", defaults: ["N"] };
    case "settings":
      return { label: "Open Settings", group: "General", defaults: ["Mod+,"] };
    case "go-board":
      return {
        label: "Go to the board",
        group: "Navigation",
        defaults: ["G S"],
      };
    case "go-machines":
      return {
        label: "Go to Machines",
        group: "Navigation",
        defaults: ["G M", "G F"],
      };
    case "go-sync":
      return { label: "Go to Sync", group: "Navigation", defaults: ["G Y"] };
    case "go-devices":
      return { label: "Go to Devices", group: "Navigation", defaults: ["G D"] };
    case "previous-session":
      return {
        label: "Previous session",
        group: "Navigation",
        defaults: ["Alt+["],
      };
    case "next-session":
      return {
        label: "Next session",
        group: "Navigation",
        defaults: ["Alt+]"],
      };
    case "jump-session":
      // Alt and a digit, matched by `jumpOf`; not a binding of its own.
      return { label: "Go to session 1–9", group: "Navigation", defaults: [] };
    case "find":
      return {
        label: "Find in the session",
        group: "Session",
        defaults: ["Mod+F"],
      };
    case "focus-composer":
      return {
        label: "Focus the composer",
        group: "Session",
        defaults: ["Mod+L"],
      };
    case "toggle-sidebar":
      return {
        label: "Show or hide the sidebar",
        group: "Panels",
        defaults: ["Mod+B"],
      };
    case "toggle-inspector":
      return {
        label: "Show or hide the summary",
        group: "Panels",
        defaults: ["Mod+Shift+B"],
      };
  }
}

/** Whether an action's keys are fixed: session 1 to 9 is Alt and a digit,
 * one key per session (agent-orchestrator keeps its "jump" fixed too). */
export function fixed(action: ShortcutAction): boolean {
  return action === "jump-session";
}

/** Which session (1 to 9) a chord jumps to: Alt and a digit, nothing
 * else held; `null` for any other chord. */
export function jumpOf(chord: Chord): number | null {
  if (chord.mod || chord.ctrl || chord.shift || chord.meta || !chord.alt) {
    return null;
  }
  const digit = Number.parseInt(chord.key, 10);
  return digit >= 1 && digit <= 9 && chord.key.length === 1 ? digit : null;
}

/** The fixed keys of session 1 to 9, as a chord to show ("⌥1–9"). */
export const JUMP_SHOWN: Chord = {
  mod: false,
  ctrl: false,
  alt: true,
  shift: false,
  meta: false,
  key: "1–9",
};

/** `value` as an action, if the registry has it. */
export function shortcutActionOf(value: string): ShortcutAction | null {
  return SHORTCUT_ACTIONS.find((action) => action === value) ?? null;
}

/** The person's overrides as the bridge keeps them: an action's keys, an
 * empty list for "not set". Ids this page does not know are a newer
 * page's, and are left alone. */
export type Overrides = Readonly<Record<string, readonly string[]>>;

/** Whether `action`'s keys differ from its defaults. */
export function modified(
  action: ShortcutAction,
  overrides: Overrides,
): boolean {
  return overrides[action] !== undefined;
}

/** The keys `action` answers to now: its override when it has one, else
 * its defaults; text that does not parse is skipped. */
export function bindingsOf(
  action: ShortcutAction,
  overrides: Overrides,
): Binding[] {
  const texts = overrides[action] ?? definition(action).defaults;
  return texts
    .slice(0, MAX_KEYS)
    .map(parseBinding)
    .filter((binding) => binding !== null);
}

/** The actions of `group`, in the registry's order. */
export function actionsIn(group: ShortcutGroup): readonly ShortcutAction[] {
  return SHORTCUT_ACTIONS.filter(
    (action) => definition(action).group === group,
  );
}
