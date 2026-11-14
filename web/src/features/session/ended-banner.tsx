// Over an ended session's composer (agent-orchestrator's terminated
// session: "History is kept. Restore the session to continue." with its
// button): what happened, and Start again where the agent can pick up its
// conversation (protocol 10's `session.resume`); where it cannot, it says
// so instead of offering it.

import "./ended-banner.css";

import { canStartAgain, isArchived } from "../../data/marks";
import { Button } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";
import { useMarkActions } from "./use-mark-actions";

export function EndedBanner({
  node,
  session,
  label,
}: {
  node: NodeKey;
  session: SessionInfo;
  /** The machine's name. */
  label: string;
}) {
  const marks = useMarkActions(node, session, label);
  const again = canStartAgain(session);
  return (
    <section className="ended-banner" aria-label="Session ended">
      <div className="ended-text">
        <p className="ended-title">
          {isArchived(session) ? "Session archived" : "Session ended"}
        </p>
        <p className="ended-detail">
          {again
            ? "History is kept. Start the session again to continue."
            : "History is kept. This agent cannot pick up where it left off."}
        </p>
      </div>
      {again ? (
        <Button
          size="sm"
          variant="primary"
          shape="round"
          icon="restore"
          busy={marks.busy === "start-again"}
          onClick={() => {
            marks.run("start-again");
          }}
        >
          Start again
        </Button>
      ) : null}
    </section>
  );
}
