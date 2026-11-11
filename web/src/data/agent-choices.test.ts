import { describe, expect, it } from "vitest";

import type { Harness, ProfileInfo } from "../wire/proto";
import {
  agentChoices,
  moveChoices,
  profileChoices,
  startingAgent,
} from "./agent-choices";
import { agentName } from "./agents";

const CLAUDE: Harness = {
  kind: "claude-code",
  path: "/bin/claude",
  version: "2.1.0 (Claude Code)",
  structured: { state: "available" },
};

const GEMINI: Harness = {
  kind: "gemini",
  path: "/bin/gemini",
  version: null,
  structured: { state: "unavailable", reason: "no ACP adapter" },
};

describe("agents", () => {
  it("are called by their names, with their versions", () => {
    expect(agentName("open-code")).toBe("OpenCode");
    expect(agentChoices([CLAUDE, GEMINI])).toEqual([
      { value: "claude-code", label: "Claude Code", detail: "2.1.0" },
      { value: "gemini", label: "Gemini" },
    ]);
  });

  it("start on one with a timeline when there is one", () => {
    expect(startingAgent([GEMINI, CLAUDE])).toBe("claude-code");
    expect(startingAgent([GEMINI])).toBe("gemini");
    expect(startingAgent([])).toBeNull();
  });

  it("list the agent's profiles after its default, with their routes", () => {
    const profiles: ProfileInfo[] = [
      {
        name: "claude-qwen",
        label: "Claude Code on Qwen",
        harness: "claude-code",
        sets: [],
        model: "qwen-coder",
      },
      { name: "codex-work", harness: "codex", sets: [] },
    ];
    expect(profileChoices(profiles, "claude-code")).toEqual([
      { value: "", label: "Default" },
      {
        value: "claude-qwen",
        label: "Claude Code on Qwen",
        detail: "qwen-coder",
      },
    ]);
  });
});

describe("moving a session between profiles", () => {
  const plain: ProfileInfo = {
    name: "claude",
    harness: "claude-code",
    sets: [],
  };
  const routed: ProfileInfo = {
    name: "claude-qwen",
    label: "Claude Code on Qwen",
    harness: "claude-code",
    sets: [],
    model: "qwen-coder",
  };

  it("goes only to a profile on the route the session runs on", () => {
    const routedMove = moveChoices(
      [plain, routed],
      "claude-code",
      "qwen-coder",
    );
    expect(routedMove.map((choice) => [choice.value, choice.disabled])).toEqual(
      [
        ["", true],
        ["claude", true],
        ["claude-qwen", false],
      ],
    );
    const unrouted = moveChoices([plain, routed], "claude-code", null);
    expect(unrouted.map((choice) => choice.disabled)).toEqual([
      false,
      false,
      true,
    ]);
  });
});
