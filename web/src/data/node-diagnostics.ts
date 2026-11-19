// A node's report as Paseo's text (ADR 0111, TRACKER 567): the same text
// `vessel_proto::diagnostics::text::report` makes for the CLI and the
// native app, held to it by `src/wire/fixtures/node-diagnostics.{json,txt}`.
// The app diagnostic adds one per machine.

import type {
  DiagnosticCount,
  DiagnosticMore,
  DiagnosticReport,
  DiagnosticSection,
  DiagnosticTool,
  DiagnosticVersion,
} from "../wire/proto";
import { words } from "./diagnostic-words";

/** The report as Paseo's text, ending with a newline. */
export function nodeReportText(report: DiagnosticReport): string {
  const sections = [
    header(report),
    section("Daemon process", report.process, (value) => {
      const rows = [
        row("PID", String(value.pid)),
        row("Parent", value.parent === null ? "none" : String(value.parent)),
      ];
      if (value.memory.state === "available") {
        rows.push(
          row("Resident bytes", count(value.memory.value.resident_bytes)),
          row("Virtual bytes", count(value.memory.value.virtual_bytes)),
        );
      } else {
        rows.push(row("Memory", `unavailable (${words(value.memory.reason)})`));
      }
      return rows;
    }),
    section("Store", report.store, (value) => [
      row("Supported schema", String(value.expected_schema)),
      row("Observed schema", String(value.observed_schema)),
      row(
        "Sessions",
        `${count(value.sessions.total)} total / ${count(value.sessions.live)} live / ${count(value.sessions.ended)} ended`,
      ),
    ]),
    section("Agents", report.agents, (agents) =>
      agents.flatMap((agent) => [
        `  ${words(agent.id)}`,
        `    Program: ${words(agent.executable)}`,
        `    Conversations: ${words(agent.structured)}`,
        `    Terminal: ${words(agent.terminal)}`,
        `    Sign-in: ${words(agent.sign_in)}`,
        `    Version: ${version(agent.version)}`,
        `    Checked at: ${agent.checked_at ?? "never"}`,
        `    Stale: ${words(agent.stale)}`,
      ]),
    ),
    section("Runtime sessions", report.sessions, (value) => [
      row("Live", count(value.live)),
      row("Parked", count(value.parked)),
    ]),
    section("Listeners", report.listeners, (value) => [
      row("Local kind", words(value.local_kind)),
      row("Local", words(value.local)),
      row("Direct", words(value.direct)),
    ]),
    section("Relays", report.relays, (value) =>
      (
        [
          ["Configured", value.configured],
          ["Callback", value.callback],
        ] as const
      ).map(([label, counts]) =>
        row(
          label,
          `${count(counts.connecting)} connecting / ${count(counts.registered)} registered / ${count(counts.refused)} refused / ${count(counts.stopped)} stopped / ${count(counts.unrecognized)} unknown`,
        ),
      ),
    ),
    section("Recent failures", report.recent_failures, (value) => [
      row("Coverage", words(value.coverage)),
      row("Since", value.observed_since),
      row("Omitted", count(value.omitted)),
      ...value.entries.map((failure) => row(failure.at, words(failure.code))),
    ]),
    ...(report.more === undefined || report.more === null
      ? []
      : moreSections(report.more)),
  ];
  return sections.join("\n");
}

/** ADR 0111's sections (Paseo's System, Disk, Workspaces, Agents, Tools). */
function moreSections(more: DiagnosticMore): string[] {
  return [
    section("System", more.system, (value) => [
      row("CPU cores", count(value.cpus)),
      row(
        "Load avg",
        value.load === undefined || value.load === null
          ? "unsupported"
          : `${hundredths(value.load.one)} ${hundredths(value.load.five)} ${hundredths(value.load.fifteen)}`,
      ),
      row(
        "Memory available",
        value.memory === undefined || value.memory === null
          ? "unsupported"
          : `${count(value.memory.available_bytes)} of ${count(value.memory.total_bytes)} bytes`,
      ),
    ]),
    section("Disk", more.disk, (value) => [
      row("Data directory", "~"),
      row(
        "Available",
        `${count(value.available_bytes)} of ${count(value.total_bytes)} bytes`,
      ),
    ]),
    section("Workspaces", more.workspaces, (value) => [
      row("Projects", count(value.projects)),
      row(
        "Workspaces",
        `${count(value.active)} active (${count(value.managed)} worktrees) / ${count(value.archived)} archived`,
      ),
    ]),
    section("Activity", more.activity, (value) => {
      const byAgent = value.builtin.map(
        (agent) => `${words(agent.harness)} ${count(agent.live)}`,
      );
      if (value.added.value > 0) {
        byAgent.push(`added ${count(value.added)}`);
      }
      return [
        row(
          "Live by agent",
          byAgent.length === 0 ? "none" : byAgent.join(", "),
        ),
        row("Waiting on a permission", count(value.waiting)),
      ];
    }),
    section("Tools", more.tools, (value) => [
      row("git", tool(value.git)),
      row("gh", tool(value.gh)),
    ]),
  ];
}

function header(report: DiagnosticReport): string {
  const node = report.node;
  return [
    "Vessel diagnostics",
    row("Collected at", report.collected_at),
    row("Node", node.id),
    row("Version", version(node.version)),
    // An architecture's name keeps its `_` (`x86_64`).
    row("Platform", `${words(node.os)} / ${node.arch}`),
    row("State", words(node.state)),
    row("Uptime", `${String(node.uptime)} ms`),
    row("Home", "~"),
    "",
  ].join("\n");
}

/** A section: its title, when it was observed and its rows, or why it is
 * unavailable; ends with a newline. */
function section<T>(
  title: string,
  section: DiagnosticSection<T>,
  fill: (value: T) => string[],
): string {
  const rows =
    section.state === "available"
      ? [row("Observed at", section.at), ...fill(section.value)]
      : [row("Unavailable", words(section.reason))];
  return [title, ...rows, ""].join("\n");
}

function row(label: string, value: string): string {
  return `  ${label}: ${value}`;
}

/** A count, saying when it saturated. */
function count(value: DiagnosticCount): string {
  return value.saturated
    ? `${String(value.value)}+ (saturated)`
    : String(value.value);
}

function version(value: DiagnosticVersion | null | undefined): string {
  if (value === null || value === undefined) {
    return "unknown";
  }
  let text = `${String(value.major)}.${String(value.minor)}.${String(value.patch)}`;
  if (value.prerelease) {
    text += " (prerelease)";
  }
  if (value.build_metadata) {
    text += " (build metadata omitted)";
  }
  return text;
}

function tool(tool: DiagnosticTool): string {
  return tool.state === "found"
    ? `found, version ${version(tool.version)}`
    : words(tool);
}

/** Hundredths as a decimal (42 is "0.42"). */
function hundredths(value: number): string {
  return `${String(Math.floor(value / 100))}.${String(value % 100).padStart(2, "0")}`;
}
