import { expect, test } from "vitest";

import type { Harness } from "../wire/proto";
import { rememberedAgent, rememberedPicks, rememberStart } from "./defaults";
import { applyChange, DEFAULT_PREFERENCES } from "./prefs-change";

const HARNESSES: Harness[] = [];

test("a start is remembered, and read back as picks", () => {
  let prefs = DEFAULT_PREFERENCES;
  for (const change of rememberStart("n-1", "codex", "work", [
    { id: "model", kind: "select", value: "gpt-5" },
    { id: "fast", kind: "toggle", on: true },
  ])) {
    prefs = applyChange(prefs, change);
  }
  expect(rememberedPicks(prefs.defaults, "n-1", "codex")).toEqual({
    profile: "work",
    picks: {
      model: { id: "model", kind: "select", value: "gpt-5" },
      fast: { id: "fast", kind: "toggle", on: true },
    },
  });
  expect(rememberedPicks(prefs.defaults, "n-2", "codex")).toBeNull();
  expect(rememberedAgent(prefs.defaults, "n-1", HARNESSES)).toBeNull();
});

test("choices past the bounds are not kept", () => {
  const many = Array.from({ length: 10 }, (_, index) => ({
    id: `o${String(index)}`,
    kind: "toggle" as const,
    on: true,
  }));
  const long = { id: "x".repeat(65), kind: "toggle" as const, on: true };
  const [, defaults] = rememberStart("n-1", "codex", "", [long, ...many]);
  expect(
    defaults?.set === "agent_defaults"
      ? Object.keys(defaults.value.choices)
      : [],
  ).toHaveLength(8);
});
