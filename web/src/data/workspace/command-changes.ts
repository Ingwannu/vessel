// What a project's commands to approve are, and what changed since the
// set last approved (ADR 0027: a repository's commands run only when a
// person approved that exact set; any change is asked again; ADR 0031:
// the set's scripts and services are in it).
//
// [Decision Log]
// - Purpose: "show exactly what will run, what changed since the last
//   approval, and send that digest to `project.approve`". A set with
//   scripts is approved for its scripts too: they are shown, every one,
//   or the set is not offered for approval.
// - Constraint: a node of protocol 11 carries, beside a set that is not
//   approved, the set last approved with its time (`last_approved`), when
//   it kept what that approval covered (approvals given from protocol 11
//   on). A node of protocol 10 carries neither that nor scripts; what the
//   page can know there is the project's folder's set and whether that
//   one is approved.
// - Alternatives: compare only with the folder (B-page's first answer:
//   wrong when the set approved last was another branch's); keep what the
//   page showed (lost with the tab, and another page or the CLI may have
//   approved since); a text diff of the whole file (the digest covers
//   commands, not comments or order of tables).
// - Choice: compare with `last_approved` when the node gives it; else
//   with the folder's set when that one is approved and differs (a node
//   of protocol 10, or an approval from before the node kept sets); else
//   say that nothing is known to compare with. Setup and teardown are
//   compared line by line (a longest common subsequence over at most 32
//   commands), a removed line followed by an added one read as one
//   changed command; scripts are compared by name.
// - Cost: a command moved within its list reads as removed and added,
//   which it is to the digest.

import type {
  CommandSet,
  ProjectCommand,
  ProjectCommands,
} from "../../wire/proto";
import { allocationForDisplay } from "./allocation-fields";
import {
  compareScripts,
  hasDeclaredPorts,
  type ScriptLine,
} from "./script-changes";

export { compareScripts, type ScriptLine };

/** A set of commands as the node lists it. */
export type ListedCommands = Extract<ProjectCommands, { state: "listed" }>;

/** One line of a list compared with another, as a diff reads it. */
export interface ListLine {
  change: "same" | "added" | "removed";
  command: ProjectCommand;
}

/** One command of a list compared with the approved one. */
export type CommandLine =
  | ListLine
  /** In the place of `was`: one command edited. */
  | { change: "changed"; command: ProjectCommand; was: ProjectCommand };

/** `after` compared with `before`, in order, removed lines before the
 * added ones that took their place. */
export function compareLists(
  before: readonly ProjectCommand[],
  after: readonly ProjectCommand[],
): ListLine[] {
  // Lengths of the longest common subsequence of every pair of suffixes.
  const rows = before.length + 1;
  const columns = after.length + 1;
  const table = new Array<number>(rows * columns).fill(0);
  const at = (i: number, j: number): number => table[i * columns + j] ?? 0;
  for (let i = before.length - 1; i >= 0; i -= 1) {
    for (let j = after.length - 1; j >= 0; j -= 1) {
      table[i * columns + j] =
        before[i] === after[j]
          ? at(i + 1, j + 1) + 1
          : Math.max(at(i + 1, j), at(i, j + 1));
    }
  }
  const lines: ListLine[] = [];
  let i = 0;
  let j = 0;
  while (i < before.length || j < after.length) {
    const old = before[i];
    const next = after[j];
    if (old !== undefined && next !== undefined && old === next) {
      lines.push({ change: "same", command: old });
      i += 1;
      j += 1;
    } else if (
      old !== undefined &&
      (next === undefined || at(i + 1, j) >= at(i, j + 1))
    ) {
      lines.push({ change: "removed", command: old });
      i += 1;
    } else if (next !== undefined) {
      lines.push({ change: "added", command: next });
      j += 1;
    }
  }
  return lines;
}

/** `after` against `before`, a run of removed commands followed by a run
 * of added ones paired, in order, into changed commands. */
export function compareCommands(
  before: readonly ProjectCommand[],
  after: readonly ProjectCommand[],
): CommandLine[] {
  const lines = compareLists(before, after);
  const paired: CommandLine[] = [];
  let index = 0;
  // The commands of a run of one kind from `index` on.
  const run = (kind: ListLine["change"]): ProjectCommand[] => {
    const taken: ProjectCommand[] = [];
    for (let line = lines[index]; line?.change === kind; line = lines[index]) {
      taken.push(line.command);
      index += 1;
    }
    return taken;
  };
  while (index < lines.length) {
    const same = run("same");
    const removed = run("removed");
    const added = run("added");
    for (const command of same) {
      paired.push({ change: "same", command });
    }
    removed.forEach((was, at) => {
      const command = added[at];
      paired.push(
        command === undefined
          ? { change: "removed", command: was }
          : { change: "changed", command, was },
      );
    });
    for (const command of added.slice(removed.length)) {
      paired.push({ change: "added", command });
    }
  }
  return paired;
}

/** What the set asked for is compared with. */
export type ComparedWith =
  /** The set the node says was approved last, and when. */
  | { kind: "last_approved"; at: string }
  /** The project folder's set, approved (a node that does not say). */
  | { kind: "folder" };

/** What the approval dialog shows beside the set asked for. */
export type Compared =
  | {
      kind: "changed";
      against: ComparedWith;
      setup: readonly CommandLine[];
      teardown: readonly CommandLine[];
      scripts: readonly ScriptLine[];
    }
  /** No approved set is known to compare with. */
  | { kind: "none" }
  /** A partial comparison must never claim hidden terminal commands are unchanged. */
  | { kind: "unsupported_terminals" }
  /** A declared port cannot disappear from a partial comparison. */
  | { kind: "unsupported_ports" };

/** A refusal the dialog can explain without offering partial approval. */
export type ApprovalRefusal = Extract<
  Compared,
  { kind: "unsupported_terminals" | "unsupported_ports" }
>;

/** Whether this page covers all fields in both snapshots. */
export type ApprovalCoverage = "complete" | ApprovalRefusal["kind"];

/** A set's lists, as both an approval and a listing carry them. */
function setOf(commands: CommandSet): Required<CommandSet> {
  // Required makes even an optional new wire field a compile error here.
  return {
    setup: commands.setup,
    teardown: commands.teardown,
    scripts: commands.scripts ?? [],
    terminals: commands.terminals ?? [],
    service_ports: commands.service_ports ?? null,
  };
}

/** Presence alone refuses a block, including a malformed empty peer value. */
function hasPortConfiguration(set: Required<CommandSet>): boolean {
  return (
    hasDeclaredPorts(set.scripts) ||
    allocationForDisplay(set.service_ports) !== null
  );
}

/** Refuse approval while a current or previous set contains commands this page cannot show. */
export function approvalCoverage(asked: ListedCommands): ApprovalCoverage {
  const now = setOf(asked);
  const previous = asked.last_approved?.commands;
  const before = previous === undefined ? null : setOf(previous);
  if (
    now.terminals.length > 0 ||
    (before !== null && before.terminals.length > 0)
  ) {
    return "unsupported_terminals";
  }
  if (
    hasPortConfiguration(now) ||
    (before !== null && hasPortConfiguration(before))
  ) {
    return "unsupported_ports";
  }
  return "complete";
}

/** `asked` against an approved set. */
function against(
  asked: ListedCommands,
  approved: CommandSet,
  basis: ComparedWith,
): Compared {
  const before = setOf(approved);
  const after = setOf(asked);
  if (before.terminals.length > 0 || after.terminals.length > 0) {
    return { kind: "unsupported_terminals" };
  }
  if (hasPortConfiguration(before) || hasPortConfiguration(after)) {
    return { kind: "unsupported_ports" };
  }
  return {
    kind: "changed",
    against: basis,
    setup: compareCommands(before.setup, after.setup),
    teardown: compareCommands(before.teardown, after.teardown),
    scripts: compareScripts(before.scripts, after.scripts),
  };
}

/** The set `asked` compared with the set last approved, or, from a node
 * that does not say, with the folder's (`folder`, `null` when it was not
 * read or could not be). */
export function compareApproved(
  asked: ListedCommands,
  folder: ProjectCommands | null,
): Compared {
  const coverage = approvalCoverage(asked);
  if (coverage !== "complete") {
    return { kind: coverage };
  }
  const last = asked.last_approved ?? null;
  if (last !== null) {
    return against(asked, last.commands, {
      kind: "last_approved",
      at: last.approved_at,
    });
  }
  if (
    folder?.state !== "listed" ||
    !folder.approved ||
    folder.digest === asked.digest
  ) {
    return { kind: "none" };
  }
  return against(asked, folder, { kind: "folder" });
}

/** Whether the dialog must read the folder's set to compare: only when
 * the node did not say which set was approved last. */
export function needsFolder(asked: ListedCommands): boolean {
  return (asked.last_approved ?? null) === null;
}
