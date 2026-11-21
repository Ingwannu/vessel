import { describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import type {
  ClientInstallation,
  Contribution,
  PluginCatalog,
} from "../wire/proto";
import type { MachineCatalog } from "./plugin-catalog";
import {
  declarationOrder,
  findSurface,
  readOnlyLine,
  settingsScreens,
  sidebarItems,
  surfaceCommands,
} from "./plugin-surfaces";

const laptop = fixtureId("NodeKey", "laptop");

const screen: Contribution = {
  id: "main",
  title: "Notes",
  content: { kind: "screen", surface: { controls: [] } },
};
const settings: Contribution = {
  id: "display",
  title: "Display",
  content: { kind: "settings_screen", surface: { controls: [] } },
};
const header: Contribution = {
  id: "main",
  title: "Notes",
  content: { kind: "sidebar", placement: "header", screen: "main" },
};
const footer: Contribution = {
  id: "main",
  title: "Notes below",
  content: { kind: "sidebar", placement: "footer", screen: "main" },
};
const opening = (
  id: string,
  kind: "open_screen" | "open_settings",
  target: string,
): Contribution => ({
  id,
  title: `Open ${id}`,
  content: {
    kind: "command",
    keywords: [],
    context: "global",
    action: { kind, screen: target },
  },
});
const running: Contribution = {
  id: "run",
  title: "Run",
  content: {
    kind: "command",
    keywords: [],
    context: "global",
    action: { kind: "invoke", method: "notes.run", input: {} },
  },
};

const notes: ClientInstallation = {
  address: {
    node: fixtureId("NodeId", "node-1"),
    plugin: "notes",
    generation: fixtureId("RuntimeGeneration", "run-1"),
  },
  required_client: 44,
  document: {
    methods: ["notes.run"],
    contributions: [
      screen,
      settings,
      header,
      footer,
      opening("show", "open_screen", "main"),
      running,
      opening("prefs", "open_settings", "display"),
    ],
    themes: [],
  },
};

const held = (mayInvoke: boolean): MachineCatalog => {
  const catalog: PluginCatalog = {
    generation: 3,
    catalog: { node: fixtureId("NodeId", "node-1"), installations: [notes] },
    may_invoke: mayInvoke,
  };
  return { catalog };
};

describe("a plugin's surfaces", () => {
  it("finds a screen for any device and a settings screen only with admin", () => {
    expect(
      findSurface(held(false), laptop, "notes", "screen", "main")?.item.title,
    ).toBe("Notes");
    expect(
      findSurface(held(false), laptop, "notes", "settings_screen", "display"),
    ).toBeNull();
    expect(
      findSurface(held(true), laptop, "notes", "settings_screen", "display")
        ?.mayInvoke,
    ).toBe(true);
    expect(
      findSurface(held(true), laptop, "notes", "screen", "display"),
    ).toBeNull();
    expect(
      findSurface(held(true), laptop, "other", "screen", "main"),
    ).toBeNull();
    expect(
      findSurface(undefined, laptop, "notes", "screen", "main"),
    ).toBeNull();
  });

  it("lists settings screens for a plugin's menu only with admin", () => {
    expect(settingsScreens(held(true), "notes").map((item) => item.id)).toEqual(
      ["display"],
    );
    expect(settingsScreens(held(false), "notes")).toEqual([]);
    expect(settingsScreens(undefined, "notes")).toEqual([]);
  });

  it("puts sidebar rows in their place, opening their screen", () => {
    const catalogs = new Map([[laptop, held(false)]]);
    expect(sidebarItems(catalogs, "header")).toEqual([
      {
        key: "laptop:notes:header:main",
        title: "Notes",
        target: {
          kind: "screen",
          node: laptop,
          plugin: "notes",
          screen: "main",
        },
      },
    ]);
    expect(sidebarItems(catalogs, "footer").map((item) => item.title)).toEqual([
      "Notes below",
    ]);
  });

  it("offers opening items, settings only with admin, in declaration order", () => {
    const place = { kind: "global" } as const;
    expect(
      surfaceCommands(new Map([[laptop, held(false)]]), place).map(
        (item) => item.target,
      ),
    ).toEqual([
      { kind: "screen", node: laptop, plugin: "notes", screen: "main" },
    ]);
    const both = surfaceCommands(new Map([[laptop, held(true)]]), place);
    expect(both.map((item) => item.target.kind)).toEqual([
      "screen",
      "settings",
    ]);
    const order = declarationOrder(new Map([[laptop, held(true)]]));
    expect(
      [
        "plugin:laptop:notes:prefs",
        "plugin:laptop:notes:run",
        "plugin:laptop:notes:show",
      ].sort((a, b) => (order.get(a) ?? 0) - (order.get(b) ?? 0)),
    ).toEqual([
      "plugin:laptop:notes:show",
      "plugin:laptop:notes:run",
      "plugin:laptop:notes:prefs",
    ]);
  });

  it("names the machine in the read-only line", () => {
    expect(readOnlyLine("laptop")).toBe(
      "Changing this needs admin access to laptop.",
    );
  });
});
