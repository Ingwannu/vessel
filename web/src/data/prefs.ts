// What the person chose for this page, kept by the bridge (ADR 0028):
// the store screens read, how a choice is sent, and the appearance put
// on `<html>`.
//
// [Decision Log]
// - Purpose: the page's origin is new on every launch (ADR 0017), so
//   nothing chosen in the browser survives one. The bridge keeps a
//   document beside the node book and answers it whole after every
//   change, to every tab, with a rising revision.
// - First paint: the bridge writes the appearance on the index's
//   `<html>` (`data-theme`, `data-ui-size`, `data-code-size`,
//   `data-content-width`), and the stylesheet draws it before any script
//   runs; `firstPaint` reads those attributes so the store starts where
//   the page already is, and nothing changes when the socket answers.
// - A change is shown at once (`applyChange`, the bridge's own rules
//   mirrored), then replaced by the bridge's answer. A reply and a push
//   can arrive in either order; a state older than the one shown is
//   dropped by its revision. A refused change puts back the last state
//   the bridge confirmed.
//
// [Decision Log: a change made before the bridge answers]
// - Problem (found under load in the end-to-end runs): a change made
//   while the page is still connecting was sent at once, refused as "not
//   connected", and taken back; and a state that arrived while a change
//   was on its way (the one pushed after the welcome, or another
//   change's) showed the bridge's document without it until its own
//   answer came. So Ctrl+B pressed as the page loaded hid the sidebar for
//   a moment and put it back, and the next Ctrl+B, reading the sidebar as
//   shown, hid it, and the bridge kept it hidden.
// - Alternatives: the bridge client queueing every op until its link
//   opens (ops a person retries by hand, the book's and pairing's, would
//   then run late and unseen); refusing changes until the link opens (a
//   key does nothing for a second after each load).
// - Choice: a change waits for the link to open (not past a refusal or
//   the client giving up, which take it back), and the changes not yet
//   answered are applied over every state that arrives, oldest first,
//   until each one's own answer. Every change sets a value (none toggles
//   or adds), so one applied over a state that holds it already is that
//   state.
// - Cost: a change made while the link stays down is shown and not kept
//   until the link opens; the list of unanswered changes is as long as a
//   person can press between two answers.
// - No read or write on a render path: screens read the store; a write is
//   one op, on a person's action.

import type { LinkState } from "../bridge/client";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";
import { applyChange, DEFAULT_PREFERENCES, keepSections } from "./prefs-change";
import { Store } from "./store";
import type {
  ContentWidth,
  Kept,
  Op,
  OpResult,
  PreferenceChange,
  Preferences,
  PreferencesState,
  Theme,
} from "../wire/bridge";

/** The page's copy of the preferences. */
export interface PrefsState {
  /** What screens show: the bridge's, or the first paint's until then,
   * with a change being sent already applied. */
  values: Preferences;
  /** Where the bridge keeps them; `null` until it has said. */
  kept: Kept | null;
  /** The last state the bridge confirmed, and its revision (0: none). */
  confirmed: Preferences;
  revision: number;
}

/** The interface sizes the bridge accepts, in whole pixels. */
export const INTERFACE_SIZES = [12, 13, 14, 15, 16, 17, 18, 19, 20] as const;

/** The code sizes. */
export const CODE_SIZES = [10, 11, 12, 13, 14, 15, 16, 17, 18] as const;

function themeOf(value: string | undefined): Theme {
  return value === "light" || value === "dark" ? value : "system";
}

function widthOf(value: string | undefined): ContentWidth {
  return value === "wide" || value === "full" ? value : "comfortable";
}

function sizeOf(
  value: string | undefined,
  sizes: readonly number[],
  fallback: number,
): number {
  const size = Number(value);
  return sizes.includes(size) ? size : fallback;
}

/** The appearance the bridge painted on `<html>`, as preferences. */
export function firstPaint(data: DOMStringMap): Preferences {
  const base = DEFAULT_PREFERENCES.appearance;
  return {
    ...DEFAULT_PREFERENCES,
    appearance: {
      ...base,
      theme: themeOf(data["theme"]),
      interface_size: sizeOf(
        data["uiSize"],
        INTERFACE_SIZES,
        base.interface_size,
      ),
      code_size: sizeOf(data["codeSize"], CODE_SIZES, base.code_size),
      content_width: widthOf(data["contentWidth"]),
    },
  };
}

/** The page's preferences. Starts from the defaults; `startPreferences`
 * reads the first paint before the first render. */
export const prefs = new Store<PrefsState>({
  values: DEFAULT_PREFERENCES,
  kept: null,
  confirmed: DEFAULT_PREFERENCES,
  revision: 0,
});

/** Put the appearance on `<html>`, as the bridge paints the index:
 * the stylesheet has a rule for each value (`ui/styles/appearance.css`). */
function paint(values: Preferences): void {
  // The unit tests run without a document; the page always has one.
  if (typeof document === "undefined") {
    return;
  }
  const data = document.documentElement.dataset;
  const appearance = values.appearance;
  if (appearance.theme === "system") {
    delete data["theme"];
  } else {
    data["theme"] = appearance.theme;
  }
  data["uiSize"] = String(appearance.interface_size);
  data["codeSize"] = String(appearance.code_size);
  data["contentWidth"] = appearance.content_width;
}

function show(next: PrefsState): void {
  prefs.set(next);
  paint(next.values);
}

/** Read the first paint into the store: before React's first render. */
export function startPreferences(): void {
  const values = firstPaint(document.documentElement.dataset);
  prefs.set({ ...prefs.get(), values, confirmed: values });
}

/** The changes shown and not answered yet, oldest first. */
let unanswered: readonly PreferenceChange[] = [];

/** `values` with every unanswered change applied over it. */
function withUnanswered(values: Preferences): Preferences {
  return unanswered.reduce(applyChange, values);
}

/** A state from the bridge (a reply or a push): shown unless an older
 * revision than the one shown, with the changes not answered yet applied
 * over it. */
export function takePreferences(state: PreferencesState): void {
  const current = prefs.get();
  if (state.revision < current.revision) {
    return;
  }
  const confirmed = keepSections(current.confirmed, state.values);
  show({
    values: keepSections(current.values, withUnanswered(confirmed)),
    kept: state.kept,
    confirmed,
    revision: state.revision,
  });
}

/** Follow the bridge's pushes; the returned function stops. */
export function followPreferences(fleet: Fleet): () => void {
  return fleet.bridge.onPreferences(takePreferences);
}

/** What a change needs of the fleet: its link to `vessel ui`, and the
 * bridge's op. Given by `prefer` (a test gives its own). */
export interface PrefsLink {
  link: () => LinkState;
  onLink: (listener: (link: LinkState) => void) => () => void;
  op: (op: Op) => Promise<OpResult>;
}

/** Whether `link` lets a change go: `true` open, `false` given up on
 * (refused, or no longer redialled), `null` not yet. */
function linkReady(link: LinkState): boolean | null {
  switch (link.kind) {
    case "open":
      return true;
    case "refused":
    case "disconnected":
      return false;
    case "connecting":
    case "closed":
      return null;
  }
}

/** Once the link opens, or is given up on: whether it opened. */
function opened(through: PrefsLink): Promise<boolean> {
  return new Promise((resolve) => {
    const now = linkReady(through.link());
    if (now !== null) {
      resolve(now);
      return;
    }
    const stop = through.onLink((link) => {
      const ready = linkReady(link);
      if (ready !== null) {
        stop();
        resolve(ready);
      }
    });
  });
}

/** Change one setting: shown at once, sent once the link is open, then
 * as the bridge answers. */
export async function preferThrough(
  through: PrefsLink,
  change: PreferenceChange,
): Promise<Outcome<void>> {
  const before = prefs.get();
  show({ ...before, values: applyChange(before.values, change) });
  unanswered = [...unanswered, change];
  const outcome = await attempt(async () => {
    if (!(await opened(through))) {
      throw new Error("The page is not connected to vessel ui.");
    }
    const result = await through.op({ op: "prefer", change });
    if (result.op !== "prefer") {
      throw new Error("vessel ui answered another op");
    }
    takePreferences(result.preferences);
  });
  unanswered = unanswered.filter((pending) => pending !== change);
  // Shown again from what the bridge confirmed and what is still on its
  // way: a refusal takes this change back, and an answer older than a
  // state already shown (another tab's change pushed before this reply)
  // is dropped by its revision, so this change, answered, must stop
  // being applied over that state.
  const now = prefs.get();
  const values = keepSections(now.values, withUnanswered(now.confirmed));
  if (values !== now.values) {
    show({ ...now, values });
  }
  return outcome;
}

/** Change one setting through `fleet` (`preferThrough`). */
export function prefer(
  fleet: Fleet,
  change: PreferenceChange,
): Promise<Outcome<void>> {
  return preferThrough(
    {
      link: () => fleet.store.get().link,
      onLink: (listener) => fleet.bridge.onLink(listener),
      op: (op) => fleet.bridge.op(op),
    },
    change,
  );
}

/** Where the bridge keeps the preferences, in a sentence for Settings;
 * `null` when it keeps them in a file, as it should. */
export function keptNote(kept: Kept | null): string | null {
  if (kept === null || kept.kind === "file") {
    return null;
  }
  const why = kept.why;
  const lead = "Kept until vessel ui stops: ";
  switch (why.kind) {
    case "no_place":
      return `${lead}this machine gives the page no place to keep them.`;
    case "no_path":
      return `${lead}${why.message}`;
    case "unreadable":
      return `${lead}${why.path} could not be read (${why.message}). Fix or remove it, then restart vessel ui.`;
    case "newer":
      return `${lead}${why.path} was written by a newer Vessel (version ${String(why.version)}); it is left as it is.`;
  }
}
