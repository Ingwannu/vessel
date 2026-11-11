// The options' layout, against the shapes real agents return (ADR 0022
// and track C's report): Claude Code's adapter offers a permissions mode,
// a model, an effort and two on/off model settings; OpenCode a model and a
// mode; Codex without its adapter nothing.

import { describe, expect, it } from "vitest";

import type { SessionOption } from "../wire/proto";
import {
  choicesFor,
  controlsOf,
  currentName,
  NO_CONTROLS,
  offersAnything,
  pick,
  pickerLabel,
  selectChoice,
  toggleChoice,
  withPicks,
} from "./options";

const CLAUDE: SessionOption[] = [
  {
    id: "mode",
    name: "Mode",
    category: "mode",
    kind: "select",
    current: "default",
    values: [
      {
        id: "default",
        name: "Default",
        description: "Standard behavior, prompts for dangerous operations",
      },
      {
        id: "acceptEdits",
        name: "Accept Edits",
        description: "Auto-accept file edit operations",
      },
      { id: "plan", name: "Plan Mode", description: "Planning mode" },
    ],
  },
  {
    id: "model",
    name: "Model",
    category: "model",
    kind: "select",
    current: "default",
    values: [
      { id: "default", name: "Default (recommended)" },
      { id: "opus", name: "Opus" },
      { id: "haiku", name: "Haiku" },
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
      { id: "max", name: "Max", group: "Deep" },
    ],
  },
  {
    id: "fast",
    name: "Fast mode",
    category: "model_config",
    kind: "select",
    current: "off",
    values: [
      { id: "off", name: "Off" },
      { id: "on", name: "On" },
    ],
  },
  {
    id: "thinking",
    name: "Thinking",
    category: "model_config",
    kind: "toggle",
    on: true,
  },
  { id: "subagents", name: "Agents", category: "other", kind: "unknown" },
];

const OPENCODE: SessionOption[] = [
  {
    id: "model",
    name: "Model",
    category: "model",
    kind: "select",
    current: "anthropic/claude",
    values: [
      { id: "anthropic/claude", name: "Claude" },
      { id: "openai/gpt", name: "GPT" },
    ],
  },
  {
    id: "mode",
    name: "Mode",
    category: "mode",
    kind: "select",
    current: "build",
    values: [
      { id: "build", name: "build" },
      { id: "plan", name: "plan" },
    ],
  },
];

describe("controlsOf", () => {
  it("puts each of Claude's options where the page shows it", () => {
    const controls = controlsOf(CLAUDE);
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
    const controls = controlsOf(CLAUDE);
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
    const controls = controlsOf(OPENCODE);
    expect(controls.model?.id).toBe("model");
    expect(controls.permissions?.id).toBe("mode");
    expect(controls.effort).toBeNull();
    expect(controls.toggles).toEqual([]);
    expect(offersAnything(controls)).toBe(true);
  });

  it("offers nothing for an agent that offers nothing", () => {
    expect(controlsOf([])).toEqual(NO_CONTROLS);
    expect(offersAnything(controlsOf([]))).toBe(false);
  });

  it("files a second option of one category with the other choices", () => {
    const second: SessionOption = {
      id: "small-model",
      name: "Small model",
      category: "model",
      kind: "select",
      current: "haiku",
      values: [{ id: "haiku", name: "Haiku" }],
    };
    const controls = controlsOf([...CLAUDE, second]);
    expect(controls.model?.id).toBe("model");
    expect(controls.selects.map((select) => select.id)).toEqual([
      "fast",
      "small-model",
    ]);
  });
});

describe("names", () => {
  it("name the value in effect, or its id when the agent does not list it", () => {
    const controls = controlsOf(CLAUDE);
    expect(controls.model === null ? "" : currentName(controls.model)).toBe(
      "Default (recommended)",
    );
    const odd = controlsOf([
      {
        id: "model",
        name: "Model",
        category: "model",
        kind: "select",
        current: "local/llama",
        values: [{ id: "anthropic/claude", name: "Claude" }],
      },
    ]);
    expect(odd.model === null ? "" : currentName(odd.model)).toBe(
      "local/llama",
    );
  });

  it("read the model then the effort, or the agent with no model", () => {
    expect(pickerLabel(controlsOf(CLAUDE), "Claude Code")).toEqual({
      main: "Default (recommended)",
      effort: "High",
    });
    expect(pickerLabel(controlsOf(OPENCODE), "OpenCode")).toEqual({
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
    const controls = controlsOf(CLAUDE);
    const thinking = controls.toggles[0];
    if (
      controls.permissions === null ||
      controls.effort === null ||
      thinking === undefined
    ) {
      throw new Error("Claude offers all three");
    }
    let picks = pick({}, selectChoice(controls.permissions, "plan"));
    // The effort the agent already has is no change.
    picks = pick(picks, selectChoice(controls.effort, "high"));
    picks = pick(picks, toggleChoice(thinking, false));
    const shown = controlsOf(withPicks(CLAUDE, picks));
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
      pick({}, { id: "mode", kind: "select", value: "acceptEdits" }),
      { id: "effort", kind: "select", value: "max" },
    );
    expect(choicesFor(OPENCODE, picks)).toEqual([]);
    expect(withPicks(OPENCODE, picks)).toEqual(OPENCODE);
  });

  it("keep the agent's order, not the order they were made in", () => {
    const picks = pick(
      pick({}, { id: "model", kind: "select", value: "openai/gpt" }),
      { id: "mode", kind: "select", value: "plan" },
    );
    expect(choicesFor(OPENCODE, picks).map((choice) => choice.id)).toEqual([
      "model",
      "mode",
    ]);
  });
});
