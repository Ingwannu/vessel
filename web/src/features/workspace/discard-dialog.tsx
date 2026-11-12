// "Discard changes?" (Paseo's confirm): what is thrown away, before it is.
// A tracked file goes back to its last commit; one the last commit does
// not have (untracked, or added since) is deleted, which the dialog says
// in its own words, since nothing brings it back.

import { Button, Dialog } from "../../ui/kit";
import type { FileChange } from "../../wire/proto";
import { GitNotice } from "./git-notice";
import type { Git } from "./use-git";

export function DiscardDialog({
  change,
  untracked,
  deletes,
  git,
  onClose,
}: {
  change: FileChange;
  /** Whether git does not track the file (`git clean` deletes it). */
  untracked: boolean;
  /** Whether discarding deletes the file: untracked, or added since the
   * last commit. */
  deletes: boolean;
  git: Git;
  onClose: () => void;
}) {
  const name = change.path.split("/").at(-1) ?? change.path;
  return (
    <Dialog
      title="Discard changes?"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="danger-solid"
            busy={git.busy === "discard"}
            onClick={() => {
              void git
                .discard({ kind: "paths", paths: [change.path] }, untracked)
                .then((done) => {
                  if (done) {
                    onClose();
                  }
                });
            }}
          >
            {deletes ? "Delete" : "Discard"}
          </Button>
        </>
      }
    >
      <p className="dialog-copy">
        {untracked
          ? `"${name}" is not tracked by git: discarding deletes the file, and nothing can bring it back.`
          : deletes
            ? `"${name}" is not in the last commit: discarding deletes the file, staged or not, and nothing can bring it back.`
            : `Changes to "${name}" will be permanently discarded: it goes back to its last commit, staged and unstaged changes alike.`}
      </p>
      {git.notice === null ? null : <GitNotice notice={git.notice} />}
    </Dialog>
  );
}
