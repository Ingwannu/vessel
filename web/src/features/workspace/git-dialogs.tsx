// The dialogs the Changes tab opens: commit, switch, create or rename a
// branch, the stash, and the confirmation before discarding a file.

import type { NodeKey } from "../../wire/bridge";
import type { FileChange, GitStatus, WorkspaceRoot } from "../../wire/proto";
import { BranchDialog } from "./branch-dialog";
import { CommitDialog } from "./commit-dialog";
import { DiscardDialog } from "./discard-dialog";
import type { GitDialog } from "./git-bar";
import { StashDialog } from "./stash-dialog";
import type { Git } from "./use-git";

/** Whether `branch` is the project's default (`origin/main` names
 * `main`). */
function isDefault(
  branch: string | null,
  defaultBranch: string | null,
): boolean {
  return (
    branch !== null &&
    defaultBranch !== null &&
    (branch === defaultBranch || defaultBranch.endsWith(`/${branch}`))
  );
}

export function GitDialogs({
  node,
  root,
  status,
  git,
  dialog,
  discarding,
  untracked,
  deletes,
  defaultBranch,
  onClose,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  status: GitStatus | null;
  git: Git;
  dialog: GitDialog | null;
  discarding: FileChange | null;
  /** Whether the file being discarded is untracked. */
  untracked: boolean;
  /** Whether discarding it deletes it (untracked, or added). */
  deletes: boolean;
  defaultBranch: string | null;
  onClose: () => void;
}) {
  return (
    <>
      {status === null || dialog === null ? null : dialog === "commit" ? (
        <CommitDialog
          status={status}
          git={git}
          isDefault={isDefault(status.branch ?? null, defaultBranch)}
          onClose={onClose}
        />
      ) : dialog === "stashes" ? (
        <StashDialog node={node} root={root} git={git} onClose={onClose} />
      ) : (
        <BranchDialog
          mode={dialog}
          node={node}
          root={root}
          status={status}
          git={git}
          onClose={onClose}
        />
      )}
      {discarding === null ? null : (
        <DiscardDialog
          change={discarding}
          untracked={untracked}
          deletes={deletes}
          git={git}
          onClose={onClose}
        />
      )}
    </>
  );
}
