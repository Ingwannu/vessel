// Agents beyond the four (protocol 11, ADR 0032): what the page calls one
// a node added by its command, and how two agents are told apart.

import { describe, expect, it } from "vitest";

import type { Harness, HarnessRef, ProfileInfo } from "../wire/proto";
import { agentChoices, profileChoices, startingAgent } from "./agent-choices";
import {
  agentKey,
  agentMark,
  agentName,
  knownAgent,
  runsTerminal,
} from "./agents";

const GOOSE: HarnessRef = { added: "goose" };

const ADDED: Harness = {
  kind: GOOSE,
  path: "/usr/local/bin/goose",
  version: null,
  structured: { state: "available" },
  terminal: { state: "unavailable", reason: "an added agent speaks ACP only" },
};

const CODEX: Harness = {
  kind: "codex",
  path: "/bin/codex",
  version: "0.50.0",
  structured: { state: "available" },
};

describe("an added agent", () => {
  it("is shown by the name its node gives it", () => {
    expect(agentName(GOOSE)).toBe("goose");
    expect(agentMark(GOOSE)).toBe("G");
    expect(agentName("claude-code")).toBe("Claude Code");
    expect(agentMark("codex")).toBe("X");
  });

  it("is told apart by a key, never as an object", () => {
    expect(agentKey(GOOSE)).toBe("added:goose");
    expect(agentKey({ added: "goose" })).toBe(agentKey(GOOSE));
    expect(agentKey("gemini")).toBe("gemini");
    expect(knownAgent(GOOSE)).toBeNull();
    expect(knownAgent("open-code")).toBe("open-code");
  });

  it("runs in no terminal, where every one of the four does", () => {
    expect(runsTerminal(ADDED)).toBe(false);
    expect(runsTerminal(CODEX)).toBe(true);
  });

  it("is not offered as a choice until the page draws it", () => {
    expect(agentChoices([ADDED, CODEX])).toEqual([
      { value: "codex", label: "Codex", detail: "0.50.0" },
    ]);
    expect(startingAgent([ADDED, CODEX])).toBe("codex");
    expect(startingAgent([ADDED])).toBeNull();
  });

  it("has its own profiles, matched by key", () => {
    const profiles: ProfileInfo[] = [
      { name: "goose", harness: GOOSE, sets: [] },
      { name: "goose-work", harness: { added: "goose" }, sets: [] },
      { name: "codex", harness: "codex", sets: [] },
    ];
    expect(
      profileChoices(profiles, GOOSE).map((choice) => choice.value),
    ).toEqual(["", "goose", "goose-work"]);
  });
});
