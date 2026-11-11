import { describe, expect, it } from "vitest";

import type { BookNode, NodeStatus } from "../wire/bridge";
import type { Harness, ProfileInfo, SessionOption } from "../wire/proto";
import {
  agentChoices,
  agentName,
  profileChoices,
  startingAgent,
} from "./agents";
import {
  type Draft,
  sendable,
  specOf,
  splitArgs,
  startingMachine,
} from "./new-session";

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

const OPTIONS: SessionOption[] = [
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
];

const DRAFT: Draft = {
  machine: "laptop",
  agent: "claude-code",
  profile: "",
  terminal: false,
  args: "",
  folder: "",
  prompt: "Tidy the overview",
  picks: { mode: { id: "mode", kind: "select", value: "plan" } },
};

function node(key: string, isDefault = false): BookNode {
  return {
    key,
    label: key,
    address: `ws://${key}`,
    transport: "ws",
    default: isDefault,
    id: null,
  };
}

function connected(name: string): NodeStatus {
  return {
    state: "connected",
    protocol: 9,
    node: {
      id: `n-${name}`,
      name,
      hostname: name,
      os: "linux",
      arch: "x86_64",
      version: "0.1.0",
      protocol: { min: 2, max: 9 },
      started_at: "2026-11-11T06:00:00Z",
      state: "serving",
    },
  };
}

describe("startingMachine", () => {
  const book = [node("laptop"), node("gpu-box", true), node("build-server")];
  const statuses: Record<string, NodeStatus> = {
    laptop: connected("laptop"),
    "gpu-box": connected("gpu-box"),
  };

  it("takes the one asked for, the last chosen, the default, the first", () => {
    expect(startingMachine(book, statuses, "laptop", "gpu-box")).toBe("laptop");
    expect(startingMachine(book, statuses, null, "laptop")).toBe("laptop");
    expect(startingMachine(book, statuses, null, null)).toBe("gpu-box");
  });

  it("only a connected one", () => {
    expect(startingMachine(book, statuses, "build-server", null)).toBe(
      "gpu-box",
    );
    expect(startingMachine(book, {}, "laptop", null)).toBeNull();
  });
});

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

describe("splitArgs", () => {
  it("splits as a shell would, without expanding anything", () => {
    expect(splitArgs("  --model opus  ")).toEqual(["--model", "opus"]);
    expect(splitArgs(`--name "two words" '$HOME' a\\ b ""`)).toEqual([
      "--name",
      "two words",
      "$HOME",
      "a b",
      "",
    ]);
    expect(splitArgs("")).toEqual([]);
  });
});

describe("specOf", () => {
  it("starts a session with the choices, once the options are known", () => {
    expect(specOf(DRAFT, CLAUDE, { kind: "ready", options: OPTIONS })).toEqual({
      harness: "claude-code",
      mode: "structured",
      model: null,
      options: [{ id: "mode", kind: "select", value: "plan" }],
    });
  });

  it("starts on the agent's defaults while they are not", () => {
    for (const state of [
      { kind: "loading" },
      { kind: "failed", message: "not logged in" },
    ] as const) {
      expect(specOf(DRAFT, CLAUDE, state)).toEqual({
        harness: "claude-code",
        mode: "structured",
        model: null,
      });
    }
  });

  it("opens a terminal with its arguments, folder, profile and title", () => {
    const spec = specOf(
      {
        ...DRAFT,
        terminal: true,
        args: "--resume last",
        folder: " /work/vessel ",
        profile: "work",
      },
      CLAUDE,
      { kind: "ready", options: OPTIONS },
    );
    expect(spec).toEqual({
      harness: "claude-code",
      mode: "terminal",
      args: ["--resume", "last"],
      size: { cols: 120, rows: 36 },
      cwd: "/work/vessel",
      profile: "work",
      title: "Tidy the overview",
    });
  });

  it("opens a terminal for an agent that only runs in one", () => {
    expect(specOf(DRAFT, GEMINI, { kind: "loading" }).mode).toBe("terminal");
  });
});

describe("sendable", () => {
  it("needs a machine, an agent, and text unless it opens a terminal", () => {
    expect(sendable(DRAFT, CLAUDE)).toEqual({ ok: true });
    expect(sendable({ ...DRAFT, prompt: " " }, CLAUDE)).toEqual({
      ok: false,
      reason: null,
    });
    expect(sendable({ ...DRAFT, prompt: "", terminal: true }, CLAUDE)).toEqual({
      ok: true,
    });
    expect(sendable({ ...DRAFT, machine: null }, CLAUDE).ok).toBe(false);
    expect(sendable(DRAFT, null).ok).toBe(false);
  });
});
