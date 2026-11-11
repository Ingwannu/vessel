import { describe, expect, it } from "vitest";

import {
  ago,
  answerEmphasis,
  harnessName,
  nodeState,
  sessionTitle,
  shortId,
  statusTone,
  statusWord,
  toolTone,
} from "./format";

describe("formatting", () => {
  it("keeps the protocol's words", () => {
    expect(statusWord({ state: "exited", code: 3 })).toBe("exited 3");
    expect(statusWord({ state: "exited", code: null })).toBe("exited");
    expect(statusWord({ state: "interrupted" })).toBe("interrupted");
    expect(harnessName("claude-code")).toBe("claude");
    expect(harnessName("open-code")).toBe("opencode");
  });

  it("titles a session by its working directory", () => {
    expect(sessionTitle("/home/me/work/vessel")).toBe("vessel");
    expect(sessionTitle("/home/me/work/vessel/")).toBe("vessel");
    expect(sessionTitle("C:\\Users\\me\\proj")).toBe("proj");
    expect(sessionTitle("/")).toBe("/");
    expect(sessionTitle(null)).toBe("~");
    expect(sessionTitle(undefined)).toBe("~");
  });

  it("shortens ids and times", () => {
    expect(shortId("3f2a9c10-0000-4000-8000-000000000000")).toBe("3f2a");
    const now = Date.parse("2026-11-07T12:00:00Z");
    expect(ago("2026-11-07T11:59:30Z", now)).toBe("30s");
    expect(ago("2026-11-07T11:46:00Z", now)).toBe("14m");
    expect(ago("2026-11-07T09:00:00Z", now)).toBe("3h");
    expect(ago("2026-11-01T12:00:00Z", now)).toBe("6d");
  });

  it("draws each state in its tone", () => {
    expect(statusTone({ state: "running" })).toBe("ok");
    expect(statusTone({ state: "starting" })).toBe("warn");
    expect(statusTone({ state: "failed", message: "boom" })).toBe("danger");
    expect(statusTone({ state: "interrupted" })).toBe("interrupt");
    expect(statusTone({ state: "exited", code: 0 })).toBe("neutral");
    expect(toolTone("in_progress")).toBe("warn");
    expect(toolTone("completed")).toBe("ok");
    expect(toolTone("failed")).toBe("danger");
    expect(nodeState(undefined)).toEqual({ word: "connecting", tone: "warn" });
    expect(answerEmphasis("allow_once", true)).toBe("primary");
    expect(answerEmphasis("reject_once", false)).toBe("secondary");
    expect(answerEmphasis("reject_always", true)).toBe("danger");
  });
});
