// Changing a project's or a worktree's repository (`git.*` writes,
// protocol 10): typed calls in the shape of `data/calls.ts`.
//
// A refusal git can explain (nothing to commit, a push refused, a
// credential wanted, a lock held) is part of the answer, not a failure:
// `refused { refusal }`, which `git-words.ts` says in plain words. A
// failure of the call itself (the machine gone, `unavailable` while
// another git holds the index lock) is the outcome's problem.

import type { NodeKey } from "../../wire/bridge";
import type {
  BranchName,
  CommitOutcome,
  CommitStage,
  FetchOutcome,
  GitStatus,
  PathSet,
  PullOutcome,
  PushOutcome,
  RenameOutcome,
  StashAction,
  StashOutcome,
  SwitchOutcome,
  WorkspaceRoot,
  WorktreeBase,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** Stage `paths`; answers the status after. */
export function stage(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  paths: PathSet,
): Promise<Outcome<GitStatus>> {
  return attempt(() => fleet.bridge.call(node, "git.stage", { root, paths }));
}

/** Unstage `paths`; answers the status after. */
export function unstage(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  paths: PathSet,
): Promise<Outcome<GitStatus>> {
  return attempt(() => fleet.bridge.call(node, "git.unstage", { root, paths }));
}

/** Restore tracked `paths` from `HEAD` and, with `untracked`, delete the
 * untracked ones named (never ignored files). */
export function discard(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  paths: PathSet,
  untracked: boolean,
): Promise<Outcome<GitStatus>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.discard", { root, paths, untracked }),
  );
}

/** Commit with the repository's own identity; its hooks run. */
export function commit(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  message: string,
  stageFirst: CommitStage,
): Promise<Outcome<CommitOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.commit", {
      root,
      message,
      stage: stageFirst,
    }),
  );
}

/** Fetch the branch's remote (else `origin`). */
export function fetch(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
): Promise<Outcome<FetchOutcome>> {
  return attempt(() => fleet.bridge.call(node, "git.fetch", { root }));
}

/** Push the current branch to its upstream, or, with `setUpstream`, to
 * `origin` and make it the upstream. Never forced. */
export function push(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  setUpstream: boolean,
): Promise<Outcome<PushOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.push", { root, set_upstream: setUpstream }),
  );
}

/** Pull, fast-forward only. */
export function pull(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
): Promise<Outcome<PullOutcome>> {
  return attempt(() => fleet.bridge.call(node, "git.pull", { root }));
}

/** Switch to `branch`, or create it from `create` first. */
export function switchBranch(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  branch: BranchName,
  create: WorktreeBase | null,
): Promise<Outcome<SwitchOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.switch", {
      root,
      branch,
      ...(create === null ? {} : { create }),
    }),
  );
}

/** Rename the current branch to `to`. */
export function renameBranch(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  to: BranchName,
): Promise<Outcome<RenameOutcome>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.rename_branch", { root, to }),
  );
}

/** Save, apply, pop or drop a stash entry (named by its index and its
 * commit, since the stash is the repository's and its indexes move). */
export function stash(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  action: StashAction,
): Promise<Outcome<StashOutcome>> {
  return attempt(() => fleet.bridge.call(node, "git.stash", { root, action }));
}
