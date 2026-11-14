// Agents beyond the four (protocol 11, ADR 0032): what the page calls one
// a node added by its command, how two agents are told apart, where one
// is offered, and what the page says when its machine will not start it.

import { describe, expect, it } from "vitest";

import type {
  AgentInfo,
  Harness,
  HarnessRef,
  ProfileInfo,
} from "../wire/proto";
import {
  agentChoices,
  chosenAgent,
  profileChoices,
  refusedStart,
  startingAgent,
} from "./agent-choices";
import { agentHeadline, agentsOn } from "./agent-status";
import {
  AGENT_KINDS,
  agentKey,
  agentMark,
  agentModes,
  agentName,
  knownAgent,
  offersTerminal,
  runsTerminal,
  sameAgent,
} from "./agents";

const GOOSE: HarnessRef = { added: "goose" };

const ADDED: Harness = {
  kind: GOOSE,
  path: "/usr/local/bin/goose",
  version: null,
  structured: { state: "available" },
  terminal: { state: "unavailable", reason: "an added agent speaks ACP only" },
};

/** What `agent.list` says of goose, and of one whose program is gone. */
const LISTED: AgentInfo[] = [
  {
    agent: GOOSE,
    detail: {
      kind: "added",
      added: {
        label: "Goose",
        program: "goose",
        args: ["acp"],
        env: [],
        secrets: [{ variable: "GOOSE_KEY", secret: "GOOSE_KEY" }],
        inherit: [],
        mcp: [],
        source: { from: "command" },
      },
    },
  },
  {
    agent: { added: "gone" },
    detail: {
      kind: "added",
      added: {
        program: "/opt/gone/bin/gone",
        args: [],
        env: [],
        secrets: [],
        inherit: [],
        mcp: [],
        source: { from: "command" },
      },
    },
  },
];

const CODEX: Harness = {
  kind: "codex",
  path: "/bin/codex",
  version: "0.50.0",
  structured: { state: "available" },
};

describe("an added agent", () => {
  it("is shown by its label, else by the name its machine gives it", () => {
    expect(agentName(GOOSE)).toBe("goose");
    expect(agentName(GOOSE, LISTED)).toBe("Goose");
    expect(agentMark(GOOSE, LISTED)).toBe("G");
    expect(agentName({ added: "gone" }, LISTED)).toBe("gone");
    expect(agentName("claude-code", LISTED)).toBe("Claude Code");
    expect(agentMark("codex")).toBe("X");
  });

  it("is told apart by a key, never as an object", () => {
    expect(agentKey(GOOSE)).toBe("added:goose");
    expect(agentKey({ added: "goose" })).toBe(agentKey(GOOSE));
    expect(sameAgent({ added: "goose" }, GOOSE)).toBe(true);
    expect(sameAgent(GOOSE, "codex")).toBe(false);
    expect(sameAgent(null, null)).toBe(true);
    expect(agentKey("gemini")).toBe("gemini");
    expect(knownAgent(GOOSE)).toBeNull();
    expect(knownAgent("open-code")).toBe("open-code");
  });

  it("runs as a conversation, without the terminal every one of the four has", () => {
    expect(runsTerminal(ADDED)).toBe(false);
    expect(runsTerminal(CODEX)).toBe(true);
    expect(agentModes(ADDED)).toEqual({
      kind: "timeline",
      noTerminal: "an added agent speaks ACP only",
    });
    expect(agentModes(CODEX)).toEqual({ kind: "both" });
    expect(offersTerminal(agentModes(ADDED))).toBe(false);
    expect(offersTerminal(null)).toBe(true);
  });

  it("is offered after the four, by its label", () => {
    expect(agentChoices([ADDED, CODEX], LISTED)).toEqual([
      { value: "codex", agent: "codex", label: "Codex", detail: "0.50.0" },
      {
        value: "added:goose",
        agent: GOOSE,
        label: "Goose",
        detail: "added",
      },
    ]);
    expect(startingAgent([ADDED, CODEX])).toBe("codex");
    expect(startingAgent([ADDED])).toEqual(GOOSE);
  });

  it("stays chosen while its machine lists it, by key", () => {
    expect(chosenAgent({ added: "goose" }, [CODEX, ADDED], "codex")).toEqual(
      GOOSE,
    );
    expect(chosenAgent({ added: "other" }, [CODEX, ADDED], null)).toBe("codex");
  });

  it("says plainly why its machine will not start it now", () => {
    expect(refusedStart(GOOSE, LISTED)).toBe(
      "Goose cannot start on this machine now: it needs the secret GOOSE_KEY, which is not set there. Set it on that machine (vessel fleet secret set NAME), then try again.",
    );
    expect(refusedStart("codex", LISTED)).toContain(
      "an install of it is running there",
    );
  });

  it("is listed in Settings, found or not, between the four found and the rest", () => {
    const listed = agentsOn([CODEX, ADDED], LISTED);
    expect(listed.map((entry) => agentKey(entry.agent))).toEqual([
      "codex",
      "added:goose",
      "added:gone",
      // Every other known agent, found or not, in the page's order (the
      // seven of ADR 0058 after the four).
      ...AGENT_KINDS.filter((kind) => kind !== "codex"),
    ]);
    const [, goose, gone] = listed;
    expect(goose === undefined ? null : agentHeadline(goose.status)).toEqual({
      headline: "Available",
      tone: "ok",
      detail: "Runs as a conversation; it has no terminal",
    });
    expect(gone === undefined ? null : agentHeadline(gone.status).detail).toBe(
      "Its program, /opt/gone/bin/gone, is not found on this machine",
    );
    // Before `agent.list` is read, the agents `harness.list` found.
    expect(agentsOn([ADDED]).map((entry) => agentKey(entry.agent))).toContain(
      "added:goose",
    );
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
