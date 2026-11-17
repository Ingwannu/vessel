// What the command palette lists (DESIGN.md §2.2): the page's own
// commands, the places (the board, Settings and each of its sections),
// the machines, every session the page knows, and the theme. A pure
// function of the fleet's state, so what the palette offers is decided
// here and tested, and the palette only draws it. In the page's words: a
// machine opens its detail in Settings, a session is called by its title.

import type { FleetState } from "./fleet";
import type { ThemePreference } from "./theme";
import { isLive, statusWord } from "./format";
import { machineState, reachWord } from "./machines";
import { nodeLabel } from "./nodes";
import {
  SETTINGS_SECTIONS,
  type SettingsSection,
  settingsSectionLabel,
} from "./settings";
import { sessionName } from "./titles";

/** The palette's groups, in the order it lists them. */
export type PaletteGroup =
  "Actions" | "Go to" | "Machines" | "Sessions" | "Theme";

/** What choosing an entry does. */
export type PaletteTarget =
  | { kind: "route"; to: string }
  | { kind: "new-session"; node: string | null }
  | { kind: "add-machine" }
  | { kind: "manage-labels" }
  | { kind: "theme"; preference: ThemePreference };

/** The places a palette entry can name, for its icon. */
export type PalettePlace =
  | "board"
  | "settings"
  | { section: SettingsSection }
  | "machine"
  | "terminal"
  | "structured"
  | "new"
  | "add-machine"
  | "labels"
  | "theme-system"
  | "theme-light"
  | "theme-dark";

export interface PaletteEntry {
  /** Unique in one listing. */
  id: string;
  label: string;
  detail?: string;
  /** Searched with the label (a session's full id). */
  keywords?: string;
  group: PaletteGroup;
  place: PalettePlace;
  target: PaletteTarget;
}

/** The board, Settings, and each of Settings' sections (its one list,
 * `settings.ts`). */
const PLACES: readonly PaletteEntry[] = [
  {
    id: "go-board",
    label: "Board",
    group: "Go to",
    place: "board",
    target: { kind: "route", to: "/sessions" },
  },
  ...SETTINGS_SECTIONS.map((section): PaletteEntry => ({
    id: `go-settings-${section}`,
    label: `Settings: ${settingsSectionLabel(section)}`,
    group: "Go to",
    place: { section },
    target: { kind: "route", to: `/settings/${section}` },
  })),
];

/** "Theme: system", "Theme: light", "Theme: dark". */
const THEMES: readonly PaletteEntry[] = (
  ["system", "light", "dark"] as const
).map((preference) => ({
  id: `theme-${preference}`,
  label: `Theme: ${preference}`,
  group: "Theme",
  place: `theme-${preference}`,
  target: { kind: "theme", preference },
}));

/** Everything the palette lists for `state`, `selected` being the machine a
 * new session starts on. Live sessions come before ended ones, each
 * newest first, so the one being worked on is near the top. */
export function paletteEntries(
  state: FleetState,
  selected: string | null,
): PaletteEntry[] {
  const machines: PaletteEntry[] = (state.book ?? []).map((node) => ({
    id: `machine-${node.key}`,
    label: nodeLabel(node, state.statuses[node.key]),
    detail: `${machineState(state.statuses[node.key]).word} · ${reachWord(node)}`,
    group: "Machines",
    place: "machine",
    target: {
      kind: "route",
      to: `/settings/machines?machine=${encodeURIComponent(node.key)}`,
    },
  }));
  const sessions = Object.entries(state.sessions)
    .flatMap(([node, list]) => list.map((session) => ({ node, session })))
    // Newest first, then live first: the sort is stable, so the live and
    // the ended each stay newest first.
    .sort((a, b) => b.session.created_at.localeCompare(a.session.created_at))
    .sort((a, b) => Number(isLive(b.session)) - Number(isLive(a.session)))
    .map(({ node, session }): PaletteEntry => ({
      id: `session-${node}-${session.id}`,
      label: sessionName(session),
      detail: `${statusWord(session.status)} · ${session.spec.cwd ?? "~"}`,
      keywords: session.id,
      group: "Sessions",
      place: session.spec.mode === "terminal" ? "terminal" : "structured",
      target: { kind: "route", to: `/sessions/${node}/${session.id}` },
    }));
  const actions: PaletteEntry[] = [
    {
      id: "new-session",
      label: "New session",
      group: "Actions",
      place: "new",
      target: { kind: "new-session", node: selected },
    },
    {
      id: "add-machine",
      label: "Add a machine",
      group: "Actions",
      place: "add-machine",
      target: { kind: "add-machine" },
    },
    {
      id: "manage-labels",
      label: "Manage labels",
      keywords: "workspace labels tags",
      group: "Actions",
      place: "labels",
      target: { kind: "manage-labels" },
    },
  ];
  return [...actions, ...PLACES, ...machines, ...sessions, ...THEMES];
}
