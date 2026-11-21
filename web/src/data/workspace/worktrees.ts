// A project's worktrees and their setup (`worktree.*`, protocol 10):
// typed calls in the shape of `data/calls.ts`.
//
// [Decision Log]
// - Purpose: the new-session page makes a worktree, follows its setup
//   and then starts the session in it; the panel and Settings remove one
//   and say what would be lost.
// - Constraint: the node answers `worktree.create` at once and runs setup
//   in the background; a session in a worktree whose setup runs is
//   refused (`conflict`), so the page must wait on the output first.
// - Choice: the calls here, the order in `data/workspace/start.ts`.

import type { NodeKey } from "../../wire/bridge";
import type {
  CreateWorktreeOutcome,
  IfLost,
  ProjectId,
  RemoveWorktreeOutcome,
  SetupChoice,
  SetupOutput,
  SnapshotList,
  WorktreeBranch,
  WorktreeId,
  WorktreeInfo,
  WorktreeList,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** The longest a read of setup's output waits for more, in seconds (the
 * node's own bound). */
export const OUTPUT_WAIT_S = 30;

/** The warning and its advisory count from a single removal attempt. */
export type KeptRemoval = Extract<RemoveWorktreeOutcome, { outcome: "kept" }>;

/** Every worktree of `node`, or of one project. */
export function listWorktrees(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId | null,
): Promise<Outcome<WorktreeList>> {
  return attempt(() =>
    fleet.bridge.call(
      node,
      "worktree.list",
      project === null ? {} : { project },
    ),
  );
}

/** Make a worktree of `project` on `branch`; setup runs (unless
 * skipped) in the background. `needs_approval` when the base's commands
 * are not approved: nothing was made. */
export function createWorktree(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
  branch: WorktreeBranch,
  setup: SetupChoice,
): Promise<Outcome<CreateWorktreeOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "worktree.create", { project, branch, setup }),
  );
}

/** A page of setup's output from byte `from`; with `wait`, the node
 * answers when there is more or setup ends (at most `OUTPUT_WAIT_S`). */
export function readSetupOutput(
  fleet: Fleet,
  node: NodeKey,
  worktree: WorktreeId,
  from: number,
  wait: boolean,
): Promise<Outcome<SetupOutput>> {
  return attempt(() =>
    fleet.bridge.call(node, "worktree.output", {
      worktree,
      from,
      ...(wait ? { wait: OUTPUT_WAIT_S } : {}),
    }),
  );
}

/** Run setup again (after an approval, a failure or a skip), with the
 * commands of the worktree's file as it is now. */
export function runSetup(
  fleet: Fleet,
  node: NodeKey,
  worktree: WorktreeId,
): Promise<Outcome<WorktreeInfo>> {
  return attempt(() => fleet.bridge.call(node, "worktree.setup", { worktree }));
}

/** Remove a worktree. With `keep`, one where something would be lost is
 * kept and the answer says why; `snapshot` keeps a snapshot first;
 * `discard` removes it anyway and deletes its branch. `endTerminals`
 * (protocol 40, ADR 0121) ends the live terminals in it first; send it
 * only after the person saw how many (Paseo's terminal warning). */
export function removeWorktree(
  fleet: Fleet,
  node: NodeKey,
  worktree: WorktreeId,
  ifLost: IfLost,
  endTerminals = false,
): Promise<Outcome<RemoveWorktreeOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "worktree.remove", {
      worktree,
      if_lost: ifLost,
      ...(endTerminals ? { end_terminals: true } : {}),
    }),
  );
}

/** A project's snapshots (kept when a worktree was removed). */
export function listSnapshots(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
): Promise<Outcome<SnapshotList>> {
  return attempt(() =>
    fleet.bridge.call(node, "worktree.snapshots", { project }),
  );
}

/** Delete one snapshot's ref. */
export function dropSnapshot(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
  worktree: WorktreeId,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "worktree.drop_snapshot", { project, worktree }),
  );
}
