import { describe, expect, it } from "vitest";

import type { SessionOption } from "../wire/proto";
import { agentHeadline, agentsOn, agentStatus, offerGroups } from "./agents";
import { claude, codexTerminalOnly } from "./fixtures/machines";

describe("an agent's status on a machine", () => {
  it("is available, needs attention with the machine's reason, or not found", () => {
    const found = [claude, codexTerminalOnly];
    expect(agentHeadline(agentStatus("claude-code", found))).toEqual({
      headline: "Available",
      tone: "ok",
      detail: null,
    });
    expect(agentHeadline(agentStatus("codex", found))).toEqual({
      headline: "Needs attention",
      tone: "warn",
      detail: "Runs in a terminal only: codex-acp is not installed",
    });
    expect(agentHeadline(agentStatus("gemini", found)).headline).toBe(
      "Not found",
    );
  });

  it("lists every agent, the installed first", () => {
    expect(agentsOn([codexTerminalOnly]).map((agent) => agent.kind)).toEqual([
      "codex",
      "claude-code",
      "gemini",
      "open-code",
    ]);
  });
});

describe("what an agent offers", () => {
  // The fake agent's options (crates/vessel-testkit, fake_acp/options.rs),
  // in the order it gives them.
  const options: SessionOption[] = [
    {
      id: "mode",
      name: "Mode",
      category: "mode",
      kind: "select",
      current: "default",
      values: [
        { id: "default", name: "Default" },
        { id: "plan", name: "Plan" },
      ],
    },
    {
      id: "model",
      name: "Model",
      category: "model",
      kind: "select",
      current: "fake-small",
      values: [
        { id: "fake-small", name: "Fake small" },
        { id: "fake-large", name: "Fake large" },
      ],
    },
    {
      id: "effort",
      name: "Effort",
      category: "thought_level",
      kind: "select",
      current: "high",
      values: [
        { id: "low", name: "Low", group: "Quick" },
        { id: "high", name: "High", group: "Deep" },
      ],
    },
    { id: "fast", name: "Fast", category: "other", kind: "toggle", on: false },
    { id: "later", name: "Later", category: "other", kind: "unknown" },
  ];

  it("groups them as models, effort, permissions, then the rest", () => {
    const groups = offerGroups(options);
    expect(groups.map((entry) => entry.group)).toEqual([
      "models",
      "effort",
      "permissions",
      "other",
    ]);
    expect(groups[0]?.rows).toEqual([
      {
        id: "model",
        name: "Model",
        values: ["Fake small", "Fake large"],
        current: "Fake small",
      },
    ]);
    // A toggle reads as on/off; a kind the page does not know is skipped.
    expect(groups[3]?.rows).toEqual([
      { id: "fast", name: "Fast", values: ["On", "Off"], current: "Off" },
    ]);
  });

  it("shows no group the agent offers nothing in", () => {
    expect(offerGroups([])).toEqual([]);
    expect(
      offerGroups(options.slice(1, 2)).map((entry) => entry.group),
    ).toEqual(["models"]);
  });
});
