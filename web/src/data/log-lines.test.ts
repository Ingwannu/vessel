import { describe, expect, it } from "vitest";

import { logText, readableLine } from "./log-lines";

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
});
