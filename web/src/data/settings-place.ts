// Where in Settings the page is: the section from the address, and the
// machine and the agent a section is about, from its query. Read from the
// router's location as `where.ts` does, for the reason it gives (the
// router's loose params come out as `any`).
//
// One query parameter, `machine`, for every section scoped to a machine
// (Machines' detail, Agents, Models, Devices, Sync), so moving between
// them keeps the machine; `agent` for the open agent's card.

import { useRouterState } from "@tanstack/react-router";

import {
  DEFAULT_SETTINGS_SECTION,
  type SettingsSection,
  settingsSectionOf,
} from "./settings";

/** What a Settings address says. */
export interface SettingsPlace {
  /** The section; an unknown or missing one is the default. */
  section: SettingsSection;
  /** Whether the address named the section exactly (else the page
   * replaces it with the section's own address). */
  exact: boolean;
  machine: string | undefined;
  agent: string | undefined;
}

/** A query value as the router writes it: plain text, or JSON text. */
function queryValue(raw: string | null): string | undefined {
  if (raw === null || raw === "") {
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "string" ? parsed : raw;
  } catch {
    return raw;
  }
}

/** The place `pathname` and `query` name, if they are under `/settings`. */
export function settingsPlace(
  pathname: string,
  query: string,
): SettingsPlace | null {
  const segments = pathname.split("/").filter((segment) => segment !== "");
  if (segments[0] !== "settings") {
    return null;
  }
  const named = segments[1] ?? "";
  const section = settingsSectionOf(decodeURIComponent(named));
  const params = new URLSearchParams(query);
  return {
    section: section ?? DEFAULT_SETTINGS_SECTION,
    exact: section !== null && segments.length === 2,
    machine: queryValue(params.get("machine")),
    agent: queryValue(params.get("agent")),
  };
}

/** The place the page is at; outside Settings, the default section. */
export function useSettingsPlace(): SettingsPlace {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const query = useRouterState({ select: (state) => state.location.searchStr });
  return (
    settingsPlace(pathname, query) ?? {
      section: DEFAULT_SETTINGS_SECTION,
      exact: false,
      machine: undefined,
      agent: undefined,
    }
  );
}
