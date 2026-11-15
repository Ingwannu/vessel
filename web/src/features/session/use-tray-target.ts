// Where a session's attached files go: the session's upload calls, what
// its agent takes and the node's limits, given to the tray once all are
// known and the session runs; nowhere otherwise (the files wait, checked
// once there is somewhere).

import { useEffect } from "react";

import { useFleet } from "../../context";
import { preparedUploads } from "../../data/submission-calls";
import { useStore } from "../../data/store";
import type { Tray } from "../../data/tray";
import type { NodeKey } from "../../wire/bridge";
import type { AgentAbilities, SessionId } from "../../wire/proto";
import type { Limits } from "./use-limits";

export function useTrayTarget(
  tray: Tray,
  node: NodeKey,
  session: SessionId,
  live: boolean,
  abilities: AgentAbilities | null,
  limits: Limits,
  agent: string,
): void {
  const fleet = useFleet();
  const status = useStore(fleet.store, (state) => state.statuses[node]);
  const recipient = status?.state === "connected" ? status.node.id : null;
  const read = limits.kind === "read" ? limits.limits : null;
  useEffect(() => {
    if (!live || read === null || recipient === null) {
      tray.target(null);
      return;
    }
    tray.target({
      node: recipient,
      calls: preparedUploads(fleet, node, recipient, session),
      abilities,
      limits: read,
      agent,
    });
  }, [tray, fleet, node, session, live, abilities, read, agent, recipient]);
}
