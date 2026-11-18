// The page's "App diagnostic" (ADR 0111, TRACKER 567), as Paseo's
// (`diagnostics/app-diagnostic-report.ts`, `components/app-diagnostic-
// sheet.tsx`): what the page knows of itself and of each machine in the
// book, then each connected machine's own report (`node.diagnostics`), as
// one text a person copies into a report.
//
// [Decision Log]
// - Purpose: Paseo's "Run" on Settings › Diagnostics: one block that says
//   which page, which machines, and what each machine says of itself.
// - Constraint: the text is pasted elsewhere. It holds no address, key,
//   token or link: the page's facts are its version and browser, a
//   machine's are its name in the book, its state, protocol and version,
//   and the node's report is content-free by its own contract (ADR 0074).
// - Choice: Paseo's shape and words, with Vessel's nouns ("machine",
//   "node"): a header, then per machine a "Machine: <name>" section and its
//   report, or "Status: unsupported by this node" below protocol 24,
//   "Status: machine is not connected", or the call's error.

import type { DiagnosticReport } from "../wire/proto";
import { nodeReportText } from "./node-diagnostics";

/** `node.diagnostics`'s protocol (ADR 0074). */
export const NODE_DIAGNOSTICS_PROTOCOL = 24;

/** What the page says of itself. */
export interface PageFacts {
  /** When the diagnostic was collected (ISO 8601). */
  collectedAt: string;
  /** vessel ui's version, or "not connected". */
  version: string;
  /** The browser, as it names itself. */
  platform: string;
  /** Machines in the book. */
  machines: number;
}

/** One machine's part of the diagnostic. */
export type MachineDiagnostic =
  | { label: string; state: "not-connected" }
  | { label: string; state: "running"; protocol: number; version: string }
  | { label: string; state: "unsupported"; protocol: number; version: string }
  | {
      label: string;
      state: "failed";
      protocol: number;
      version: string;
      message: string;
    }
  | {
      label: string;
      state: "done";
      protocol: number;
      version: string;
      report: DiagnosticReport;
    };

/** A step of the run, as the dialog's progress rows show it (Paseo's
 * "Client" then one per host). */
export interface DiagnosticStep {
  label: string;
  state: "running" | "done" | "failed";
}

/** The run's steps: the page, then each machine. */
export function diagnosticSteps(
  machines: readonly MachineDiagnostic[],
): DiagnosticStep[] {
  return [
    { label: "Page", state: "done" },
    ...machines.map((machine): DiagnosticStep => {
      switch (machine.state) {
        case "running":
          return { label: machine.label, state: "running" };
        case "failed":
          return { label: machine.label, state: "failed" };
        case "not-connected":
        case "unsupported":
        case "done":
          return { label: machine.label, state: "done" };
      }
    }),
  ];
}

/** The whole text, once no machine is still running; ends with a
 * newline. */
export function appDiagnosticText(
  page: PageFacts,
  machines: readonly MachineDiagnostic[],
): string {
  const sections = [
    lines("Vessel page diagnostics", [
      ["Collected at", page.collectedAt],
      ["Page version", page.version],
      ["Platform", page.platform],
      ["Saved machines", String(page.machines)],
    ]),
  ];
  for (const machine of machines) {
    sections.push(...machineSections(machine));
  }
  return sections.join("\n");
}

function machineSections(machine: MachineDiagnostic): string[] {
  if (machine.state === "not-connected") {
    return [
      lines(`Machine diagnostics: ${machine.label}`, [
        ["Status", "machine is not connected"],
      ]),
    ];
  }
  const head = lines(`Machine: ${machine.label}`, [
    ["Status", "connected"],
    ["Protocol", String(machine.protocol)],
    ["Version", machine.version],
  ]);
  switch (machine.state) {
    case "running":
      return [head];
    case "unsupported":
      return [
        head,
        lines(`Node diagnostics: ${machine.label}`, [
          ["Status", "unsupported by this node"],
        ]),
      ];
    case "failed":
      return [
        head,
        lines(`Machine diagnostics: ${machine.label}`, [
          ["Error", machine.message],
        ]),
      ];
    case "done":
      return [head, nodeReportText(machine.report)];
  }
}

/** A title and its `  Label: value` rows, ending with a newline. */
function lines(title: string, rows: readonly [string, string][]): string {
  return [
    title,
    ...rows.map(([label, value]) => `  ${label}: ${value}`),
    "",
  ].join("\n");
}
