// The lines under the new-session page's box: what a worktree's start
// shows (`useStarter`), why the last Send failed, why Send is off (with
// "Ask again" when the machine would not start the agent), and an agent
// that did not say what it offers.

import "./new-session.css";

import type { ReactNode } from "react";

import type { ControlsState } from "../../data/options";
import { Button } from "../../ui/kit";

export function NewSessionNotes({
  starting,
  error,
  blocked,
  controls,
  agent,
  onAskAgain,
}: {
  /** The worktree's start, while there is one. */
  starting: ReactNode;
  /** Why the last Send failed. */
  error: string | null;
  /** Why Send is off, when there is something to say. */
  blocked: string | null;
  controls: ControlsState;
  /** The agent's name. */
  agent: string;
  /** Ask the machine again, where it would not start the agent. */
  onAskAgain: (() => void) | null;
}) {
  return (
    <div className="new-session-notes">
      {starting}
      {error === null ? null : (
        <p className="new-session-error" role="alert">
          {error}
        </p>
      )}
      {blocked === null ? null : (
        <p className="new-session-note">
          {blocked}
          {onAskAgain === null ? null : (
            <>
              {" "}
              <Button size="xs" variant="ghost" onClick={onAskAgain}>
                Ask again
              </Button>
            </>
          )}
        </p>
      )}
      {controls.kind === "failed" ? (
        <p className="new-session-note" role="status">
          {agent} did not say what it offers: {controls.message} It starts on
          its own defaults.
        </p>
      ) : null}
    </div>
  );
}
