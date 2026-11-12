// Removing a worktree that would lose work (Paseo's archive warning,
// GOAT's snapshot before cleanup): the node first answers `kept` with
// every reason that applies; this dialog says each, then offers to keep a
// snapshot (a commit under the project's refs, restorable as a branch)
// and remove, or to discard, saying first what discarding destroys.

import { useState } from "react";

import { useFleet } from "../../context";
import { removeWorktree } from "../../data/workspace/worktrees";
import {
  keptReasonWords,
  snapshotCovers,
} from "../../data/workspace/worktree-words";
import { Banner, Button, Dialog } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { IfLost, KeptReason, WorktreeInfo } from "../../wire/proto";

export function RemoveWorktreeDialog({
  node,
  worktree,
  reasons,
  onClose,
  onRemoved,
}: {
  node: NodeKey;
  worktree: WorktreeInfo;
  /** What the node said would be lost. */
  reasons: readonly KeptReason[];
  onClose: () => void;
  onRemoved: () => void;
}) {
  const fleet = useFleet();
  const [busy, setBusy] = useState<IfLost | null>(null);
  const [kept, setKept] = useState<readonly KeptReason[]>(reasons);
  const [error, setError] = useState<string | null>(null);
  const branch = worktree.branch ?? worktree.created_branch ?? "this worktree";
  const inUse = kept.some((reason) => reason.kind === "in_use");
  const snapshotable = !inUse && kept.every(snapshotCovers);
  const remove = (ifLost: IfLost) => {
    setBusy(ifLost);
    setError(null);
    void removeWorktree(fleet, node, worktree.id, ifLost).then((outcome) => {
      setBusy(null);
      if (!outcome.ok) {
        setError(outcome.problem.message);
        return;
      }
      if (outcome.value.outcome === "kept") {
        setKept(outcome.value.reasons);
        setError(
          ifLost === "snapshot"
            ? "The snapshot was refused, so nothing was removed."
            : "Nothing was removed.",
        );
        return;
      }
      const snapshot = outcome.value.snapshot ?? null;
      toast(
        snapshot === null
          ? `Removed ${branch}`
          : `Removed ${branch}; its work is kept in snapshot ${snapshot.commit.slice(0, 7)}`,
      );
      onRemoved();
    });
  };
  return (
    <Dialog
      title={`Remove "${branch}"?`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="danger"
            disabled={inUse || busy !== null}
            busy={busy === "discard"}
            onClick={() => {
              remove("discard");
            }}
          >
            Discard and remove
          </Button>
          <Button
            variant="primary"
            disabled={!snapshotable || busy !== null}
            busy={busy === "snapshot"}
            onClick={() => {
              remove("snapshot");
            }}
          >
            Keep a snapshot and remove
          </Button>
        </>
      }
    >
      <p className="dialog-copy">Removing it now would lose:</p>
      <ul className="kept-list">
        {kept.map((reason, index) => (
          <li key={index}>{keptReasonWords(reason)}</li>
        ))}
      </ul>
      {inUse ? (
        <p className="dialog-copy">
          End the session in it (or wait for setup) before removing it.
        </p>
      ) : (
        <p className="dialog-copy">
          {snapshotable
            ? "A snapshot keeps every change and commit in a commit of the project's own; it can be made a branch again later. "
            : "A snapshot cannot keep all of this. "}
          Discarding deletes the folder at {worktree.path}, its uncommitted
          changes and its branch {worktree.created_branch ?? ""}; commits on no
          other branch are lost.
        </p>
      )}
      {error === null ? null : <Banner tone="error">{error}</Banner>}
    </Dialog>
  );
}
