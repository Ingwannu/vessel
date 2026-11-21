import { describe, expect, it } from "vitest";

import { settingsPlace } from "./settings-place";
import type { Where } from "./where";
import { fixtureId } from "../testing/ids";

/** A place at `path` with `query`'s values, as `useWhere` reads it. */
function at(path: string, query: Partial<Where> = {}): Where {
  return {
    segments: path.split("/").filter((segment) => segment !== ""),
    node: undefined,
    machine: undefined,
    agent: undefined,
    plugin: undefined,
    screen: undefined,
    ...query,
  };
}

describe("a Settings address", () => {
  it("names a section, a machine and an agent", () => {
    expect(
      settingsPlace(
        at("/settings/agents", {
          machine: fixtureId("NodeKey", "gpu-box"),
          agent: "codex",
        }),
      ),
    ).toEqual({
      section: "agents",
      exact: true,
      machine: "gpu-box",
      agent: "codex",
    });
  });

  it("is the default section when it names none or an unknown one", () => {
    expect(settingsPlace(at("/settings"))).toMatchObject({
      section: "general",
      exact: false,
    });
    expect(settingsPlace(at("/settings/fleet"))).toMatchObject({
      section: "general",
      exact: false,
    });
    expect(settingsPlace(at("/settings/machines/extra"))?.exact).toBe(false);
  });

  it("names the plugin settings screen open in Settings › Plugins", () => {
    expect(
      settingsPlace(
        at("/settings/plugins", {
          machine: fixtureId("NodeKey", "gpu-box"),
          plugin: "prefs",
          screen: "display",
        }),
      ),
    ).toMatchObject({
      section: "plugins",
      machine: "gpu-box",
      pluginScreen: { plugin: "prefs", screen: "display" },
    });
    expect(
      settingsPlace(at("/settings/plugins", { plugin: "prefs" }))?.pluginScreen,
    ).toBeUndefined();
  });

  it("is nothing outside Settings", () => {
    expect(settingsPlace(at("/sessions"))).toBeNull();
  });
});
