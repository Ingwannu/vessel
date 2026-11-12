import { describe, expect, it } from "vitest";

import type { SessionOption } from "../wire/proto";
import {
  agentHeadline,
  agentsOn,
  agentStatus,
  askedWhen,
  offerRows,
} from "./agent-status";
import { controlsOf } from "./options";
import { claude, codexTerminalOnly } from "./fixtures/machines";
import { fixtureId } from "../testing/ids";

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
      id: fixtureId("OptionId", "mode"),
      name: "Mode",
      category: "mode",
      kind: "select",
      current: fixtureId("OptionValueId", "default"),
      values: [
        { id: fixtureId("OptionValueId", "default"), name: "Default" },
        { id: fixtureId("OptionValueId", "plan"), name: "Plan" },
      ],
    },
    {
      id: fixtureId("OptionId", "model"),
      name: "Model",
      category: "model",
      kind: "select",
      current: fixtureId("OptionValueId", "fake-small"),
      values: [
        { id: fixtureId("OptionValueId", "fake-small"), name: "Fake small" },
        { id: fixtureId("OptionValueId", "fake-large"), name: "Fake large" },
      ],
    },
    {
      id: fixtureId("OptionId", "effort"),
      name: "Effort",
      category: "thought_level",
      kind: "select",
      current: fixtureId("OptionValueId", "high"),
      values: [
        { id: fixtureId("OptionValueId", "low"), name: "Low", group: "Quick" },
        { id: fixtureId("OptionValueId", "high"), name: "High", group: "Deep" },
      ],
    },
    {
      id: fixtureId("OptionId", "fast"),
      name: "Fast",
      category: "other",
      kind: "toggle",
      on: false,
    },
    {
      id: fixtureId("OptionId", "later"),
      name: "Later",
      category: "other",
      kind: "unknown",
    },
  ];

  it("lists them as models, effort, permissions, then the rest, in the page's words", () => {
    expect(offerRows(controlsOf(options, "Fake"))).toEqual([
      {
        id: "model",
        label: "Model",
        values: ["Fake small", "Fake large"],
        current: "Fake small",
      },
      {
        id: "effort",
        label: "Effort",
        values: ["Low", "High"],
        current: "High",
      },
      {
        id: "mode",
        label: "Permissions",
        values: ["Default", "Plan"],
        current: "Default",
      },
      // A toggle reads as on/off; a kind the page does not know is skipped.
      { id: "fast", label: "Fast", values: ["On", "Off"], current: "Off" },
    ]);
  });

  it("names a second option of a category by its own name", () => {
    const select = (id: string, name: string): SessionOption => ({
      id: fixtureId("OptionId", id),
      name,
      category: "model",
      kind: "select",
      current: fixtureId("OptionValueId", "a"),
      values: [{ id: fixtureId("OptionValueId", "a"), name: "A" }],
    });
    const two = [select("model", "Model"), select("fallback", "Fallback")];
    expect(offerRows(controlsOf(two, "Fake")).map((row) => row.label)).toEqual([
      "Model",
      "Fallback",
    ]);
    expect(offerRows(controlsOf([], "Fake"))).toEqual([]);
  });

  it("says when the machine asked", () => {
    const now = Date.parse("2026-11-11T07:00:00Z");
    expect(askedWhen("2026-11-11T06:59:30Z", now)).toBe("Asked just now");
    expect(askedWhen("2026-11-11T06:57:00Z", now)).toBe("Asked 3m ago");
  });
});
