import { describe, expect, it } from "vitest";

import { settingsPlace } from "./settings-place";

describe("a Settings address", () => {
  it("names a section, a machine and an agent", () => {
    expect(
      settingsPlace("/settings/agents", "?machine=gpu-box&agent=codex"),
    ).toEqual({
      section: "agents",
      exact: true,
      machine: "gpu-box",
      agent: "codex",
    });
  });

  it("is the default section when it names none or an unknown one", () => {
    expect(settingsPlace("/settings", "")).toMatchObject({
      section: "appearance",
      exact: false,
    });
    expect(settingsPlace("/settings/fleet", "")).toMatchObject({
      section: "appearance",
      exact: false,
    });
    expect(settingsPlace("/settings/machines/extra", "")?.exact).toBe(false);
  });

  it("reads the router's JSON-quoted values, and is nothing outside Settings", () => {
    expect(settingsPlace("/settings/sync", '?machine="laptop"')?.machine).toBe(
      "laptop",
    );
    expect(settingsPlace("/sessions", "")).toBeNull();
  });
});
