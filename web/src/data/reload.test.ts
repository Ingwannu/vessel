import { describe, expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { SessionInfo } from "../wire/proto";
import { liveSession } from "./fixtures/machines";
import {
  canReload,
  RELOAD_PROTOCOL,
  RELOAD_WORDS,
  reloadFailure,
} from "./reload";

const LOADED: SessionInfo = {
  ...liveSession,
  harness_session: fixtureId("HarnessSessionId", "h1"),
};

describe("who is offered Reload agent", () => {
  test("a structured session with a conversation, from the reload protocol on", () => {
    expect(canReload(LOADED, RELOAD_PROTOCOL)).toBe(true);
    expect(canReload(LOADED, RELOAD_PROTOCOL - 1)).toBe(false);
    expect(canReload(LOADED, null)).toBe(false);
  });

  test("an ended session too: the node starts it again", () => {
    const ended: SessionInfo = {
      ...LOADED,
      status: { state: "exited", code: 0 },
    };
    expect(canReload(ended, RELOAD_PROTOCOL)).toBe(true);
  });

  test("one whose agent has not named its conversation yet: the node says why", () => {
    const fresh: SessionInfo = { ...LOADED, harness_session: null };
    expect(canReload(fresh, RELOAD_PROTOCOL)).toBe(true);
  });

  test("not a session the node cannot reload", () => {
    const cannot: SessionInfo = {
      ...LOADED,
      abilities: {
        steer: false,
        resume: false,
        image: false,
        audio: false,
        fork: false,
      },
    };
    expect(canReload(cannot, RELOAD_PROTOCOL)).toBe(false);
    const terminal: SessionInfo = {
      ...LOADED,
      spec: {
        harness: "claude-code",
        mode: "terminal",
        args: [],
        size: { cols: 80, rows: 24 },
      },
    };
    expect(canReload(terminal, RELOAD_PROTOCOL)).toBe(false);
  });
});

describe("Paseo's words", () => {
  test("the row, its tooltip and its toasts", () => {
    expect(RELOAD_WORDS).toEqual({
      label: "Reload agent",
      tooltip: "Reload agent to update skills, MCPs or login status.",
      reloading: "Reloading agent...",
      reloaded: "Reloaded agent",
      failed: "Failed to reload agent",
    });
  });

  test("a failure says the node's message, else Paseo's line", () => {
    expect(reloadFailure("its agent has no conversation yet")).toBe(
      "its agent has no conversation yet",
    );
    expect(reloadFailure("  ")).toBe("Failed to reload agent");
  });
});
