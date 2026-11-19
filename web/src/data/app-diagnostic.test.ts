// The page's app diagnostic (ADR 0111): Paseo's header and per-host
// sections with Vessel's nouns, a row per step, and each machine's report.

import { describe, expect, it } from "vitest";

import {
  appDiagnosticText,
  diagnosticSteps,
  type MachineDiagnostic,
} from "./app-diagnostic";

const PAGE = {
  collectedAt: "2026-11-18T21:00:00.000Z",
  version: "0.4.0",
  platform: "a browser",
  machines: 3,
};

const MACHINES: MachineDiagnostic[] = [
  { label: "laptop", state: "unsupported", protocol: 20, version: "0.3.0" },
  { label: "gone", state: "not-connected" },
  {
    label: "gpu-box",
    state: "failed",
    protocol: 32,
    version: "0.4.0",
    message: "busy",
  },
];

describe("the app diagnostic", () => {
  it("says what each machine could not report, in Paseo's shape", () => {
    expect(appDiagnosticText(PAGE, MACHINES)).toBe(
      [
        "Vessel page diagnostics",
        "  Collected at: 2026-11-18T21:00:00.000Z",
        "  Page version: 0.4.0",
        "  Platform: a browser",
        "  Saved machines: 3",
        "",
        "Machine: laptop",
        "  Status: connected",
        "  Protocol: 20",
        "  Version: 0.3.0",
        "",
        "Node diagnostics: laptop",
        "  Status: unsupported by this node",
        "",
        "Machine diagnostics: gone",
        "  Status: machine is not connected",
        "",
        "Machine: gpu-box",
        "  Status: connected",
        "  Protocol: 32",
        "  Version: 0.4.0",
        "",
        "Machine diagnostics: gpu-box",
        "  Error: busy",
        "",
      ].join("\n"),
    );
  });

  it("lists the page, then each machine, with its step", () => {
    const running: MachineDiagnostic = {
      label: "laptop",
      state: "running",
      protocol: 32,
      version: "0.4.0",
    };
    expect(diagnosticSteps([running, ...MACHINES])).toEqual([
      { label: "Page", state: "done" },
      { label: "laptop", state: "running" },
      { label: "laptop", state: "done" },
      { label: "gone", state: "done" },
      { label: "gpu-box", state: "failed" },
    ]);
  });
});
