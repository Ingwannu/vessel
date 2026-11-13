// Which project or worktree a session's code panel reads: the session's
// own worktree when it has one, else its project's folder. A session
// started in a typed folder has neither (its `workspace` is absent) and
// its panel offers to add the folder as a project instead.

import type { ErrorCode, SessionInfo, WorkspaceRoot } from "../../wire/proto";
import type { Problem } from "../outcome";

/** The root a session's panel reads, or `null` when it is in no project. */
export function sessionRoot(session: SessionInfo): WorkspaceRoot | null {
  const workspace = session.workspace ?? null;
  if (workspace === null) {
    return null;
  }
  const worktree = workspace.worktree ?? null;
  return worktree === null
    ? { kind: "project", project: workspace.project }
    : { kind: "worktree", worktree };
}

/** A root as text, to key what is kept per root. */
export function rootKey(root: WorkspaceRoot): string {
  return root.kind === "project"
    ? `project:${root.project}`
    : `worktree:${root.worktree}`;
}

/** The session's branch, when it is in a repository on one. */
export function sessionBranch(session: SessionInfo): string | null {
  return session.workspace?.branch ?? null;
}

/** The node's error code behind `problem`, when the node answered with
 * one (`conflict` for a write the file changed under, `unsupported` for
 * a folder without git, `not_found`, `unavailable` while git holds a
 * lock). */
export function nodeError(problem: Problem): ErrorCode | null {
  return problem.from === "call" && problem.failure.kind === "remote"
    ? problem.failure.error.code
    : null;
}
