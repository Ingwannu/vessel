// A session's mark actions, as one hook for every place that offers them
// (the title bar's menu, the ended session's banner, the archive):
// pin, read or unread, start again, archive (agent-orchestrator's
// confirmation first, which says when it stops the agent), unarchive, and
// delete (its confirmation, then what became of the worktree). Each
// answer is put into the machine's list at once.

import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";

import { useFleet } from "../../context";
import {
  deleteSession,
  markSession,
  resumeSession,
} from "../../data/conversation-calls";
import { isLive } from "../../data/format";
import { deletedLine, type MarkAction } from "../../data/marks";
import { errorCode, type Problem } from "../../data/outcome";
import { holdRead } from "../../data/read-hold";
import { trayOf } from "../../data/trays";
import { Button, Dialog } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";

/** A start again's refusal, in the page's words. */
function againProblem(problem: Problem): string {
  const code = errorCode(problem);
  if (code === "unsupported") {
    return "This agent cannot pick up where it left off.";
  }
  if (code === "harness_not_installed") {
    return "The agent is no longer installed on this machine.";
  }
  if (code === "unavailable") {
    return "The machine is stopping; try again once it is back.";
  }
  if (code === "invalid_request") {
    return `Its profile is gone: ${problem.message}`;
  }
  // `conflict` (already live, another start under way, its log still
  // being written, its folder gone) and `spawn_failed` carry the node's
  // own words, which say which.
  return problem.message;
}

export interface MarkActions {
  run: (action: MarkAction) => void;
  /** The action on its way, if one is. */
  busy: MarkAction | null;
  /** A confirmation, when one is open. */
  dialog: ReactNode;
}

export function useMarkActions(
  node: NodeKey,
  session: SessionInfo,
  /** The machine's name. */
  label: string,
): MarkActions {
  const fleet = useFleet();
  const navigate = useNavigate();
  const [busy, setBusy] = useState<MarkAction | null>(null);
  const [asking, setAsking] = useState<"archive" | "delete" | null>(null);
  const done = (action: MarkAction, value: SessionInfo | null) => {
    setBusy(null);
    if (value !== null) {
      fleet.putSession(node, value);
    }
    if (action === "mark-unread") {
      holdRead(session.id);
    }
  };
  const mark = (action: MarkAction) => {
    const change =
      action === "pin"
        ? { mark: "pin" as const }
        : action === "unpin"
          ? { mark: "unpin" as const }
          : action === "archive"
            ? { mark: "archive" as const }
            : action === "unarchive"
              ? { mark: "unarchive" as const }
              : action === "mark-unread"
                ? { mark: "unread" as const }
                : { mark: "read" as const, through: session.next_seq };
    setBusy(action);
    void markSession(fleet, node, session.id, change).then((outcome) => {
      done(action, outcome.ok ? outcome.value : null);
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const again = () => {
    setBusy("start-again");
    void resumeSession(fleet, node, session.id).then((outcome) => {
      done("start-again", outcome.ok ? outcome.value : null);
      if (!outcome.ok) {
        toast(againProblem(outcome.problem), "error");
        void fleet.refreshSessions(node);
      }
    });
  };
  const remove = () => {
    setAsking(null);
    setBusy("delete");
    void deleteSession(fleet, node, session.id).then((outcome) => {
      setBusy(null);
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
        return;
      }
      fleet.dropSession(node, session.id);
      fleet.drafts.clear({ kind: "session", session: session.id });
      trayOf(session.id).dispose();
      const worktree = deletedLine(outcome.value);
      toast(
        worktree === null ? "Session deleted" : `Session deleted. ${worktree}`,
      );
      void navigate({ to: "/sessions" });
    });
  };
  const close = () => {
    setAsking(null);
  };
  const dialog =
    asking === "archive" ? (
      <Dialog
        title="Are you sure you want to archive this session?"
        onClose={close}
        footer={
          <>
            <Button variant="ghost" onClick={close}>
              No
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                setAsking(null);
                mark("archive");
              }}
            >
              Confirm
            </Button>
          </>
        }
      >
        <p className="hint">
          {isLive(session)
            ? `Archiving stops the agent on ${label} first. `
            : ""}
          You can always restore it from the Archive section later.
        </p>
      </Dialog>
    ) : asking === "delete" ? (
      <Dialog
        title="Delete this session permanently?"
        onClose={close}
        footer={
          <>
            <Button variant="ghost" onClick={close}>
              Cancel
            </Button>
            <Button variant="danger-solid" onClick={remove}>
              Delete
            </Button>
          </>
        }
      >
        <p className="hint">
          Its conversation and attachments are deleted for good. Its worktree is
          removed only if nothing in it would be lost; the agent&apos;s own
          history on {label} is kept.
        </p>
      </Dialog>
    ) : null;
  return {
    busy,
    dialog,
    run: (action) => {
      switch (action) {
        case "archive":
        case "delete":
          setAsking(action);
          return;
        case "start-again":
          again();
          return;
        case "pin":
        case "unpin":
        case "mark-unread":
        case "mark-read":
        case "unarchive":
          mark(action);
          return;
      }
    },
  };
}
