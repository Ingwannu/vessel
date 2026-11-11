// A session's title bar (agent-orchestrator's session tab, in the frame's
// bar): the agent's mark, the title (renamed in place), its state when it
// is not simply running, and the session's actions. Milestone 9's line of
// protocol words under it is gone; the inspector's Details hold them.

import "./session-bar.css";

import { agentMark, agentName } from "../../data/agents";
import { isLive, statusTone, statusWord } from "../../data/format";
import { sessionName } from "../../data/titles";
import { Avatar, Badge } from "../../ui/kit";
import type { SessionInfo } from "../../wire/proto";
import { TitleBar } from "../shell/title-bar";
import { SessionActions } from "./session-actions";
import { SessionTitle } from "./session-title";

export function SessionBar({
  node,
  label,
  session,
  renamed,
}: {
  node: string;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  /** The title its log last reported, fresher than the polled list. */
  renamed: string | null;
}) {
  const live = isLive(session);
  return (
    <TitleBar>
      <Avatar
        letter={agentMark(session.spec.harness)}
        label={agentName(session.spec.harness)}
      />
      <SessionTitle
        node={node}
        session={session.id}
        name={sessionName(session, renamed)}
        canRename={live}
      />
      {session.status.state === "running" ? null : (
        <Badge tone={statusTone(session.status)}>
          {statusWord(session.status)}
        </Badge>
      )}
      <span className="session-bar-end">
        <SessionActions node={node} label={label} session={session} />
      </span>
    </TitleBar>
  );
}
