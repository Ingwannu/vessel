// The theme (DESIGN.md §4, ADR 0021): system, light or dark, system by
// default, as all three references have it.
//
// [Decision Log]
// - Purpose: the owner found no dark mode; a page that follows the system
//   shows the operator's own choice from the first visit.
// - Constraint: the content security policy forbids an inline script
//   (ADR 0017), so nothing can set the theme before the stylesheet paints.
// - Choice: CSS decides. Every colour token is `light-dark()` and `:root`
//   is `color-scheme: light dark`, so with no attribute the page follows
//   the system, live, from the first paint. A stored choice of light or
//   dark sets `data-theme` on <html>, which pins `color-scheme`; this runs
//   from main.tsx before React's first render.
// - Cost: a stored choice that differs from the system can show the
//   system's scheme for the moment between the stylesheet and the module
//   script; with no choice stored (the default) there is no flash at all.
// - Before: light whatever the system preferred (ADR 0018), which ADR
//   0021 replaced.

import { Store } from "./store";

/** The operator's choice. */
export type ThemePreference = "system" | "light" | "dark";

/** The scheme the page shows. */
export type Scheme = "light" | "dark";

const KEY = "vessel.theme";

/** `value` as a preference, if it is one. */
export function preferenceOf(value: string | null): ThemePreference | null {
  return value === "system" || value === "light" || value === "dark"
    ? value
    : null;
}

function stored(): ThemePreference {
  return preferenceOf(window.localStorage.getItem(KEY)) ?? "system";
}

/** The current choice; screens read it with `useStore(theme, …)`. */
export const theme = new Store<ThemePreference>("system");

/** Put `preference` on the page: an attribute for light or dark, none
 * for system. */
function apply(preference: ThemePreference): void {
  const root = document.documentElement;
  switch (preference) {
    case "system":
      delete root.dataset["theme"];
      return;
    case "light":
    case "dark":
      root.dataset["theme"] = preference;
      return;
  }
}

/** Read the stored choice and put it on the page: before the first
 * render. */
export function applyTheme(): void {
  const preference = stored();
  theme.set(preference);
  apply(preference);
}

/** Choose `preference`, keep it, and show it. */
export function chooseTheme(preference: ThemePreference): void {
  if (preference === "system") {
    window.localStorage.removeItem(KEY);
  } else {
    window.localStorage.setItem(KEY, preference);
  }
  theme.set(preference);
  apply(preference);
}

/** The scheme the page shows now. */
export function currentScheme(): Scheme {
  const preference = theme.get();
  if (preference !== "system") {
    return preference;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/** Call `listener` whenever the scheme may have changed: a choice made
 * here, or the system's scheme changing under "system". Returns the
 * function that stops it. */
export function onSchemeChange(listener: () => void): () => void {
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", listener);
  const stop = theme.subscribe(listener);
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
