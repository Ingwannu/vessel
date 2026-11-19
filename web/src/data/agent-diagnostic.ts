// An agent's diagnostic as Paseo's text (ADR 0111, TRACKER 568 and 67):
// the same text `vessel_proto::agents::AgentDiagnostic::text` makes, held
// to it by `src/wire/fixtures/agent-diagnostic.{json,txt}`. The dialog
// shows and copies this text, as Paseo's provider diagnostic sheet does.

import type {
  AgentDiagnostic,
  CatalogStatus,
  CheckStep,
  Installed,
  ProbeStep,
  SignedIn,
  UnknownReason,
} from "../wire/proto";

/** How a multi-line value's later lines are indented (Paseo's). */
const CONTINUED = "\n    ";

/** The diagnostic as Paseo's text, ending with a newline. */
export function agentDiagnosticText(diagnostic: AgentDiagnostic): string {
  const rows: [string, string][] = [
    [
      "Command source",
      diagnostic.command.source === "built_in" ? "built in" : "added",
    ],
    [
      "Configured command",
      [diagnostic.command.program, ...(diagnostic.command.args ?? [])].join(
        " ",
      ),
    ],
    [
      "Search path matches",
      diagnostic.matches.length === 0
        ? "none"
        : diagnostic.matches.join(CONTINUED),
    ],
    ...installedRows(diagnostic.installed),
    ["Auth", signedIn(diagnostic.signed_in)],
    ["ACP spawn", step(diagnostic.probe.spawn)],
    ["ACP initialize", initialize(diagnostic)],
    ["ACP session/new", step(diagnostic.probe.session)],
    ["Probe took", `${String(diagnostic.probe.took)} ms`],
    ...catalog(diagnostic.status),
  ];
  return (
    `${diagnostic.title}\n` +
    rows.map(([label, value]) => `  ${label}: ${value}\n`).join("")
  );
}

function installedRows(installed: Installed): [string, string][] {
  switch (installed.state) {
    case "found":
      return [
        ["Resolved path", installed.path],
        ["Version", installed.version ?? "unknown"],
        [
          "Conversations",
          installed.structured.state === "available"
            ? "available"
            : `unavailable: ${installed.structured.reason}`,
        ],
      ];
    case "missing":
      return [
        ["Resolved path", `not found (${installed.program})`],
        ["Version", "unknown"],
        ["Conversations", "unavailable: not installed"],
      ];
    case "too_old":
    case "too_new":
      return [
        ["Resolved path", installed.path],
        ["Version", installed.version ?? "unknown"],
        [
          "Conversations",
          `unavailable: it speaks ACP ${String(installed.speaks)}, Vessel speaks ${String(installed.needs)}`,
        ],
      ];
    case "unknown":
      return [
        ["Resolved path", `unknown (${unknown(installed.reason)})`],
        ["Version", "unknown"],
        ["Conversations", "unknown"],
      ];
  }
}

function signedIn(signedIn: SignedIn): string {
  switch (signedIn.state) {
    case "yes":
      return "signed in";
    case "no":
      return "not signed in";
    case "not_required":
      return "not required (a model route)";
    case "unknown":
      return `unknown (${unknown(signedIn.reason)})`;
  }
}

function unknown(reason: UnknownReason): string {
  switch (reason.reason) {
    case "not_installed":
      return "not installed";
    case "not_checked":
      return "not checked";
    case "in_use":
      return "in use by a session";
    case "not_said":
      return "the agent did not say";
    case "no_home":
      return "no home for the profile";
    case "unread":
      return "its answer was not read";
    case "timed_out":
      return `timed out after ${String(reason.after)} ms`;
    case "failed":
      return `failed at ${checkStep(reason.step)}`;
  }
}

function checkStep(step: CheckStep): string {
  switch (step) {
    case "spawn":
      return "spawn";
    case "initialize":
      return "initialize";
    case "session_new":
      return "session/new";
    case "exited":
      return "exit";
  }
}

function step(step: ProbeStep): string {
  switch (step.state) {
    case "ok":
      return "ok";
    case "failed":
      return `error: ${step.reason}`;
    case "not_reached":
      return "not reached";
  }
}

function initialize(diagnostic: AgentDiagnostic): string {
  const version = diagnostic.probe.agent_version;
  return diagnostic.probe.initialize.state === "ok" &&
    version !== undefined &&
    version !== null
    ? `ok (agent ${version})`
    : step(diagnostic.probe.initialize);
}

function catalog(status: CatalogStatus): [string, string][] {
  switch (status.state) {
    case "ready":
      return [
        ["Models", String(status.models)],
        ["Status", "Ready"],
      ];
    case "error":
      return [
        ["Models", "—"],
        ["Status", `Error: ${status.message}`],
      ];
    case "unavailable":
      return [
        ["Models", "—"],
        ["Status", "Unavailable"],
      ];
  }
}
