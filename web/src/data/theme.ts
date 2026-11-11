// White and black is the design; dark is the same system inverted
// (DESIGN.md §4, ADR 0018). The operator's choice is kept in
// `localStorage`. Until there is one the page is light, whatever the
// system prefers: the white canvas is the product's look, not one of two
// equal skins, and a first visit should show it. Applied before the first
// paint, so the page never flashes the other theme.

export type Theme = "light" | "dark";

const KEY = "vessel.theme";

export function storedTheme(): Theme | null {
  const stored = window.localStorage.getItem(KEY);
  return stored === "dark" || stored === "light" ? stored : null;
}

export function currentTheme(): Theme {
  return storedTheme() ?? "light";
}

export function applyTheme(theme: Theme = currentTheme()): void {
  document.documentElement.dataset.theme = theme;
}

export function chooseTheme(theme: Theme): void {
  window.localStorage.setItem(KEY, theme);
  applyTheme(theme);
}
