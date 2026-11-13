// What a project's commands to approve are, and what changed from a set
// already approved (ADR 0027: a repository's commands run only when a
// person approved that exact set; any change is asked again).
//
// [Decision Log]
// - Purpose: "show exactly what will run, what changed since the last
//   approval, and send that digest to `project.approve`".
// - Constraint: the node keeps only the digests it approved, not the
//   commands (store schema 9's `approvals`), so it cannot say what the
//   last approved set was. What the page can know: the set in the
//   project's folder as it is, and whether that one is approved. A new
//   worktree's set is read from its base branch, which is where a pull or
//   an agent's edit changes it.
// - Choice: compare the set asked for with the folder's when the
//   folder's is approved and differs (the common case: a branch changed
//   `vessel.toml`); otherwise say that no approved set is known to
//   compare with. Each list is compared line by line (a longest common
//   subsequence over at most 32 commands), so a moved command reads as
//   removed and added, which it is to the digest.
// - Cost: a set approved for another branch and then changed again is
//   compared with the folder's, not with that one. The wire gap is
//   reported in the plan ("Track B-page: progress").

import type { ProjectCommand, ProjectCommands } from "../../wire/proto";

/** A set of commands as the node lists it. */
export type ListedCommands = Extract<ProjectCommands, { state: "listed" }>;

/** One line of a list compared with an approved one. */
export interface CommandLine {
  change: "same" | "added" | "removed";
  command: ProjectCommand;
}

/** `after` compared with `before`, in order, removed lines before the
 * added ones that took their place. */
export function compareLists(
  before: readonly ProjectCommand[],
  after: readonly ProjectCommand[],
): CommandLine[] {
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
  const lines: CommandLine[] = [];
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

/** What the approval dialog shows beside the set asked for. */
export type Compared =
  /** The folder's approved set, and each list against it. */
  | {
      kind: "changed";
      setup: readonly CommandLine[];
      teardown: readonly CommandLine[];
    }
  /** No approved set is known to compare with. */
  | { kind: "none" };

/** The set `asked` compared with the folder's (`folder`, `null` when it
 * could not be read). */
export function compareWithFolder(
  asked: ListedCommands,
  folder: ProjectCommands | null,
): Compared {
  if (
    folder?.state !== "listed" ||
    !folder.approved ||
    folder.digest === asked.digest
  ) {
    return { kind: "none" };
  }
  return {
    kind: "changed",
    setup: compareLists(folder.setup, asked.setup),
    teardown: compareLists(folder.teardown, asked.teardown),
  };
}
