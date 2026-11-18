// The page's diagnostic text against the Rust renderer's (ADR 0111): the
// fixtures in `src/wire/fixtures/` hold JSON a node sends and the text
// `vessel-proto` makes of it (rewritten by the gate's `ts` step).

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import type { AgentDiagnostic, DiagnosticReport } from "../wire/proto";
import { agentDiagnosticText } from "./agent-diagnostic";
import { words } from "./diagnostic-words";
import { nodeReportText } from "./node-diagnostics";

function fixture(name: string): string {
  return readFileSync(
    join(import.meta.dirname, "../wire/fixtures", name),
    "utf8",
  );
}

function parsed(name: string): unknown {
  return JSON.parse(fixture(name));
}

// The fixtures are the Rust types' own JSON (their Rust test reads them
// back); these check the shape far enough to name it.
function isDiagnostics(value: unknown): value is AgentDiagnostic[] {
  return (
    Array.isArray(value) &&
    value.every(
      (entry: unknown) =>
        typeof entry === "object" &&
        entry !== null &&
        "title" in entry &&
        "probe" in entry,
    )
  );
}

function isReport(value: unknown): value is DiagnosticReport {
  return (
    typeof value === "object" &&
    value !== null &&
    "collected_at" in value &&
    "node" in value
  );
}

function agents(): AgentDiagnostic[] {
  const value = parsed("agent-diagnostic.json");
  if (!isDiagnostics(value)) throw new Error("not agents' diagnostics");
  return value;
}

function report(): DiagnosticReport {
  const value = parsed("node-diagnostics.json");
  if (!isReport(value)) throw new Error("not a report");
  return value;
}

describe("the diagnostic text", () => {
  it("renders agents' diagnostics as the Rust renderer does", () => {
    expect(agents().map(agentDiagnosticText).join("\n")).toBe(
      fixture("agent-diagnostic.txt"),
    );
  });

  it("renders a node's report as the Rust renderer does", () => {
    expect(nodeReportText(report())).toBe(fixture("node-diagnostics.txt"));
  });

  it("ends an older node's report with its recent failures", () => {
    const older: DiagnosticReport = { ...report(), more: null };
    const text = nodeReportText(older);
    expect(text.endsWith("  2026-11-18T20:59:30Z: not found\n")).toBe(true);
    expect(text).not.toContain("System\n");
  });

  it("reads wire values by one rule", () => {
    expect(words(null)).toBe("none");
    expect(words(true)).toBe("yes");
    expect(words("not_checked")).toBe("not checked");
    expect(words(["a_b", 2])).toBe("a b, 2");
    expect(words({ state: "available" })).toBe("available");
    expect(
      words({ state: "unknown", reason: { reason: "timed_out", after: 5 } }),
    ).toBe("unknown (reason: timed out (after: 5))");
    expect(words({ b_c: 1, a: "x_y" })).toBe("a: x y, b c: 1");
  });
});
