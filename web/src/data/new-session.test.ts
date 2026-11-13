import { describe, expect, it } from "vitest";

import type { Harness, SessionOption } from "../wire/proto";
import { startedModel } from "./models";
import {
  type Draft,
  draftControls,
  type OptionsState,
  sendable,
  startsTerminal,
  specOf,
  splitArgs,
} from "./new-session";
import { fixtureId } from "../testing/ids";

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

/** An agent added on the machine (protocol 11): a conversation only. */
const HELPER: Harness = {
  kind: { added: "helper" },
  path: "/opt/helper/bin/helper",
  version: null,
  structured: { state: "available" },
  terminal: { state: "unavailable", reason: "an added agent has none" },
};

const LOADING: OptionsState = { kind: "loading" };

const OPTIONS: SessionOption[] = [
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
];

const DRAFT: Draft = {
  machine: fixtureId("NodeKey", "laptop"),
  agent: "claude-code",
  profile: "",
  terminal: false,
  args: "",
  folder: "",
  prompt: "Tidy the overview",
  chosen: false,
  picks: {
    [fixtureId("OptionId", "mode")]: {
      id: fixtureId("OptionId", "mode"),
      kind: "select",
      value: fixtureId("OptionValueId", "plan"),
    },
  },
};

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
    expect(sendable(DRAFT, CLAUDE, LOADING)).toEqual({ ok: true });
    expect(sendable({ ...DRAFT, prompt: " " }, CLAUDE, LOADING)).toEqual({
      ok: false,
      reason: null,
    });
    expect(
      sendable({ ...DRAFT, prompt: "", terminal: true }, CLAUDE, LOADING),
    ).toEqual({
      ok: true,
    });
    expect(sendable({ ...DRAFT, machine: null }, CLAUDE, LOADING).ok).toBe(
      false,
    );
    expect(sendable(DRAFT, null, LOADING).ok).toBe(false);
  });

  it("says why when the machine will not start the agent now", () => {
    expect(
      sendable(DRAFT, HELPER, { kind: "refused", message: "Needs a key." }),
    ).toEqual({ ok: false, reason: "Needs a key." });
  });

  it("says why when the machine runs the agent neither way", () => {
    const broken: Harness = {
      ...HELPER,
      structured: { state: "unavailable", reason: "its program is gone" },
    };
    expect(sendable(DRAFT, broken, LOADING)).toEqual({
      ok: false,
      reason: "helper cannot run on this machine: its program is gone",
    });
  });
});

describe("an agent added on the machine", () => {
  it("never opens a terminal, whatever was asked for another agent", () => {
    const draft = { ...DRAFT, agent: HELPER.kind, terminal: true };
    expect(startsTerminal(draft, HELPER)).toBe(false);
    expect(specOf(draft, HELPER, LOADING)).toMatchObject({
      harness: { added: "helper" },
      mode: "structured",
    });
  });
});

describe("draftControls", () => {
  it("shows the picks, the loading, the failure, or a terminal's none", () => {
    const ready = draftControls(DRAFT, CLAUDE, {
      kind: "ready",
      options: OPTIONS,
    });
    expect(
      ready.kind === "ready" ? ready.controls.permissions?.current : null,
    ).toBe("plan");
    expect(draftControls(DRAFT, CLAUDE, { kind: "loading" })).toEqual({
      kind: "loading",
    });
    expect(
      draftControls(DRAFT, CLAUDE, {
        kind: "failed",
        message: "not logged in",
      }),
    ).toEqual({ kind: "failed", message: "not logged in" });
    expect(
      draftControls({ ...DRAFT, terminal: true }, CLAUDE, { kind: "loading" }),
    ).toEqual({ kind: "terminal" });
  });
});

describe("startedModel", () => {
  it("is the route's model, else the one asked for, else none", () => {
    const base = {
      id: fixtureId("SessionId", "a"),
      created_by: { kind: "owner" },
      status: { state: "running" },
      created_at: "2026-11-11T08:00:00Z",
      next_seq: 0,
    } as const;
    expect(
      startedModel({
        ...base,
        spec: { harness: "claude-code", mode: "structured" },
        model: { route: "qwen-coder", model: "qwen/qwen3-coder" },
      }),
    ).toBe("qwen/qwen3-coder");
    expect(
      startedModel({
        ...base,
        spec: { harness: "claude-code", mode: "structured", model: "opus" },
      }),
    ).toBe("opus");
    expect(
      startedModel({
        ...base,
        spec: {
          harness: "codex",
          mode: "terminal",
          args: [],
          size: { cols: 80, rows: 24 },
        },
      }),
    ).toBeNull();
  });
});
