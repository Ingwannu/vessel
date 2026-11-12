import { describe, expect, test } from "vitest";

import type { PreferencesState } from "../wire/bridge";
import { firstPaint, keptNote, prefs, takePreferences } from "./prefs";
import { applyChange, DEFAULT_PREFERENCES, keepSections } from "./prefs-change";
import { fixtureId } from "../testing/ids";

describe("the first paint", () => {
  test("the attributes the bridge wrote, and the defaults for the rest", () => {
    const values = firstPaint({
      theme: "dark",
      uiSize: "16",
      codeSize: "13",
      contentWidth: "wide",
    });
    expect(values.appearance).toEqual({
      ...DEFAULT_PREFERENCES.appearance,
      theme: "dark",
      interface_size: 16,
      code_size: 13,
      content_width: "wide",
    });
  });

  test("a value out of range or missing is the default", () => {
    const values = firstPaint({ uiSize: "40", theme: "sepia" });
    expect(values.appearance.interface_size).toBe(14);
    expect(values.appearance.theme).toBe("system");
  });
});

describe("a change applied as the bridge applies it", () => {
  test("one setting, the others kept", () => {
    const next = applyChange(DEFAULT_PREFERENCES, {
      set: "send",
      value: "mod_enter",
    });
    expect(next.composer.send).toBe("mod_enter");
    expect(next.appearance).toBe(DEFAULT_PREFERENCES.appearance);
  });

  test("shortcut edits together; default removes the override", () => {
    const set = applyChange(DEFAULT_PREFERENCES, {
      set: "shortcuts",
      value: {
        edits: [
          { action: "find", to: { kind: "bindings", keys: ["Mod+K"] } },
          { action: "palette", to: { kind: "bindings", keys: ["/"] } },
        ],
      },
    });
    expect(set.shortcuts).toEqual({ find: ["Mod+K"], palette: ["/"] });
    const back = applyChange(set, {
      set: "shortcuts",
      value: { edits: [{ action: "find", to: { kind: "default" } }] },
    });
    expect(back.shortcuts).toEqual({ palette: ["/"] });
    expect(applyChange(back, { set: "reset_shortcuts" }).shortcuts).toEqual({});
  });

  test("a machine's defaults, and forgetting them", () => {
    const agent = applyChange(DEFAULT_PREFERENCES, {
      set: "default_agent",
      value: { node: fixtureId("NodeId", "n-1"), agent: "codex" },
    });
    const choices = applyChange(agent, {
      set: "agent_defaults",
      value: {
        node: fixtureId("NodeId", "n-1"),
        agent: "codex",
        profile: null,
        choices: {
          [fixtureId("OptionId", "model")]: {
            kind: "select",
            value: fixtureId("OptionValueId", "gpt"),
          },
        },
      },
    });
    expect(choices.defaults[fixtureId("NodeId", "n-1")]).toEqual({
      agent: "codex",
      agents: {
        codex: {
          profile: null,
          choices: { model: { kind: "select", value: "gpt" } },
        },
      },
    });
    expect(
      applyChange(choices, {
        set: "forget_defaults",
        value: { node: fixtureId("NodeId", "n-1") },
      }).defaults,
    ).toEqual({});
  });
});

function state(revision: number, theme: "light" | "dark"): PreferencesState {
  return {
    revision,
    kept: { kind: "file", path: "/tmp/ui-preferences.json" },
    values: {
      ...DEFAULT_PREFERENCES,
      appearance: { ...DEFAULT_PREFERENCES.appearance, theme },
    },
  };
}

test("an older state than the one shown is dropped", () => {
  takePreferences(state(5, "dark"));
  takePreferences(state(4, "light"));
  expect(prefs.get().values.appearance.theme).toBe("dark");
  takePreferences(state(6, "light"));
  expect(prefs.get().revision).toBe(6);
  expect(prefs.get().values.appearance.theme).toBe("light");
});

test("where they are kept, said only when it is not a file", () => {
  expect(keptNote({ kind: "file", path: "/x" })).toBeNull();
  expect(keptNote({ kind: "this_launch", why: { kind: "no_place" } })).toBe(
    "Kept until vessel ui stops: this machine gives the page no place to keep them.",
  );
  expect(
    keptNote({
      kind: "this_launch",
      why: {
        kind: "unreadable",
        path: "/c/ui-preferences.json",
        message: "not JSON",
      },
    }),
  ).toContain("Fix or remove it, then restart vessel ui.");
});

describe("a push keeps what did not change", () => {
  test("each section said the same keeps its object", () => {
    const before = DEFAULT_PREFERENCES;
    // The bridge's push: a whole new document, one setting changed.
    const pushed = applyChange(structuredClone(before), {
      set: "inspector_panel",
      value: "hidden",
    });
    const kept = keepSections(before, pushed);
    expect(kept.layout).toBe(pushed.layout);
    expect(kept.layout.inspector).toBe("hidden");
    expect(kept.sidebar).toBe(before.sidebar);
    expect(kept.appearance).toBe(before.appearance);
    expect(kept.shortcuts).toBe(before.shortcuts);
    // Nothing changed: the same document.
    expect(keepSections(before, structuredClone(before))).toBe(before);
  });
});
