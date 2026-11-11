// The theme (DESIGN.md §4, ADR 0021): system, light or dark, system by
// default, as all three references have it.
//
// [Decision Log]
// - Purpose: the owner found no dark mode; a page that follows the system
//   shows the operator's own choice from the first visit.
// - Constraint: the content security policy forbids an inline script
//   (ADR 0017), and the page's origin is new on every launch, so a choice
//   kept in the browser (milestone 10 kept it in `localStorage`) was gone
//   at every launch and flashed the system's scheme first.
// - Choice: CSS decides. Every colour token is `light-dark()` and `:root`
//   is `color-scheme: light dark`, so with no attribute the page follows
//   the system, live. A choice of light or dark is a preference the bridge
//   keeps (ADR 0025), and the bridge writes it as `data-theme` on the
//   index's `<html>`, which pins `color-scheme` before any script runs: no
//   flash on any load. `data/prefs.ts` keeps the attribute in step with
//   later changes.

import type { Fleet } from "./fleet";
import type { Outcome } from "./outcome";
import { prefer, prefs } from "./prefs";
import { applyChange } from "./prefs-change";
import type { Theme } from "../wire/bridge";

/** The operator's choice. */
export type ThemePreference = Theme;

/** The scheme the page shows. */
export type Scheme = "light" | "dark";

/** Choose `preference`: kept by the bridge, shown at once. */
export function chooseTheme(
  fleet: Fleet,
  preference: ThemePreference,
): Promise<Outcome<void>> {
  return prefer(fleet, { set: "theme", value: preference });
}

/** Show `preference` without keeping it: the kit's gallery, which holds
 * no data and has no bridge. */
export function showTheme(preference: ThemePreference): void {
  const current = prefs.get();
  prefs.set({
    ...current,
    values: applyChange(current.values, { set: "theme", value: preference }),
  });
  const data = document.documentElement.dataset;
  if (preference === "system") {
    delete data["theme"];
  } else {
    data["theme"] = preference;
  }
}

/** The scheme the page shows now. */
export function currentScheme(): Scheme {
  const preference = prefs.get().values.appearance.theme;
  if (preference !== "system") {
    return preference;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/** Call `listener` whenever the scheme may have changed: a choice made
 * here or in another tab, or the system's scheme changing under
 * "system". Returns the function that stops it. */
export function onSchemeChange(listener: () => void): () => void {
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", listener);
  let shown = prefs.get().values.appearance.theme;
  const stop = prefs.subscribe(() => {
    const next = prefs.get().values.appearance.theme;
    if (next !== shown) {
      shown = next;
      listener();
    }
  });
  return () => {
    media.removeEventListener("change", listener);
    stop();
  };
}

/** A colour token's value as the page resolves it now (an `rgb()`), for
 * script that needs a colour, such as xterm.js's theme: the token's own
 * text is `light-dark(…)`, which only CSS can resolve. */
export function cssColor(token: string): string {
  const probe = document.createElement("span");
  probe.style.color = `var(${token})`;
  probe.style.display = "none";
  document.body.append(probe);
  const color = getComputedStyle(probe).color;
  probe.remove();
  return color;
}
