// Removing a worktree that would lose work (Paseo's archive warning,
// GOAT's snapshot before cleanup): the node first answers `kept` with
// every reason that applies; this dialog says each, then offers to keep a
// snapshot (a commit under the project's refs, restorable as a branch)
// and remove, or to discard, saying first what discarding destroys.
//
// Terminals (ADR 0121, TRACKER 829): from protocol 40 a live terminal no
// longer has to be closed by hand. When live terminals are what keeps it
// (no live session in it), the dialog says Paseo's terminal warning with
// their count above the usual buttons, and those buttons ask the node to
// end them first (`end_terminals`). Paseo ends a worktree's terminals on
// archive without a word; Vessel asks (the ADR's ruling 2).

import { useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { isLive } from "../../data/format";
import {
  liveTerminalsIn,
  mayEndTerminals,
  speaksWorkspacePlaces,
} from "../../data/workspace/terminal-place";
import { workspaceTerminals } from "../../data/workspace/terminals";
import { removeWorktree } from "../../data/workspace/worktrees";
import {
  closesTerminals,
  keptReasonWords,
  snapshotCovers,
} from "../../data/workspace/worktree-words";
import { Banner, Button, Dialog } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  IfLost,
  KeptReason,
  SessionInfo,
  WorktreeInfo,
} from "../../wire/proto";

const NO_SESSIONS: readonly SessionInfo[] = [];

/** How many live terminals are in the worktree, read when the dialog
 * opens and after each answer (`null` until read, or below 40). */
function useLiveTerminals(
  node: NodeKey,
  worktree: WorktreeInfo,
  protocol: number | null,
  asked: number,
): number | null {
  const fleet = useFleet();
  const [count, setCount] = useState<number | null>(null);
  const speaks = speaksWorkspacePlaces(protocol);
  useEffect(() => {
    if (!speaks) {
      return undefined;
    }
    let live = true;
    void workspaceTerminals(fleet, node, null).then((result) => {
      if (live) {
        setCount(
          result.ok
            ? liveTerminalsIn(result.value.terminals, worktree.path)
            : null,
        );
      }
    });
    return () => {
      live = false;
    };
  }, [fleet, node, speaks, worktree.path, asked]);
  return speaks ? count : null;
}

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
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const sessions = useFleetState(
    (state) => state.sessions[node] ?? NO_SESSIONS,
  );
  const [busy, setBusy] = useState<IfLost | null>(null);
  const [kept, setKept] = useState<readonly KeptReason[]>(reasons);
  const [error, setError] = useState<string | null>(null);
  const [asked, setAsked] = useState(0);
  const liveTerminals = useLiveTerminals(node, worktree, protocol, asked);
  const branch = worktree.branch ?? worktree.created_branch ?? "this worktree";
  const inUse = kept.some((reason) => reason.kind === "in_use");
  const sessionInIt = sessions.some(
    (session) => session.workspace?.worktree === worktree.id && isLive(session),
  );
  const endTerminals = mayEndTerminals({
    protocol,
    inUse,
    liveTerminals: liveTerminals ?? 0,
    sessionInIt,
  });
  // With the terminals ended first, `in_use` no longer holds the removal.
  const blocked = inUse && !endTerminals;
  const others = kept.filter((reason) => reason.kind !== "in_use");
  const snapshotable = !blocked && others.every(snapshotCovers);
  const remove = (ifLost: IfLost) => {
    setBusy(ifLost);
    setError(null);
    void removeWorktree(fleet, node, worktree.id, ifLost, endTerminals).then(
      (outcome) => {
        setBusy(null);
        setAsked((count) => count + 1);
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
      },
    );
  };
  return (
    <RemoveWorktreeView
      worktree={worktree}
      kept={kept}
      busy={busy}
      error={error}
      blocked={blocked}
      snapshotable={snapshotable}
      closing={endTerminals ? liveTerminals : null}
      onClose={onClose}
      onRemove={remove}
    />
  );
}

/** The dialog as drawn from its state (kept apart so a test renders each
 * state without a node). `closing` is how many live terminals the
 * removal ends first, or `null` when it ends none. */
export function RemoveWorktreeView({
  worktree,
  kept,
  busy,
  error,
  blocked,
  snapshotable,
  closing,
  onClose,
  onRemove,
}: {
  worktree: WorktreeInfo;
  kept: readonly KeptReason[];
  busy: IfLost | null;
  error: string | null;
  blocked: boolean;
  snapshotable: boolean;
  closing: number | null;
  onClose: () => void;
  onRemove: (ifLost: IfLost) => void;
}) {
  const branch = worktree.branch ?? worktree.created_branch ?? "this worktree";
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
            disabled={blocked || busy !== null}
            busy={busy === "discard"}
            onClick={() => {
              onRemove("discard");
            }}
          >
            Discard and remove
          </Button>
          <Button
            variant="primary"
            disabled={!snapshotable || busy !== null}
            busy={busy === "snapshot"}
            onClick={() => {
              onRemove("snapshot");
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
      {blocked ? (
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
      {closing === null ? null : (
        // Above the buttons, as Paseo's confirm says the cost before its
        // button. A Banner until the kit's Alert reaches the node line
        // (TRACKER 820 moves it with the rest).
        <Banner tone="warn">{closesTerminals(closing)}</Banner>
      )}
      {error === null ? null : <Banner tone="error">{error}</Banner>}
    </Dialog>
  );
}
