import { describe, expect, it } from "vitest";

import {
  DEFAULT_SETTINGS_SECTION,
  SETTINGS_GROUPS,
  SETTINGS_SECTIONS,
  settingsGroupOf,
  settingsSectionLabel,
  settingsSectionOf,
  settingsSectionsIn,
} from "./settings";

describe("the sections of Settings", () => {
  it("reads a section from an address, and nothing else", () => {
    expect(settingsSectionOf("machines")).toBe("machines");
    expect(settingsSectionOf("Machines")).toBeNull();
    expect(settingsSectionOf("")).toBeNull();
    expect(settingsSectionOf("fleet")).toBeNull();
  });

  it("puts every section under exactly one heading, in the list's order", () => {
    const grouped = SETTINGS_GROUPS.flatMap((group) =>
      settingsSectionsIn(group),
    );
    expect(grouped).toEqual([...SETTINGS_SECTIONS]);
    for (const section of SETTINGS_SECTIONS) {
      expect(settingsSectionsIn(settingsGroupOf(section))).toContain(section);
    }
  });

  it("names every section, each differently", () => {
    const labels = SETTINGS_SECTIONS.map(settingsSectionLabel);
    expect(new Set(labels).size).toBe(SETTINGS_SECTIONS.length);
    expect(SETTINGS_SECTIONS).toContain(DEFAULT_SETTINGS_SECTION);
  });
});
