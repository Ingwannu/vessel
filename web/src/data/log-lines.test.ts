import { describe, expect, it } from "vitest";

import { logContents, logText, readableLine } from "./log-lines";

describe("a node's log lines (ADR 0111)", () => {
  it("drops a terminal's colours and control characters, keeps a tab", () => {
    const esc = String.fromCharCode(0x1b);
    const bell = String.fromCharCode(0x07);
    expect(
      readableLine(
        `${esc}[2m2026${esc}[0m ${esc}[32m INFO${esc}[0m started\tok${bell}${esc}c`,
      ),
    ).toBe("2026  INFO started\tok");
  });

  it("leaves plain text, and text beyond ASCII, as it is", () => {
    expect(readableLine("node listening — 노드 ✓")).toBe(
      "node listening — 노드 ✓",
    );
  });

  it("joins lines one per line", () => {
    expect(logText(["one", "two"])).toBe("one\ntwo");
  });

  it("says the file is empty only when it is, and a failure as one", () => {
    const file = {
      state: "file",
      path: "~/.vessel/logs/vesseld.log",
      cut: false,
    } as const;
    expect(logContents({ ok: true, value: { ...file, lines: [] } })).toEqual({
      kind: "empty",
    });
    expect(
      logContents({ ok: true, value: { ...file, lines: ["one", "two"] } }),
    ).toEqual({ kind: "lines", text: "one\ntwo" });
    expect(
      logContents({
        ok: true,
        value: { state: "no_file", writes_to: "terminal" },
      }),
    ).toEqual({ kind: "none" });
    expect(
      logContents({
        ok: false,
        problem: { from: "op", reason: "failed", message: "the node is gone" },
      }),
    ).toEqual({ kind: "error", message: "the node is gone" });
  });
});
