// The approval dialog's sentences: what changed since the set approved
// last, said plainly (the brief: "added, removed and changed commands,
// said plainly"), and what a script's change was.

import type { ScriptEntry, ScriptKind } from "../../wire/proto";
import { when } from "../format";
import type { CommandLine, Compared, ScriptLine } from "./command-changes";

/** What a script or a service is called in a sentence. */
export function scriptKindWord(kind: ScriptKind): string {
  switch (kind) {
    case "script":
      return "script";
    case "service":
      return "service";
  }
}

/** What differs between two entries of one name ("its command", "now a
 * service"), joined; empty when nothing does. */
export function scriptChangeWords(was: ScriptEntry, now: ScriptEntry): string {
  const parts: string[] = [];
  if (was.kind !== now.kind) {
    parts.push(`now a ${scriptKindWord(now.kind)}`);
  }
  if (was.command !== now.command) {
    parts.push("its command");
  }
  if ((was.dir ?? null) !== (now.dir ?? null)) {
    parts.push("its folder");
  }
  if ((was.port ?? null) !== (now.port ?? null)) {
    parts.push("its port");
  }
  return parts.join(" and ");
}

/** "setup commands: 1 added and 1 changed", or `null` when the list is
 * the same. */
function listPart(name: string, lines: readonly CommandLine[]): string | null {
  const said: string[] = [];
  const changes: CommandLine["change"][] = ["added", "changed", "removed"];
  for (const change of changes) {
    const count = lines.filter((line) => line.change === change).length;
    if (count > 0) {
      said.push(`${String(count)} ${change}`);
    }
  }
  if (said.length === 0) {
    return null;
  }
  return `${name} commands: ${said.join(" and ")}`;
}

/** "script lint added", "service web changed (its command)". */
function scriptPart(line: ScriptLine): string | null {
  const named = `${scriptKindWord(line.script.kind)} ${line.script.name}`;
  switch (line.change) {
    case "same":
      return null;
    case "added":
      return `${named} added`;
    case "removed":
      return `${named} removed`;
    case "changed":
      return `${named} changed (${scriptChangeWords(line.was, line.script)})`;
  }
}

/** What the dialog says above the lists: what the set is compared with,
 * and what changed. */
export function comparedWords(compared: Compared): string {
  if (compared.kind === "unsupported_terminals") {
    return "This set or its previous approval includes terminal commands that this window cannot show. Approval is unavailable here.";
  }
  if (compared.kind === "unsupported_ports") {
    return "This set or its previous approval includes service port declarations or allocation settings that this window cannot show. Approval is unavailable here.";
  }
  if (compared.kind === "none") {
    return "No earlier approval of this project's commands is known, so every command is shown as it is.";
  }
  const parts = [
    listPart("setup", compared.setup),
    listPart("teardown", compared.teardown),
    ...compared.scripts.map(scriptPart),
  ].filter((part): part is string => part !== null);
  const changed =
    parts.length === 0
      ? "nothing that runs reads differently"
      : parts.join("; ");
  switch (compared.against.kind) {
    case "last_approved":
      return `Since the set approved ${when(compared.against.at)}: ${changed}.`;
    case "folder":
      return `Against the set approved for the project's folder (this machine does not say which set was approved last): ${changed}.`;
  }
}
