// Removing a worktree that would lose work (Paseo's archive warning,
// GOAT's snapshot before cleanup): the node first answers `kept` with
// every reason that applies; this dialog says each, then offers to keep a
// snapshot (a commit under the project's refs, restorable as a branch)
// and remove, or to discard, saying first what discarding destroys.
//
// Since protocol 40 the dialog can ask to end the worktree's terminals
// after showing their count (ADR 0121).

import { useCallback, useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  liveTerminalsIn,
  removalState,
  sessionInWorktree,
  speaksWorkspacePlaces,
} from "../../data/workspace/terminal-place";
import { workspaceTerminals } from "../../data/workspace/terminals";
import { type KeptRemoval, removeWorktree } from "../../data/workspace/worktrees";
import {
  closesTerminals,
  removalReasonWords,
  snapshotCovers,
} from "../../data/workspace/worktree-words";
import { Alert, Banner, Button, Dialog } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  DiffStat,
  IfLost,
  KeptReason,
  SessionInfo,
  WorktreeInfo,
} from "../../wire/proto";

const NO_SESSIONS: readonly SessionInfo[] = [];

interface Count {
  /** The live terminals in the worktree; `null` until read, after a failed
   * read, or below 40. */
  count: number | null;
  /** Why the last read failed. */
  problem: string | null;
  /** Read it again now; the fresh count, or `null` when the read failed. */
  recount: () => Promise<number | null>;
}

/** How many live terminals are in the worktree: read when the dialog opens,
 * after each answer, and again on a press that would end them, so the
 * count said is the count ended (as the tab close re-checks its impact). */
function useLiveTerminals(
  node: NodeKey,
  worktree: WorktreeInfo,
  protocol: number | null,
  asked: number,
): Count {
  const fleet = useFleet();
  const [count, setCount] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const speaks = speaksWorkspacePlaces(protocol);
  const read = useCallback(async (): Promise<number | null> => {
    const result = await workspaceTerminals(fleet, node, null);
    const fresh = result.ok
      ? liveTerminalsIn(result.value.terminals, worktree.path)
      : null;
    setCount(fresh);
    setProblem(result.ok ? null : result.problem.message);
    return fresh;
  }, [fleet, node, worktree.path]);
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
        setProblem(result.ok ? null : result.problem.message);
      }
    });
    return () => {
      live = false;
    };
  }, [fleet, node, speaks, worktree.path, asked]);
  return {
    count: speaks ? count : null,
    problem: speaks ? problem : null,
    recount: read,
  };
}

export function RemoveWorktreeDialog({
  node,
  worktree,
  warning,
  onClose,
  onRemoved,
}: {
  node: NodeKey;
  worktree: WorktreeInfo;
  warning: KeptRemoval;
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
  const [current, setCurrent] = useState(warning);
  const kept = current.reasons;
  const [error, setError] = useState<string | null>(null);
  const [asked, setAsked] = useState(0);
  const live = useLiveTerminals(node, worktree, protocol, asked);
  const branch = worktree.branch ?? worktree.created_branch ?? "this worktree";
  const inUse = kept.some((reason) => reason.kind === "in_use");
  const state = removalState({
    protocol,
    inUse,
    liveTerminals: live.count,
    sessionInIt: sessionInWorktree(sessions, worktree),
  });
  // A failed count is said, and holds the buttons as "checking" would.
  const checking = state.checking && live.problem === null;
  const others = kept.filter((reason) => reason.kind !== "in_use");
  const snapshotable = !state.blocked && others.every(snapshotCovers);
  const send = (ifLost: IfLost) => {
    void removeWorktree(
      fleet,
      node,
      worktree.id,
      ifLost,
      state.endTerminals,
    ).then((outcome) => {
      setBusy(null);
      setAsked((count) => count + 1);
      if (!outcome.ok) {
        setError(outcome.problem.message);
        return;
      }
      if (outcome.value.outcome === "kept") {
        setCurrent(outcome.value);
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
  const remove = (ifLost: IfLost) => {
    setBusy(ifLost);
    setError(null);
    if (!state.endTerminals) {
      send(ifLost);
      return;
    }
    // The count said must be the count ended: a terminal opened since
    // the dialog said it stops the press, and the warning says the new
    // count for the person to press again.
    const said = live.count;
    void live.recount().then((fresh) => {
      if (fresh === null || fresh !== said) {
        setBusy(null);
        return;
      }
      send(ifLost);
    });
  };
  return (
    <RemoveWorktreeView
      worktree={worktree}
      kept={kept}
      diffStat={current.diff_stat ?? null}
      busy={busy}
      error={error ?? live.problem}
      blocked={state.blocked}
      checking={checking}
      snapshotable={snapshotable}
      closing={state.endTerminals ? live.count : null}
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
  diffStat = null,
  busy,
  error,
  blocked,
  checking,
  snapshotable,
  closing,
  onClose,
  onRemove,
}: {
  worktree: WorktreeInfo;
  kept: readonly KeptReason[];
  diffStat?: DiffStat | null;
  busy: IfLost | null;
  error: string | null;
  blocked: boolean;
  /** The terminals are still being counted: nothing is said about them
   * or about a session yet. */
  checking: boolean;
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
          <li key={index}>{removalReasonWords(reason, diffStat)}</li>
        ))}
      </ul>
      {checking ? null : blocked ? (
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
        // button; in the dialog's flow, so the kit's Alert (sm warning).
        <Alert
          size="sm"
          variant="warning"
          description={closesTerminals(closing)}
        />
      )}
      {error === null ? null : <Banner tone="error">{error}</Banner>}
    </Dialog>
  );
}
