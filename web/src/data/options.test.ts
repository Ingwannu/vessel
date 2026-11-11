// The options' layout, against the shapes real agents return (ADR 0022
// and track C's report): Claude Code's adapter offers a permissions mode,
// a model, an effort and two on/off model settings; OpenCode a model and a
// mode; Codex without its adapter nothing.

import { describe, expect, it } from "vitest";

import type { SessionOption } from "../wire/proto";
import {
  controlsOf,
  currentName,
  NO_CONTROLS,
  offersAnything,
  ownName,
  pickerLabel,
  pickerName,
  reportedControls,
  selectChoice,
  toggleChoice,
} from "./options";
import { choicesFor, pick, withPicks } from "./picks";
import { fixtureId } from "../testing/ids";

const CLAUDE: SessionOption[] = [
  {
    id: fixtureId("OptionId", "mode"),
    name: "Mode",
    category: "mode",
    kind: "select",
    current: fixtureId("OptionValueId", "default"),
    values: [
      {
        id: fixtureId("OptionValueId", "default"),
        name: "Default",
        description: "Standard behavior, prompts for dangerous operations",
      },
      {
        id: fixtureId("OptionValueId", "acceptEdits"),
        name: "Accept Edits",
        description: "Auto-accept file edit operations",
      },
      {
        id: fixtureId("OptionValueId", "plan"),
        name: "Plan Mode",
        description: "Planning mode",
      },
    ],
  },
  {
    id: fixtureId("OptionId", "model"),
    name: "Model",
    category: "model",
    kind: "select",
    current: fixtureId("OptionValueId", "default"),
    values: [
      {
        id: fixtureId("OptionValueId", "default"),
        name: "Default (recommended)",
      },
      { id: fixtureId("OptionValueId", "opus"), name: "Opus" },
      { id: fixtureId("OptionValueId", "haiku"), name: "Haiku" },
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
      { id: fixtureId("OptionValueId", "max"), name: "Max", group: "Deep" },
    ],
  },
  {
    id: fixtureId("OptionId", "fast"),
    name: "Fast mode",
    category: "model_config",
    kind: "select",
    current: fixtureId("OptionValueId", "off"),
    values: [
      { id: fixtureId("OptionValueId", "off"), name: "Off" },
      { id: fixtureId("OptionValueId", "on"), name: "On" },
    ],
  },
  {
    id: fixtureId("OptionId", "thinking"),
    name: "Thinking",
    category: "model_config",
    kind: "toggle",
    on: true,
  },
  {
    id: fixtureId("OptionId", "subagents"),
    name: "Agents",
    category: "other",
    kind: "unknown",
  },
];

const OPENCODE: SessionOption[] = [
  {
    id: fixtureId("OptionId", "model"),
    name: "Model",
    category: "model",
    kind: "select",
    current: fixtureId("OptionValueId", "anthropic/claude"),
    values: [
      { id: fixtureId("OptionValueId", "anthropic/claude"), name: "Claude" },
      { id: fixtureId("OptionValueId", "openai/gpt"), name: "GPT" },
    ],
  },
  {
    id: fixtureId("OptionId", "mode"),
    name: "Mode",
    category: "mode",
    kind: "select",
    current: fixtureId("OptionValueId", "build"),
    values: [
      { id: fixtureId("OptionValueId", "build"), name: "build" },
      { id: fixtureId("OptionValueId", "plan"), name: "plan" },
    ],
  },
];

describe("controlsOf", () => {
  it("puts each of Claude's options where the page shows it", () => {
    const controls = controlsOf(CLAUDE, "Claude Code");
    expect(controls.permissions?.id).toBe("mode");
    expect(controls.model?.id).toBe("model");
    expect(controls.effort?.id).toBe("effort");
    // The on/off select is a submenu, the toggle a switch row, and the
    // kind the page does not know is skipped.
    expect(controls.selects.map((select) => select.id)).toEqual(["fast"]);
    expect(controls.toggles).toEqual([
      { id: "thinking", name: "Thinking", description: null, on: true },
    ]);
  });

  it("keeps the agent's words, descriptions and groups", () => {
    const controls = controlsOf(CLAUDE, "Claude Code");
    expect(controls.permissions?.values[1]).toEqual({
      id: "acceptEdits",
      name: "Accept Edits",
      description: "Auto-accept file edit operations",
    });
    expect(controls.effort?.values.map((value) => value.group)).toEqual([
      "Quick",
      "Deep",
      "Deep",
    ]);
  });

  it("offers no effort for an agent without one", () => {
    const controls = controlsOf(OPENCODE, "OpenCode");
    expect(controls.model?.id).toBe("model");
    expect(controls.permissions?.id).toBe("mode");
    expect(controls.effort).toBeNull();
    expect(controls.toggles).toEqual([]);
    expect(offersAnything(controls)).toBe(true);
  });

  it("offers nothing for an agent that offers nothing", () => {
    expect(controlsOf([], "Claude Code")).toEqual(NO_CONTROLS);
    expect(offersAnything(controlsOf([], "Claude Code"))).toBe(false);
  });

  it("files a second option of one category with the other choices", () => {
    const second: SessionOption = {
      id: fixtureId("OptionId", "small-model"),
      name: "Small model",
      category: "model",
      kind: "select",
      current: fixtureId("OptionValueId", "haiku"),
      values: [{ id: fixtureId("OptionValueId", "haiku"), name: "Haiku" }],
    };
    const controls = controlsOf([...CLAUDE, second], "Claude Code");
    expect(controls.model?.id).toBe("model");
    expect(controls.selects.map((select) => select.id)).toEqual([
      "fast",
      "small-model",
    ]);
  });
});

describe("names", () => {
  it("name the value in effect, or its id when the agent does not list it", () => {
    const controls = controlsOf(CLAUDE, "Claude Code");
    expect(controls.model === null ? "" : currentName(controls.model)).toBe(
      "Default (recommended)",
    );
    const odd = controlsOf(
      [
        {
          id: fixtureId("OptionId", "model"),
          name: "Model",
          category: "model",
          kind: "select",
          current: fixtureId("OptionValueId", "local/llama"),
          values: [
            {
              id: fixtureId("OptionValueId", "anthropic/claude"),
              name: "Claude",
            },
          ],
        },
      ],
      "Claude Code",
    );
    expect(odd.model === null ? "" : currentName(odd.model)).toBe(
      "local/llama",
    );
  });

  it("read the model then the effort, or the agent with no model", () => {
    expect(
      pickerLabel(controlsOf(CLAUDE, "Claude Code"), "Claude Code"),
    ).toEqual({
      main: "Default (recommended)",
      effort: "High",
    });
    expect(pickerLabel(controlsOf(OPENCODE, "OpenCode"), "OpenCode")).toEqual({
      main: "Claude",
      effort: null,
    });
    expect(pickerLabel(NO_CONTROLS, "Codex")).toEqual({
      main: "Codex",
      effort: null,
    });
  });
});

describe("picks", () => {
  it("show what the session will start with, and send only changes", () => {
    const controls = controlsOf(CLAUDE, "Claude Code");
    const thinking = controls.toggles[0];
    if (
      controls.permissions === null ||
      controls.effort === null ||
      thinking === undefined
    ) {
      throw new Error("Claude offers all three");
    }
    let picks = pick(
      {},
      selectChoice(controls.permissions, fixtureId("OptionValueId", "plan")),
    );
    // The effort the agent already has is no change.
    picks = pick(
      picks,
      selectChoice(controls.effort, fixtureId("OptionValueId", "high")),
    );
    picks = pick(picks, toggleChoice(thinking, false));
    const shown = controlsOf(withPicks(CLAUDE, picks), "Claude Code");
    expect(shown.permissions?.current).toBe("plan");
    expect(shown.toggles[0]?.on).toBe(false);
    expect(choicesFor(CLAUDE, picks)).toEqual([
      { id: "mode", kind: "select", value: "plan" },
      { id: "thinking", kind: "toggle", on: false },
    ]);
  });

  it("drop a pick the agent does not offer", () => {
    // Made for Claude, then the agent changed to OpenCode.
    const picks = pick(
      pick(
        {},
        {
          id: fixtureId("OptionId", "mode"),
          kind: "select",
          value: fixtureId("OptionValueId", "acceptEdits"),
        },
      ),
      {
        id: fixtureId("OptionId", "effort"),
        kind: "select",
        value: fixtureId("OptionValueId", "max"),
      },
    );
    expect(choicesFor(OPENCODE, picks)).toEqual([]);
    expect(withPicks(OPENCODE, picks)).toEqual(OPENCODE);
  });

  it("keep the agent's order, not the order they were made in", () => {
    const picks = pick(
      pick(
        {},
        {
          id: fixtureId("OptionId", "model"),
          kind: "select",
          value: fixtureId("OptionValueId", "openai/gpt"),
        },
      ),
      {
        id: fixtureId("OptionId", "mode"),
        kind: "select",
        value: fixtureId("OptionValueId", "plan"),
      },
    );
    expect(choicesFor(OPENCODE, picks).map((choice) => choice.id)).toEqual([
      "model",
      "mode",
    ]);
  });
});

describe("ownName", () => {
  it("says whose an option is when the page already uses its name", () => {
    // Claude Code calls one of its options "Agent"; the menu already has a
    // row the page calls "Agent" (which program runs the session).
    expect(ownName("Agent", "Claude Code")).toBe("Claude Code agent");
    expect(ownName(" agent ", "Claude Code")).toBe("Claude Code agent");
    expect(ownName("Model", "OpenCode")).toBe("OpenCode model");
    expect(ownName("EFFORT", "Codex")).toBe("Codex effort");
    expect(ownName("Permissions", "Codex")).toBe("Codex permissions");
  });

  it("keeps every other name exactly as the agent gave it", () => {
    expect(ownName("Fast mode", "Claude Code")).toBe("Fast mode");
    expect(ownName("Agents", "Claude Code")).toBe("Agents");
    expect(ownName("Sub-agent", "Claude Code")).toBe("Sub-agent");
    expect(ownName("", "Claude Code")).toBe("");
  });

  it("names an agent's other options through it, and leaves the slots alone", () => {
    const agent: SessionOption = {
      id: fixtureId("OptionId", "agent"),
      name: "Agent",
      category: "other",
      kind: "select",
      current: fixtureId("OptionValueId", "default"),
      values: [
        { id: fixtureId("OptionValueId", "default"), name: "Default" },
        { id: fixtureId("OptionValueId", "reviewer"), name: "reviewer" },
      ],
    };
    const secondModel: SessionOption = {
      id: fixtureId("OptionId", "small_model"),
      name: "Model",
      category: "model",
      kind: "select",
      current: fixtureId("OptionValueId", "haiku"),
      values: [{ id: fixtureId("OptionValueId", "haiku"), name: "Haiku" }],
    };
    const toggle: SessionOption = {
      id: fixtureId("OptionId", "effort_toggle"),
      name: "Effort",
      category: "other",
      kind: "toggle",
      on: true,
    };
    const controls = controlsOf(
      [...CLAUDE, agent, secondModel, toggle],
      "Claude Code",
    );
    // The first model is in the page's slot; its own name is not shown.
    expect(controls.model?.id).toBe("model");
    expect(
      controls.selects
        .filter((select) => select.id !== "fast")
        .map((select) => [select.id, select.name]),
    ).toEqual([
      ["agent", "Claude Code agent"],
      ["small_model", "Claude Code model"],
    ]);
    expect(
      controls.toggles.find((found) => found.id === "effort_toggle")?.name,
    ).toBe("Claude Code effort");
    // Only the name shown changes: the choice is still the agent's option.
    const shown = controls.selects.find((select) => select.id === "agent");
    expect(
      shown === undefined
        ? null
        : selectChoice(shown, fixtureId("OptionValueId", "reviewer")),
    ).toEqual({ id: "agent", kind: "select", value: "reviewer" });
  });
});

describe("reportedControls", () => {
  it("loads until the log reports, unless the node never will", () => {
    expect(reportedControls(null, 9, "Claude Code")).toEqual({
      kind: "loading",
    });
    expect(reportedControls(null, null, "Claude Code")).toEqual({
      kind: "loading",
    });
    expect(reportedControls(null, 8, "Claude Code").kind).toBe("failed");
    const ready = reportedControls(OPENCODE, 9, "OpenCode");
    expect(ready.kind === "ready" ? ready.controls.model?.id : null).toBe(
      "model",
    );
    expect(reportedControls([], 9, "Claude Code")).toEqual({
      kind: "ready",
      controls: NO_CONTROLS,
    });
  });
});

describe("pickerName", () => {
  it("names the control and what it shows", () => {
    expect(
      pickerName(
        { kind: "ready", controls: controlsOf(CLAUDE, "Claude Code") },
        "Claude Code",
      ),
    ).toBe("Agent, model and effort: Claude Code, Default (recommended), High");
    expect(pickerName({ kind: "ready", controls: NO_CONTROLS }, "Codex")).toBe(
      "Agent, model and effort: Codex",
    );
    expect(pickerName({ kind: "loading" }, "Codex")).toBe(
      "Agent, model and effort: Codex, loading models",
    );
  });
});
