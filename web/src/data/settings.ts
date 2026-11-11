// The sections of Settings, and nothing else about them (milestone 10).
//
// [Decision Log]
// - Purpose: Settings is a page with its own list of sections, as
//   agent-orchestrator and GOAT have it: the sidebar shows the list while
//   the page shows the chosen section. Two parts of the page need the same
//   list (the sidebar that draws it and the route that renders a section),
//   and they are built separately.
// - Constraint: a section's id is part of an address (`/settings/<id>`),
//   so it arrives as text and must be checked before it is used.
// - Alternatives: each side keeping its own list (they drift, and a
//   section could be reachable without a row, or a row lead nowhere); the
//   list in a component file (the sidebar would import a route's file, or
//   the reverse, against the layers `npm run lint` holds).
// - Choice: the list lives here, in `data/`, as one closed set. A section
//   is added in this file and nowhere else; the label, the group and the
//   address follow from it, and the switches below stop compiling until
//   the new one is given each.
// - Not here: icons and what a section contains. Those are components'.

/** Every section, in the order the list shows them. */
export const SETTINGS_SECTIONS = [
  "appearance",
  "agents",
  "models",
  "machines",
  "devices",
  "sync",
  "about",
] as const;

/** One section of Settings. */
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

/** The headings the list is divided by. */
export const SETTINGS_GROUPS = ["app", "agents", "machines", "about"] as const;

/** One heading of the list. */
export type SettingsGroup = (typeof SETTINGS_GROUPS)[number];

/** The section `/settings` opens when the address names none. */
export const DEFAULT_SETTINGS_SECTION: SettingsSection = "appearance";

/** `value` as a section, if it names one. */
export function settingsSectionOf(value: string): SettingsSection | null {
  return SETTINGS_SECTIONS.find((section) => section === value) ?? null;
}

/** What the list and the page's title call `section`. */
export function settingsSectionLabel(section: SettingsSection): string {
  switch (section) {
    case "appearance":
      return "Appearance";
    case "agents":
      return "Agents";
    case "models":
      return "Models";
    case "machines":
      return "Machines";
    case "devices":
      return "Devices";
    case "sync":
      return "Sync";
    case "about":
      return "About";
  }
}

/** The heading `section` sits under. */
export function settingsGroupOf(section: SettingsSection): SettingsGroup {
  switch (section) {
    case "appearance":
      return "app";
    case "agents":
    case "models":
      return "agents";
    case "machines":
    case "devices":
    case "sync":
      return "machines";
    case "about":
      return "about";
  }
}

/** What the list calls `group`. */
export function settingsGroupLabel(group: SettingsGroup): string {
  switch (group) {
    case "app":
      return "App";
    case "agents":
      return "Agents";
    case "machines":
      return "Machines";
    case "about":
      return "About";
  }
}

/** The sections under `group`, in the list's order. */
export function settingsSectionsIn(
  group: SettingsGroup,
): readonly SettingsSection[] {
  return SETTINGS_SECTIONS.filter(
    (section) => settingsGroupOf(section) === group,
  );
}
