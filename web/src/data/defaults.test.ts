import { expect, test } from "vitest";

import type { Harness } from "../wire/proto";
import {
  rememberedAgent,
  rememberedPicks,
  rememberStart,
  startingPicks,
} from "./defaults";
import { applyChange, DEFAULT_PREFERENCES } from "./prefs-change";
import { fixtureId } from "../testing/ids";

const HARNESSES: Harness[] = [];

test("a start is remembered, and read back as picks", () => {
  let prefs = DEFAULT_PREFERENCES;
  for (const change of rememberStart(
    fixtureId("NodeId", "n-1"),
    "codex",
    "work",
    [
      {
        id: fixtureId("OptionId", "model"),
        kind: "select",
        value: fixtureId("OptionValueId", "gpt-5"),
      },
      { id: fixtureId("OptionId", "fast"), kind: "toggle", on: true },
    ],
  )) {
    prefs = applyChange(prefs, change);
  }
  expect(
    rememberedPicks(prefs.defaults, fixtureId("NodeId", "n-1"), "codex"),
  ).toEqual({
    profile: "work",
    picks: {
      model: { id: "model", kind: "select", value: "gpt-5" },
      fast: { id: "fast", kind: "toggle", on: true },
    },
  });
  expect(
    rememberedPicks(prefs.defaults, fixtureId("NodeId", "n-2"), "codex"),
  ).toBeNull();
  expect(
    rememberedAgent(prefs.defaults, fixtureId("NodeId", "n-1"), HARNESSES),
  ).toBeNull();
});

test("choices past the bounds are not kept", () => {
  const many = Array.from({ length: 10 }, (_, index) => ({
    id: fixtureId("OptionId", `o${String(index)}`),
    kind: "toggle" as const,
    on: true,
  }));
  const long = {
    id: fixtureId("OptionId", "x".repeat(65)),
    kind: "toggle" as const,
    on: true,
  };
  const [, defaults] = rememberStart(fixtureId("NodeId", "n-1"), "codex", "", [
    long,
    ...many,
  ]);
  expect(
    defaults?.set === "agent_defaults"
      ? Object.keys(defaults.value.choices)
      : [],
  ).toHaveLength(8);
});

test("a new session starts as the last one did until the person chooses, the default profile included", () => {
  let prefs = DEFAULT_PREFERENCES;
  for (const change of rememberStart(
    fixtureId("NodeId", "n-1"),
    "codex",
    "work",
    [],
  )) {
    prefs = applyChange(prefs, change);
  }
  const node = fixtureId("NodeId", "n-1");
  const untouched = { profile: "", picks: {}, chosen: false };
  expect(startingPicks(untouched, prefs.defaults, node, "codex")).toEqual({
    profile: "work",
    picks: {},
  });
  // Choosing the default profile ("") is a choice: "work" does not come
  // back.
  expect(
    startingPicks(
      { ...untouched, chosen: true },
      prefs.defaults,
      node,
      "codex",
    ),
  ).toEqual({ profile: "", picks: {} });
  // Nothing kept for this machine: what the draft says.
  expect(
    startingPicks(
      untouched,
      prefs.defaults,
      fixtureId("NodeId", "n-2"),
      "codex",
    ),
  ).toEqual({ profile: "", picks: {} });
});
