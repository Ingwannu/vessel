// Where in Settings the page is: the section from the address, and the
// machine and the agent a section is about, from its query (read once, by
// `where.ts`).
//
// One query parameter, `machine`, for every section scoped to a machine
// (Machines' detail, Agents, Models, Devices, Sync), so moving between
// them keeps the machine; `agent` for the open agent's card.

import {
  DEFAULT_SETTINGS_SECTION,
  type SettingsSection,
  settingsSectionOf,
} from "./settings";
import { useWhere, type Where } from "./where";
import type { NodeKey } from "../wire/bridge";

/** What a Settings address says. */
export interface SettingsPlace {
  /** The section; an unknown or missing one is the default. */
  section: SettingsSection;
  /** Whether the address named the section exactly (else the page
   * replaces it with the section's own address). */
  exact: boolean;
  machine: NodeKey | undefined;
  agent: string | undefined;
  /** The plugin settings screen open in Settings › Plugins, if any. */
  pluginScreen: { plugin: string; screen: string } | undefined;
}

/** The place `where` names, if it is under `/settings`. */
export function settingsPlace(where: Where): SettingsPlace | null {
  if (where.segments[0] !== "settings") {
    return null;
  }
  const section = settingsSectionOf(where.segments[1] ?? "");
  return {
    section: section ?? DEFAULT_SETTINGS_SECTION,
    exact: section !== null && where.segments.length === 2,
    machine: where.machine,
    agent: where.agent,
    pluginScreen:
      where.plugin === undefined || where.screen === undefined
        ? undefined
        : { plugin: where.plugin, screen: where.screen },
  };
}

/** The place the page is at, or `null` outside Settings: a page leaving
 * Settings renders once more at its new address before it unmounts, and
 * must not then read that address as a Settings one. */
export function useSettingsPlace(): SettingsPlace | null {
  return settingsPlace(useWhere());
}
