import { describe, expect, test } from "vitest";

import type { LinkState } from "../bridge/client";
import type { Op, PreferencesState } from "../wire/bridge";
import {
  firstPaint,
  keptNote,
  preferThrough,
  type PrefsLink,
  prefs,
  takePreferences,
} from "./prefs";
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

  test("the send key and the delivery are the composer's two settings", () => {
    const steer = applyChange(DEFAULT_PREFERENCES, {
      set: "delivery",
      value: "steer",
    });
    const both = applyChange(steer, { set: "send", value: "mod_enter" });
    expect(both.composer).toEqual({ send: "mod_enter", delivery: "steer" });
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

describe("a change made before the bridge answers", () => {
  /** A link to `vessel ui` that a test opens, and the ops sent on it. */
  function fakeLink(first: LinkState) {
    let link = first;
    const listeners = new Set<(link: LinkState) => void>();
    const sent: Op[] = [];
    let answer: (state: PreferencesState) => void = () => undefined;
    const through: PrefsLink = {
      link: () => link,
      onLink: (listener) => {
        listeners.add(listener);
        return () => listeners.delete(listener);
      },
      op: (op) => {
        sent.push(op);
        return new Promise((resolve) => {
          answer = (preferences) => {
            resolve({ op: "prefer", preferences });
          };
        });
      },
    };
    return {
      through,
      sent,
      move: (next: LinkState) => {
        link = next;
        for (const listener of [...listeners]) {
          listener(next);
        }
      },
      answer: (state: PreferencesState) => {
        answer(state);
      },
    };
  }

  /** The bridge's document at `revision`, the sidebar `sidebar`. */
  function withSidebar(
    revision: number,
    sidebar: "shown" | "hidden",
  ): PreferencesState {
    return {
      revision,
      kept: { kind: "file", path: "/tmp/ui-preferences.json" },
      values: {
        ...DEFAULT_PREFERENCES,
        layout: { ...DEFAULT_PREFERENCES.layout, sidebar },
      },
    };
  }

  const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

  test("waits for the link, is kept over a state sent before it, and goes once the link opens", async () => {
    prefs.set({
      values: DEFAULT_PREFERENCES,
      kept: null,
      confirmed: DEFAULT_PREFERENCES,
      revision: 0,
    });
    const link = fakeLink({ kind: "connecting", attempt: 1 });
    const hide = preferThrough(link.through, {
      set: "sidebar_panel",
      value: "hidden",
    });
    expect(prefs.get().values.layout.sidebar).toBe("hidden");
    // The state pushed after the welcome, from before the change.
    takePreferences(withSidebar(10, "shown"));
    expect(prefs.get().values.layout.sidebar).toBe("hidden");
    expect(link.sent).toEqual([]);
    link.move({ kind: "open", version: "0.1.0" });
    await settled();
    expect(link.sent).toEqual([
      { op: "prefer", change: { set: "sidebar_panel", value: "hidden" } },
    ]);
    // Pressed again before the first answer: the second goes after it.
    const show = preferThrough(link.through, {
      set: "sidebar_panel",
      value: "shown",
    });
    expect(prefs.get().values.layout.sidebar).toBe("shown");
    link.answer(withSidebar(11, "hidden"));
    expect((await hide).ok).toBe(true);
    // The first answer does not take back the second change.
    expect(prefs.get().values.layout.sidebar).toBe("shown");
    link.answer(withSidebar(12, "shown"));
    expect((await show).ok).toBe(true);
    expect(prefs.get().values.layout.sidebar).toBe("shown");
    expect(prefs.get().confirmed.layout.sidebar).toBe("shown");
  });

  test("answered after a newer state, stops being applied over it", async () => {
    prefs.set({
      values: DEFAULT_PREFERENCES,
      kept: null,
      confirmed: DEFAULT_PREFERENCES,
      revision: 0,
    });
    const link = fakeLink({ kind: "open", version: "0.1.0" });
    const hide = preferThrough(link.through, {
      set: "sidebar_panel",
      value: "hidden",
    });
    await settled();
    // Another tab hid it and showed it again: its push (revision 12)
    // arrives before this change's own answer (revision 11).
    takePreferences(withSidebar(12, "shown"));
    expect(prefs.get().values.layout.sidebar).toBe("hidden");
    link.answer(withSidebar(11, "hidden"));
    expect((await hide).ok).toBe(true);
    expect(prefs.get().values.layout.sidebar).toBe("shown");
  });

  test("is taken back when the link is given up on", async () => {
    prefs.set({
      values: DEFAULT_PREFERENCES,
      kept: null,
      confirmed: DEFAULT_PREFERENCES,
      revision: 0,
    });
    const link = fakeLink({ kind: "connecting", attempt: 1 });
    const hide = preferThrough(link.through, {
      set: "sidebar_panel",
      value: "hidden",
    });
    link.move({ kind: "refused" });
    expect((await hide).ok).toBe(false);
    expect(prefs.get().values.layout.sidebar).toBe("shown");
    expect(link.sent).toEqual([]);
  });
});
