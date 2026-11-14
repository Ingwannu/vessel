// What git says about a project or a worktree (`git.*` reads, protocol
// 10): typed calls in the shape of `data/calls.ts`. The writes are
// `git-writes.ts`'s.
//
// Every list the node answers is bounded and says `more`; a file's diff
// is paged by hunk (`more_from`), a file at a commit by byte. The page
// asks for the next page when the reader reaches it, never for all.

import type { NodeKey } from "../../wire/bridge";
import type {
  BranchList,
  CommitPage,
  DiffScope,
  DiffSummary,
  FileDiff,
  FileRead,
  GitStatus,
  RelPath,
  StashList,
  WorkspaceRoot,
  WorktreeBase,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** Branch, upstream, base and changed files of `root`. */
export function readStatus(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
): Promise<Outcome<GitStatus>> {
  return attempt(() => fleet.bridge.call(node, "git.status", { root }));
}

/** The files `scope` changes, with their counts. */
export function readChanges(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  scope: DiffScope,
  ignoreWhitespace: boolean,
): Promise<Outcome<DiffSummary>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.changes", {
      root,
      scope,
      ignore_whitespace: ignoreWhitespace,
    }),
  );
}

/** One file's diff in `scope`, its hunks from `fromHunk` (the previous
 * page's `more_from`). */
export function readFileDiff(
  fleet: Fleet,
  node: NodeKey,
  diff: {
    root: WorkspaceRoot;
    scope: DiffScope;
    path: RelPath;
    ignoreWhitespace: boolean;
    fromHunk: number | null;
  },
): Promise<Outcome<FileDiff>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.diff", {
      root: diff.root,
      scope: diff.scope,
      path: diff.path,
      ignore_whitespace: diff.ignoreWhitespace,
      ...(diff.fromHunk === null ? {} : { from_hunk: diff.fromHunk }),
    }),
  );
}

/** A page of `path` as it is at `at` (the old side of a diff). */
export function showFile(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  at: WorktreeBase,
  path: RelPath,
  offset: number,
): Promise<Outcome<FileRead>> {
  return attempt(() =>
    fleet.bridge.call(node, "git.show", { root, at, path, offset }),
  );
}

/** Local and remote-tracking branches. */
export function readBranches(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
): Promise<Outcome<BranchList>> {
  return attempt(() => fleet.bridge.call(node, "git.branches", { root }));
}

/** A page of the log from `HEAD`, `skip` commits in. */
export function readLog(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  skip: number,
): Promise<Outcome<CommitPage>> {
  return attempt(() => fleet.bridge.call(node, "git.log", { root, skip }));
}

/** The repository's stash (shared by all of its worktrees). */
export function readStashes(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
): Promise<Outcome<StashList>> {
  return attempt(() => fleet.bridge.call(node, "git.stashes", { root }));
}
