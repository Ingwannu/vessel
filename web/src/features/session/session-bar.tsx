// A session's title bar (agent-orchestrator's session tab, in the frame's
// bar): the agent's mark, the title (renamed in place), its state when it
// is not simply running, and the session's actions. Milestone 9's line of
// protocol words under it is gone; the inspector's Details hold them.

import "./session-bar.css";

import { useAgents } from "../../context";
import { agentMark, agentName } from "../../data/agents";
import { isLive, statusTone, statusWord } from "../../data/format";
import { sessionName } from "../../data/titles";
import { Avatar, Badge } from "../../ui/kit";
import type { SessionInfo } from "../../wire/proto";
import { TitleBar } from "../shell/title-bar";
import { SessionPlaceChip } from "../workspace/branch-chip";
import { SessionActions } from "./session-actions";
import { SessionTitle } from "./session-title";
import type { NodeKey } from "../../wire/bridge";

export function SessionBar({
  node,
  label,
  session,
  renamed,
  onPlace,
}: {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  /** The title its log last reported, fresher than the polled list. */
  renamed: string | null;
  /** Opens the session's changes from its branch's chip. */
  onPlace?: () => void;
}) {
  const live = isLive(session);
  const agents = useAgents(node);
  return (
    <TitleBar>
      <Avatar
        letter={agentMark(session.spec.harness, agents)}
        label={agentName(session.spec.harness, agents)}
      />
      <SessionTitle
        node={node}
        session={session.id}
        name={sessionName(session, renamed)}
        canRename={live}
      />
      <SessionPlaceChip
        node={node}
        session={session}
        {...(onPlace === undefined ? {} : { onOpen: onPlace })}
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
