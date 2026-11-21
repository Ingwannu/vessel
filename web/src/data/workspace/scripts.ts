// The legacy script RPC already resolves the checkout and the exact
// approved configuration on its owning node. These calls use that path
// while the broader workspace service API is still staged.
import type { NodeKey } from "../../wire/bridge";
import type {
  ScriptKind,
  ScriptList,
  ScriptPlace,
  ScriptRefusal,
  ScriptRunOutcome,
  TerminalId,
  TerminalInfo,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

export const SCRIPT_PROTOCOL = 11;

export type ScriptAction =
  { kind: "run"; name: string } | { kind: "restart"; terminal: TerminalId };

export interface ScriptRow {
  name: string;
  kind: ScriptKind;
  terminal: TerminalInfo | null;
  configured: boolean;
}

export function listScripts(
  fleet: Fleet,
  node: NodeKey,
  place: ScriptPlace,
): Promise<Outcome<ScriptList>> {
  return attempt(() => fleet.bridge.call(node, "script.list", { place }));
}

export function runScript(
  fleet: Fleet,
  node: NodeKey,
  place: ScriptPlace,
  action: ScriptAction,
): Promise<Outcome<ScriptRunOutcome>> {
  return attempt(() =>
    action.kind === "run"
      ? fleet.bridge.call(node, "script.run", { place, script: action.name })
      : fleet.bridge.call(node, "script.restart", {
          terminal: action.terminal,
        }),
  );
}

export function stopScript(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
): Promise<Outcome<null>> {
  return attempt(() => fleet.bridge.call(node, "terminal.kill", { terminal }));
}

export function scriptIsLive(terminal: TerminalInfo | null): boolean {
  return (
    terminal?.status.state === "starting" ||
    terminal?.status.state === "running"
  );
}

export function scriptRows(list: ScriptList): ScriptRow[] {
  const rows = new Map<string, ScriptRow>();
  if (list.commands.state === "listed") {
    for (const script of list.commands.scripts ?? []) {
      rows.set(script.name, {
        name: script.name,
        kind: script.kind,
        terminal: null,
        configured: true,
      });
    }
  }
  // The node orders runs newest first. Prefer a live run over any kept
  // output, including live scripts removed from the current checkout.
  for (const terminal of list.runs) {
    const kind = terminal.kind;
    if (kind.kind !== "script" && kind.kind !== "service") continue;
    const current = rows.get(kind.script);
    const live = scriptIsLive(terminal);
    if (current === undefined && !live) continue;
    if (current?.terminal != null && (scriptIsLive(current.terminal) || !live))
      continue;
    rows.set(kind.script, {
      name: kind.script,
      kind: live ? kind.kind : (current?.kind ?? kind.kind),
      terminal,
      configured: current?.configured ?? false,
    });
  }
  return [...rows.values()];
}

export function scriptStatus(row: ScriptRow): string {
  switch (row.terminal?.service?.state) {
    case "starting":
      return "Starting";
    case "running":
      return "Running";
    case "backing_off":
      return "Restarting";
    case "failed":
      return "Failed";
    case "exited":
      return "Exited";
    case "stopped":
      return "Stopped";
    case "unknown":
      return "Unknown";
    case undefined:
      break;
  }
  switch (row.terminal?.status.state) {
    case "starting":
      return "Starting";
    case "running":
      return "Running";
    case "failed":
      return "Failed";
    case "exited":
      return "Exited";
    default:
      return "Stopped";
  }
}

export function scriptRefusal(reason: ScriptRefusal): string {
  switch (reason.kind) {
    case "no_project":
      return "This workspace has no project file.";
    case "no_such_script":
      return "The project file no longer defines this script.";
    case "no_such_dir":
      return "The script’s directory is unavailable in this checkout.";
    case "invalid_file":
      return reason.message;
    case "already_running":
      return "This script is already running.";
    case "unknown":
      return "The machine refused this action.";
  }
}
