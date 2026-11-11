// What the person chose for this page, kept by the bridge (ADR 0025):
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
// - No read or write on a render path: screens read the store; a write is
//   one op, on a person's action.

import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";
import { applyChange, DEFAULT_PREFERENCES } from "./prefs-change";
import { Store } from "./store";
import type {
  ContentWidth,
  Kept,
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

/** A state from the bridge (a reply or a push): shown unless an older
 * revision than the one shown. */
export function takePreferences(state: PreferencesState): void {
  const current = prefs.get();
  if (state.revision < current.revision) {
    return;
  }
  show({
    values: state.values,
    kept: state.kept,
    confirmed: state.values,
    revision: state.revision,
  });
}

/** Follow the bridge's pushes; the returned function stops. */
export function followPreferences(fleet: Fleet): () => void {
  return fleet.bridge.onPreferences(takePreferences);
}

/** Change one setting: shown at once, then as the bridge answers. */
export async function prefer(
  fleet: Fleet,
  change: PreferenceChange,
): Promise<Outcome<void>> {
  const before = prefs.get();
  show({ ...before, values: applyChange(before.values, change) });
  const outcome = await attempt(async () => {
    const result = await fleet.bridge.op({ op: "prefer", change });
    if (result.op !== "prefer") {
      throw new Error("vessel ui answered another op");
    }
    takePreferences(result.preferences);
  });
  if (!outcome.ok) {
    const now = prefs.get();
    show({ ...now, values: now.confirmed });
  }
  return outcome;
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
