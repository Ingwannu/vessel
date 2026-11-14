// The git button's quick action and why each action is off (GOAT's
// `GitActionsControl.logic.ts`): one button names the action that makes
// sense now, its menu holds the rest, and an action that cannot run says
// why instead of failing after a round trip.

import type { GitStatus } from "../../wire/proto";

/** The actions of the git menu that depend on the repository's state. */
export type GitAction = "commit" | "push" | "pull" | "fetch";

/** What the quick button does now: commit when there are changes, else
 * push when ahead, pull when behind, else fetch. */
export function quickAction(status: GitStatus): GitAction {
  if (status.files.length > 0) {
    return "commit";
  }
  const upstream = status.upstream ?? null;
  if (upstream === null) {
    return status.branch === null || status.branch === undefined
      ? "fetch"
      : "push";
  }
  if (upstream.ahead > 0) {
    return "push";
  }
  return upstream.behind > 0 ? "pull" : "fetch";
}

/** What the action is called on its button. */
export function actionLabel(action: GitAction): string {
  switch (action) {
    case "commit":
      return "Commit";
    case "push":
      return "Push";
    case "pull":
      return "Pull";
    case "fetch":
      return "Fetch";
  }
}

/** Why `action` cannot run on `status` (`null`: it can). GOAT's words
 * where it has them. */
export function actionBlocked(
  action: GitAction,
  status: GitStatus,
): string | null {
  const detached = status.branch === null || status.branch === undefined;
  const upstream = status.upstream ?? null;
  switch (action) {
    case "commit":
      return status.files.length === 0
        ? "Worktree is clean. Make changes before committing."
        : null;
    case "push":
      if (detached) {
        return "Detached HEAD: check out a branch before pushing.";
      }
      if (upstream === null) {
        return null;
      }
      if (upstream.ahead > 0 && upstream.behind > 0) {
        return "Branch has diverged from upstream. Rebase/merge first.";
      }
      if (upstream.behind > 0) {
        return "Branch is behind upstream. Pull/rebase before pushing.";
      }
      return upstream.ahead === 0 ? "No local commits to push." : null;
    case "pull":
      if (detached) {
        return "Detached HEAD: check out a branch before pulling.";
      }
      return upstream === null ? "This branch has no upstream to pull." : null;
    case "fetch":
      return null;
  }
}
